//! Thư viện tài liệu: bọc [`tsuscan_vault::Vault`] cho Dart.
//!
//! `Vault` giữ `rusqlite::Connection` (Send nhưng không Sync) ⇒ bọc trong `Mutex`: flutter_rust_bridge chạy hàm trên
//! nhóm luồng, mỗi lúc chỉ một luồng chạm kết nối. Khóa CSDL đi qua cầu dưới dạng byte, bị xóa khỏi bộ nhớ ngay sau
//! khi mở kho - Dart không giữ khóa lâu hơn một lần gọi (phiếu 20261119-05 mục 2).

use std::io::Cursor;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use flutter_rust_bridge::frb;
use image::{DynamicImage, ImageFormat, imageops::FilterType};
use tsuscan_vault::store::{DB_FILE, DocumentFilter, NewPage};
use tsuscan_vault::{Key32, Vault};
use zeroize::Zeroizing;

use super::error::{ApiError, ApiErrorKind, ApiResult};

/// Chất lượng JPEG khi nhập ảnh (đủ cho OCR, ~1/3 dung lượng PNG).
const IMPORT_JPEG_QUALITY: u8 = 88;
/// Cạnh dài tối đa của ảnh thu nhỏ.
const MAX_THUMB_SIDE: u32 = 1024;
/// Độ dài tối đa tên tài liệu/thư mục/nhãn (ký tự).
pub const MAX_NAME_CHARS: usize = 200;
/// Mốc lớn nhất của bộ chọn số bản ghi (`DATA_TABLE.md` mục 2).
pub const MAX_PAGE_SIZE: u32 = 200;

/// Tài liệu (DTO).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentDto {
    /// Id.
    pub id: String,
    /// Tiêu đề.
    pub title: String,
    /// Thư mục chứa (`None` = gốc).
    pub folder_id: Option<String>,
    /// Lúc tạo (ms Unix).
    pub created_ms: i64,
    /// Lúc sửa (ms Unix).
    pub updated_ms: i64,
    /// Số trang.
    pub page_count: i64,
}

/// Trang (DTO, không gồm ảnh - lấy riêng bằng [`LibraryHandle::page_thumbnail`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageDto {
    /// Id.
    pub id: String,
    /// Thứ tự (0-based).
    pub position: i64,
    /// Rộng (px).
    pub width: u32,
    /// Cao (px).
    pub height: u32,
    /// Chữ OCR.
    pub ocr_text: String,
}

/// Thư mục (DTO).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderDto {
    /// Id.
    pub id: String,
    /// Thư mục cha.
    pub parent_id: Option<String>,
    /// Tên.
    pub name: String,
}

/// Nhãn (DTO).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagDto {
    /// Id.
    pub id: String,
    /// Tên.
    pub name: String,
}

/// Kết quả tìm kiếm (DTO).
#[derive(Debug, Clone, PartialEq)]
pub struct SearchHitDto {
    /// Id tài liệu.
    pub document_id: String,
    /// Tiêu đề.
    pub title: String,
    /// Điểm liên quan.
    pub score: f64,
    /// Khớp đúng cả dấu.
    pub accent_exact: bool,
}

/// Một trang danh sách tài liệu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentPage {
    /// Bản ghi của trang.
    pub items: Vec<DocumentDto>,
    /// Tổng số bản ghi khớp bộ lọc.
    pub total: u64,
}

/// Sinh khóa CSDL 32 byte ngẫu nhiên (app cất vào kho bí mật của thiết bị).
///
/// # Errors
/// `Internal` khi hệ điều hành không cấp số ngẫu nhiên.
pub fn generate_db_key() -> ApiResult<Vec<u8>> {
    Ok(Key32::random()?.as_bytes().to_vec())
}

/// Thư mục `root` đã có kho chưa.
#[frb(sync)]
#[must_use]
pub fn library_exists(root: String) -> bool {
    PathBuf::from(root).join(DB_FILE).is_file()
}

fn key_from(bytes: Vec<u8>) -> ApiResult<Key32> {
    let bytes = Zeroizing::new(bytes);
    Key32::from_slice(&bytes).map_err(|_| ApiError::new(ApiErrorKind::InvalidInput, "khóa CSDL phải 32 byte"))
}

fn clean_name(name: &str) -> ApiResult<&str> {
    let t = name.trim();
    if t.is_empty() || t.chars().count() > MAX_NAME_CHARS || t.chars().any(char::is_control) {
        return Err(ApiError::new(ApiErrorKind::InvalidInput, "tên không hợp lệ"));
    }
    Ok(t)
}

/// Kho đã mở khóa. Dart giữ đối tượng này (opaque); hủy ⇒ khóa chủ tự xóa khỏi bộ nhớ.
#[frb(opaque)]
pub struct LibraryHandle {
    inner: Mutex<Vault>,
}

impl LibraryHandle {
    fn wrap(v: Vault) -> Self {
        Self { inner: Mutex::new(v) }
    }

    fn lock(&self) -> MutexGuard<'_, Vault> {
        // Luồng khác panic giữa chừng: giao dịch SQLite đã rollback, dữ liệu vẫn nhất quán ⇒ dùng tiếp.
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Tạo kho mới (lần chạy đầu). Không bao giờ ghi đè kho có sẵn.
    ///
    /// # Errors
    /// `AlreadyExists`, `InvalidInput` (khóa sai độ dài), `Storage`.
    pub fn create(root: String, db_key: Vec<u8>) -> ApiResult<LibraryHandle> {
        let key = key_from(db_key)?;
        Ok(Self::wrap(Vault::create(&PathBuf::from(root), &key)?))
    }

    /// Mở kho có sẵn.
    ///
    /// # Errors
    /// `WrongKey`, `NotFound`, `Storage`.
    pub fn open(root: String, db_key: Vec<u8>) -> ApiResult<LibraryHandle> {
        let key = key_from(db_key)?;
        Ok(Self::wrap(Vault::open(&PathBuf::from(root), &key)?))
    }

    // ------------------------------------------------------------ thư mục

    /// Danh sách thư mục.
    ///
    /// # Errors
    /// `Storage`.
    pub fn list_folders(&self) -> ApiResult<Vec<FolderDto>> {
        Ok(self
            .lock()
            .list_folders()?
            .into_iter()
            .map(|f| FolderDto {
                id: f.id,
                parent_id: f.parent_id,
                name: f.name,
            })
            .collect())
    }

    /// Tạo thư mục, trả id.
    ///
    /// # Errors
    /// `InvalidInput`, `NotFound` (thư mục cha), `Storage`.
    pub fn create_folder(&self, name: String, parent_id: Option<String>) -> ApiResult<String> {
        let name = clean_name(&name)?;
        Ok(self.lock().create_folder(name, parent_id.as_deref())?)
    }

    /// Đổi tên thư mục.
    ///
    /// # Errors
    /// `InvalidInput`, `NotFound`, `Storage`.
    pub fn rename_folder(&self, id: String, name: String) -> ApiResult<()> {
        let name = clean_name(&name)?;
        Ok(self.lock().rename_folder(&id, name)?)
    }

    /// Xóa thư mục (tài liệu bên trong chuyển ra thư mục cha theo quy tắc của kho).
    ///
    /// # Errors
    /// `NotFound`, `Storage`.
    pub fn delete_folder(&self, id: String) -> ApiResult<()> {
        Ok(self.lock().delete_folder(&id)?)
    }

    // ------------------------------------------------------------ nhãn

    /// Danh sách nhãn.
    ///
    /// # Errors
    /// `Storage`.
    pub fn list_tags(&self) -> ApiResult<Vec<TagDto>> {
        Ok(self
            .lock()
            .list_tags()?
            .into_iter()
            .map(|t| TagDto {
                id: t.id,
                name: t.name,
            })
            .collect())
    }

    /// Tạo nhãn (trùng tên ⇒ trả id nhãn có sẵn theo quy tắc của kho).
    ///
    /// # Errors
    /// `InvalidInput`, `Storage`.
    pub fn create_tag(&self, name: String) -> ApiResult<String> {
        let name = clean_name(&name)?;
        Ok(self.lock().create_tag(name)?)
    }

    /// Xóa nhãn.
    ///
    /// # Errors
    /// `NotFound`, `Storage`.
    pub fn delete_tag(&self, id: String) -> ApiResult<()> {
        Ok(self.lock().delete_tag(&id)?)
    }

    /// Gắn/gỡ nhãn khỏi tài liệu.
    ///
    /// # Errors
    /// `NotFound`, `Storage`.
    pub fn set_tag(&self, document_id: String, tag_id: String, on: bool) -> ApiResult<()> {
        Ok(self.lock().set_tag(&document_id, &tag_id, on)?)
    }

    /// Nhãn của một tài liệu.
    ///
    /// # Errors
    /// `Storage`.
    pub fn document_tags(&self, document_id: String) -> ApiResult<Vec<TagDto>> {
        Ok(self
            .lock()
            .document_tags(&document_id)?
            .into_iter()
            .map(|t| TagDto {
                id: t.id,
                name: t.name,
            })
            .collect())
    }

    // ------------------------------------------------------------ tài liệu

    /// Một trang danh sách tài liệu (mới sửa trước) kèm tổng số - phân trang tại CSDL (`DATA_TABLE.md` mục 4).
    ///
    /// # Errors
    /// `Storage`.
    pub fn list_documents(
        &self,
        folder_id: Option<String>,
        root_only: bool,
        tag_id: Option<String>,
        limit: u32,
        offset: u64,
    ) -> ApiResult<DocumentPage> {
        // Phạm vi thư mục: `folder_id` có giá trị ⇒ thư mục đó; không có + `root_only` ⇒ gốc; còn lại ⇒ mọi thư mục.
        let folder = match (&folder_id, root_only) {
            (Some(id), _) => Some(Some(id.as_str())),
            (None, true) => Some(None),
            (None, false) => None,
        };
        let filter = DocumentFilter {
            folder,
            tag: tag_id.as_deref(),
        };
        let vault = self.lock();
        let total = vault.count_documents(filter)?;
        let items = vault
            .list_documents_page(filter, limit.clamp(1, MAX_PAGE_SIZE), offset)?
            .into_iter()
            .map(doc_dto)
            .collect();
        Ok(DocumentPage { items, total })
    }

    /// Một tài liệu.
    ///
    /// # Errors
    /// `NotFound`, `Storage`.
    pub fn document(&self, id: String) -> ApiResult<DocumentDto> {
        Ok(doc_dto(self.lock().document(&id)?))
    }

    /// Tạo tài liệu rỗng, trả id.
    ///
    /// # Errors
    /// `InvalidInput`, `NotFound` (thư mục), `Storage`.
    pub fn create_document(&self, title: String, folder_id: Option<String>) -> ApiResult<String> {
        let title = clean_name(&title)?;
        Ok(self.lock().create_document(title, folder_id.as_deref())?)
    }

    /// Đổi tên tài liệu.
    ///
    /// # Errors
    /// `InvalidInput`, `NotFound`, `Storage`.
    pub fn rename_document(&self, id: String, title: String) -> ApiResult<()> {
        let title = clean_name(&title)?;
        Ok(self.lock().update_document(&id, Some(title), None)?)
    }

    /// Chuyển tài liệu sang thư mục (`None` = gốc).
    ///
    /// # Errors
    /// `NotFound`, `Storage`.
    pub fn move_document(&self, id: String, folder_id: Option<String>) -> ApiResult<()> {
        Ok(self
            .lock()
            .update_document(&id, None, Some(folder_id.as_deref()))?)
    }

    /// Xóa hẳn tài liệu (tệp trang, chữ OCR, chỉ mục).
    ///
    /// # Errors
    /// `NotFound`, `Storage`.
    pub fn delete_document(&self, id: String) -> ApiResult<()> {
        Ok(self.lock().delete_document(&id)?)
    }

    // ------------------------------------------------------------ trang

    /// Trang của tài liệu theo thứ tự.
    ///
    /// # Errors
    /// `Storage`.
    pub fn list_pages(&self, document_id: String) -> ApiResult<Vec<PageDto>> {
        Ok(self
            .lock()
            .list_pages(&document_id)?
            .into_iter()
            .map(|p| PageDto {
                id: p.id,
                position: p.position,
                width: p.width,
                height: p.height,
                ocr_text: p.ocr_text,
            })
            .collect())
    }

    /// Nhập một ảnh (JPEG/PNG/WebP) làm trang cuối: giải mã có giới hạn, xoay theo EXIF rồi mã hóa lại JPEG - siêu
    /// dữ liệu gốc (vị trí GPS, máy chụp) bị bỏ. Trả id trang.
    ///
    /// # Errors
    /// `UnsupportedImage`, `ImageTooLarge`, `NotFound`, `Storage`.
    pub fn import_image_page(&self, document_id: String, image: Vec<u8>) -> ApiResult<String> {
        let img = tsuscan_imaging::decode(&image)?;
        drop(image);
        let (w, h) = (img.width(), img.height());
        let jpeg = encode_jpeg(&img, IMPORT_JPEG_QUALITY)?;
        drop(img);
        let page = NewPage {
            image: &jpeg,
            width: w,
            height: h,
            dpi: 0,
            ocr_text: "",
            ocr_blob: None,
        };
        Ok(self.lock().add_page(&document_id, page)?)
    }

    /// Ảnh thu nhỏ JPEG của trang, cạnh dài ≤ `max_side` (kẹp 64..=1024). Giải mã trong Rust, Dart chỉ nhận ảnh nhỏ.
    ///
    /// # Errors
    /// `NotFound`, `UnsupportedImage`, `Storage`.
    pub fn page_thumbnail(&self, page_id: String, max_side: u32) -> ApiResult<Vec<u8>> {
        let bytes = self.lock().page_image(&page_id)?;
        let img = tsuscan_imaging::decode(&bytes)?;
        drop(bytes);
        let side = max_side.clamp(64, MAX_THUMB_SIDE);
        let thumb = if img.width().max(img.height()) > side {
            img.resize(side, side, FilterType::Triangle)
        } else {
            img
        };
        encode_jpeg(&thumb, 80)
    }

    /// Ảnh trang đầy đủ (byte đã giải mã khỏi tệp mã hóa) - để xem phóng to/xuất.
    ///
    /// # Errors
    /// `NotFound`, `Storage`.
    pub fn page_image(&self, page_id: String) -> ApiResult<Vec<u8>> {
        Ok(self.lock().page_image(&page_id)?.to_vec())
    }

    /// Xóa hẳn một trang.
    ///
    /// # Errors
    /// `NotFound`, `Storage`.
    pub fn delete_page(&self, page_id: String) -> ApiResult<()> {
        Ok(self.lock().delete_page(&page_id)?)
    }

    // ------------------------------------------------------------ tìm kiếm

    /// Tìm không dấu trên tiêu đề + chữ OCR (truy vấn < 2 ký tự ⇒ rỗng).
    ///
    /// # Errors
    /// `Storage`.
    pub fn search(&self, query: String, limit: u32) -> ApiResult<Vec<SearchHitDto>> {
        let limit = limit.clamp(1, 500) as usize;
        Ok(self
            .lock()
            .search(&query, limit)?
            .into_iter()
            .map(|h| SearchHitDto {
                document_id: h.document_id,
                title: h.title,
                score: h.score,
                accent_exact: h.accent_exact,
            })
            .collect())
    }
}

fn doc_dto(d: tsuscan_vault::store::DocumentInfo) -> DocumentDto {
    DocumentDto {
        id: d.id,
        title: d.title,
        folder_id: d.folder_id,
        created_ms: d.created_ms,
        updated_ms: d.updated_ms,
        page_count: d.page_count,
    }
}

fn encode_jpeg(img: &DynamicImage, quality: u8) -> ApiResult<Vec<u8>> {
    let rgb = img.to_rgb8();
    let mut out = Cursor::new(Vec::new());
    let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality);
    rgb.write_with_encoder(enc)
        .map_err(|_| ApiError::new(ApiErrorKind::Internal, "mã hóa JPEG"))?;
    let bytes = out.into_inner();
    debug_assert_eq!(image::guess_format(&bytes).ok(), Some(ImageFormat::Jpeg));
    Ok(bytes)
}

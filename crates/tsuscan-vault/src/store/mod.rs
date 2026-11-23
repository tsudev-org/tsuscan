//! Kho trên máy: CSDL SQLite3MC + thư mục tệp trang mã hóa luồng (ADR-0009 mục 3.1).
//!
//! ```text
//! <gốc>/vault.db             CSDL mã hóa toàn bộ (ChaCha20-Poly1305), khóa CSDL do app giữ trong kho bí mật
//! <gốc>/files/<2 ký tự>/<id>.tsf  ảnh trang, mỗi tệp một FK, FK bọc bằng DK của tài liệu, DK bọc bằng MK
//! ```
//!
//! MK nằm trong bảng `meta` của CSDL đã mã hóa (dùng ngoại tuyến không cần tài khoản); khi bật đồng bộ, MK được bọc
//! thêm bằng mật khẩu + 24 từ ([`crate::keys::KeyBackup`]).

pub mod schema;
mod search;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use zeroize::Zeroizing;

pub use search::SearchHit;

use crate::error::{Result, VaultError};
use crate::hlc::{Clock, Hlc, wall_now_ms};
use crate::key::{Key32, fill_random};
use crate::keys::{self, context};
use crate::stream;

/// Tên tệp CSDL trong thư mục gốc của kho.
pub const DB_FILE: &str = "vault.db";
/// Thư mục tệp trang.
pub const FILES_DIR: &str = "files";
/// Giới hạn kích thước một ảnh trang đưa vào kho (200 MiB) - chặn lỗi gọi nhầm làm đầy bộ nhớ.
pub const MAX_FILE_BYTES: u64 = 200 * 1024 * 1024;

/// Thông tin một tài liệu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentInfo {
    /// Id.
    pub id: String,
    /// Tiêu đề.
    pub title: String,
    /// Thư mục chứa (`None` = gốc).
    pub folder_id: Option<String>,
    /// Lúc tạo (ms Unix).
    pub created_ms: i64,
    /// Lúc sửa gần nhất (ms Unix).
    pub updated_ms: i64,
    /// Số trang chưa xóa.
    pub page_count: i64,
}

/// Thông tin một trang.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageInfo {
    /// Id.
    pub id: String,
    /// Thứ tự trong tài liệu (0-based, liên tục).
    pub position: i64,
    /// Rộng (px).
    pub width: u32,
    /// Cao (px).
    pub height: u32,
    /// DPI.
    pub dpi: u32,
    /// Chữ OCR thuần.
    pub ocr_text: String,
}

/// Trang mới đưa vào kho.
#[derive(Debug, Clone, Copy)]
pub struct NewPage<'a> {
    /// Byte ảnh trang (đã nén - JPEG/PNG/G4 - do tầng tài liệu quyết định).
    pub image: &'a [u8],
    /// Rộng (px).
    pub width: u32,
    /// Cao (px).
    pub height: u32,
    /// DPI.
    pub dpi: u32,
    /// Chữ OCR thuần (lập chỉ mục tìm kiếm).
    pub ocr_text: &'a str,
    /// Kết quả OCR đầy đủ đã tuần tự hóa (hộp chữ, độ tin cậy) - kho không đọc nội dung.
    pub ocr_blob: Option<&'a [u8]>,
}

/// Thư mục.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folder {
    /// Id.
    pub id: String,
    /// Thư mục cha.
    pub parent_id: Option<String>,
    /// Tên.
    pub name: String,
}

/// Nhãn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    /// Id.
    pub id: String,
    /// Tên.
    pub name: String,
}

/// Tài liệu trong thùng rác.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrashedDocument {
    /// Thông tin tài liệu.
    pub info: DocumentInfo,
    /// Lúc bỏ vào thùng rác (ms Unix).
    pub trashed_ms: i64,
}

/// Số ngày tài liệu nằm trong thùng rác trước khi bị xóa hẳn.
pub const TRASH_RETENTION_DAYS: i64 = 30;
/// [`TRASH_RETENTION_DAYS`] tính bằng ms.
pub const TRASH_RETENTION_MS: i64 = TRASH_RETENTION_DAYS * 24 * 60 * 60 * 1000;

/// Bộ lọc danh sách tài liệu (AND giữa các trường).
#[derive(Debug, Clone, Copy, Default)]
pub struct DocumentFilter<'a> {
    /// Chỉ tài liệu trong thư mục này (`Some(None)` = gốc, `None` = mọi thư mục).
    pub folder: Option<Option<&'a str>>,
    /// Chỉ tài liệu có nhãn này.
    pub tag: Option<&'a str>,
}

/// Kho đã mở khóa. Hủy đối tượng ⇒ MK tự xóa khỏi bộ nhớ, kết nối đóng.
pub struct Vault {
    conn: Connection,
    root: PathBuf,
    mk: Key32,
    clock: Clock,
}

impl std::fmt::Debug for Vault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vault")
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

impl Vault {
    /// Tạo kho mới trong `root` với khóa CSDL `db_key`. Không bao giờ ghi đè kho có sẵn.
    ///
    /// # Errors
    /// [`VaultError::Format`] khi đã có kho; lỗi vào/ra, SQLite.
    pub fn create(root: &Path, db_key: &Key32) -> Result<Self> {
        let db = root.join(DB_FILE);
        if db.exists() {
            return Err(VaultError::Format("thư mục đã có kho"));
        }
        fs::create_dir_all(root.join(FILES_DIR))?;
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let mut conn = Connection::open_with_flags(&db, flags)?;
        apply_key(&conn, db_key)?;
        schema::migrate(&mut conn)?;
        let mk = Key32::random()?;
        let mut node = [0u8; 8];
        fill_random(&mut node)?;
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO meta(k, v) VALUES ('mk', ?1)",
            [mk.as_bytes().as_slice()],
        )?;
        tx.execute("INSERT INTO meta(k, v) VALUES ('node', ?1)", [node.as_slice()])?;
        tx.commit()?;
        Ok(Self {
            conn,
            root: root.to_owned(),
            mk,
            clock: Clock::new(u64::from_le_bytes(node), None),
        })
    }

    /// Mở kho có sẵn. Sai khóa ⇒ [`VaultError::WrongDbKey`], tệp không bị đụng tới.
    ///
    /// # Errors
    /// [`VaultError::NotFound`] khi chưa có kho; [`VaultError::WrongDbKey`]; lỗi SQLite.
    pub fn open(root: &Path, db_key: &Key32) -> Result<Self> {
        let db = root.join(DB_FILE);
        if !db.is_file() {
            return Err(VaultError::NotFound("kho"));
        }
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let mut conn = Connection::open_with_flags(&db, flags)?;
        apply_key(&conn, db_key)?;
        schema::migrate(&mut conn)?;
        let mk_bytes: Zeroizing<Vec<u8>> =
            Zeroizing::new(conn.query_row("SELECT v FROM meta WHERE k = 'mk'", [], |r| r.get(0))?);
        let mk = Key32::from_slice(&mk_bytes)?;
        let node: Vec<u8> = conn.query_row("SELECT v FROM meta WHERE k = 'node'", [], |r| r.get(0))?;
        let node: [u8; 8] = node.try_into().map_err(|_| VaultError::Format("id thiết bị"))?;
        let last: Option<Vec<u8>> = conn
            .query_row("SELECT v FROM meta WHERE k = 'hlc'", [], |r| r.get(0))
            .optional()?;
        let last = last
            .and_then(|v| String::from_utf8(v).ok())
            .and_then(|s| Hlc::decode(&s).ok());
        fs::create_dir_all(root.join(FILES_DIR))?;
        Ok(Self {
            conn,
            root: root.to_owned(),
            mk,
            clock: Clock::new(u64::from_le_bytes(node), last),
        })
    }

    /// Khóa chủ (để tạo [`crate::keys::KeyBackup`] khi bật đồng bộ).
    #[must_use]
    pub fn master_key(&self) -> &Key32 {
        &self.mk
    }

    /// Đổi khóa CSDL (xoay khóa trong kho bí mật của thiết bị). Mã hóa lại toàn bộ tệp CSDL.
    ///
    /// # Errors
    /// Lỗi SQLite.
    pub fn rekey(&self, new_key: &Key32) -> Result<()> {
        // `PRAGMA rekey` của SQLite3MC không chạy được ở chế độ WAL.
        self.conn
            .query_row("PRAGMA journal_mode = DELETE", [], |_| Ok(()))?;
        self.conn
            .pragma_update(None, "hexrekey", new_key.to_hex().as_str())?;
        self.conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
        Ok(())
    }

    fn tick(&mut self) -> Result<String> {
        let h = self.clock.tick(wall_now_ms())?.encode();
        self.conn.execute(
            "INSERT OR REPLACE INTO meta(k, v) VALUES ('hlc', ?1)",
            [h.as_bytes()],
        )?;
        Ok(h)
    }

    // ---------------------------------------------------------------- thư mục

    /// Tạo thư mục.
    ///
    /// # Errors
    /// [`VaultError::NotFound`] khi thư mục cha không tồn tại; lỗi SQLite.
    pub fn create_folder(&mut self, name: &str, parent_id: Option<&str>) -> Result<String> {
        if let Some(p) = parent_id {
            self.require("folders", p, "thư mục")?;
        }
        let id = new_id()?;
        let hlc = self.tick()?;
        self.conn.execute(
            "INSERT INTO folders(id, parent_id, name, hlc) VALUES (?1, ?2, ?3, ?4)",
            params![id, parent_id, name.trim(), hlc],
        )?;
        Ok(id)
    }

    /// Đổi tên thư mục.
    ///
    /// # Errors
    /// [`VaultError::NotFound`]; lỗi SQLite.
    pub fn rename_folder(&mut self, id: &str, name: &str) -> Result<()> {
        self.require("folders", id, "thư mục")?;
        let hlc = self.tick()?;
        self.conn.execute(
            "UPDATE folders SET name = ?2, hlc = ?3 WHERE id = ?1",
            params![id, name.trim(), hlc],
        )?;
        Ok(())
    }

    /// Xóa thư mục: tài liệu và thư mục con chuyển lên thư mục cha (không xóa tài liệu).
    ///
    /// # Errors
    /// [`VaultError::NotFound`]; lỗi SQLite.
    pub fn delete_folder(&mut self, id: &str) -> Result<()> {
        self.require("folders", id, "thư mục")?;
        let hlc = self.tick()?;
        let tx = self.conn.transaction()?;
        let parent: Option<String> =
            tx.query_row("SELECT parent_id FROM folders WHERE id = ?1", [id], |r| r.get(0))?;
        tx.execute(
            "UPDATE documents SET folder_id = ?2, hlc = ?3 WHERE folder_id = ?1",
            params![id, parent, hlc],
        )?;
        tx.execute(
            "UPDATE folders SET parent_id = ?2, hlc = ?3 WHERE parent_id = ?1",
            params![id, parent, hlc],
        )?;
        tx.execute(
            "UPDATE folders SET deleted = 1, parent_id = NULL, hlc = ?2 WHERE id = ?1",
            params![id, hlc],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Mọi thư mục chưa xóa, sắp theo tên.
    ///
    /// # Errors
    /// Lỗi SQLite.
    pub fn list_folders(&self) -> Result<Vec<Folder>> {
        let mut st = self
            .conn
            .prepare("SELECT id, parent_id, name FROM folders WHERE deleted = 0 ORDER BY name, id")?;
        let rows = st.query_map([], |r| {
            Ok(Folder {
                id: r.get(0)?,
                parent_id: r.get(1)?,
                name: r.get(2)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    // ---------------------------------------------------------------- nhãn

    /// Tạo nhãn (trùng tên không phân biệt hoa thường/dấu ⇒ trả nhãn có sẵn).
    ///
    /// # Errors
    /// Lỗi SQLite.
    pub fn create_tag(&mut self, name: &str) -> Result<String> {
        let name = name.trim();
        let key = crate::normalize::vi_fold_case(name);
        for t in self.list_tags()? {
            if crate::normalize::vi_fold_case(&t.name) == key {
                return Ok(t.id);
            }
        }
        let id = new_id()?;
        let hlc = self.tick()?;
        self.conn.execute(
            "INSERT INTO tags(id, name, hlc) VALUES (?1, ?2, ?3)",
            params![id, name, hlc],
        )?;
        Ok(id)
    }

    /// Mọi nhãn chưa xóa, sắp theo tên.
    ///
    /// # Errors
    /// Lỗi SQLite.
    pub fn list_tags(&self) -> Result<Vec<Tag>> {
        let mut st = self
            .conn
            .prepare("SELECT id, name FROM tags WHERE deleted = 0 ORDER BY name, id")?;
        let rows = st.query_map([], |r| {
            Ok(Tag {
                id: r.get(0)?,
                name: r.get(1)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Xóa nhãn (gỡ khỏi mọi tài liệu).
    ///
    /// # Errors
    /// [`VaultError::NotFound`]; lỗi SQLite.
    pub fn delete_tag(&mut self, id: &str) -> Result<()> {
        self.require("tags", id, "nhãn")?;
        let hlc = self.tick()?;
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE document_tags SET deleted = 1, hlc = ?2 WHERE tag_id = ?1",
            params![id, hlc],
        )?;
        tx.execute(
            "UPDATE tags SET deleted = 1, hlc = ?2 WHERE id = ?1",
            params![id, hlc],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Gắn/gỡ nhãn cho tài liệu.
    ///
    /// # Errors
    /// [`VaultError::NotFound`]; lỗi SQLite.
    pub fn set_tag(&mut self, document_id: &str, tag_id: &str, on: bool) -> Result<()> {
        self.require("documents", document_id, "tài liệu")?;
        self.require("tags", tag_id, "nhãn")?;
        let hlc = self.tick()?;
        self.conn.execute(
            "INSERT INTO document_tags(document_id, tag_id, hlc, deleted) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(document_id, tag_id) DO UPDATE SET hlc = excluded.hlc, deleted = excluded.deleted",
            params![document_id, tag_id, hlc, i64::from(!on)],
        )?;
        Ok(())
    }

    /// Nhãn của một tài liệu.
    ///
    /// # Errors
    /// Lỗi SQLite.
    pub fn document_tags(&self, document_id: &str) -> Result<Vec<Tag>> {
        let mut st = self.conn.prepare(
            "SELECT t.id, t.name FROM document_tags d JOIN tags t ON t.id = d.tag_id
             WHERE d.document_id = ?1 AND d.deleted = 0 AND t.deleted = 0 ORDER BY t.name, t.id",
        )?;
        let rows = st.query_map([document_id], |r| {
            Ok(Tag {
                id: r.get(0)?,
                name: r.get(1)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    // ---------------------------------------------------------------- tài liệu

    /// Tạo tài liệu rỗng (sinh DK riêng, bọc bằng MK).
    ///
    /// # Errors
    /// [`VaultError::NotFound`] khi thư mục không tồn tại; lỗi SQLite.
    pub fn create_document(&mut self, title: &str, folder_id: Option<&str>) -> Result<String> {
        if let Some(f) = folder_id {
            self.require("folders", f, "thư mục")?;
        }
        let id = new_id()?;
        let dk = Key32::random()?;
        let wrapped = keys::wrap(&self.mk, &context::document(&id), &dk)?;
        let hlc = self.tick()?;
        let now = now_ms();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO documents(id, folder_id, title, created_ms, updated_ms, wrapped_dk, hlc)
             VALUES (?1, ?2, ?3, ?4, ?4, ?5, ?6)",
            params![id, folder_id, title.trim(), now, wrapped, hlc],
        )?;
        search::reindex(&tx, &id)?;
        tx.commit()?;
        Ok(id)
    }

    /// Đổi tiêu đề và/hoặc thư mục (`folder_id: Some(None)` = chuyển ra gốc).
    ///
    /// # Errors
    /// [`VaultError::NotFound`]; lỗi SQLite.
    pub fn update_document(
        &mut self,
        id: &str,
        title: Option<&str>,
        folder_id: Option<Option<&str>>,
    ) -> Result<()> {
        self.require("documents", id, "tài liệu")?;
        if let Some(Some(f)) = folder_id {
            self.require("folders", f, "thư mục")?;
        }
        let hlc = self.tick()?;
        let tx = self.conn.transaction()?;
        if let Some(t) = title {
            tx.execute(
                "UPDATE documents SET title = ?2 WHERE id = ?1",
                params![id, t.trim()],
            )?;
        }
        if let Some(f) = folder_id {
            tx.execute(
                "UPDATE documents SET folder_id = ?2 WHERE id = ?1",
                params![id, f],
            )?;
        }
        tx.execute(
            "UPDATE documents SET hlc = ?2, updated_ms = ?3 WHERE id = ?1",
            params![id, hlc, now_ms()],
        )?;
        search::reindex(&tx, id)?;
        tx.commit()?;
        Ok(())
    }

    /// Xóa tài liệu: để lại bia mộ (cho đồng bộ), xóa hẳn tệp trang, chữ OCR và chỉ mục tìm kiếm.
    ///
    /// # Errors
    /// [`VaultError::NotFound`]; lỗi SQLite/vào ra.
    pub fn delete_document(&mut self, id: &str) -> Result<()> {
        self.require("documents", id, "tài liệu")?;
        let hlc = self.tick()?;
        let files: Vec<String> = {
            let mut st = self.conn.prepare("SELECT id FROM files WHERE document_id = ?1")?;
            st.query_map([id], |r| r.get(0))?
                .collect::<std::result::Result<_, _>>()?
        };
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE pages SET deleted = 1, file_id = NULL, ocr_text = '', ocr_blob = NULL, hlc = ?2
             WHERE document_id = ?1",
            params![id, hlc],
        )?;
        tx.execute("DELETE FROM files WHERE document_id = ?1", [id])?;
        tx.execute(
            "UPDATE document_tags SET deleted = 1, hlc = ?2 WHERE document_id = ?1",
            params![id, hlc],
        )?;
        tx.execute(
            "UPDATE documents SET deleted = 1, title = '', hlc = ?2, updated_ms = ?3 WHERE id = ?1",
            params![id, hlc, now_ms()],
        )?;
        search::reindex(&tx, id)?;
        tx.commit()?;
        for f in files {
            remove_if_exists(&self.file_path(&f))?;
        }
        Ok(())
    }

    /// Một tài liệu.
    ///
    /// # Errors
    /// [`VaultError::NotFound`]; lỗi SQLite.
    pub fn document(&self, id: &str) -> Result<DocumentInfo> {
        self.conn
            .query_row(
                &format!("{DOC_SELECT} WHERE d.id = ?1 AND d.deleted = 0 AND d.trashed_ms IS NULL"),
                [id],
                doc_row,
            )
            .optional()?
            .ok_or(VaultError::NotFound("tài liệu"))
    }

    /// Danh sách tài liệu theo bộ lọc, mới sửa trước.
    ///
    /// # Errors
    /// Lỗi SQLite.
    pub fn list_documents(&self, filter: DocumentFilter<'_>) -> Result<Vec<DocumentInfo>> {
        self.list_documents_page(filter, u32::MAX, 0)
    }

    /// Một trang của [`Self::list_documents`]: bỏ `offset` bản ghi đầu, lấy tối đa `limit` (phân trang tại CSDL,
    /// `DATA_TABLE.md` mục 4).
    ///
    /// # Errors
    /// Lỗi SQLite.
    pub fn list_documents_page(
        &self,
        filter: DocumentFilter<'_>,
        limit: u32,
        offset: u64,
    ) -> Result<Vec<DocumentInfo>> {
        let (folder_any, folder) = folder_params(filter);
        let sql = format!(
            "{DOC_SELECT} WHERE d.deleted = 0 AND d.trashed_ms IS NULL AND {DOC_FILTER}
             ORDER BY d.updated_ms DESC, d.id LIMIT ?4 OFFSET ?5"
        );
        let mut st = self.conn.prepare(&sql)?;
        let offset = i64::try_from(offset).unwrap_or(i64::MAX);
        let rows = st.query_map(
            params![folder_any, folder, filter.tag, i64::from(limit), offset],
            doc_row,
        )?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Số tài liệu khớp bộ lọc (dòng tóm tắt của vùng bản ghi).
    ///
    /// # Errors
    /// Lỗi SQLite.
    pub fn count_documents(&self, filter: DocumentFilter<'_>) -> Result<u64> {
        let (folder_any, folder) = folder_params(filter);
        let n: i64 = self.conn.query_row(
            &format!("SELECT COUNT(*) FROM documents d WHERE d.deleted = 0 AND d.trashed_ms IS NULL AND {DOC_FILTER}"),
            params![folder_any, folder, filter.tag],
            |r| r.get(0),
        )?;
        Ok(u64::try_from(n).unwrap_or(0))
    }

    // ---------------------------------------------------------------- trang

    /// Thêm trang vào cuối tài liệu: ảnh mã hóa luồng bằng FK mới (bọc bằng DK), chữ OCR vào chỉ mục.
    ///
    /// # Errors
    /// [`VaultError::NotFound`]; [`VaultError::Format`] khi ảnh quá lớn; lỗi vào/ra, SQLite.
    pub fn add_page(&mut self, document_id: &str, page: NewPage<'_>) -> Result<String> {
        let dk = self.document_key(document_id)?;
        let (file_id, fk_wrapped) = self.write_file(&dk, page.image)?;
        let page_id = new_id()?;
        let hlc = self.tick()?;
        let tx = self.conn.transaction()?;
        insert_file_row(&tx, &file_id, document_id, &fk_wrapped, page.image.len())?;
        let pos: i64 = tx.query_row(
            "SELECT COUNT(*) FROM pages WHERE document_id = ?1 AND deleted = 0",
            [document_id],
            |r| r.get(0),
        )?;
        tx.execute(
            "INSERT INTO pages(id, document_id, position, file_id, width, height, dpi, ocr_text, ocr_blob, hlc)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                page_id,
                document_id,
                pos,
                file_id,
                page.width,
                page.height,
                page.dpi,
                page.ocr_text,
                page.ocr_blob,
                hlc
            ],
        )?;
        tx.execute(
            "UPDATE documents SET updated_ms = ?2, hlc = ?3 WHERE id = ?1",
            params![document_id, now_ms(), hlc],
        )?;
        search::reindex(&tx, document_id)?;
        if let Err(e) = tx.commit() {
            remove_if_exists(&self.file_path(&file_id))?;
            return Err(e.into());
        }
        Ok(page_id)
    }

    /// Các trang chưa xóa theo thứ tự.
    ///
    /// # Errors
    /// Lỗi SQLite.
    pub fn list_pages(&self, document_id: &str) -> Result<Vec<PageInfo>> {
        let mut st = self.conn.prepare(
            "SELECT id, position, width, height, dpi, ocr_text FROM pages
             WHERE document_id = ?1 AND deleted = 0 ORDER BY position",
        )?;
        let rows = st.query_map([document_id], |r| {
            Ok(PageInfo {
                id: r.get(0)?,
                position: r.get(1)?,
                width: r.get(2)?,
                height: r.get(3)?,
                dpi: r.get(4)?,
                ocr_text: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Byte ảnh của trang (giải mã + kiểm toàn vẹn).
    ///
    /// # Errors
    /// [`VaultError::NotFound`]; [`VaultError::Decrypt`] khi tệp bị sửa/tráo; lỗi vào/ra.
    pub fn page_image(&self, page_id: &str) -> Result<Zeroizing<Vec<u8>>> {
        let row: Option<(String, Option<String>)> = self
            .conn
            .query_row(
                "SELECT document_id, file_id FROM pages WHERE id = ?1 AND deleted = 0",
                [page_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let (doc, file) = row.ok_or(VaultError::NotFound("trang"))?;
        let file = file.ok_or(VaultError::NotFound("ảnh trang"))?;
        let dk = self.document_key(&doc)?;
        self.read_file(&dk, &file)
    }

    /// OCR đầy đủ đã tuần tự hóa của trang (nếu có).
    ///
    /// # Errors
    /// [`VaultError::NotFound`]; lỗi SQLite.
    pub fn page_ocr_blob(&self, page_id: &str) -> Result<Option<Vec<u8>>> {
        self.conn
            .query_row(
                "SELECT ocr_blob FROM pages WHERE id = ?1 AND deleted = 0",
                [page_id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or(VaultError::NotFound("trang"))
    }

    /// Cập nhật kết quả OCR của trang (chạy lại OCR, người dùng sửa chữ).
    ///
    /// # Errors
    /// [`VaultError::NotFound`]; lỗi SQLite.
    pub fn set_page_ocr(&mut self, page_id: &str, text: &str, blob: Option<&[u8]>) -> Result<()> {
        let doc = self.page_document(page_id)?;
        let hlc = self.tick()?;
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE pages SET ocr_text = ?2, ocr_blob = ?3, hlc = ?4 WHERE id = ?1",
            params![page_id, text, blob, hlc],
        )?;
        tx.execute(
            "UPDATE documents SET updated_ms = ?2, hlc = ?3 WHERE id = ?1",
            params![doc, now_ms(), hlc],
        )?;
        search::reindex(&tx, &doc)?;
        tx.commit()?;
        Ok(())
    }

    /// Xóa trang (bia mộ + xóa hẳn tệp ảnh), đánh lại thứ tự các trang còn lại.
    ///
    /// # Errors
    /// [`VaultError::NotFound`]; lỗi SQLite/vào ra.
    pub fn delete_page(&mut self, page_id: &str) -> Result<()> {
        let doc = self.page_document(page_id)?;
        let file: Option<String> =
            self.conn
                .query_row("SELECT file_id FROM pages WHERE id = ?1", [page_id], |r| r.get(0))?;
        let hlc = self.tick()?;
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE pages SET deleted = 1, file_id = NULL, ocr_text = '', ocr_blob = NULL, hlc = ?2 WHERE id = ?1",
            params![page_id, hlc],
        )?;
        if let Some(f) = &file {
            tx.execute("DELETE FROM files WHERE id = ?1", [f])?;
        }
        renumber(&tx, &doc, None, &hlc)?;
        tx.execute(
            "UPDATE documents SET updated_ms = ?2, hlc = ?3 WHERE id = ?1",
            params![doc, now_ms(), hlc],
        )?;
        search::reindex(&tx, &doc)?;
        tx.commit()?;
        if let Some(f) = file {
            remove_if_exists(&self.file_path(&f))?;
        }
        Ok(())
    }

    /// Sắp lại trang: `order` phải là hoán vị đúng của mọi trang chưa xóa.
    ///
    /// # Errors
    /// [`VaultError::Format`] khi `order` không phải hoán vị; lỗi SQLite.
    pub fn reorder_pages(&mut self, document_id: &str, order: &[&str]) -> Result<()> {
        let mut current: Vec<String> = self.list_pages(document_id)?.into_iter().map(|p| p.id).collect();
        let mut wanted: Vec<String> = order.iter().map(|s| (*s).to_owned()).collect();
        current.sort();
        wanted.sort();
        if current != wanted {
            return Err(VaultError::Format("thứ tự trang không khớp danh sách trang"));
        }
        let hlc = self.tick()?;
        let tx = self.conn.transaction()?;
        renumber(&tx, document_id, Some(order), &hlc)?;
        tx.execute(
            "UPDATE documents SET updated_ms = ?2, hlc = ?3 WHERE id = ?1",
            params![document_id, now_ms(), hlc],
        )?;
        search::reindex(&tx, document_id)?;
        tx.commit()?;
        Ok(())
    }

    // ---------------------------------------------------------------- thùng rác

    /// Bỏ tài liệu vào thùng rác: giữ nguyên trang/tệp, ẩn khỏi danh sách và tìm kiếm. Xóa hẳn sau
    /// [`TRASH_RETENTION_DAYS`] ngày ([`Self::purge_trash`]) hoặc khi người dùng dọn thùng rác.
    ///
    /// # Errors
    /// [`VaultError::NotFound`]; lỗi SQLite.
    pub fn trash_document(&mut self, id: &str) -> Result<()> {
        self.require("documents", id, "tài liệu")?;
        let hlc = self.tick()?;
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE documents SET trashed_ms = COALESCE(trashed_ms, ?2), hlc = ?3 WHERE id = ?1",
            params![id, now_ms(), hlc],
        )?;
        search::reindex(&tx, id)?;
        tx.commit()?;
        Ok(())
    }

    /// Đưa tài liệu ra khỏi thùng rác.
    ///
    /// # Errors
    /// [`VaultError::NotFound`] (kể cả tài liệu không nằm trong thùng rác); lỗi SQLite.
    pub fn restore_document(&mut self, id: &str) -> Result<()> {
        self.require_trashed(id)?;
        let hlc = self.tick()?;
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE documents SET trashed_ms = NULL, hlc = ?2, updated_ms = ?3 WHERE id = ?1",
            params![id, hlc, now_ms()],
        )?;
        search::reindex(&tx, id)?;
        tx.commit()?;
        Ok(())
    }

    /// Tài liệu trong thùng rác, mới bỏ vào trước.
    ///
    /// # Errors
    /// Lỗi SQLite.
    pub fn list_trash(&self) -> Result<Vec<TrashedDocument>> {
        let sql = format!(
            "{DOC_SELECT_TRASH} WHERE d.deleted = 0 AND d.trashed_ms IS NOT NULL ORDER BY d.trashed_ms DESC, d.id"
        );
        let mut st = self.conn.prepare(&sql)?;
        let rows = st.query_map([], |r| {
            Ok(TrashedDocument {
                info: doc_row(r)?,
                trashed_ms: r.get(6)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Xóa hẳn tài liệu đang trong thùng rác (người dùng chọn "Xóa vĩnh viễn").
    ///
    /// # Errors
    /// [`VaultError::NotFound`] khi tài liệu không trong thùng rác; lỗi SQLite/vào ra.
    pub fn purge_document(&mut self, id: &str) -> Result<()> {
        self.require_trashed(id)?;
        self.delete_document(id)
    }

    /// Xóa hẳn mọi tài liệu đã nằm trong thùng rác từ trước `now_ms - older_than_ms` (`older_than_ms = 0` ⇒ dọn sạch
    /// thùng rác). Trả số tài liệu đã xóa. App gọi khi mở kho với [`TRASH_RETENTION_MS`].
    ///
    /// # Errors
    /// Lỗi SQLite/vào ra.
    pub fn purge_trash(&mut self, now_ms: i64, older_than_ms: i64) -> Result<usize> {
        let cutoff = now_ms.saturating_sub(older_than_ms.max(0));
        let ids: Vec<String> = {
            let mut st = self.conn.prepare(
                "SELECT id FROM documents WHERE deleted = 0 AND trashed_ms IS NOT NULL AND trashed_ms <= ?1",
            )?;
            st.query_map([cutoff], |r| r.get(0))?
                .collect::<std::result::Result<_, _>>()?
        };
        for id in &ids {
            self.delete_document(id)?;
        }
        Ok(ids.len())
    }

    // ---------------------------------------------------------------- sửa trang

    /// Thay ảnh của trang (xoay, cắt, lọc lại...): ghi tệp mới bằng FK mới rồi xóa hẳn tệp cũ. Chữ OCR giữ nguyên
    /// nếu `ocr` là `None`, ngược lại thay bằng `(chữ, blob)`.
    ///
    /// # Errors
    /// [`VaultError::NotFound`]; [`VaultError::Format`] khi ảnh quá lớn; lỗi vào/ra, SQLite.
    pub fn replace_page_image(
        &mut self,
        page_id: &str,
        image: &[u8],
        width: u32,
        height: u32,
        ocr: Option<(&str, Option<&[u8]>)>,
    ) -> Result<()> {
        let doc = self.page_document(page_id)?;
        let old: Option<String> =
            self.conn
                .query_row("SELECT file_id FROM pages WHERE id = ?1", [page_id], |r| r.get(0))?;
        let dk = self.document_key(&doc)?;
        let (file_id, fk_wrapped) = self.write_file(&dk, image)?;
        let hlc = self.tick()?;
        let res = (|| -> Result<()> {
            let tx = self.conn.transaction()?;
            insert_file_row(&tx, &file_id, &doc, &fk_wrapped, image.len())?;
            tx.execute(
                "UPDATE pages SET file_id = ?2, width = ?3, height = ?4, hlc = ?5 WHERE id = ?1",
                params![page_id, file_id, width, height, hlc],
            )?;
            if let Some((text, blob)) = ocr {
                tx.execute(
                    "UPDATE pages SET ocr_text = ?2, ocr_blob = ?3 WHERE id = ?1",
                    params![page_id, text, blob],
                )?;
            }
            if let Some(f) = &old {
                tx.execute("DELETE FROM files WHERE id = ?1", [f])?;
            }
            tx.execute(
                "UPDATE documents SET updated_ms = ?2, hlc = ?3 WHERE id = ?1",
                params![doc, now_ms(), hlc],
            )?;
            search::reindex(&tx, &doc)?;
            tx.commit()?;
            Ok(())
        })();
        if let Err(e) = res {
            remove_if_exists(&self.file_path(&file_id))?;
            return Err(e);
        }
        if let Some(f) = old {
            remove_if_exists(&self.file_path(&f))?;
        }
        Ok(())
    }

    /// Chuyển các trang sang tài liệu `target` (ghép/tách tài liệu), chèn tại vị trí `at` (`None` = cuối), giữ thứ tự
    /// của `page_ids`. Ảnh được giải mã bằng DK nguồn rồi mã hóa lại bằng FK mới bọc bằng DK đích (khóa của tài liệu
    /// nguồn không bao giờ dùng được cho tài liệu đích); tệp cũ bị xóa hẳn. Trang giữ nguyên id và chữ OCR.
    ///
    /// # Errors
    /// [`VaultError::NotFound`]; [`VaultError::Format`] khi danh sách rỗng/lặp hoặc trang đã thuộc `target`; lỗi vào/ra,
    /// SQLite.
    pub fn move_pages(&mut self, page_ids: &[&str], target: &str, at: Option<usize>) -> Result<()> {
        if page_ids.is_empty() {
            return Err(VaultError::Format("không có trang nào để chuyển"));
        }
        let mut seen = std::collections::HashSet::new();
        if !page_ids.iter().all(|p| seen.insert(*p)) {
            return Err(VaultError::Format("trang bị lặp"));
        }
        let target_dk = self.document_key(target)?;
        let mut moves = Vec::with_capacity(page_ids.len());
        for pid in page_ids {
            let src = self.page_document(pid)?;
            if src == target {
                return Err(VaultError::Format("trang đã thuộc tài liệu đích"));
            }
            moves.push((*pid, src));
        }
        // Ghi các tệp mới trước; lỗi giữa chừng ⇒ xóa tệp đã ghi, CSDL không đổi.
        let mut written: Vec<(String, Vec<u8>, usize, Option<String>)> = Vec::with_capacity(moves.len());
        let res = (|| -> Result<()> {
            for (pid, src) in &moves {
                let old: Option<String> =
                    self.conn
                        .query_row("SELECT file_id FROM pages WHERE id = ?1", [pid], |r| r.get(0))?;
                let Some(old) = old else {
                    return Err(VaultError::NotFound("ảnh trang"));
                };
                let src_dk = self.document_key(src)?;
                let data = self.read_file(&src_dk, &old)?;
                let (fid, wrapped) = self.write_file(&target_dk, &data)?;
                written.push((fid, wrapped, data.len(), Some(old)));
            }
            Ok(())
        })();
        if let Err(e) = res {
            for (fid, ..) in &written {
                remove_if_exists(&self.file_path(fid))?;
            }
            return Err(e);
        }
        let hlc = self.tick()?;
        let mut sources: Vec<String> = moves.iter().map(|(_, s)| s.clone()).collect();
        sources.sort();
        sources.dedup();
        let res = (|| -> Result<()> {
            let tx = self.conn.transaction()?;
            let mut order: Vec<String> = {
                let mut st = tx.prepare(
                    "SELECT id FROM pages WHERE document_id = ?1 AND deleted = 0 ORDER BY position",
                )?;
                st.query_map([target], |r| r.get(0))?
                    .collect::<std::result::Result<_, _>>()?
            };
            let at = at.unwrap_or(order.len()).min(order.len());
            for ((pid, _), (fid, wrapped, len, old)) in moves.iter().zip(&written) {
                insert_file_row(&tx, fid, target, wrapped, *len)?;
                tx.execute(
                    "UPDATE pages SET document_id = ?2, file_id = ?3, hlc = ?4 WHERE id = ?1",
                    params![pid, target, fid, hlc],
                )?;
                if let Some(o) = old {
                    tx.execute("DELETE FROM files WHERE id = ?1", [o])?;
                }
            }
            for (i, (pid, _)) in moves.iter().enumerate() {
                order.insert(at + i, (*pid).to_owned());
            }
            let refs: Vec<&str> = order.iter().map(String::as_str).collect();
            renumber(&tx, target, Some(&refs), &hlc)?;
            for s in sources.iter().map(String::as_str).chain(std::iter::once(target)) {
                if s != target {
                    renumber(&tx, s, None, &hlc)?;
                }
                tx.execute(
                    "UPDATE documents SET updated_ms = ?2, hlc = ?3 WHERE id = ?1",
                    params![s, now_ms(), hlc],
                )?;
                search::reindex(&tx, s)?;
            }
            tx.commit()?;
            Ok(())
        })();
        if let Err(e) = res {
            for (fid, ..) in &written {
                remove_if_exists(&self.file_path(fid))?;
            }
            return Err(e);
        }
        for (_, _, _, old) in written {
            if let Some(o) = old {
                remove_if_exists(&self.file_path(&o))?;
            }
        }
        Ok(())
    }

    /// Id tài liệu chứa trang.
    ///
    /// # Errors
    /// [`VaultError::NotFound`]; lỗi SQLite.
    pub fn page_document_id(&self, page_id: &str) -> Result<String> {
        self.page_document(page_id)
    }

    // ---------------------------------------------------------------- tìm kiếm

    /// Tìm tài liệu theo tiêu đề + chữ OCR, không dấu, tiền tố; khớp đúng dấu xếp cao hơn.
    /// Truy vấn ngắn hơn 2 ký tự (sau chuẩn hóa) trả rỗng.
    ///
    /// # Errors
    /// Lỗi SQLite.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>> {
        search::search(&self.conn, query, limit)
    }

    // ---------------------------------------------------------------- nội bộ

    fn require(&self, table: &'static str, id: &str, what: &'static str) -> Result<()> {
        let sql = format!("SELECT 1 FROM {table} WHERE id = ?1 AND deleted = 0");
        self.conn
            .query_row(&sql, [id], |_| Ok(()))
            .optional()?
            .ok_or(VaultError::NotFound(what))
    }

    fn require_trashed(&self, id: &str) -> Result<()> {
        self.conn
            .query_row(
                "SELECT 1 FROM documents WHERE id = ?1 AND deleted = 0 AND trashed_ms IS NOT NULL",
                [id],
                |_| Ok(()),
            )
            .optional()?
            .ok_or(VaultError::NotFound("tài liệu trong thùng rác"))
    }

    fn page_document(&self, page_id: &str) -> Result<String> {
        self.conn
            .query_row(
                "SELECT document_id FROM pages WHERE id = ?1 AND deleted = 0",
                [page_id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or(VaultError::NotFound("trang"))
    }

    fn document_key(&self, document_id: &str) -> Result<Key32> {
        let wrapped: Vec<u8> = self
            .conn
            .query_row(
                "SELECT wrapped_dk FROM documents WHERE id = ?1 AND deleted = 0",
                [document_id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or(VaultError::NotFound("tài liệu"))?;
        keys::unwrap(&self.mk, &context::document(document_id), &wrapped)
    }

    fn file_path(&self, file_id: &str) -> PathBuf {
        let shard = file_id.get(..2).unwrap_or("00");
        self.root
            .join(FILES_DIR)
            .join(shard)
            .join(format!("{file_id}.tsf"))
    }
}

/// Điều kiện lọc dùng chung cho danh sách + đếm: ?1 = mọi thư mục, ?2 = thư mục, ?3 = nhãn.
const DOC_FILTER: &str = "(?1 = 1 OR d.folder_id IS ?2)
    AND (?3 IS NULL OR EXISTS (SELECT 1 FROM document_tags t
         WHERE t.document_id = d.id AND t.tag_id = ?3 AND t.deleted = 0))";

fn folder_params(filter: DocumentFilter<'_>) -> (i64, Option<&str>) {
    match filter.folder {
        None => (1, None),
        Some(f) => (0, f),
    }
}

const DOC_SELECT: &str = "SELECT d.id, d.title, d.folder_id, d.created_ms, d.updated_ms,
    (SELECT COUNT(*) FROM pages p WHERE p.document_id = d.id AND p.deleted = 0) FROM documents d";

const DOC_SELECT_TRASH: &str = "SELECT d.id, d.title, d.folder_id, d.created_ms, d.updated_ms,
    (SELECT COUNT(*) FROM pages p WHERE p.document_id = d.id AND p.deleted = 0), d.trashed_ms FROM documents d";

fn insert_file_row(
    tx: &rusqlite::Transaction<'_>,
    file_id: &str,
    document_id: &str,
    wrapped_fk: &[u8],
    len: usize,
) -> Result<()> {
    tx.execute(
        "INSERT INTO files(id, document_id, wrapped_fk, plain_len, cipher_len) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            file_id,
            document_id,
            wrapped_fk,
            len as i64,
            stream::encrypted_len(len as u64) as i64
        ],
    )?;
    Ok(())
}

fn doc_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<DocumentInfo> {
    Ok(DocumentInfo {
        id: r.get(0)?,
        title: r.get(1)?,
        folder_id: r.get(2)?,
        created_ms: r.get(3)?,
        updated_ms: r.get(4)?,
        page_count: r.get(5)?,
    })
}

/// Đặt khóa, kiểm đúng thuật toán và đọc thử lược đồ (sai khóa ⇒ lỗi ngay tại đây).
fn apply_key(conn: &Connection, key: &Key32) -> Result<()> {
    conn.pragma_update(None, "hexkey", key.to_hex().as_str())
        .map_err(|_| VaultError::WrongDbKey)?;
    let cipher: String = conn
        .query_row("PRAGMA cipher", [], |r| r.get(0))
        .map_err(|_| VaultError::WrongDbKey)?;
    if cipher != "chacha20" {
        return Err(VaultError::Format("CSDL không dùng ChaCha20-Poly1305"));
    }
    conn.query_row("SELECT COUNT(*) FROM sqlite_schema", [], |r| r.get::<_, i64>(0))
        .map_err(|_| VaultError::WrongDbKey)?;
    conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "secure_delete", "ON")?;
    Ok(())
}

fn renumber(
    tx: &rusqlite::Transaction<'_>,
    document_id: &str,
    order: Option<&[&str]>,
    hlc: &str,
) -> Result<()> {
    let ids: Vec<String> = if let Some(o) = order {
        o.iter().map(|s| (*s).to_owned()).collect()
    } else {
        let mut st =
            tx.prepare("SELECT id FROM pages WHERE document_id = ?1 AND deleted = 0 ORDER BY position")?;
        st.query_map([document_id], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?
    };
    for (i, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE pages SET position = ?2, hlc = ?3 WHERE id = ?1 AND position IS NOT ?2",
            params![id, i as i64, hlc],
        )?;
    }
    Ok(())
}

impl Vault {
    /// Ghi tệp mã hóa (tạm + đổi tên ⇒ không bao giờ để lại tệp ghi dở). Trả id tệp + FK đã bọc bằng DK.
    fn write_file(&self, dk: &Key32, data: &[u8]) -> Result<(String, Vec<u8>)> {
        if data.len() as u64 > MAX_FILE_BYTES {
            return Err(VaultError::Format("ảnh trang quá lớn"));
        }
        let id = new_id()?;
        let fk = Key32::random()?;
        let path = self.file_path(&id);
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("tmp");
        let res = (|| -> Result<()> {
            let mut f = fs::File::create(&tmp)?;
            stream::encrypt(
                &fk,
                context::file(&id).as_bytes(),
                data.len() as u64,
                data,
                &mut f,
            )?;
            f.flush()?;
            f.sync_all()?;
            fs::rename(&tmp, &path)?;
            Ok(())
        })();
        if let Err(e) = res {
            let _ = fs::remove_file(&tmp);
            return Err(e);
        }
        let wrapped = keys::wrap(dk, &context::file(&id), &fk)?;
        Ok((id, wrapped))
    }

    fn read_file(&self, dk: &Key32, file_id: &str) -> Result<Zeroizing<Vec<u8>>> {
        let (wrapped, plain_len): (Vec<u8>, i64) = self
            .conn
            .query_row(
                "SELECT wrapped_fk, plain_len FROM files WHERE id = ?1",
                [file_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
            .ok_or(VaultError::NotFound("tệp"))?;
        let fk = keys::unwrap(dk, &context::file(file_id), &wrapped)?;
        let f = fs::File::open(self.file_path(file_id))?;
        let cap = usize::try_from(plain_len)
            .unwrap_or(0)
            .min(MAX_FILE_BYTES as usize);
        let mut out = Zeroizing::new(Vec::with_capacity(cap));
        let n = stream::decrypt(
            &fk,
            context::file(file_id).as_bytes(),
            std::io::BufReader::new(f),
            &mut *out,
        )?;
        if i64::try_from(n).ok() != Some(plain_len) {
            return Err(VaultError::Decrypt);
        }
        Ok(out)
    }
}

fn remove_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Id ngẫu nhiên 128 bit dạng hex thường.
///
/// # Errors
/// [`VaultError::Random`].
pub fn new_id() -> Result<String> {
    let mut b = [0u8; 16];
    fill_random(&mut b)?;
    Ok(Key32::hex_of(&b))
}

fn now_ms() -> i64 {
    i64::try_from(wall_now_ms()).unwrap_or(i64::MAX)
}

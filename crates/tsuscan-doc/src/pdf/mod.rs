//! Ghi PDF: ảnh trang nén thông minh + lớp chữ OCR ẩn tìm được, tùy chọn PDF/A-2b hoặc mật khẩu AES-256.
//!
//! PDF/A cấm mã hóa (ISO 19005-2 §6.1.3) ⇒ hai tùy chọn loại trừ nhau, báo lỗi thay vì âm thầm bỏ một bên.

pub mod crypt;
pub mod font;
pub mod guard;
pub mod icc;
pub mod read;
pub mod xmp;

use pdf_writer::types::{CidFontType, FontFlags, OutputIntentSubtype, SystemInfo, TextRenderingMode};
use pdf_writer::{Content, Filter, Finish, Name, Pdf, Rect, Ref, Str};
use sha2::{Digest, Sha256};
use tsuscan_ml::OcrPageResult;

use crate::compress::{self, ColorMode, Encoding, Quality};
use crate::error::{DocError, Result};
use crate::model::{Document, PRODUCER, Page, Timestamp};

pub use crypt::{Crypt, Permissions};

/// Số trang tối đa một tệp xuất (chặn dùng hết bộ nhớ).
pub const MAX_PAGES: usize = 2000;

/// Mật khẩu bảo vệ tệp.
#[derive(Clone, PartialEq, Eq)]
pub struct Protection {
    /// Mật khẩu mở tệp (bắt buộc, khác rỗng).
    pub user_password: String,
    /// Mật khẩu chủ (đổi quyền). Rỗng ⇒ ngẫu nhiên, không ai đổi quyền được.
    pub owner_password: String,
    /// Quyền khi mở bằng mật khẩu người dùng.
    pub permissions: Permissions,
}

impl std::fmt::Debug for Protection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Protection")
            .field("permissions", &self.permissions)
            .finish_non_exhaustive()
    }
}

/// Tùy chọn ghi PDF.
#[derive(Debug, Clone, Default)]
pub struct PdfOptions {
    /// Ghi chuẩn lưu trữ PDF/A-2b.
    pub pdfa: bool,
    /// Mật khẩu AES-256 (không dùng cùng `pdfa`).
    pub protection: Option<Protection>,
    /// Chế độ màu.
    pub color: ColorMode,
    /// Chất lượng/dung lượng.
    pub quality: Quality,
    /// Bỏ lớp chữ OCR (mặc định có nếu trang đã OCR).
    pub no_text_layer: bool,
}

/// Ghi tài liệu ra PDF.
///
/// # Errors
/// - [`DocError::Empty`] khi không có trang; [`DocError::LimitExceeded`] khi quá [`MAX_PAGES`].
/// - [`DocError::InvalidOption`] khi vừa PDF/A vừa mật khẩu.
/// - Lỗi nén ảnh / số ngẫu nhiên.
pub fn write_pdf(doc: &Document, opts: &PdfOptions) -> Result<Vec<u8>> {
    doc.ensure_pages()?;
    if doc.pages.len() > MAX_PAGES {
        return Err(DocError::LimitExceeded("quá 2.000 trang"));
    }
    if opts.pdfa && opts.protection.is_some() {
        return Err(DocError::InvalidOption("PDF/A không cho phép đặt mật khẩu"));
    }
    let crypt = match &opts.protection {
        Some(p) => Some(Crypt::new(&p.user_password, &p.owner_password, p.permissions)?),
        None => None,
    };
    let created = doc.meta.created.unwrap_or_else(|| Timestamp::now(0));
    let mut w = Writer {
        pdf: Pdf::new(),
        next: 1,
        crypt,
    };
    w.build(doc, opts, created)
}

struct Writer {
    pdf: Pdf,
    next: i32,
    crypt: Option<Crypt>,
}

/// Đối tượng phông dùng chung cho mọi trang.
struct FontRefs {
    type0: Ref,
}

impl Writer {
    fn alloc(&mut self) -> Ref {
        let r = Ref::new(self.next);
        self.next += 1;
        r
    }

    /// Mã hóa dữ liệu luồng/chuỗi khi có mật khẩu.
    fn seal(&self, data: Vec<u8>) -> Result<Vec<u8>> {
        match &self.crypt {
            Some(c) => c.encrypt(&data),
            None => Ok(data),
        }
    }

    /// Chuỗi văn bản PDF: ASCII in được giữ nguyên, còn lại UTF-16BE có BOM.
    fn text_bytes(&self, s: &str) -> Result<Vec<u8>> {
        let raw = if s.bytes().all(|b| (0x20..0x7F).contains(&b)) {
            s.as_bytes().to_vec()
        } else {
            let mut v = vec![0xFE, 0xFF];
            v.extend(s.encode_utf16().flat_map(u16::to_be_bytes));
            v
        };
        self.seal(raw)
    }

    fn build(&mut self, doc: &Document, opts: &PdfOptions, created: Timestamp) -> Result<Vec<u8>> {
        let catalog_id = self.alloc();
        let tree_id = self.alloc();
        let info_id = self.alloc();
        let wants_text = !opts.no_text_layer
            && doc.pages.iter().any(|p| {
                p.ocr
                    .as_ref()
                    .is_some_and(|o| o.lines.iter().any(|l| !l.text.trim().is_empty()))
            });
        let font = if wants_text {
            Some(self.write_font()?)
        } else {
            None
        };

        let mut page_ids = Vec::with_capacity(doc.pages.len());
        let mut hasher = Sha256::new();
        for page in &doc.pages {
            let id = self.write_page(page, tree_id, font.as_ref(), opts, &mut hasher)?;
            page_ids.push(id);
        }
        let count = page_ids.len() as i32;
        self.pdf.pages(tree_id).kids(page_ids).count(count);

        let meta_id = self.alloc();
        let xmp = xmp::packet(&doc.meta, created, opts.pdfa);
        let xmp_data = self.seal(xmp)?;
        self.pdf.metadata(meta_id, &xmp_data);

        let icc_id = if opts.pdfa {
            let id = self.alloc();
            let icc = compress::deflate(&icc::srgb_profile());
            self.pdf.icc_profile(id, &icc).n(3).filter(Filter::FlateDecode);
            Some(id)
        } else {
            None
        };

        {
            let mut cat = self.pdf.catalog(catalog_id);
            cat.pages(tree_id).metadata(meta_id);
            if let Some(icc) = icc_id {
                cat.output_intents()
                    .push()
                    .subtype(OutputIntentSubtype::PDFA)
                    .output_condition_identifier(pdf_writer::TextStr("sRGB IEC61966-2.1"))
                    .info(pdf_writer::TextStr("sRGB IEC61966-2.1"))
                    .dest_output_profile(icc);
            }
            if self.crypt.is_some() {
                // AES-256 (R6) với PDF 1.7 cần khai báo phần mở rộng Adobe cấp 8.
                cat.insert(Name(b"Extensions"))
                    .dict()
                    .insert(Name(b"ADBE"))
                    .dict()
                    .pair(Name(b"BaseVersion"), Name(b"1.7"))
                    .pair(Name(b"ExtensionLevel"), 8);
            }
        }

        self.write_info(info_id, doc, created)?;
        hasher.update(created.unix.to_be_bytes());
        hasher.update(doc.meta.title.as_bytes());
        // /ID luôn nằm ở dạng rõ: tệp mã hóa dùng ID ngẫu nhiên, không để băm nội dung giúp xác nhận một bản ảnh đoán
        // trước. Tệp không mã hóa dùng băm (tất định, dễ kiểm).
        let file_id = if self.crypt.is_some() {
            crypt::random_bytes::<16>()?.to_vec()
        } else {
            hasher.finalize()[..16].to_vec()
        };
        self.pdf.set_file_id((file_id.clone(), file_id));

        let encrypt_id = if let Some(c) = self.crypt.clone() {
            let id = self.alloc();
            let mut d = self.pdf.indirect(id).dict();
            d.pair(Name(b"Filter"), Name(b"Standard"))
                .pair(Name(b"V"), 5)
                .pair(Name(b"R"), 6)
                .pair(Name(b"Length"), 256)
                .pair(Name(b"P"), c.p)
                .pair(Name(b"O"), Str(&c.o))
                .pair(Name(b"U"), Str(&c.u))
                .pair(Name(b"OE"), Str(&c.oe))
                .pair(Name(b"UE"), Str(&c.ue))
                .pair(Name(b"Perms"), Str(&c.perms))
                .pair(Name(b"EncryptMetadata"), true)
                .pair(Name(b"StmF"), Name(b"StdCF"))
                .pair(Name(b"StrF"), Name(b"StdCF"));
            d.insert(Name(b"CF"))
                .dict()
                .insert(Name(b"StdCF"))
                .dict()
                .pair(Name(b"Type"), Name(b"CryptFilter"))
                .pair(Name(b"CFM"), Name(b"AESV3"))
                .pair(Name(b"AuthEvent"), Name(b"DocOpen"))
                .pair(Name(b"Length"), 32);
            d.finish();
            Some(id)
        } else {
            None
        };

        let pdf = std::mem::replace(&mut self.pdf, Pdf::new());
        let mut bytes = pdf.finish();
        if let Some(id) = encrypt_id {
            inject_encrypt(&mut bytes, id)?;
        }
        Ok(bytes)
    }

    fn write_info(&mut self, id: Ref, doc: &Document, created: Timestamp) -> Result<()> {
        let m = &doc.meta;
        let mut entries: Vec<(&'static [u8], Vec<u8>)> = Vec::new();
        if !m.title.is_empty() {
            entries.push((b"Title", self.text_bytes(&m.title)?));
        }
        if !m.author.is_empty() {
            entries.push((b"Author", self.text_bytes(&m.author)?));
        }
        if !m.subject.is_empty() {
            entries.push((b"Subject", self.text_bytes(&m.subject)?));
        }
        if !m.keywords.is_empty() {
            entries.push((b"Keywords", self.text_bytes(&m.keywords.join(", "))?));
        }
        entries.push((b"Creator", self.text_bytes(PRODUCER)?));
        entries.push((b"Producer", self.text_bytes(PRODUCER)?));
        let date = created.pdf_date();
        entries.push((b"CreationDate", self.seal(date.clone().into_bytes())?));
        entries.push((b"ModDate", self.seal(date.into_bytes())?));
        // Chuỗi đã mã hóa sẵn ⇒ ghi byte thô qua `pair` (DocumentInfo deref sang Dict).
        let mut d = self.pdf.document_info(id);
        for (k, v) in &entries {
            d.pair(Name(k), Str(v));
        }
        d.finish();
        Ok(())
    }

    fn write_font(&mut self) -> Result<FontRefs> {
        let type0 = self.alloc();
        let cid = self.alloc();
        let desc = self.alloc();
        let file = self.alloc();
        let map = self.alloc();
        let tounicode = self.alloc();
        let name = Name(font::FONT_NAME.as_bytes());

        self.pdf
            .type0_font(type0)
            .base_font(name)
            .encoding_predefined(Name(b"Identity-H"))
            .descendant_font(cid)
            .to_unicode(tounicode);
        let mut c = self.pdf.cid_font(cid);
        c.subtype(CidFontType::Type2)
            .base_font(name)
            .system_info(SystemInfo {
                registry: Str(b"Adobe"),
                ordering: Str(b"Identity"),
                supplement: 0,
            })
            .font_descriptor(desc)
            .default_width(f32::from(font::GLYPH_WIDTH));
        c.pair(Name(b"CIDToGIDMap"), map);
        c.finish();
        self.pdf
            .font_descriptor(desc)
            .name(name)
            .flags(FontFlags::FIXED_PITCH | FontFlags::SYMBOLIC)
            .bbox(Rect::new(0.0, 0.0, f32::from(font::GLYPH_WIDTH), 1000.0))
            .italic_angle(0.0)
            .ascent(1000.0)
            .descent(0.0)
            .cap_height(1000.0)
            .stem_v(80.0)
            .font_file2(file);
        let ttf = font::glyphless_ttf();
        let ttf_len = ttf.len() as i32;
        let data = self.seal(compress::deflate(&ttf))?;
        self.pdf
            .stream(file, &data)
            .filter(Filter::FlateDecode)
            .pair(Name(b"Length1"), ttf_len);
        let data = self.seal(compress::deflate(&font::cid_to_gid_map()))?;
        self.pdf.stream(map, &data).filter(Filter::FlateDecode);
        let data = self.seal(compress::deflate(&font::to_unicode_cmap()))?;
        self.pdf.stream(tounicode, &data).filter(Filter::FlateDecode);
        Ok(FontRefs { type0 })
    }

    fn write_page(
        &mut self,
        page: &Page,
        tree: Ref,
        font: Option<&FontRefs>,
        opts: &PdfOptions,
        hasher: &mut Sha256,
    ) -> Result<Ref> {
        let page_id = self.alloc();
        let image_id = self.alloc();
        let content_id = self.alloc();
        let dpi = page.effective_dpi();
        let (w_pt, h_pt) = page.size_pt();

        let enc = compress::encode_page_image(&page.image, dpi, opts.color, opts.quality)?;
        hasher.update(&enc.data);
        let data = self.seal(enc.data)?;
        {
            let mut img = self.pdf.image_xobject(image_id, &data);
            img.width(enc.width as i32).height(enc.height as i32);
            match enc.encoding {
                Encoding::Jpeg { gray } => {
                    img.filter(Filter::DctDecode);
                    img.bits_per_component(8);
                    if gray {
                        img.color_space().device_gray();
                    } else {
                        img.color_space().device_rgb();
                    }
                }
                Encoding::CcittG4 => {
                    img.filter(Filter::CcittFaxDecode);
                    img.bits_per_component(1);
                    img.color_space().device_gray();
                    img.decode_parms()
                        .pair(Name(b"K"), -1)
                        .pair(Name(b"Columns"), enc.width as i32)
                        .pair(Name(b"Rows"), enc.height as i32)
                        .pair(Name(b"BlackIs1"), false);
                }
            }
        }

        let mut content = Content::new();
        content
            .save_state()
            .transform([w_pt, 0.0, 0.0, h_pt, 0.0, 0.0])
            .x_object(Name(b"Im0"))
            .restore_state();
        let text_drawn = match (font, &page.ocr, opts.no_text_layer) {
            (Some(_), Some(ocr), false) => {
                text_layer(&mut content, ocr, 72.0 / dpi, h_pt);
                true
            }
            _ => false,
        };
        let raw = content.finish();
        let data = self.seal(compress::deflate(&raw))?;
        self.pdf.stream(content_id, &data).filter(Filter::FlateDecode);

        let mut p = self.pdf.page(page_id);
        p.parent(tree)
            .media_box(Rect::new(0.0, 0.0, w_pt, h_pt))
            .contents(content_id);
        let mut res = p.resources();
        res.x_objects().pair(Name(b"Im0"), image_id);
        if let (true, Some(f)) = (text_drawn, font) {
            res.fonts().pair(Name(b"F0"), f.type0);
        }
        res.finish();
        p.finish();
        Ok(page_id)
    }
}

/// Vẽ lớp chữ ẩn: mỗi dòng OCR là một lệnh `Tj` ở chế độ tô 3, giãn ngang (Tz) cho khớp bề rộng dòng, xoay
/// theo cạnh dưới tứ giác. `s` = point/điểm ảnh, `h_pt` = cao trang (đổi trục y).
fn text_layer(content: &mut Content, ocr: &OcrPageResult, s: f32, h_pt: f32) {
    content.begin_text();
    content.set_text_rendering_mode(TextRenderingMode::Invisible);
    for line in &ocr.lines {
        let text = line.text.trim();
        if text.is_empty() {
            continue;
        }
        let [tl, tr, br, bl] = line.quad.pts;
        let to_pt = |p: tsuscan_imaging::Point| (p.x as f32 * s, h_pt - p.y as f32 * s);
        let (blx, bly) = to_pt(bl);
        let (brx, bry) = to_pt(br);
        let (tlx, tly) = to_pt(tl);
        let (trx, try_) = to_pt(tr);
        let base_len = (brx - blx).hypot(bry - bly);
        let top_len = (trx - tlx).hypot(try_ - tly);
        let len = f32::midpoint(base_len, top_len);
        let height = f32::midpoint((tlx - blx).hypot(tly - bly), (trx - brx).hypot(try_ - bry));
        if !(len.is_finite() && height.is_finite()) || len < 0.5 || height < 0.5 {
            continue;
        }
        let angle = (bry - bly).atan2(brx - blx);
        let (sin, cos) = angle.sin_cos();
        let size = height;
        let n = text.chars().count() as f32;
        let natural = n * f32::from(font::GLYPH_WIDTH) / 1000.0 * size;
        let tz = (len / natural * 100.0).clamp(1.0, 10_000.0);
        // Gốc dòng chữ: góc dưới-trái nâng 15% chiều cao (chừa phần chân chữ g, p, y).
        let (ox, oy) = (blx - sin * 0.15 * height, bly + cos * 0.15 * height);
        content.set_font(Name(b"F0"), size);
        content.set_horizontal_scaling(tz);
        content.set_text_matrix([cos, sin, -sin, cos, ox, oy]);
        content.show(Str(&font::encode_text(text)));
    }
    content.end_text();
}

/// Chèn `/Encrypt n 0 R` vào từ điển trailer (sau bảng xref nên không đổi offset đối tượng).
fn inject_encrypt(bytes: &mut Vec<u8>, id: Ref) -> Result<()> {
    let marker = b"trailer\n<<";
    let pos = bytes
        .windows(marker.len())
        .rposition(|w| w == marker)
        .ok_or(DocError::Encode("không tìm thấy trailer".into()))?;
    let insert = format!("\n  /Encrypt {} 0 R", id.get());
    let at = pos + marker.len();
    bytes.splice(at..at, insert.into_bytes());
    Ok(())
}

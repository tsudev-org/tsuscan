//! Nhập PDF (dữ liệu KHÔNG tin cậy): đọc + vẽ từng trang thành ảnh bằng `hayro` (thuần Rust, crate gốc
//! `forbid(unsafe_code)`, có giải mã RC4/AES-128/AES-256).
//!
//! Vẽ thành ảnh thay vì giữ nguyên đối tượng PDF: trang nhập đi chung đường với ảnh chụp (lọc, OCR, che, xuất lại),
//! và mọi thứ ẩn trong tệp gốc (JavaScript, tệp đính kèm, lớp chữ cũ, siêu dữ liệu) bị bỏ hẳn - cần cho "che thông
//! tin xóa hẳn" (`redact`). Giới hạn: ≤ 256 MiB, ≤ [`MAX_IMPORT_PAGES`] trang, ≤ 40 MP/trang, chặn bom giải nén
//! ([`super::guard`]); mọi panic của bộ đọc
//! được chặn tại biên (phòng thủ nhiều lớp, fuzz ở `fuzz/fuzz_targets/pdf_import.rs`).

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use hayro::hayro_syntax::Pdf;
use hayro::{PixmapSettings, RenderCache, RenderSettings, hayro_interpret::InterpreterSettings};
use image::{DynamicImage, RgbImage};

use super::guard;
use crate::error::{DocError, Result};
use crate::model::Page;

/// Dung lượng PDF tối đa nhận vào.
pub const MAX_PDF_BYTES: usize = 256 * 1024 * 1024;
/// Số trang tối đa nhập một lần.
pub const MAX_IMPORT_PAGES: usize = 1000;
/// Điểm ảnh tối đa mỗi trang vẽ ra.
pub const MAX_PAGE_PIXELS: f64 = 40_000_000.0;
/// Cạnh tối đa mỗi trang vẽ ra.
pub const MAX_PAGE_SIDE: f64 = 14_000.0;

/// Tùy chọn nhập PDF.
#[derive(Debug, Clone)]
pub struct ImportOptions {
    /// DPI vẽ trang (mặc định 200: đủ cho OCR, nhẹ bộ nhớ).
    pub dpi: f32,
    /// Mật khẩu (rỗng = không mật khẩu).
    pub password: String,
    /// Chỉ nhập các trang này (đánh số từ 0); rỗng = tất cả.
    pub pages: Vec<usize>,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            dpi: 200.0,
            password: String::new(),
            pages: Vec::new(),
        }
    }
}

/// `LoadPdfError`/`DecryptionError` của hayro 0.9 nằm trong module riêng tư (không đặt tên được từ ngoài) ⇒ phân
/// loại theo tên biến thể ở dạng `Debug`. Test `aes256_password_is_required...` canh khi nâng phiên bản hayro.
fn map_load_error(e: &impl std::fmt::Debug, had_password: bool) -> DocError {
    let s = format!("{e:?}");
    if s.contains("PasswordProtected") {
        DocError::PasswordRequired
    } else if s.contains("UnsupportedAlgorithm") {
        DocError::UnsupportedEncryption
    } else if s.starts_with("Decryption") && had_password {
        DocError::PasswordRequired
    } else {
        DocError::InvalidPdf
    }
}

fn open(bytes: &[u8], password: &str) -> Result<Pdf> {
    if bytes.len() > MAX_PDF_BYTES {
        return Err(DocError::LimitExceeded("PDF lớn hơn 256 MiB"));
    }
    guard::scan_raw(bytes)?;
    let data = Arc::new(bytes.to_vec());
    let pw = password.to_owned();
    let pdf = catch_unwind(AssertUnwindSafe(|| Pdf::new_with_password(data, &pw)))
        .map_err(|_| DocError::InvalidPdf)?
        .map_err(|e| map_load_error(&e, !password.is_empty()))?;
    catch_unwind(AssertUnwindSafe(|| guard::check_objects(&pdf))).map_err(|_| DocError::InvalidPdf)??;
    Ok(pdf)
}

/// Có phải PDF không (theo chữ ký `%PDF-` trong 1 KiB đầu, như các trình đọc phổ biến).
#[must_use]
pub fn is_pdf(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(1024)];
    head.windows(5).any(|w| w == b"%PDF-")
}

/// Đếm số trang.
///
/// # Errors
/// [`DocError::InvalidPdf`], [`DocError::PasswordRequired`], [`DocError::LimitExceeded`].
pub fn page_count(bytes: &[u8], password: &str) -> Result<usize> {
    let pdf = open(bytes, password)?;
    Ok(pdf.pages().len())
}

/// Vẽ các trang PDF thành ảnh RGB (nền trắng), DPI theo tùy chọn (tự hạ nếu trang quá lớn).
///
/// # Errors
/// - [`DocError::InvalidPdf`] khi tệp hỏng/không có trang/kích thước trang vô lý.
/// - [`DocError::PasswordRequired`], [`DocError::UnsupportedEncryption`].
/// - [`DocError::LimitExceeded`] khi quá dung lượng hoặc số trang.
/// - [`DocError::InvalidArgument`] khi chỉ số trang ngoài phạm vi.
pub fn import_pdf(bytes: &[u8], opts: &ImportOptions) -> Result<Vec<Page>> {
    let pdf = open(bytes, &opts.password)?;
    let pages = pdf.pages();
    if pages.is_empty() {
        return Err(DocError::InvalidPdf);
    }
    let wanted: Vec<usize> = if opts.pages.is_empty() {
        (0..pages.len()).collect()
    } else {
        opts.pages.clone()
    };
    if wanted.len() > MAX_IMPORT_PAGES {
        return Err(DocError::LimitExceeded("quá 1.000 trang"));
    }
    let dpi = if opts.dpi.is_finite() {
        opts.dpi.clamp(36.0, 600.0)
    } else {
        200.0
    };
    let cache = RenderCache::new();
    let interp = InterpreterSettings::default();
    let render_settings = RenderSettings::default();
    let mut out = Vec::with_capacity(wanted.len());
    for idx in wanted {
        let page = pages
            .get(idx)
            .ok_or(DocError::InvalidArgument("số trang ngoài phạm vi"))?;
        let (w_pt, h_pt) = page.render_dimensions();
        let (w_pt, h_pt) = (f64::from(w_pt), f64::from(h_pt));
        if !(w_pt.is_finite() && h_pt.is_finite()) || w_pt < 1.0 || h_pt < 1.0 {
            return Err(DocError::InvalidPdf);
        }
        let mut scale = f64::from(dpi) / 72.0;
        scale = scale
            .min(MAX_PAGE_SIDE / w_pt)
            .min(MAX_PAGE_SIDE / h_pt)
            .min((MAX_PAGE_PIXELS / (w_pt * h_pt)).sqrt());
        // hayro cắt phần lẻ khi nhân tỉ lệ ⇒ tính tỉ lệ x/y riêng cho đúng số điểm ảnh làm tròn (A4 300 DPI = 3508,
        // không phải 3507).
        let pw = (w_pt * scale).round().max(1.0);
        let ph = (h_pt * scale).round().max(1.0);
        let px = PixmapSettings {
            x_scale: ((pw + 0.01) / w_pt) as f32,
            y_scale: ((ph + 0.01) / h_pt) as f32,
            bg_color: hayro::vello_cpu::color::palette::css::WHITE,
        };
        let pixmap = catch_unwind(AssertUnwindSafe(|| {
            hayro::render(page, &cache, &interp, &render_settings, &px)
        }))
        .map_err(|_| DocError::InvalidPdf)?;
        let (w, h) = (u32::from(pixmap.width()), u32::from(pixmap.height()));
        let rgba = pixmap.data_as_u8_slice();
        // Nền trắng đục ⇒ RGBA nhân trước alpha = RGB thẳng.
        let rgb: Vec<u8> = rgba
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect();
        let img = RgbImage::from_raw(w, h, rgb).ok_or(DocError::InvalidPdf)?;
        out.push(Page::new(DynamicImage::ImageRgb8(img)).with_dpi((scale * 72.0) as f32));
    }
    Ok(out)
}

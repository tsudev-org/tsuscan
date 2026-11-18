//! Tầng tài liệu của Tsu Scan (P3).
//!
//! - [`pdf`]: ghi PDF có lớp chữ OCR ẩn tìm được, PDF/A-2b, mật khẩu AES-256; nhập PDF (vẽ thành ảnh).
//! - [`compress`]: nén thông minh (JPEG màu/xám, CCITT G4 đen trắng).
//! - [`export`]: DOCX, XLSX, TXT, Markdown, JPEG/PNG/WebP.
//! - [`import`]: nhận dạng + nhập PDF/ảnh (HEIC qua bộ giải mã hệ điều hành).
//! - [`redact`]: che thông tin xóa hẳn dữ liệu; [`signature`]: chữ ký tay; [`barcode`]: QR/mã vạch.
//!
//! Mọi đầu vào không tin cậy trả `Err`, không panic. Quyết định thiết kế: `docs/adr/0008-*`.

// Bộ dựng XML/PDF nối chuỗi rất nhiều: `push_str(&format!(..))` dễ đọc hơn `write!` + bỏ kết quả.
#![allow(clippy::format_push_string)]

pub mod barcode;
pub mod compress;
pub mod error;
pub mod export;
pub mod import;
pub mod model;
pub mod pdf;
pub mod redact;
pub mod signature;

pub use compress::{ColorMode, Quality};
pub use error::{DocError, Result};
pub use model::{Document, Metadata, Page, Timestamp};
pub use pdf::{PdfOptions, Permissions, Protection, write_pdf};

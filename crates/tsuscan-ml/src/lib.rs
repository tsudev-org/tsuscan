//! AI trên thiết bị của Tsu Scan.
//!
//! - [`manifest`] - manifest mô hình + nạp tệp có kiểm sha256/chữ ký (tệp mô hình là dữ liệu không tin cậy).
//! - [`runtime`] - suy luận ONNX bằng tract (Rust thuần, build từ nguồn, chạy được WASM).
//! - [`ocr`] - dò dòng + nhận dạng chữ tiếng Việt/Anh, thứ tự đọc.
//! - [`orientation`] - hướng trang 0/90/180/270.
//! - [`dewarp`] - làm phẳng trang cong (UVDoc).
//! - [`pages`] - tách hai trang sách, xóa ngón tay (cổ điển).
//! - [`layout`] - bố cục đoạn/bảng từ dòng OCR.
//! - [`metrics`] - CER/WER.
//!
//! Không I/O mạng: tải mô hình là việc của app/script, crate này chỉ đọc tệp đã có và kiểm toàn vẹn (ADR-0002).

pub mod dewarp;
pub mod error;
pub mod layout;
pub mod manifest;
pub mod metrics;
pub mod ocr;
pub mod orientation;
/// Chuyển sang `tsuscan_imaging::pages` (thuật toán cổ điển, app dùng không cần kéo theo tract); giữ đường dẫn cũ.
pub use tsuscan_imaging::pages;
pub mod prep;
pub mod runtime;
#[cfg(feature = "synth")]
pub mod synth;

pub use error::{MlError, Result};
pub use manifest::{Manifest, ModelStore};
pub use ocr::{OcrEngine, OcrPageResult};

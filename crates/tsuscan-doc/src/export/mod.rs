//! Xuất tài liệu sang định dạng sửa được và ảnh: DOCX, XLSX, TXT, Markdown, JPEG/PNG/WebP.
//!
//! Chữ lấy từ kết quả OCR + phân tích bố cục (`tsuscan_ml::layout`): tiêu đề, đoạn văn, bảng.

pub mod docx;
pub mod image;
pub mod text;
pub mod xlsx;
pub mod zip;

use tsuscan_ml::layout::{self, Block};

use crate::model::Page;

/// Khối bố cục của một trang (rỗng khi trang chưa OCR).
#[must_use]
pub fn page_blocks(page: &Page) -> Vec<Block> {
    page.ocr
        .as_ref()
        .map(|o| layout::analyze(o, f64::from(page.image.width())))
        .unwrap_or_default()
}

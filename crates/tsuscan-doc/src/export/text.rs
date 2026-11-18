//! Xuất TXT (chữ thuần theo thứ tự đọc) và Markdown (giữ tiêu đề, đoạn, bảng).

use tsuscan_ml::layout;

use super::page_blocks;
use crate::model::Document;

/// Ký tự ngắt trang trong TXT (form feed, giống `pdftotext`).
pub const PAGE_BREAK: char = '\u{000C}';

/// Chữ thuần của cả tài liệu: các dòng theo thứ tự đọc, trang ngăn bằng form feed.
#[must_use]
pub fn to_txt(doc: &Document) -> String {
    doc.pages
        .iter()
        .map(|p| {
            p.ocr
                .as_ref()
                .map(tsuscan_ml::OcrPageResult::text)
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(&format!("\n{PAGE_BREAK}"))
}

/// Markdown của cả tài liệu: tiêu đề tài liệu (nếu có) rồi từng trang, ngăn bằng `---`.
#[must_use]
pub fn to_markdown(doc: &Document) -> String {
    let mut out = String::new();
    if !doc.meta.title.trim().is_empty() {
        out.push_str("# ");
        out.push_str(doc.meta.title.trim());
        out.push_str("\n\n");
    }
    let pages: Vec<String> = doc
        .pages
        .iter()
        .map(|p| layout::to_markdown(&page_blocks(p)))
        .collect();
    out.push_str(&pages.join("\n\n---\n\n"));
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

//! Tìm kiếm FTS5 không dấu (`.standards/docs/SEARCH_AND_FILTER.md` mục 3-5).
//!
//! Mỗi tài liệu một dòng trong bảng `search` (rowid = rowid của `documents`). Khớp trên cột không dấu, tiền tố từng từ,
//! AND giữa các từ; điểm = bm25 (tiêu đề nặng gấp 10 nội dung) × 0,8 nếu bản có dấu không khớp. Hòa điểm ⇒ tài liệu
//! sửa gần hơn trước, rồi theo id (thứ tự xác định).

use std::collections::HashSet;

use rusqlite::{Connection, params};

use crate::error::Result;
use crate::normalize::{vi_fold_case, vi_normalize};

/// Số ký tự tối thiểu của truy vấn (sau chuẩn hóa).
pub const MIN_QUERY_CHARS: usize = 2;
/// Hệ số khi chỉ khớp bản không dấu.
pub const ACCENTLESS_FACTOR: f64 = 0.8;

/// Một kết quả tìm kiếm.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    /// Id tài liệu.
    pub document_id: String,
    /// Tiêu đề (gốc, có dấu - để hiển thị/tô sáng bằng [`crate::normalize::vi_normalize_with_map`]).
    pub title: String,
    /// Điểm (lớn hơn = liên quan hơn).
    pub score: f64,
    /// Khớp đúng cả dấu.
    pub accent_exact: bool,
}

/// Lập lại dòng chỉ mục của tài liệu từ tiêu đề + chữ OCR các trang chưa xóa. Tài liệu đã xóa ⇒ bỏ dòng.
pub(super) fn reindex(conn: &Connection, document_id: &str) -> Result<()> {
    let (rowid, title, deleted): (i64, String, i64) = conn.query_row(
        "SELECT rowid, title, deleted FROM documents WHERE id = ?1",
        [document_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    conn.execute("DELETE FROM search WHERE rowid = ?1", [rowid])?;
    if deleted != 0 {
        return Ok(());
    }
    let mut st =
        conn.prepare("SELECT ocr_text FROM pages WHERE document_id = ?1 AND deleted = 0 ORDER BY position")?;
    let body = st
        .query_map([document_id], |r| r.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?
        .join("\n");
    conn.execute(
        "INSERT INTO search(rowid, title_norm, body_norm, title_acc, body_acc) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            rowid,
            vi_normalize(&title),
            vi_normalize(&body),
            vi_fold_case(&title),
            vi_fold_case(&body)
        ],
    )?;
    Ok(())
}

/// Tách từ giống bộ tách `unicode61` (chữ/số liên tiếp) rồi dựng biểu thức FTS5 tiền tố, AND giữa các từ.
fn fts_query(columns: &str, text: &str) -> Option<String> {
    let terms: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{}\"*", t.replace('"', "\"\"")))
        .collect();
    if terms.is_empty() {
        None
    } else {
        Some(format!("{{{columns}}} : ({})", terms.join(" AND ")))
    }
}

pub(super) fn search(conn: &Connection, query: &str, limit: usize) -> Result<Vec<SearchHit>> {
    let norm = vi_normalize(query);
    if norm.chars().filter(|c| !c.is_whitespace()).count() < MIN_QUERY_CHARS || limit == 0 {
        return Ok(Vec::new());
    }
    let Some(q_norm) = fts_query("title_norm body_norm", &norm) else {
        return Ok(Vec::new());
    };
    let exact: HashSet<i64> = match fts_query("title_acc body_acc", &vi_fold_case(query)) {
        Some(q) => {
            let mut st = conn.prepare("SELECT rowid FROM search WHERE search MATCH ?1")?;
            st.query_map([q], |r| r.get(0))?
                .collect::<std::result::Result<_, _>>()?
        }
        None => HashSet::new(),
    };
    let mut st = conn.prepare(
        "SELECT s.rowid, d.id, d.title, d.updated_ms, bm25(search, 10.0, 1.0, 0.0, 0.0)
         FROM search s JOIN documents d ON d.rowid = s.rowid
         WHERE search MATCH ?1 AND d.deleted = 0",
    )?;
    let mut hits: Vec<(SearchHit, i64)> = st
        .query_map([q_norm], |r| {
            let rowid: i64 = r.get(0)?;
            let bm25: f64 = r.get(4)?;
            let accent_exact = exact.contains(&rowid);
            let factor = if accent_exact { 1.0 } else { ACCENTLESS_FACTOR };
            // bm25 của SQLite: âm, càng nhỏ càng liên quan.
            Ok((
                SearchHit {
                    document_id: r.get(1)?,
                    title: r.get(2)?,
                    score: -bm25 * factor,
                    accent_exact,
                },
                r.get(3)?,
            ))
        })?
        .collect::<std::result::Result<_, _>>()?;
    hits.sort_by(|(a, au), (b, bu)| {
        b.score
            .total_cmp(&a.score)
            .then(bu.cmp(au))
            .then_with(|| a.document_id.cmp(&b.document_id))
    });
    hits.truncate(limit);
    Ok(hits.into_iter().map(|(h, _)| h).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_builder_escapes_and_splits() {
        assert_eq!(
            fts_query("c", "hop dong").as_deref(),
            Some("{c} : (\"hop\"* AND \"dong\"*)")
        );
        assert_eq!(
            fts_query("c", "a\"b OR c*").as_deref(),
            Some("{c} : (\"a\"* AND \"b\"* AND \"OR\"* AND \"c\"*)")
        );
        assert_eq!(fts_query("c", " ,.; "), None);
    }
}

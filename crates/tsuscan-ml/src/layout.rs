//! Bố cục trang từ kết quả OCR: gom hàng thành đoạn văn và bảng (hình học thuần, không cần mô hình).
//!
//! - Hàng có ≥ 3 ô tách rời (khoảng trống ngang lớn) là hàng dạng bảng; ≥ 2 hàng dạng bảng liền nhau có cột thẳng
//!   hàng ⇒ một bảng. Cột gom theo mép trái các ô.
//! - Hàng còn lại gom thành đoạn khi khoảng cách dọc nhỏ và lề trái gần nhau; hàng căn giữa đứng riêng (tiêu đề).
//!
//! Dùng cho xuất DOCX/Markdown (P3) và hiển thị chữ OCR có cấu trúc (P7).

use crate::ocr::{OcrLine, OcrPageResult};

/// Một khối bố cục.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    /// Đoạn văn (các dòng nối bằng khoảng trắng).
    Paragraph(String),
    /// Tiêu đề / dòng căn giữa đứng riêng.
    Heading(String),
    /// Bảng: hàng × cột (ô trống là chuỗi rỗng).
    Table(Vec<Vec<String>>),
}

#[derive(Debug, Clone)]
struct Row {
    cells: Vec<(f64, f64, String)>,
    y0: f64,
    y1: f64,
}

fn x_bounds(l: &OcrLine) -> (f64, f64) {
    let xs = l.quad.pts.map(|p| p.x);
    (
        xs.into_iter().fold(f64::INFINITY, f64::min),
        xs.into_iter().fold(f64::NEG_INFINITY, f64::max),
    )
}

fn y_bounds(l: &OcrLine) -> (f64, f64) {
    let ys = l.quad.pts.map(|p| p.y);
    (
        ys.into_iter().fold(f64::INFINITY, f64::min),
        ys.into_iter().fold(f64::NEG_INFINITY, f64::max),
    )
}

/// Phân tích bố cục. `page_width` dùng để nhận dòng căn giữa.
#[must_use]
pub fn analyze(page: &OcrPageResult, page_width: f64) -> Vec<Block> {
    let mut rows: Vec<Row> = Vec::new();
    let mut last_row = None;
    for l in page.lines.iter().filter(|l| !l.text.is_empty()) {
        let (x0, x1) = x_bounds(l);
        let (y0, y1) = y_bounds(l);
        match rows.last_mut() {
            Some(r) if last_row == Some(l.row) => {
                r.cells.push((x0, x1, l.text.clone()));
                r.y0 = r.y0.min(y0);
                r.y1 = r.y1.max(y1);
            }
            _ => rows.push(Row {
                cells: vec![(x0, x1, l.text.clone())],
                y0,
                y1,
            }),
        }
        last_row = Some(l.row);
    }
    let mut heights: Vec<f64> = rows.iter().map(|r| r.y1 - r.y0).collect();
    heights.sort_by(f64::total_cmp);
    let line_h = heights.get(heights.len() / 2).copied().unwrap_or(20.0).max(1.0);

    let tabular: Vec<bool> = rows.iter().map(|r| is_tabular(r, line_h)).collect();
    let mut blocks = Vec::new();
    let mut i = 0;
    let mut para: Vec<String> = Vec::new();
    let mut para_left = 0.0;
    let mut prev_bottom = f64::NEG_INFINITY;
    let flush = |para: &mut Vec<String>, blocks: &mut Vec<Block>| {
        if !para.is_empty() {
            blocks.push(Block::Paragraph(para.join(" ")));
            para.clear();
        }
    };
    while i < rows.len() {
        // Bảng: chuỗi ≥ 2 hàng dạng bảng liền nhau.
        let mut j = i;
        while j < rows.len() && tabular[j] {
            j += 1;
        }
        if j - i >= 2 {
            flush(&mut para, &mut blocks);
            blocks.push(Block::Table(build_table(&rows[i..j], line_h)));
            prev_bottom = rows[j - 1].y1;
            i = j;
            continue;
        }
        let r = &rows[i];
        let text = r.cells.iter().map(|c| c.2.as_str()).collect::<Vec<_>>().join(" ");
        let left = r.cells.first().map_or(0.0, |c| c.0);
        let right = r.cells.last().map_or(0.0, |c| c.1);
        let centered = (f64::midpoint(left, right) - page_width / 2.0).abs() < page_width * 0.04
            && left > page_width * 0.15
            && r.cells.len() == 1;
        let gap = r.y0 - prev_bottom;
        if centered {
            flush(&mut para, &mut blocks);
            blocks.push(Block::Heading(text));
        } else {
            if gap > 0.9 * line_h || (left - para_left).abs() > 1.5 * line_h {
                flush(&mut para, &mut blocks);
            }
            if para.is_empty() {
                para_left = left;
            }
            para.push(text);
        }
        prev_bottom = r.y1;
        i += 1;
    }
    flush(&mut para, &mut blocks);
    blocks
}

fn is_tabular(r: &Row, line_h: f64) -> bool {
    if r.cells.len() < 3 {
        return false;
    }
    r.cells.windows(2).all(|w| w[1].0 - w[0].1 > 0.8 * line_h)
}

fn build_table(rows: &[Row], line_h: f64) -> Vec<Vec<String>> {
    // Cột = cụm mép trái các ô (sai lệch ≤ 1,5 chiều cao dòng).
    let mut cols: Vec<f64> = Vec::new();
    for r in rows {
        for c in &r.cells {
            if !cols.iter().any(|&x| (x - c.0).abs() <= 1.5 * line_h) {
                cols.push(c.0);
            }
        }
    }
    cols.sort_by(f64::total_cmp);
    rows.iter()
        .map(|r| {
            let mut out = vec![String::new(); cols.len()];
            for c in &r.cells {
                let k = cols
                    .iter()
                    .enumerate()
                    .min_by(|a, b| (a.1 - c.0).abs().total_cmp(&(b.1 - c.0).abs()))
                    .map_or(0, |(k, _)| k);
                if out[k].is_empty() {
                    out[k].clone_from(&c.2);
                } else {
                    out[k] = format!("{} {}", out[k], c.2);
                }
            }
            out
        })
        .collect()
}

/// Xuất Markdown (tiêu đề `##`, đoạn, bảng GFM).
#[must_use]
pub fn to_markdown(blocks: &[Block]) -> String {
    let esc = |s: &str| s.replace('|', "\\|");
    let mut out = Vec::new();
    for b in blocks {
        match b {
            Block::Heading(t) => out.push(format!("## {t}")),
            Block::Paragraph(t) => out.push(t.clone()),
            Block::Table(rows) => {
                let n = rows.iter().map(Vec::len).max().unwrap_or(0);
                let mut t = Vec::new();
                for (i, r) in rows.iter().enumerate() {
                    let cells: Vec<String> =
                        (0..n).map(|k| esc(r.get(k).map_or("", String::as_str))).collect();
                    t.push(format!("| {} |", cells.join(" | ")));
                    if i == 0 {
                        t.push(format!("|{}|", vec![" --- "; n].join("|")));
                    }
                }
                out.push(t.join("\n"));
            }
        }
    }
    out.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tsuscan_imaging::{Point, Quad};

    fn line(text: &str, x0: f64, y0: f64, x1: f64, row: usize) -> OcrLine {
        let quad = Quad::from_unordered([
            Point { x: x0, y: y0 },
            Point { x: x1, y: y0 },
            Point { x: x1, y: y0 + 20.0 },
            Point { x: x0, y: y0 + 20.0 },
        ])
        .unwrap();
        OcrLine {
            text: text.into(),
            quad,
            confidence: 1.0,
            row,
        }
    }

    #[test]
    fn doan_tieu_de_va_bang() {
        let lines = vec![
            line("HỢP ĐỒNG", 400.0, 20.0, 600.0, 0),
            line("Dòng một của đoạn", 100.0, 80.0, 900.0, 1),
            line("dòng hai của đoạn.", 100.0, 105.0, 500.0, 2),
            line("Đoạn mới sau khoảng trống", 100.0, 180.0, 900.0, 3),
            line("STT", 100.0, 260.0, 140.0, 4),
            line("Tên hàng", 200.0, 260.0, 400.0, 4),
            line("Thành tiền", 700.0, 260.0, 850.0, 4),
            line("1", 100.0, 290.0, 115.0, 5),
            line("Giấy A4", 200.0, 290.0, 330.0, 5),
            line("850.000", 700.0, 290.0, 800.0, 5),
        ];
        let blocks = analyze(&OcrPageResult { lines }, 1000.0);
        assert_eq!(
            blocks,
            vec![
                Block::Heading("HỢP ĐỒNG".into()),
                Block::Paragraph("Dòng một của đoạn dòng hai của đoạn.".into()),
                Block::Paragraph("Đoạn mới sau khoảng trống".into()),
                Block::Table(vec![
                    vec!["STT".into(), "Tên hàng".into(), "Thành tiền".into()],
                    vec!["1".into(), "Giấy A4".into(), "850.000".into()],
                ]),
            ]
        );
        let md = to_markdown(&blocks);
        assert!(md.starts_with("## HỢP ĐỒNG\n\nDòng một"));
        assert!(
            md.contains("| STT | Tên hàng | Thành tiền |\n| --- | --- | --- |\n| 1 | Giấy A4 | 850.000 |")
        );
    }

    #[test]
    fn trang_rong() {
        assert_eq!(analyze(&OcrPageResult::default(), 100.0), Vec::<Block>::new());
        assert_eq!(
            to_markdown(&[Block::Table(vec![vec!["a|b".into()]])]),
            "| a\\|b |\n| --- |"
        );
    }
}

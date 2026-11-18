//! OCR trên thiết bị: dò dòng (PP-OCRv5 det) → nắn từng dòng → nhận dạng (AsterNet) → thứ tự đọc.
//!
//! Lựa chọn engine và số đo: `docs/adr/0007-*`.

pub mod det;
pub mod rec;
mod vocab;

use image::RgbImage;
use tsuscan_imaging::Quad;

pub use det::{DetOptions, TextBox, TextDetector};
pub use rec::{LineText, TextRecognizer};

use crate::error::Result;
use crate::manifest::ModelStore;

/// Mã mô hình dò dòng trong manifest.
pub const DET_MODEL: &str = "ocr-det";
/// Mã mô hình nhận dạng trong manifest.
pub const REC_MODEL: &str = "ocr-rec-vi";

/// Một dòng đã nhận dạng.
#[derive(Debug, Clone)]
pub struct OcrLine {
    /// Chữ.
    pub text: String,
    /// Tứ giác trên ảnh trang.
    pub quad: Quad,
    /// Tin cậy nhận dạng 0..1.
    pub confidence: f32,
    /// Chỉ số hàng (các dòng cùng hàng ngang có cùng chỉ số, ví dụ hai cột chữ ký).
    pub row: usize,
}

/// Kết quả OCR một trang.
#[derive(Debug, Clone, Default)]
pub struct OcrPageResult {
    /// Các dòng theo thứ tự đọc (trên → dưới, trái → phải).
    pub lines: Vec<OcrLine>,
}

impl OcrPageResult {
    /// Chữ thuần: mỗi hàng một dòng, các ô cùng hàng cách nhau một khoảng trắng.
    #[must_use]
    pub fn text(&self) -> String {
        let mut out = String::new();
        let mut cur_row = None;
        for l in &self.lines {
            if l.text.is_empty() {
                continue;
            }
            match cur_row {
                Some(r) if r == l.row => out.push(' '),
                Some(_) => out.push('\n'),
                None => {}
            }
            out.push_str(&l.text);
            cur_row = Some(l.row);
        }
        out
    }
}

/// Engine OCR đầy đủ.
#[derive(Debug)]
pub struct OcrEngine {
    det: TextDetector,
    rec: TextRecognizer,
}

impl OcrEngine {
    /// Dựng từ hai bộ byte mô hình đã kiểm.
    ///
    /// # Errors
    /// Mô hình không dịch được.
    pub fn from_bytes(det: &[u8], rec: &[u8], opts: DetOptions) -> Result<Self> {
        Ok(Self {
            det: TextDetector::from_bytes(det, opts)?,
            rec: TextRecognizer::from_bytes(rec)?,
        })
    }

    /// Nạp từ kho mô hình (kiểm kích thước + sha256 + chữ ký nếu bật).
    ///
    /// # Errors
    /// Thiếu tệp, sai toàn vẹn, hoặc mô hình không dịch được.
    pub fn from_store(store: &ModelStore) -> Result<Self> {
        Self::from_bytes(
            &store.load_verified(DET_MODEL)?,
            &store.load_verified(REC_MODEL)?,
            DetOptions::default(),
        )
    }

    /// Bộ dò dòng.
    #[must_use]
    pub fn detector(&self) -> &TextDetector {
        &self.det
    }

    /// Bộ nhận dạng.
    #[must_use]
    pub fn recognizer(&self) -> &TextRecognizer {
        &self.rec
    }

    /// OCR một trang (ảnh đã nắn phối cảnh, đúng chiều).
    ///
    /// # Errors
    /// Lỗi suy luận.
    pub fn recognize(&self, page: &RgbImage) -> Result<OcrPageResult> {
        crate::prep::check_image(page)?;
        let boxes = self.det.detect(page)?;
        let ordered = reading_order(boxes.into_iter().map(|b| b.quad).collect());
        let mut lines = Vec::with_capacity(ordered.len());
        for (quad, row) in ordered {
            let t = self.rec.recognize_quad(page, &quad)?;
            lines.push(OcrLine {
                text: t.text,
                quad,
                confidence: t.confidence,
                row,
            });
        }
        Ok(OcrPageResult { lines })
    }
}

fn bounds(q: &Quad) -> (f64, f64, f64, f64) {
    let xs = q.pts.map(|p| p.x);
    let ys = q.pts.map(|p| p.y);
    let min = |v: [f64; 4]| v.into_iter().fold(f64::INFINITY, f64::min);
    let max = |v: [f64; 4]| v.into_iter().fold(f64::NEG_INFINITY, f64::max);
    (min(xs), min(ys), max(xs), max(ys))
}

/// Sắp các hộp theo thứ tự đọc: gom thành hàng khi chồng lấn dọc ≥ 50% chiều cao hộp thấp hơn,
/// hàng xếp trên → dưới, trong hàng trái → phải. Trả về (hộp, chỉ số hàng).
#[must_use]
pub fn reading_order(mut quads: Vec<Quad>) -> Vec<(Quad, usize)> {
    quads.sort_by(|a, b| {
        let (_, ay0, _, ay1) = bounds(a);
        let (_, by0, _, by1) = bounds(b);
        (ay0 + ay1).total_cmp(&(by0 + by1))
    });
    let mut rows: Vec<(f64, f64, Vec<Quad>)> = Vec::new();
    for q in quads {
        let (_, y0, _, y1) = bounds(&q);
        let hit = rows.iter_mut().rev().take(2).find(|(ry0, ry1, _)| {
            let overlap = y1.min(*ry1) - y0.max(*ry0);
            overlap >= 0.5 * (y1 - y0).min(*ry1 - *ry0)
        });
        match hit {
            Some((ry0, ry1, v)) => {
                *ry0 = ry0.min(y0);
                *ry1 = ry1.max(y1);
                v.push(q);
            }
            None => rows.push((y0, y1, vec![q])),
        }
    }
    rows.sort_by(|a, b| (a.0 + a.1).total_cmp(&(b.0 + b.1)));
    let mut out = Vec::new();
    for (i, (_, _, mut v)) in rows.into_iter().enumerate() {
        v.sort_by(|a, b| bounds(a).0.total_cmp(&bounds(b).0));
        out.extend(v.into_iter().map(|q| (q, i)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tsuscan_imaging::Point;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Quad {
        Quad::from_unordered([
            Point { x: x0, y: y0 },
            Point { x: x1, y: y0 },
            Point { x: x1, y: y1 },
            Point { x: x0, y: y1 },
        ])
        .unwrap()
    }

    #[test]
    fn thu_tu_doc_gom_hang() {
        let q = vec![
            rect(300.0, 52.0, 400.0, 70.0), // cùng hàng với ô trái, hơi lệch
            rect(10.0, 100.0, 200.0, 120.0),
            rect(10.0, 50.0, 200.0, 70.0),
        ];
        let o = reading_order(q);
        let xs: Vec<(f64, usize)> = o.iter().map(|(q, r)| (q.pts[0].x, *r)).collect();
        assert_eq!(xs, vec![(10.0, 0), (300.0, 0), (10.0, 1)]);
        let page = OcrPageResult {
            lines: o
                .into_iter()
                .zip(["A", "B", "C"])
                .map(|((quad, row), t)| OcrLine {
                    text: t.into(),
                    quad,
                    confidence: 1.0,
                    row,
                })
                .collect(),
        };
        assert_eq!(page.text(), "A B\nC");
    }
}

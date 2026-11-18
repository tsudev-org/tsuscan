//! Dò dòng chữ bằng PP-OCRv5 mobile det (DB: Differentiable Binarization) + hậu xử lý.

use image::{GrayImage, Luma, RgbImage};
use imageproc::region_labelling::{Connectivity, connected_components};
use tsuscan_imaging::{Point, Quad};

use crate::error::{MlError, Result};
use crate::prep::{Channels, Normalize, resize_limit, to_tensor};
use crate::runtime::{Dim, Session};

/// Tham số dò dòng chữ.
#[derive(Debug, Clone, Copy)]
pub struct DetOptions {
    /// Cạnh dài tối đa của ảnh đưa vào mô hình (px).
    pub limit_side: u32,
    /// Ngưỡng nhị phân bản đồ xác suất.
    pub thresh: f32,
    /// Điểm trung bình tối thiểu của một vùng.
    pub box_thresh: f32,
    /// Hệ số nở vùng (DB co vùng chữ khi huấn luyện, phải nở lại).
    pub unclip_ratio: f64,
    /// Cạnh ngắn tối thiểu của hộp sau khi nở (px ở ảnh mô hình).
    pub min_side: f64,
    /// Số vùng tối đa (giữ vùng điểm cao nhất) - chặn ảnh nhiễu sinh hàng vạn vùng làm nghẽn bước nhận dạng.
    pub max_candidates: usize,
}

impl Default for DetOptions {
    fn default() -> Self {
        Self {
            limit_side: 1280,
            thresh: 0.3,
            box_thresh: 0.6,
            unclip_ratio: 1.6,
            min_side: 3.0,
            max_candidates: 1000,
        }
    }
}

/// Một vùng chữ.
#[derive(Debug, Clone)]
pub struct TextBox {
    /// Tứ giác bao (TL, TR, BR, BL) trên ảnh gốc.
    pub quad: Quad,
    /// Điểm tin cậy trung bình 0..1.
    pub score: f32,
}

/// Bộ dò dòng chữ.
#[derive(Debug)]
pub struct TextDetector {
    session: Session,
    opts: DetOptions,
}

impl TextDetector {
    /// Dựng từ byte mô hình đã kiểm.
    ///
    /// # Errors
    /// Mô hình không dịch được.
    pub fn from_bytes(bytes: &[u8], opts: DetOptions) -> Result<Self> {
        let session = Session::from_bytes(
            "ocr-det",
            bytes,
            &[
                Dim::Fixed(1),
                Dim::Fixed(3),
                Dim::Multiple("h", 32),
                Dim::Multiple("w", 32),
            ],
        )?;
        Ok(Self { session, opts })
    }

    /// Có chạy một kế hoạch cho mọi kích thước ảnh (hình dạng ký hiệu) không.
    #[must_use]
    pub fn is_symbolic(&self) -> bool {
        self.session.is_symbolic()
    }

    /// Bản đồ xác suất chữ (ảnh xám 0..255) ở độ phân giải mô hình, kèm hệ số về ảnh gốc.
    ///
    /// # Errors
    /// Ảnh rỗng hoặc lỗi suy luận.
    pub fn probability_map(&self, img: &RgbImage) -> Result<(Vec<f32>, u32, u32, f64, f64)> {
        let (resized, sx, sy) = resize_limit(img, self.opts.limit_side, 32);
        let (w, h) = resized.dimensions();
        let input = to_tensor(&resized, Channels::Bgr, Normalize::IMAGENET)?;
        let out = self.session.run(input)?;
        let prob = out
            .first()
            .ok_or_else(|| MlError::Inference("ocr-det: không có đầu ra".into()))?;
        let view = prob
            .to_plain_array_view::<f32>()
            .map_err(|e| MlError::Inference(format!("{e:#}")))?;
        if view.len() != (w * h) as usize {
            return Err(MlError::Inference(format!(
                "ocr-det: đầu ra {:?} khác {w}x{h}",
                view.shape()
            )));
        }
        Ok((view.iter().copied().collect(), w, h, sx, sy))
    }

    /// Dò các vùng chữ.
    ///
    /// # Errors
    /// Ảnh rỗng hoặc lỗi suy luận.
    pub fn detect(&self, img: &RgbImage) -> Result<Vec<TextBox>> {
        let (prob, w, h, sx, sy) = self.probability_map(img)?;
        Ok(boxes_from_map(
            &prob,
            w,
            h,
            sx,
            sy,
            &self.opts,
            img.width(),
            img.height(),
        ))
    }
}

/// Hậu xử lý DB: nhị phân → thành phần liên thông → hình chữ nhật xoay nhỏ nhất → nở vùng → về tọa độ gốc.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn boxes_from_map(
    prob: &[f32],
    w: u32,
    h: u32,
    sx: f64,
    sy: f64,
    opts: &DetOptions,
    orig_w: u32,
    orig_h: u32,
) -> Vec<TextBox> {
    let bin = GrayImage::from_fn(w, h, |x, y| {
        Luma([u8::from(prob[(y * w + x) as usize] > opts.thresh)])
    });
    let labels = connected_components(&bin, Connectivity::Eight, Luma([0]));
    let mut comps: Vec<(Vec<imageproc::point::Point<i32>>, f64)> = Vec::new();
    for (x, y, l) in labels.enumerate_pixels() {
        let id = l.0[0] as usize;
        if id == 0 {
            continue;
        }
        if comps.len() < id {
            comps.resize_with(id, || (Vec::new(), 0.0));
        }
        comps[id - 1]
            .0
            .push(imageproc::point::Point::new(x as i32, y as i32));
        comps[id - 1].1 += f64::from(prob[(y * w + x) as usize]);
    }
    let mut out = Vec::new();
    for (pts, sum) in comps {
        if pts.len() < 4 {
            continue;
        }
        let score = (sum / pts.len() as f64) as f32;
        if score < opts.box_thresh {
            continue;
        }
        let rect = imageproc::geometry::min_area_rect(&pts);
        let p: Vec<Point> = rect
            .iter()
            .map(|q| Point {
                x: f64::from(q.x),
                y: f64::from(q.y),
            })
            .collect();
        let Some(quad) = unclip_rect(&p, opts.unclip_ratio, opts.min_side) else {
            continue;
        };
        let scaled: Vec<Point> = quad
            .iter()
            .map(|q| Point {
                x: ((q.x + 0.5) * sx).clamp(0.0, f64::from(orig_w)),
                y: ((q.y + 0.5) * sy).clamp(0.0, f64::from(orig_h)),
            })
            .collect();
        if let Ok(quad) = Quad::from_unordered([scaled[0], scaled[1], scaled[2], scaled[3]]) {
            out.push(TextBox { quad, score });
        }
    }
    if out.len() > opts.max_candidates {
        out.sort_by(|a, b| b.score.total_cmp(&a.score));
        out.truncate(opts.max_candidates);
    }
    out
}

/// Nở hình chữ nhật (4 đỉnh theo thứ tự vòng) một khoảng `d = diện tích * ratio / chu vi` về mọi phía.
fn unclip_rect(p: &[Point], ratio: f64, min_side: f64) -> Option<[Point; 4]> {
    if p.len() != 4 {
        return None;
    }
    // Pixel rời rạc: cạnh chữ nhật tính từ tâm điểm ảnh ⇒ cộng 1 cho mỗi chiều.
    let a = p[0].dist(p[1]) + 1.0;
    let b = p[1].dist(p[2]) + 1.0;
    let d = a * b * ratio / (2.0 * (a + b));
    let c = Point {
        x: p.iter().map(|q| q.x).sum::<f64>() / 4.0,
        y: p.iter().map(|q| q.y).sum::<f64>() / 4.0,
    };
    let unit = |u: Point, v: Point| {
        let (dx, dy) = (v.x - u.x, v.y - u.y);
        let n = dx.hypot(dy);
        if n < 1e-9 { (1.0, 0.0) } else { (dx / n, dy / n) }
    };
    let ua = unit(p[0], p[1]);
    let ub = unit(p[1], p[2]);
    let (ha, hb) = (a / 2.0 + d, b / 2.0 + d);
    if ha.min(hb) * 2.0 < min_side {
        return None;
    }
    let corner = |sa: f64, sb: f64| Point {
        x: c.x + sa * ha * ua.0 + sb * hb * ub.0,
        y: c.y + sa * ha * ua.1 + sb * hb * ub.1,
    };
    Some([
        corner(-1.0, -1.0),
        corner(1.0, -1.0),
        corner(1.0, 1.0),
        corner(-1.0, 1.0),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hau_xu_ly_tim_hai_vung_va_no_ra() {
        let (w, h) = (64u32, 32u32);
        let mut prob = vec![0f32; (w * h) as usize];
        for y in 10..14 {
            for x in 5..40 {
                prob[(y * w + x) as usize] = 0.9;
            }
        }
        for y in 20..24 {
            for x in 50..60 {
                prob[(y * w + x) as usize] = 0.4; // dưới box_thresh ⇒ bỏ
            }
        }
        let boxes = boxes_from_map(&prob, w, h, 2.0, 2.0, &DetOptions::default(), 128, 64);
        assert_eq!(boxes.len(), 1);
        let q = &boxes[0].quad;
        let xs: Vec<f64> = q.pts.iter().map(|p| p.x).collect();
        let ys: Vec<f64> = q.pts.iter().map(|p| p.y).collect();
        let (x0, x1) = (
            xs.iter().copied().fold(f64::MAX, f64::min),
            xs.iter().copied().fold(0.0, f64::max),
        );
        let (y0, y1) = (
            ys.iter().copied().fold(f64::MAX, f64::min),
            ys.iter().copied().fold(0.0, f64::max),
        );
        // Vùng gốc x 10..80, y 20..28 (×2) nở ra mỗi phía ≈ d = 35*4*1.6/78 ≈ 2,9 px mô hình.
        assert!(
            x0 < 10.0 && x1 > 80.0 && y0 < 20.0 && y1 > 28.0,
            "{x0} {x1} {y0} {y1}"
        );
        assert!(y1 - y0 < 24.0);
    }

    #[test]
    fn gioi_han_so_vung() {
        let (w, h) = (64u32, 64u32);
        let mut prob = vec![0f32; (w * h) as usize];
        for by in 0..8 {
            for bx in 0..8 {
                for y in 0..4 {
                    for x in 0..4 {
                        prob[((by * 8 + y) * w + bx * 8 + x) as usize] = 0.9;
                    }
                }
            }
        }
        let opts = DetOptions {
            max_candidates: 10,
            min_side: 0.0,
            ..DetOptions::default()
        };
        assert_eq!(boxes_from_map(&prob, w, h, 1.0, 1.0, &opts, 64, 64).len(), 10);
    }

    #[test]
    fn bo_vung_qua_nho() {
        assert!(unclip_rect(&[Point { x: 0.0, y: 0.0 }; 4], 0.1, 3.0).is_none());
        assert!(unclip_rect(&[Point { x: 0.0, y: 0.0 }; 3], 1.6, 3.0).is_none());
    }
}

//! Nắn phối cảnh tứ giác về hình chữ nhật.

use image::{DynamicImage, Rgb, RgbImage};

use crate::decode::check_dimensions;
use crate::error::{ImagingError, Result};
use crate::geometry::{Homography, Point, Quad};

/// Chất lượng nội suy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Interpolation {
    /// Song tuyến: nhanh, dùng cho xem trước.
    Bilinear,
    /// Bicubic (Catmull-Rom): nét hơn, dùng khi lưu trang.
    #[default]
    Bicubic,
}

/// Khổ giấy đoán được từ tỉ lệ cạnh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaperSize {
    /// ISO A-series (tỉ lệ √2).
    A4,
    /// US Letter (11 / 8,5).
    Letter,
    /// Thẻ ID-1 (CCCD, thẻ ngân hàng): 85,60 x 53,98 mm.
    Id1,
}

impl PaperSize {
    /// Tỉ lệ cạnh dài / cạnh ngắn.
    #[must_use]
    pub fn ratio(self) -> f64 {
        match self {
            Self::A4 => std::f64::consts::SQRT_2,
            Self::Letter => 11.0 / 8.5,
            Self::Id1 => 85.60 / 53.98,
        }
    }

    /// Đoán khổ khi tỉ lệ `long/short` lệch dưới 3% so với một khổ chuẩn.
    #[must_use]
    pub fn guess(ratio: f64) -> Option<Self> {
        [Self::A4, Self::Letter, Self::Id1]
            .into_iter()
            .map(|p| (p, (ratio / p.ratio() - 1.0).abs()))
            .filter(|(_, e)| *e < 0.03)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(p, _)| p)
    }
}

/// Tùy chọn nắn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WarpOptions {
    /// Cách nội suy.
    pub interpolation: Interpolation,
    /// Bắt tỉ lệ theo khổ giấy chuẩn khi lệch < 3%.
    pub snap_paper: bool,
    /// Giới hạn cạnh dài của ảnh ra (0 = không giới hạn ngoài [`crate::decode::MAX_PIXELS`]).
    pub max_side: u32,
}

impl Default for WarpOptions {
    fn default() -> Self {
        Self {
            interpolation: Interpolation::Bicubic,
            snap_paper: true,
            max_side: 0,
        }
    }
}

/// Kết quả nắn.
#[derive(Debug, Clone)]
pub struct Warped {
    /// Ảnh đã nắn.
    pub image: RgbImage,
    /// Khổ giấy đoán được (nếu có).
    pub paper: Option<PaperSize>,
}

/// Kích thước ô xử lý: giữ vùng nguồn cần đọc nằm gọn trong bộ đệm CPU.
const TILE: u32 = 64;

/// Kích thước ra (rộng, cao) và khổ giấy từ độ dài cạnh tứ giác.
#[must_use]
pub fn output_size(quad: &Quad, opts: &WarpOptions) -> (u32, u32, Option<PaperSize>) {
    let [top, right, bottom, left] = quad.side_lengths();
    let mut w = top.max(bottom).max(1.0);
    let mut h = left.max(right).max(1.0);
    let mut paper = None;
    if opts.snap_paper {
        let (long, short) = if h >= w { (h, w) } else { (w, h) };
        if let Some(p) = PaperSize::guess(long / short) {
            paper = Some(p);
            // Giữ cạnh dài, chỉnh cạnh ngắn cho đúng tỉ lệ.
            if h >= w {
                w = h / p.ratio();
            } else {
                h = w / p.ratio();
            }
        }
    }
    if opts.max_side > 0 {
        let long = w.max(h);
        let limit = f64::from(opts.max_side);
        if long > limit {
            w *= limit / long;
            h *= limit / long;
        }
    }
    (w.round().max(1.0) as u32, h.round().max(1.0) as u32, paper)
}

/// Nắn vùng `quad` của ảnh về hình chữ nhật.
///
/// # Errors
/// [`ImagingError::InvalidQuad`] khi tứ giác suy biến/không lồi; [`ImagingError::TooLarge`] khi ảnh ra
/// vượt giới hạn.
pub fn warp(img: &DynamicImage, quad: &Quad, opts: WarpOptions) -> Result<Warped> {
    let rgb = img.to_rgb8();
    warp_rgb(&rgb, quad, opts)
}

/// Như [`warp`] cho ảnh RGB8 sẵn có (tránh sao chép).
///
/// # Errors
/// Xem [`warp`].
pub fn warp_rgb(src: &RgbImage, quad: &Quad, opts: WarpOptions) -> Result<Warped> {
    if src.width() == 0 || src.height() == 0 {
        return Err(ImagingError::TooSmall {
            width: src.width(),
            height: src.height(),
        });
    }
    if !quad.is_convex() || quad.area() < 4.0 {
        return Err(ImagingError::InvalidQuad("tứ giác phải lồi và có diện tích"));
    }
    let (w, h, paper) = output_size(quad, &opts);
    check_dimensions(w, h)?;
    let (fw, fh) = (f64::from(w), f64::from(h));
    let rect = [
        Point::new(0.0, 0.0),
        Point::new(fw, 0.0),
        Point::new(fw, fh),
        Point::new(0.0, fh),
    ];
    // Ánh xạ ngược: điểm ra → điểm nguồn.
    let hmap = Homography::from_points(&rect, &quad.pts)?;
    let m = hmap.matrix();

    let mut out = RgbImage::new(w, h);
    for ty in (0..h).step_by(TILE as usize) {
        for tx in (0..w).step_by(TILE as usize) {
            for y in ty..(ty + TILE).min(h) {
                for x in tx..(tx + TILE).min(w) {
                    let (u, v) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
                    let z = m[2][0] * u + m[2][1] * v + m[2][2];
                    if z.abs() < 1e-12 {
                        continue;
                    }
                    let sx = (m[0][0] * u + m[0][1] * v + m[0][2]) / z - 0.5;
                    let sy = (m[1][0] * u + m[1][1] * v + m[1][2]) / z - 0.5;
                    let px = match opts.interpolation {
                        Interpolation::Bilinear => sample_bilinear(src, sx, sy),
                        Interpolation::Bicubic => sample_bicubic(src, sx, sy),
                    };
                    out.put_pixel(x, y, px);
                }
            }
        }
    }
    Ok(Warped { image: out, paper })
}

fn fetch(src: &RgbImage, x: i64, y: i64) -> [f32; 3] {
    let xc = x.clamp(0, i64::from(src.width()) - 1) as u32;
    let yc = y.clamp(0, i64::from(src.height()) - 1) as u32;
    let p = src.get_pixel(xc, yc).0;
    [f32::from(p[0]), f32::from(p[1]), f32::from(p[2])]
}

fn to_px(c: [f32; 3]) -> Rgb<u8> {
    Rgb(c.map(|v| v.round().clamp(0.0, 255.0) as u8))
}

/// Lấy mẫu song tuyến, biên kẹp.
#[must_use]
pub fn sample_bilinear(src: &RgbImage, x: f64, y: f64) -> Rgb<u8> {
    if !x.is_finite() || !y.is_finite() {
        return Rgb([0, 0, 0]);
    }
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = ((x - x0) as f32, (y - y0) as f32);
    let (xi, yi) = (x0 as i64, y0 as i64);
    let a = fetch(src, xi, yi);
    let b = fetch(src, xi + 1, yi);
    let c = fetch(src, xi, yi + 1);
    let d = fetch(src, xi + 1, yi + 1);
    let mut o = [0.0f32; 3];
    for k in 0..3 {
        let top = a[k] + (b[k] - a[k]) * fx;
        let bot = c[k] + (d[k] - c[k]) * fx;
        o[k] = top + (bot - top) * fy;
    }
    to_px(o)
}

fn catmull_rom(t: f32) -> [f32; 4] {
    let t2 = t * t;
    let t3 = t2 * t;
    [
        0.5 * (-t3 + 2.0 * t2 - t),
        1.5 * t3 - 2.5 * t2 + 1.0,
        0.5 * (-3.0 * t3 + 4.0 * t2 + t),
        0.5 * (t3 - t2),
    ]
}

/// Lấy mẫu bicubic Catmull-Rom, biên kẹp.
#[must_use]
pub fn sample_bicubic(src: &RgbImage, x: f64, y: f64) -> Rgb<u8> {
    if !x.is_finite() || !y.is_finite() {
        return Rgb([0, 0, 0]);
    }
    let (x0, y0) = (x.floor(), y.floor());
    let wx = catmull_rom((x - x0) as f32);
    let wy = catmull_rom((y - y0) as f32);
    let (xi, yi) = (x0 as i64, y0 as i64);
    let mut o = [0.0f32; 3];
    for (j, wyj) in wy.iter().enumerate() {
        for (i, wxi) in wx.iter().enumerate() {
            let p = fetch(src, xi - 1 + i as i64, yi - 1 + j as i64);
            let wgt = wxi * wyj;
            for k in 0..3 {
                o[k] += p[k] * wgt;
            }
        }
    }
    to_px(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paper_guess() {
        assert_eq!(PaperSize::guess(1.414), Some(PaperSize::A4));
        assert_eq!(PaperSize::guess(1.30), Some(PaperSize::Letter));
        assert_eq!(PaperSize::guess(1.58), Some(PaperSize::Id1));
        assert_eq!(PaperSize::guess(1.0), None);
        assert_eq!(PaperSize::guess(2.0), None);
    }

    #[test]
    fn warps_axis_aligned_region_exactly() {
        let src = RgbImage::from_fn(100, 80, |x, y| Rgb([x as u8 * 2, y as u8 * 3, 50]));
        let q = Quad::from_unordered([
            Point::new(10.0, 10.0),
            Point::new(60.0, 10.0),
            Point::new(60.0, 40.0),
            Point::new(10.0, 40.0),
        ])
        .unwrap();
        for interpolation in [Interpolation::Bilinear, Interpolation::Bicubic] {
            let opts = WarpOptions {
                interpolation,
                snap_paper: false,
                max_side: 0,
            };
            let out = warp(&DynamicImage::ImageRgb8(src.clone()), &q, opts).unwrap();
            assert_eq!(out.image.dimensions(), (50, 30));
            assert_eq!(out.paper, None);
            for (x, y) in [(0, 0), (25, 15), (49, 29)] {
                assert_eq!(out.image.get_pixel(x, y), src.get_pixel(x + 10, y + 10));
            }
        }
    }

    #[test]
    fn snaps_to_a4_and_limits_size() {
        let q = Quad::from_unordered([
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(100.0, 143.0),
            Point::new(0.0, 143.0),
        ])
        .unwrap();
        let (w, h, p) = output_size(&q, &WarpOptions::default());
        assert_eq!(p, Some(PaperSize::A4));
        assert_eq!((w, h), (101, 143));
        let landscape = Quad::from_unordered([
            Point::new(0.0, 0.0),
            Point::new(143.0, 0.0),
            Point::new(143.0, 100.0),
            Point::new(0.0, 100.0),
        ])
        .unwrap();
        let (w, h, _) = output_size(
            &landscape,
            &WarpOptions {
                max_side: 50,
                ..WarpOptions::default()
            },
        );
        assert_eq!((w, h), (50, 35));
    }

    #[test]
    fn rejects_bad_input() {
        let src = RgbImage::new(10, 10);
        let concave = Quad {
            pts: [
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                Point::new(2.0, 2.0),
                Point::new(0.0, 10.0),
            ],
        };
        assert!(warp_rgb(&src, &concave, WarpOptions::default()).is_err());
        assert!(
            warp_rgb(
                &RgbImage::new(0, 0),
                &Quad::full_frame(5, 5),
                WarpOptions::default()
            )
            .is_err()
        );
        let huge = Quad::full_frame(40_000, 40_000);
        assert!(matches!(
            warp_rgb(&src, &huge, WarpOptions::default()),
            Err(ImagingError::TooLarge { .. })
        ));
        assert_eq!(sample_bilinear(&src, f64::NAN, 0.0), Rgb([0, 0, 0]));
        assert_eq!(sample_bicubic(&src, 0.0, f64::INFINITY), Rgb([0, 0, 0]));
    }
}

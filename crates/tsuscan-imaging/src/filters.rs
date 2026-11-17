//! Bộ lọc trang sau khi nắn. Mỗi bộ lọc là hàm thuần: cùng đầu vào cho cùng đầu ra.

use image::{DynamicImage, GrayImage, Luma, Rgb, RgbImage, imageops};

use crate::error::{ImagingError, Result};

/// Các bộ lọc có sẵn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Filter {
    /// Giữ nguyên.
    Original,
    /// Cân trắng + kéo dãn mức.
    Auto,
    /// Như `Auto` và tăng bão hòa nhẹ.
    Color,
    /// Xám đã kéo dãn mức.
    Gray,
    /// Đen trắng Sauvola.
    BlackWhite,
    /// Khử bóng: chia cho nền ước lượng.
    ShadowRemoval,
    /// Bảng trắng: khử nền/lóa + tăng nét + làm rõ màu bút.
    Whiteboard,
}

impl Filter {
    /// Mọi bộ lọc theo thứ tự hiển thị.
    pub const ALL: [Self; 7] = [
        Self::Original,
        Self::Auto,
        Self::Color,
        Self::Gray,
        Self::BlackWhite,
        Self::ShadowRemoval,
        Self::Whiteboard,
    ];

    /// Tên mã ổn định (CLI, cấu hình, tên tệp ảnh vàng).
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::Auto => "auto",
            Self::Color => "color",
            Self::Gray => "gray",
            Self::BlackWhite => "bw",
            Self::ShadowRemoval => "shadow",
            Self::Whiteboard => "whiteboard",
        }
    }

    /// Phân tích tên mã.
    ///
    /// # Errors
    /// [`ImagingError::InvalidArgument`] khi tên không có.
    pub fn from_code(code: &str) -> Result<Self> {
        Self::ALL
            .into_iter()
            .find(|f| f.code() == code)
            .ok_or(ImagingError::InvalidArgument("bộ lọc không tồn tại"))
    }
}

/// Áp bộ lọc. `Gray`/`BlackWhite` trả ảnh `Luma8`, còn lại `Rgb8`.
///
/// `dpi` dùng để chọn cửa sổ Sauvola/khử bóng; `None` thì đoán theo khổ A4 (bề rộng 8,27 inch).
#[must_use]
pub fn apply(img: &RgbImage, filter: Filter, dpi: Option<f32>) -> DynamicImage {
    let dpi = dpi
        .filter(|d| d.is_finite() && *d > 0.0)
        .unwrap_or_else(|| estimate_dpi(img));
    match filter {
        Filter::Original => DynamicImage::ImageRgb8(img.clone()),
        Filter::Auto => DynamicImage::ImageRgb8(auto_levels(img)),
        Filter::Color => DynamicImage::ImageRgb8(saturate(&auto_levels(img), 1.3)),
        Filter::Gray => DynamicImage::ImageLuma8(stretch_gray(&to_gray(&auto_levels(img)))),
        Filter::BlackWhite => DynamicImage::ImageLuma8(sauvola(&to_gray(img), sauvola_window(dpi), 0.34)),
        Filter::ShadowRemoval => DynamicImage::ImageRgb8(remove_shadow(img, dpi)),
        Filter::Whiteboard => DynamicImage::ImageRgb8(whiteboard(img, dpi)),
    }
}

/// Đoán DPI giả định trang rộng 8,27 inch (A4 dọc).
#[must_use]
pub fn estimate_dpi(img: &RgbImage) -> f32 {
    (img.width().min(img.height()) as f32 / 8.27).max(10.0)
}

/// Cửa sổ Sauvola ≈ 0,1 inch, lẻ, tối thiểu 15 px.
#[must_use]
pub fn sauvola_window(dpi: f32) -> u32 {
    let w = ((dpi * 0.1).round() as u32).max(15);
    w | 1
}

fn luma(p: Rgb<u8>) -> f32 {
    0.299 * f32::from(p[0]) + 0.587 * f32::from(p[1]) + 0.114 * f32::from(p[2])
}

fn to_gray(img: &RgbImage) -> GrayImage {
    GrayImage::from_fn(img.width(), img.height(), |x, y| {
        Luma([luma(*img.get_pixel(x, y)).round() as u8])
    })
}

fn clamp_u8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

/// Phân vị `q` (0..1) của histogram 256 mức.
fn percentile(hist: &[u64; 256], q: f64) -> u8 {
    let total: u64 = hist.iter().sum();
    if total == 0 {
        return 0;
    }
    let target = (total as f64 * q).ceil().max(1.0) as u64;
    let mut acc = 0;
    for (i, c) in hist.iter().enumerate() {
        acc += c;
        if acc >= target {
            return i as u8;
        }
    }
    255
}

/// Cân trắng theo "điểm trắng" (phân vị 99 từng kênh) + điểm đen chung (phân vị 1 của độ sáng).
fn auto_levels(img: &RgbImage) -> RgbImage {
    let mut hist = [[0u64; 256]; 3];
    let mut lhist = [0u64; 256];
    for p in img.pixels() {
        for k in 0..3 {
            hist[k][p[k] as usize] += 1;
        }
        lhist[clamp_u8(luma(*p)) as usize] += 1;
    }
    let black = f32::from(percentile(&lhist, 0.01));
    let white: [f32; 3] = [0, 1, 2].map(|k| f32::from(percentile(&hist[k], 0.99)));
    let mut lut = [[0u8; 256]; 3];
    for k in 0..3 {
        let span = (white[k] - black).max(32.0);
        for (v, slot) in lut[k].iter_mut().enumerate() {
            *slot = clamp_u8((v as f32 - black) * 255.0 / span);
        }
    }
    let mut out = img.clone();
    for p in out.pixels_mut() {
        for k in 0..3 {
            p[k] = lut[k][p[k] as usize];
        }
    }
    out
}

fn saturate(img: &RgbImage, amount: f32) -> RgbImage {
    let mut out = img.clone();
    for p in out.pixels_mut() {
        let l = luma(*p);
        for k in 0..3 {
            p[k] = clamp_u8(l + (f32::from(p[k]) - l) * amount);
        }
    }
    out
}

fn stretch_gray(img: &GrayImage) -> GrayImage {
    let mut hist = [0u64; 256];
    for p in img.pixels() {
        hist[p[0] as usize] += 1;
    }
    let lo = f32::from(percentile(&hist, 0.005));
    let hi = f32::from(percentile(&hist, 0.995));
    let span = (hi - lo).max(32.0);
    let mut out = img.clone();
    for p in out.pixels_mut() {
        p[0] = clamp_u8((f32::from(p[0]) - lo) * 255.0 / span);
    }
    out
}

/// Nhị phân hóa Sauvola: `T = m * (1 + k * (s / R - 1))`, R = 128, qua ảnh tích phân tổng và tổng bình phương.
#[must_use]
pub fn sauvola(gray: &GrayImage, window: u32, k: f32) -> GrayImage {
    let (w, h) = (gray.width() as usize, gray.height() as usize);
    let stride = w + 1;
    let mut sum = vec![0u64; stride * (h + 1)];
    let mut sq = vec![0u64; stride * (h + 1)];
    let raw = gray.as_raw();
    for y in 0..h {
        let (mut rs, mut rq) = (0u64, 0u64);
        for x in 0..w {
            let v = u64::from(raw[y * w + x]);
            rs += v;
            rq += v * v;
            sum[(y + 1) * stride + x + 1] = sum[y * stride + x + 1] + rs;
            sq[(y + 1) * stride + x + 1] = sq[y * stride + x + 1] + rq;
        }
    }
    let r = (window / 2) as usize;
    let mut out = GrayImage::new(w as u32, h as u32);
    for y in 0..h {
        let (y0, y1) = (y.saturating_sub(r), (y + r + 1).min(h));
        for x in 0..w {
            let (x0, x1) = (x.saturating_sub(r), (x + r + 1).min(w));
            let n = ((x1 - x0) * (y1 - y0)) as f64;
            let rect = |t: &[u64]| {
                t[y1 * stride + x1] + t[y0 * stride + x0] - t[y0 * stride + x1] - t[y1 * stride + x0]
            };
            let mean = rect(&sum) as f64 / n;
            let var = (rect(&sq) as f64 / n - mean * mean).max(0.0);
            let t = mean * (1.0 + f64::from(k) * (var.sqrt() / 128.0 - 1.0));
            let v = if f64::from(raw[y * w + x]) > t { 255 } else { 0 };
            out.put_pixel(x as u32, y as u32, Luma([v]));
        }
    }
    out
}

/// Ước lượng nền từng kênh: thu nhỏ, lọc cực đại (xóa nét chữ tối), làm mờ, phóng lại.
fn background(img: &RgbImage, dpi: f32) -> RgbImage {
    let (w, h) = img.dimensions();
    let factor = 4u32;
    let (sw, sh) = ((w / factor).max(1), (h / factor).max(1));
    let small = imageops::resize(img, sw, sh, imageops::FilterType::Triangle);
    // Nét chữ dày tối đa ~ 0,08 inch ⇒ bán kính lọc cực đại theo DPI ở ảnh thu nhỏ.
    let radius = ((dpi * 0.08 / factor as f32).ceil() as u32).clamp(2, 24);
    let maxed = max_filter(&small, radius);
    let blurred = imageproc::filter::gaussian_blur_f32(&maxed, radius as f32 * 1.5);
    imageops::resize(&blurred, w, h, imageops::FilterType::Triangle)
}

/// Lọc cực đại tách được (ngang rồi dọc), cửa sổ vuông bán kính `r`.
fn max_filter(img: &RgbImage, r: u32) -> RgbImage {
    let (w, h) = img.dimensions();
    let pass = |src: &RgbImage, horizontal: bool| {
        RgbImage::from_fn(w, h, |x, y| {
            let mut m = [0u8; 3];
            let (lo, hi, fixed) = if horizontal {
                (x.saturating_sub(r), (x + r).min(w - 1), y)
            } else {
                (y.saturating_sub(r), (y + r).min(h - 1), x)
            };
            for t in lo..=hi {
                let p = if horizontal {
                    src.get_pixel(t, fixed)
                } else {
                    src.get_pixel(fixed, t)
                };
                for k in 0..3 {
                    m[k] = m[k].max(p[k]);
                }
            }
            Rgb(m)
        })
    };
    pass(&pass(img, true), false)
}

fn divide_by_background(img: &RgbImage, bg: &RgbImage) -> RgbImage {
    let mut out = img.clone();
    for (p, b) in out.pixels_mut().zip(bg.pixels()) {
        for k in 0..3 {
            let d = f32::from(b[k]).max(16.0);
            p[k] = clamp_u8(f32::from(p[k]) * 245.0 / d);
        }
    }
    out
}

fn remove_shadow(img: &RgbImage, dpi: f32) -> RgbImage {
    let bg = background(img, dpi);
    divide_by_background(img, &bg)
}

fn whiteboard(img: &RgbImage, dpi: f32) -> RgbImage {
    let flat = remove_shadow(img, dpi);
    // Lóa sau khi chia nền gần như trắng tuyệt đối: đẩy hẳn về trắng; nét bút giữ màu.
    let mut out = flat.clone();
    for p in out.pixels_mut() {
        let l = luma(*p);
        if l > 225.0 {
            *p = Rgb([255, 255, 255]);
        }
    }
    let sharpened = unsharp(&out, 1.2, 0.8);
    saturate(&sharpened, 1.4)
}

fn unsharp(img: &RgbImage, sigma: f32, amount: f32) -> RgbImage {
    let blurred = imageproc::filter::gaussian_blur_f32(img, sigma);
    let mut out = img.clone();
    for (p, b) in out.pixels_mut().zip(blurred.pixels()) {
        for k in 0..3 {
            let v = f32::from(p[k]);
            p[k] = clamp_u8(v + (v - f32::from(b[k])) * amount);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_roundtrip() {
        for f in Filter::ALL {
            assert_eq!(Filter::from_code(f.code()).unwrap(), f);
        }
        assert!(Filter::from_code("khong-co").is_err());
    }

    #[test]
    fn window_and_dpi() {
        assert_eq!(sauvola_window(72.0), 15);
        assert_eq!(sauvola_window(300.0), 31);
        assert_eq!(sauvola_window(400.0) % 2, 1);
        assert!((estimate_dpi(&RgbImage::new(827, 1169)) - 100.0).abs() < 0.01);
    }

    #[test]
    fn sauvola_separates_text_from_paper() {
        let g = GrayImage::from_fn(60, 60, |x, y| {
            Luma([if (20..40).contains(&x) && (28..32).contains(&y) {
                30
            } else {
                200
            }])
        });
        let b = sauvola(&g, 15, 0.34);
        assert_eq!(b.get_pixel(30, 30)[0], 0);
        assert_eq!(b.get_pixel(5, 5)[0], 255);
    }

    #[test]
    fn shadow_removal_flattens_gradient() {
        // Giấy trắng bị tối dần sang phải: sau khử bóng hai đầu gần bằng nhau.
        let img = RgbImage::from_fn(200, 100, |x, _| {
            let v = 240 - (x as u8 / 2);
            Rgb([v, v, v])
        });
        let out = remove_shadow(&img, 100.0);
        let l = |x| i32::from(out.get_pixel(x, 50)[0]);
        assert!((l(10) - l(190)).abs() < 12, "{} {}", l(10), l(190));
    }

    #[test]
    fn handles_tiny_and_uniform_images() {
        let img = RgbImage::from_pixel(1, 1, Rgb([10, 20, 30]));
        for f in Filter::ALL {
            let out = apply(&img, f, Some(f32::NAN));
            assert_eq!((out.width(), out.height()), (1, 1));
        }
        assert_eq!(percentile(&[0; 256], 0.5), 0);
    }
}

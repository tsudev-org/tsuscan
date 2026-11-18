//! Chữ ký tay: tách nét mực từ ảnh chụp chữ ký trên giấy thành ảnh nền trong suốt, rồi đóng lên trang.
//!
//! Đây là chữ ký HÌNH (như ký tay lên bản in), KHÔNG phải chữ ký số PAdES - không chứng minh tính toàn vẹn.

use image::{DynamicImage, GrayImage, Rgba, RgbaImage, imageops::FilterType};

use crate::error::{DocError, Result};
use crate::model::Page;

/// Tách chữ ký: nền giấy → trong suốt, nét mực → màu `ink` (None = giữ màu mực gốc), độ đục theo độ đậm nét.
/// Ảnh ra được cắt sát nét (chừa lề 2%).
///
/// # Errors
/// [`DocError::InvalidArgument`] khi ảnh rỗng hoặc không thấy nét mực.
pub fn extract(img: &DynamicImage, ink: Option<[u8; 3]>) -> Result<RgbaImage> {
    if img.width() == 0 || img.height() == 0 {
        return Err(DocError::InvalidArgument("ảnh chữ ký rỗng"));
    }
    let rgb = img.to_rgb8();
    let gray: GrayImage = img.to_luma8();
    let mut hist = [0u32; 256];
    for p in gray.pixels() {
        hist[usize::from(p.0[0])] += 1;
    }
    let total = u64::from(gray.width()) * u64::from(gray.height());
    let pct = |q: f64| -> u8 {
        let target = (total as f64 * q) as u64;
        let mut acc = 0u64;
        for (v, &c) in hist.iter().enumerate() {
            acc += u64::from(c);
            if acc > target {
                return v as u8;
            }
        }
        255
    };
    // Giấy = phân vị 60% (nét mực chiếm ít diện tích), mực = phân vị 1%.
    let paper = f32::from(pct(0.60));
    let dark = f32::from(pct(0.01));
    if paper - dark < 40.0 {
        return Err(DocError::InvalidArgument(
            "không thấy nét mực tương phản với giấy",
        ));
    }
    // Bỏ 25% khoảng gần màu giấy (bóng, vân giấy), phần còn lại tăng tuyến tính tới đục hẳn.
    let lo = dark;
    let hi = paper - (paper - dark) * 0.25;
    let mut out = RgbaImage::new(gray.width(), gray.height());
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0u32, 0u32);
    for (x, y, p) in gray.enumerate_pixels() {
        let v = f32::from(p.0[0]);
        let a = ((hi - v) / (hi - lo)).clamp(0.0, 1.0);
        let a8 = (a * 255.0).round() as u8;
        if a8 > 24 {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
        let c = ink.unwrap_or(rgb.get_pixel(x, y).0);
        out.put_pixel(x, y, Rgba([c[0], c[1], c[2], if a8 > 24 { a8 } else { 0 }]));
    }
    if x0 > x1 || y0 > y1 {
        return Err(DocError::InvalidArgument("không thấy nét mực"));
    }
    let m = (gray.width().max(gray.height()) / 50).max(1);
    let (cx0, cy0) = (x0.saturating_sub(m), y0.saturating_sub(m));
    let cx1 = (x1 + m).min(gray.width() - 1);
    let cy1 = (y1 + m).min(gray.height() - 1);
    Ok(image::imageops::crop_imm(&out, cx0, cy0, cx1 - cx0 + 1, cy1 - cy0 + 1).to_image())
}

/// Đóng chữ ký lên trang tại (x, y) góc trên-trái, rộng `width` điểm ảnh (giữ tỉ lệ). Trộn alpha thẳng vào ảnh
/// trang (ảnh trang thành RGB).
///
/// # Errors
/// [`DocError::InvalidArgument`] khi kích thước/tọa độ không hợp lệ.
pub fn stamp(page: &mut Page, sig: &RgbaImage, x: i64, y: i64, width: u32) -> Result<()> {
    if width == 0 || sig.width() == 0 || sig.height() == 0 || width > 20_000 {
        return Err(DocError::InvalidArgument("kích thước chữ ký không hợp lệ"));
    }
    let height = ((u64::from(sig.height()) * u64::from(width)) / u64::from(sig.width())).max(1) as u32;
    if height > 20_000 {
        return Err(DocError::InvalidArgument("kích thước chữ ký không hợp lệ"));
    }
    let scaled = image::imageops::resize(sig, width, height, FilterType::CatmullRom);
    let mut rgb = page.image.to_rgb8();
    for (sx, sy, p) in scaled.enumerate_pixels() {
        let (tx, ty) = (x + i64::from(sx), y + i64::from(sy));
        if tx < 0 || ty < 0 || tx >= i64::from(rgb.width()) || ty >= i64::from(rgb.height()) {
            continue;
        }
        let a = u32::from(p.0[3]);
        if a == 0 {
            continue;
        }
        let dst = rgb.get_pixel_mut(tx as u32, ty as u32);
        for c in 0..3 {
            dst.0[c] = ((u32::from(p.0[c]) * a + u32::from(dst.0[c]) * (255 - a) + 127) / 255) as u8;
        }
    }
    page.image = DynamicImage::ImageRgb8(rgb);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    fn photo() -> DynamicImage {
        // Giấy hơi xám 230 + nét chéo xanh đậm.
        DynamicImage::ImageRgb8(RgbImage::from_fn(200, 100, |x, y| {
            if (60..140).contains(&x) && (i64::from(y) - i64::from(x) / 2).abs() < 3 {
                Rgb([20, 30, 120])
            } else {
                Rgb([230, 228, 225])
            }
        }))
    }

    #[test]
    fn extracts_and_stamps() {
        let sig = extract(&photo(), Some([0, 0, 160])).unwrap();
        assert!(
            sig.width() < 120 && sig.height() < 60,
            "{}x{}",
            sig.width(),
            sig.height()
        );
        assert!(sig.pixels().any(|p| p.0[3] == 255));
        assert!(sig.pixels().any(|p| p.0[3] == 0));
        let mut page = Page::new(DynamicImage::ImageRgb8(RgbImage::from_pixel(
            300,
            300,
            Rgb([255, 255, 255]),
        )));
        stamp(&mut page, &sig, 250, -10, 100).unwrap();
        let rgb = page.image.to_rgb8();
        assert!(rgb.pixels().any(|p| p.0 == [0, 0, 160]));
        assert!(stamp(&mut page, &sig, 0, 0, 0).is_err());
    }

    #[test]
    fn rejects_blank() {
        let blank = DynamicImage::ImageRgb8(RgbImage::from_pixel(50, 50, Rgb([240, 240, 240])));
        assert!(extract(&blank, None).is_err());
        assert!(extract(&DynamicImage::new_rgb8(0, 0), None).is_err());
    }
}

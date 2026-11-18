//! Xuất ảnh trang: JPEG (nén của crate này), PNG, WebP không mất dữ liệu. Ảnh ra KHÔNG mang EXIF/GPS.

use image::{DynamicImage, ImageEncoder};

use crate::compress::{self, ColorMode, ImageKind, Quality};
use crate::error::{DocError, Result};

/// Định dạng ảnh xuất.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// JPEG.
    Jpeg,
    /// PNG.
    Png,
    /// WebP không mất dữ liệu.
    WebP,
}

impl ImageFormat {
    /// Phần mở rộng tệp.
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::WebP => "webp",
        }
    }
}

/// Mã hóa một ảnh trang. `mode` quyết định màu/xám/đen trắng như khi ghi PDF.
///
/// # Errors
/// [`DocError::Image`]/[`DocError::Encode`] khi bộ mã hóa lỗi.
pub fn export_image(
    img: &DynamicImage,
    format: ImageFormat,
    mode: ColorMode,
    quality: Quality,
) -> Result<Vec<u8>> {
    let kind = match mode {
        ColorMode::Auto => compress::classify(img),
        ColorMode::Color => ImageKind::Color,
        ColorMode::Gray => ImageKind::Gray,
        ColorMode::Bilevel => ImageKind::Bilevel,
    };
    let img = match kind {
        ImageKind::Color => DynamicImage::ImageRgb8(img.to_rgb8()),
        ImageKind::Gray => DynamicImage::ImageLuma8(img.to_luma8()),
        ImageKind::Bilevel => DynamicImage::ImageLuma8(compress::to_bilevel(&img.to_luma8())),
    };
    let mut out = Vec::new();
    match format {
        ImageFormat::Jpeg => {
            let gray = !matches!(kind, ImageKind::Color);
            let raw = img.as_bytes();
            return compress::encode_jpeg(raw, img.width(), img.height(), gray, quality.jpeg_quality());
        }
        ImageFormat::Png => {
            image::codecs::png::PngEncoder::new_with_quality(
                &mut out,
                image::codecs::png::CompressionType::Best,
                image::codecs::png::FilterType::Adaptive,
            )
            .write_image(img.as_bytes(), img.width(), img.height(), img.color().into())?;
        }
        ImageFormat::WebP => {
            // Bộ mã hóa WebP của `image` chỉ nhận RGB(A)/L8.
            image::codecs::webp::WebPEncoder::new_lossless(&mut out)
                .write_image(img.as_bytes(), img.width(), img.height(), img.color().into())
                .map_err(DocError::Image)?;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_all_formats() {
        let img = DynamicImage::ImageRgb8(image::RgbImage::from_fn(64, 48, |x, y| {
            image::Rgb([(x * 4) as u8, (y * 5) as u8, 90])
        }));
        for f in [ImageFormat::Jpeg, ImageFormat::Png, ImageFormat::WebP] {
            let b = export_image(&img, f, ColorMode::Auto, Quality::High).unwrap();
            let back = image::load_from_memory(&b).unwrap();
            assert_eq!((back.width(), back.height()), (64, 48), "{}", f.extension());
        }
        let bw = export_image(&img, ImageFormat::Png, ColorMode::Bilevel, Quality::High).unwrap();
        let back = image::load_from_memory(&bw).unwrap().to_luma8();
        assert!(back.pixels().all(|p| p.0[0] == 0 || p.0[0] == 255));
    }
}

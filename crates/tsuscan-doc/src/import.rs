//! Nhập tệp người dùng chọn (dữ liệu KHÔNG tin cậy): nhận dạng theo nội dung (không tin phần mở rộng).
//!
//! - PDF → vẽ từng trang ([`crate::pdf::read`]).
//! - JPEG/PNG/WebP → giải mã có giới hạn của `tsuscan-imaging` (xoay theo EXIF, bỏ EXIF/GPS).
//! - HEIC/HEIF/AVIF → [`DocError::NeedsPlatformDecoder`]: lõi Rust không chứa bộ giải mã HEVC (bằng sáng chế +
//!   không có bản thuần Rust đủ chín). App giải mã bằng API hệ điều hành (Android `ImageDecoder`, Windows WIC,
//!   macOS ImageIO) rồi gọi [`from_rgba`]. Lý do: ADR-0008.

use image::{DynamicImage, RgbaImage};

use crate::error::{DocError, Result};
use crate::model::Page;
use crate::pdf::read::{self, ImportOptions};

/// Loại tệp nhận ra từ nội dung.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKind {
    /// PDF.
    Pdf,
    /// JPEG/PNG/WebP.
    Image,
    /// Họ ISO-BMFF ảnh (HEIC/HEIF/AVIF): cần bộ giải mã hệ điều hành.
    Heif,
    /// Không nhận ra.
    Unknown,
}

/// Nhãn `ftyp` của họ ảnh HEIF (HEIC, HEIF, AVIF).
const HEIF: [&[u8; 4]; 9] = [
    b"heic", b"heix", b"hevc", b"hevx", b"heim", b"heis", b"mif1", b"msf1", b"avif",
];

/// Nhận dạng tệp theo chữ ký.
#[must_use]
pub fn sniff(bytes: &[u8]) -> InputKind {
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        let brand = &bytes[8..12];
        if HEIF.iter().any(|b| *b == brand) {
            return InputKind::Heif;
        }
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF])
        || bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || (bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP")
    {
        return InputKind::Image;
    }
    if read::is_pdf(bytes) {
        return InputKind::Pdf;
    }
    InputKind::Unknown
}

/// Nhập một tệp thành các trang.
///
/// # Errors
/// [`DocError::UnsupportedFormat`], [`DocError::NeedsPlatformDecoder`], lỗi đọc PDF/ảnh.
pub fn import(bytes: &[u8], opts: &ImportOptions) -> Result<Vec<Page>> {
    match sniff(bytes) {
        InputKind::Pdf => read::import_pdf(bytes, opts),
        InputKind::Image => Ok(vec![Page::new(tsuscan_imaging::decode(bytes)?)]),
        InputKind::Heif => Err(DocError::NeedsPlatformDecoder),
        InputKind::Unknown => Err(DocError::UnsupportedFormat),
    }
}

/// Trang từ điểm ảnh RGBA 8 bit do hệ điều hành giải mã (HEIC...). Nền trong suốt được trộn với trắng.
///
/// # Errors
/// [`DocError::InvalidArgument`] khi kích thước không khớp dữ liệu; [`DocError::Imaging`] khi vượt giới hạn.
pub fn from_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Result<Page> {
    tsuscan_imaging::decode::check_dimensions(width, height)?;
    let img = RgbaImage::from_raw(width, height, rgba)
        .ok_or(DocError::InvalidArgument("dữ liệu RGBA sai độ dài"))?;
    let rgb = image::RgbImage::from_fn(width, height, |x, y| {
        let [r, g, b, a] = img.get_pixel(x, y).0;
        let a = u32::from(a);
        let mix = |c: u8| ((u32::from(c) * a + 255 * (255 - a) + 127) / 255) as u8;
        image::Rgb([mix(r), mix(g), mix(b)])
    });
    Ok(Page::new(DynamicImage::ImageRgb8(rgb)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffs_formats() {
        assert_eq!(sniff(b"%PDF-1.7\n..."), InputKind::Pdf);
        assert_eq!(sniff(b"\0\0\0\x18ftypheic\0\0\0\0"), InputKind::Heif);
        assert_eq!(sniff(b"\0\0\0\x18ftypisom\0\0\0\0"), InputKind::Unknown);
        assert_eq!(sniff(&[0xFF, 0xD8, 0xFF, 0xE0]), InputKind::Image);
        assert_eq!(sniff(b"RIFF\0\0\0\0WEBPVP8 "), InputKind::Image);
        assert_eq!(sniff(b"hello"), InputKind::Unknown);
        assert!(matches!(
            import(b"\0\0\0\x18ftypmif1\0\0\0\0", &ImportOptions::default()),
            Err(DocError::NeedsPlatformDecoder)
        ));
        assert!(matches!(
            import(b"xyz", &ImportOptions::default()),
            Err(DocError::UnsupportedFormat)
        ));
    }

    #[test]
    fn rgba_blends_on_white() {
        let p = from_rgba(2, 1, vec![0, 0, 0, 0, 0, 0, 0, 255]).unwrap();
        let rgb = p.image.to_rgb8();
        assert_eq!(rgb.get_pixel(0, 0).0, [255, 255, 255]);
        assert_eq!(rgb.get_pixel(1, 0).0, [0, 0, 0]);
        assert!(from_rgba(2, 2, vec![0; 3]).is_err());
        assert!(from_rgba(0, 2, vec![]).is_err());
    }
}

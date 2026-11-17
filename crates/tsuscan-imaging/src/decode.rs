//! Giải mã ảnh không tin cậy (JPEG/PNG/WebP) với giới hạn tài nguyên và hướng EXIF.

use std::io::Cursor;

use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits};

use crate::error::{ImagingError, Result};

/// Số điểm ảnh tối đa chấp nhận (100 MP).
pub const MAX_PIXELS: u64 = 100_000_000;
/// Bộ nhớ cấp phát tối đa cho bộ giải mã (512 MiB).
pub const MAX_ALLOC_BYTES: u64 = 512 * 1024 * 1024;
/// Cạnh tối đa (chặn ảnh dạng sợi 1 x 100 triệu).
pub const MAX_SIDE: u32 = 30_000;

/// Giải mã ảnh từ bộ nhớ, áp dụng giới hạn và xoay theo EXIF orientation.
///
/// # Errors
/// - [`ImagingError::UnsupportedFormat`] khi không phải JPEG/PNG/WebP.
/// - [`ImagingError::TooLarge`] khi vượt [`MAX_PIXELS`]/[`MAX_SIDE`].
/// - [`ImagingError::Image`] khi dữ liệu hỏng hoặc vượt [`MAX_ALLOC_BYTES`].
pub fn decode(bytes: &[u8]) -> Result<DynamicImage> {
    let format = image::guess_format(bytes).map_err(|_| ImagingError::UnsupportedFormat)?;
    if !matches!(format, ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::WebP) {
        return Err(ImagingError::UnsupportedFormat);
    }
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = Limits::default();
    limits.max_alloc = Some(MAX_ALLOC_BYTES);
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    reader.limits(limits);

    let mut decoder = reader.into_decoder()?;
    let (width, height) = decoder.dimensions();
    check_dimensions(width, height)?;
    // `Limits::max_alloc` KHÔNG bao bộ đệm đầu ra khi giải mã qua `into_decoder` (image 0.25): tự chặn,
    // nếu không PNG 100 MP RGBA 16 bit khai báo hợp lệ sẽ xin 800 MB.
    if decoder.total_bytes() > MAX_ALLOC_BYTES {
        return Err(ImagingError::TooLarge {
            width,
            height,
            max_pixels: MAX_PIXELS,
        });
    }
    // Hướng EXIF hỏng không đáng để từ chối cả ảnh: coi như không xoay.
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut img = DynamicImage::from_decoder(decoder)?;
    img.apply_orientation(orientation);
    Ok(img)
}

/// Kiểm tra kích thước nằm trong giới hạn xử lý.
///
/// # Errors
/// [`ImagingError::TooSmall`] khi một cạnh bằng 0, [`ImagingError::TooLarge`] khi vượt giới hạn.
pub fn check_dimensions(width: u32, height: u32) -> Result<()> {
    if width == 0 || height == 0 {
        return Err(ImagingError::TooSmall { width, height });
    }
    if u64::from(width) * u64::from(height) > MAX_PIXELS || width > MAX_SIDE || height > MAX_SIDE {
        return Err(ImagingError::TooLarge {
            width,
            height,
            max_pixels: MAX_PIXELS,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageEncoder, RgbImage, codecs::png::PngEncoder};

    fn png_bytes(w: u32, h: u32) -> Vec<u8> {
        let img = RgbImage::from_fn(w, h, |x, y| image::Rgb([x as u8, y as u8, 7]));
        let mut out = Vec::new();
        PngEncoder::new(&mut out)
            .write_image(img.as_raw(), w, h, image::ExtendedColorType::Rgb8)
            .unwrap();
        out
    }

    #[test]
    fn decodes_png() {
        let img = decode(&png_bytes(5, 3)).unwrap();
        assert_eq!((img.width(), img.height()), (5, 3));
    }

    #[test]
    fn rejects_garbage_and_unsupported() {
        assert!(matches!(
            decode(b"khong phai anh"),
            Err(ImagingError::UnsupportedFormat)
        ));
        assert!(matches!(decode(b""), Err(ImagingError::UnsupportedFormat)));
        // Chữ ký GIF: định dạng nhận ra được nhưng không hỗ trợ.
        assert!(matches!(
            decode(b"GIF89a\x01\x00\x01\x00"),
            Err(ImagingError::UnsupportedFormat)
        ));
    }

    #[test]
    fn rejects_truncated_png() {
        let bytes = png_bytes(64, 64);
        assert!(decode(&bytes[..bytes.len() / 2]).is_err());
    }

    #[test]
    fn rejects_huge_declared_size() {
        // PNG khai báo 20000x20000 (400 MP) nhưng không có dữ liệu.
        let mut bytes = png_bytes(1, 1);
        bytes[16..20].copy_from_slice(&20_000u32.to_be_bytes());
        bytes[20..24].copy_from_slice(&20_000u32.to_be_bytes());
        assert!(decode(&bytes).is_err());
        assert!(matches!(
            check_dimensions(20_000, 20_000),
            Err(ImagingError::TooLarge { .. })
        ));
        assert!(matches!(
            check_dimensions(0, 5),
            Err(ImagingError::TooSmall { .. })
        ));
        assert!(matches!(
            check_dimensions(40_000, 1),
            Err(ImagingError::TooLarge { .. })
        ));
    }

    #[test]
    fn rejects_output_buffer_over_alloc_limit() {
        // PNG 10000x10000 RGBA 16 bit: đúng 100 MP (qua kiểm điểm ảnh) nhưng cần 800 MB bộ đệm.
        let mut bytes = png_bytes(1, 1);
        bytes[16..20].copy_from_slice(&10_000u32.to_be_bytes());
        bytes[20..24].copy_from_slice(&10_000u32.to_be_bytes());
        bytes[24] = 16; // độ sâu bit
        bytes[25] = 6; // RGBA
        let crc = crc32(&bytes[12..29]);
        bytes[29..33].copy_from_slice(&crc.to_be_bytes());
        assert!(matches!(decode(&bytes), Err(ImagingError::TooLarge { .. })));
    }

    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for &b in data {
            crc ^= u32::from(b);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    #[test]
    fn applies_exif_orientation_jpeg() {
        // JPEG 4x2 có APP1 EXIF orientation=6 (xoay 90° theo chiều kim đồng hồ) ⇒ 2x4.
        let img = RgbImage::from_pixel(4, 2, image::Rgb([200, 10, 10]));
        let mut jpg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new(&mut jpg)
            .encode_image(&img)
            .unwrap();
        let exif: Vec<u8> = [
            b"Exif\0\0".as_slice(),
            b"MM\0\x2a\0\0\0\x08",
            b"\0\x01",
            b"\x01\x12\0\x03\0\0\0\x01\0\x06\0\0",
            b"\0\0\0\0",
        ]
        .concat();
        let len = u16::try_from(exif.len() + 2).unwrap().to_be_bytes();
        let mut with_exif = vec![0xFF, 0xD8, 0xFF, 0xE1, len[0], len[1]];
        with_exif.extend_from_slice(&exif);
        with_exif.extend_from_slice(&jpg[2..]);
        let out = decode(&with_exif).unwrap();
        assert_eq!((out.width(), out.height()), (2, 4));
    }
}

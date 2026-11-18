//! Đọc QR/mã vạch trên ảnh trang (cổng ZXing `rxing`, chỉ bộ giải mã): QR (VietQR, hóa đơn điện tử), Data Matrix,
//! PDF417 (giấy tờ), Aztec, mã 1D (EAN/UPC/Code 128/Code 39/ITF...). Chạy hoàn toàn trên máy.

use std::panic::{AssertUnwindSafe, catch_unwind};

use image::{DynamicImage, imageops::FilterType};
use rxing::common::HybridBinarizer;
use rxing::multi::{GenericMultipleBarcodeReader, MultipleBarcodeReader};
use rxing::{BinaryBitmap, DecodeHints, Luma8LuminanceSource, MultiFormatReader};

/// Cạnh dài tối đa khi dò (lớn hơn thì thu nhỏ: đủ cho QR chiếm ≥ 3% bề rộng trang A4 300 DPI).
pub const MAX_SIDE: u32 = 3000;

/// Một mã đọc được.
#[derive(Debug, Clone, PartialEq)]
pub struct Barcode {
    /// Loại mã (ví dụ `qrcode`, `ean 13`, `code 128`).
    pub format: String,
    /// Nội dung đã giải mã.
    pub text: String,
    /// Các điểm định vị theo tọa độ ảnh gốc.
    pub points: Vec<(f32, f32)>,
}

/// Dò mọi mã trên ảnh. Không thấy mã ⇒ danh sách rỗng (không phải lỗi).
#[must_use]
pub fn scan(img: &DynamicImage) -> Vec<Barcode> {
    if img.width() == 0 || img.height() == 0 {
        return Vec::new();
    }
    let long = img.width().max(img.height());
    let (gray, k) = if long > MAX_SIDE {
        let k = MAX_SIDE as f32 / long as f32;
        let w = ((img.width() as f32 * k) as u32).max(1);
        let h = ((img.height() as f32 * k) as u32).max(1);
        (img.resize_exact(w, h, FilterType::Triangle).to_luma8(), k)
    } else {
        (img.to_luma8(), 1.0)
    };
    let (w, h) = gray.dimensions();
    let found = catch_unwind(AssertUnwindSafe(|| {
        let Ok(src) = Luma8LuminanceSource::new(gray.into_raw(), w, h) else {
            return Vec::new();
        };
        let mut bitmap = BinaryBitmap::new(HybridBinarizer::new(src));
        let hints = DecodeHints {
            TryHarder: Some(true),
            ..DecodeHints::default()
        };
        let mut reader = GenericMultipleBarcodeReader::new(MultiFormatReader::default());
        reader
            .decode_multiple_with_hints(&mut bitmap, &hints)
            .unwrap_or_default()
    }))
    // Bộ giải mã bên thứ ba panic với ảnh lạ: coi như không thấy mã (fuzz ghi nhận riêng).
    .unwrap_or_default();
    let mut out: Vec<Barcode> = Vec::new();
    for r in found {
        let b = Barcode {
            format: r.getBarcodeFormat().to_string(),
            text: r.getText().to_owned(),
            points: r.getPoints().iter().map(|p| (p.x / k, p.y / k)).collect(),
        };
        if !out.iter().any(|o| o.text == b.text && o.format == b.format) {
            out.push(b);
        }
    }
    out
}

//! Chế độ quét Sách (tách 2 trang theo gáy) và QR/mã vạch qua đúng bề mặt API mà Dart gọi.
#![allow(clippy::expect_used)]

use std::io::Cursor;

use image::{DynamicImage, GrayImage, ImageFormat, Luma, Rgb, RgbImage};
use rxing::{BarcodeFormat, MultiFormatWriter, Writer};
use tsuscan_ffi::api::error::ApiResult;
use tsuscan_ffi::api::scan::{CapturedImage, scan_barcodes_preview};

fn png(img: &DynamicImage) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    assert!(img.write_to(&mut out, ImageFormat::Png).is_ok());
    out.into_inner()
}

/// Ảnh hai trang 1200x800: hai khối chữ + khe/bóng gáy ở `gx`.
fn spread(gx: u32) -> RgbImage {
    let (w, h) = (1200u32, 800u32);
    let mut img = RgbImage::from_pixel(w, h, Rgb([240, 238, 230]));
    for y in (40..h - 40).step_by(24) {
        for x in (60..1140).step_by(9) {
            for dy in 0..10 {
                for dx in 0..5 {
                    img.put_pixel(x + dx, y + dy, Rgb([30, 30, 30]));
                }
            }
        }
    }
    for x in gx - 50..gx + 50 {
        let shade = 200 - (40 - (i64::from(x) - i64::from(gx)).abs().min(40)) as u8 * 3;
        for y in 0..h {
            img.put_pixel(x, y, Rgb([shade, shade, shade]));
        }
    }
    img
}

#[test]
fn sach_ca_khung_tach_dung_gay() -> ApiResult<()> {
    let cap = CapturedImage::decode(png(&DynamicImage::ImageRgb8(spread(640))))?;
    let pages = cap.split_book(Vec::new())?.expect("phải tìm thấy gáy");
    assert_eq!(pages.len(), 2);
    let (l, r) = (&pages[0].corners, &pages[1].corners);
    // Trái: TL (0,0) → TR (gáy, 0); phải bắt đầu đúng ở gáy; hai trang ghép khít.
    assert!(l[0].abs() < 1e-6 && r[2] > 0.999, "{l:?} {r:?}");
    let g = l[2];
    assert!((g - 640.0 / 1200.0).abs() < 0.02, "gáy {g}");
    assert!((r[0] - g).abs() < 1e-9 && (r[6] - l[4]).abs() < 1e-9);
    Ok(())
}

#[test]
fn sach_trong_vung_dung_phoi_canh_chieu_nguoc() -> ApiResult<()> {
    // Sách nằm giữa nền tối 1600x1100, vùng (200,150)-(1400,950).
    let mut canvas = RgbImage::from_pixel(1600, 1100, Rgb([40, 50, 60]));
    image::imageops::overlay(&mut canvas, &spread(600), 200, 150);
    let cap = CapturedImage::decode(png(&DynamicImage::ImageRgb8(canvas)))?;
    let quad = vec![
        200.0 / 1600.0,
        150.0 / 1100.0,
        1400.0 / 1600.0,
        150.0 / 1100.0,
        1400.0 / 1600.0,
        950.0 / 1100.0,
        200.0 / 1600.0,
        950.0 / 1100.0,
    ];
    let pages = cap.split_book(quad.clone())?.expect("phải tìm thấy gáy");
    let gx = pages[0].corners[2] * 1600.0;
    assert!((gx - 800.0).abs() < 25.0, "gáy ở x={gx} (cần ≈ 800)");
    assert_eq!(&pages[0].corners[..2], &quad[..2]);
    assert_eq!(&pages[1].corners[2..6], &quad[2..6]);
    Ok(())
}

#[test]
fn trang_don_khong_tach() -> ApiResult<()> {
    let plain = RgbImage::from_pixel(1200, 800, Rgb([250, 250, 250]));
    let cap = CapturedImage::decode(png(&DynamicImage::ImageRgb8(plain)))?;
    assert!(cap.split_book(Vec::new())?.is_none());
    // Góc hỏng bị từ chối, không panic.
    assert!(cap.split_book(vec![0.0; 3]).is_err());
    Ok(())
}

fn qr_gray(text: &str, module: u32) -> GrayImage {
    let m = MultiFormatWriter
        .encode(text, &BarcodeFormat::QR_CODE, 0, 0)
        .expect("mã QR");
    let (w, h) = (m.getWidth(), m.getHeight());
    let pad = 4 * module;
    GrayImage::from_fn(w * module + 2 * pad, h * module + 2 * pad, |x, y| {
        let inside = x >= pad && y >= pad && x < pad + w * module && y < pad + h * module;
        Luma([if inside && m.get((x - pad) / module, (y - pad) / module) {
            0
        } else {
            255
        }])
    })
}

#[test]
fn qr_tren_khung_xem_truoc_co_stride() -> ApiResult<()> {
    let text = "https://tsuscan.tsudev.com/?thu=1";
    let (w, h, stride) = (1440u32, 1080u32, 1472u32);
    let mut frame = GrayImage::from_pixel(w, h, Luma([180]));
    image::imageops::overlay(&mut frame, &qr_gray(text, 6), 500, 300);
    let mut buf = vec![0u8; (stride * (h - 1) + w) as usize];
    for (y, row) in frame.rows().enumerate() {
        for (x, p) in row.enumerate() {
            buf[y * stride as usize + x] = p.0[0];
        }
    }
    let found = scan_barcodes_preview(buf, w, h, stride)?;
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].text, text);
    assert!(found[0].format.to_lowercase().contains("qr"));
    // Khung trơn: rỗng, không lỗi; kích thước sai: lỗi.
    assert_eq!(
        scan_barcodes_preview(vec![128; 640 * 480], 640, 480, 640)?,
        Vec::new()
    );
    assert!(scan_barcodes_preview(vec![0; 10], 640, 480, 640).is_err());
    Ok(())
}

#[test]
fn qr_tren_anh_chup() -> ApiResult<()> {
    let mut page = GrayImage::from_pixel(1600, 1200, Luma([255]));
    image::imageops::overlay(&mut page, &qr_gray("HD-2026-000123", 5), 100, 100);
    let cap = CapturedImage::decode(png(&DynamicImage::ImageLuma8(page)))?;
    let found = cap.barcodes();
    assert!(found.iter().any(|b| b.text == "HD-2026-000123"), "{found:?}");
    Ok(())
}

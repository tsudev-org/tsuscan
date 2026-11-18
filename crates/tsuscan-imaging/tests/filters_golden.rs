//! So mỗi bộ lọc với ảnh vàng `testdata/golden/filters/<mã>.png` theo PSNR (≥ 40 dB).
//!
//! Đổi thuật toán có chủ đích: chạy `TSUSCAN_BLESS=1 cargo test -p tsuscan-imaging --test filters_golden`,
//! xem lại ảnh mới bằng mắt rồi commit cùng PR.

use std::path::PathBuf;

use image::{DynamicImage, Rgb, RgbImage};
use tsuscan_imaging::filters::{Filter, apply};

/// Trang mẫu dựng thủ tục: giấy ngả vàng, bóng tối dần, "dòng chữ" tối, nét bút màu, vệt lóa.
fn sample_page() -> RgbImage {
    let (w, h) = (240u32, 320u32);
    RgbImage::from_fn(w, h, |x, y| {
        let (xf, yf) = (f64::from(x), f64::from(y));
        let shade = 1.0 - 0.35 * (xf / f64::from(w)) * (yf / f64::from(h));
        let mut c = [236.0 * shade, 228.0 * shade, 205.0 * shade];
        let line = (y % 18) < 5 && (20..220).contains(&x) && (x / 7) % 5 != 4 && y > 30;
        if line {
            c = [40.0, 38.0, 45.0];
        }
        if (yf - 0.5 * xf - 150.0).abs() < 2.0 && (60..180).contains(&x) {
            c = [200.0, 30.0, 40.0];
        }
        let glare = (-((xf - 170.0).powi(2) + (yf - 80.0).powi(2)) / 600.0).exp() * 50.0;
        Rgb(c.map(|v| (v + glare).round().clamp(0.0, 255.0) as u8))
    })
}

fn psnr(a: &DynamicImage, b: &DynamicImage) -> f64 {
    let (a, b) = (a.to_rgb8(), b.to_rgb8());
    assert_eq!(a.dimensions(), b.dimensions());
    let mse = a
        .as_raw()
        .iter()
        .zip(b.as_raw())
        .map(|(x, y)| (f64::from(*x) - f64::from(*y)).powi(2))
        .sum::<f64>()
        / a.as_raw().len() as f64;
    if mse == 0.0 {
        f64::INFINITY
    } else {
        10.0 * (255.0 * 255.0 / mse).log10()
    }
}

#[test]
fn filters_match_golden_images() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/golden/filters");
    let bless = std::env::var_os("TSUSCAN_BLESS").is_some();
    let page = sample_page();
    for f in Filter::ALL {
        let out = apply(&page, f, Some(30.0));
        let path = dir.join(format!("{}.png", f.code()));
        if bless {
            std::fs::create_dir_all(&dir).unwrap();
            out.save(&path).unwrap();
            continue;
        }
        let golden = image::open(&path).unwrap_or_else(|e| panic!("thiếu ảnh vàng {}: {e}", path.display()));
        let p = psnr(&out, &golden);
        assert!(
            p >= 40.0,
            "bộ lọc {} lệch ảnh vàng: PSNR {p:.1} dB < 40",
            f.code()
        );
    }
}

#[test]
fn filters_are_deterministic_and_typed() {
    let page = sample_page();
    for f in Filter::ALL {
        let a = apply(&page, f, None);
        let b = apply(&page, f, None);
        assert_eq!(a.as_bytes(), b.as_bytes(), "{} không thuần", f.code());
        let gray = matches!(f, Filter::Gray | Filter::BlackWhite);
        assert_eq!(matches!(a, DynamicImage::ImageLuma8(_)), gray);
    }
    // Đen trắng chỉ có 0/255.
    let bw = apply(&page, Filter::BlackWhite, None).to_luma8();
    assert!(bw.pixels().all(|p| p[0] == 0 || p[0] == 255));
}

//! Kiểm thử bền vững chạy được trên stable (bổ sung cho `fuzz/` cần nightly): dữ liệu hỏng/ngẫu nhiên chỉ
//! được trả `Err`, không panic. Tất định theo seed.
#![allow(clippy::unwrap_used)] // Hàm phụ dựng dữ liệu mẫu nằm ngoài #[test].

use std::io::Cursor;

use image::{DynamicImage, GrayImage, ImageFormat, Luma, RgbImage};
use tsuscan_imaging::synth::Rng;
use tsuscan_imaging::warp::{WarpOptions, warp_rgb};
use tsuscan_imaging::{DetectOptions, Point, Quad, decode, detect_luma};

fn encoded(format: ImageFormat) -> Vec<u8> {
    let img = RgbImage::from_fn(48, 32, |x, y| {
        image::Rgb([(x * 5) as u8, (y * 7) as u8, ((x + y) * 3) as u8])
    });
    let mut out = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(img).write_to(&mut out, format).unwrap();
    out.into_inner()
}

#[test]
fn mutated_images_never_panic() {
    let mut rng = Rng::new(2026);
    for format in [ImageFormat::Png, ImageFormat::Jpeg, ImageFormat::WebP] {
        let base = encoded(format);
        assert!(decode(&base).is_ok(), "{format:?} gốc phải giải mã được");
        for _ in 0..1500 {
            let mut bytes = base.clone();
            match rng.index(4) {
                0 => {
                    for _ in 0..=rng.index(8) {
                        let i = rng.index(bytes.len());
                        bytes[i] = rng.next_u64() as u8;
                    }
                }
                1 => bytes.truncate(rng.index(bytes.len())),
                2 => {
                    // Phần đầu (khai báo kích thước) bị ghi giá trị cực đại.
                    let i = rng.index(bytes.len().min(64));
                    bytes[i] = 0xFF;
                }
                _ => {
                    let i = rng.index(bytes.len());
                    let extra: Vec<u8> = (0..rng.index(64)).map(|_| rng.next_u64() as u8).collect();
                    bytes.splice(i..i, extra);
                }
            }
            let _ = decode(&bytes);
        }
    }
}

#[test]
fn random_frames_and_quads_never_panic() {
    let mut rng = Rng::new(7);
    for _ in 0..40 {
        let (w, h) = (16 + rng.index(300) as u32, 16 + rng.index(300) as u32);
        let mode = rng.index(3);
        let img = GrayImage::from_fn(w, h, |x, y| {
            Luma([match mode {
                0 => (rng.next_u64() & 0xFF) as u8,
                1 => {
                    if (x / 8 + y / 8) % 2 == 0 {
                        0
                    } else {
                        255
                    }
                }
                _ => ((x * y) % 256) as u8,
            }])
        });
        let _ = detect_luma(&img, DetectOptions::default());
        let rgb = DynamicImage::ImageLuma8(img).to_rgb8();
        let mut coord = || rng.range(-1e6, 1e6);
        let pts = [
            Point::new(coord(), coord()),
            Point::new(coord(), coord()),
            Point::new(coord(), coord()),
            Point::new(coord(), coord()),
        ];
        if let Ok(q) = Quad::from_unordered(pts) {
            let _ = warp_rgb(
                &rgb,
                &q,
                WarpOptions {
                    max_side: 128,
                    ..WarpOptions::default()
                },
            );
        }
    }
    // Cực trị: tứ giác siêu nhỏ, siêu lớn, có NaN.
    let rgb = RgbImage::new(32, 32);
    let tiny = Quad {
        pts: [
            Point::new(1.0, 1.0),
            Point::new(1.1, 1.0),
            Point::new(1.1, 1.1),
            Point::new(1.0, 1.1),
        ],
    };
    assert!(warp_rgb(&rgb, &tiny, WarpOptions::default()).is_err());
    let huge = Quad::full_frame(1_000_000, 1_000_000);
    assert!(warp_rgb(&rgb, &huge, WarpOptions::default()).is_err());
    assert!(Quad::from_unordered([Point::new(f64::NAN, 0.0); 4]).is_err());
}

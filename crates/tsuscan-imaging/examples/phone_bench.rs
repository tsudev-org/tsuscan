//! Đo dò mép khung xem trước 1440x1080 trên điện thoại Android thật, theo cỡ ảnh làm việc (chọn
//! `PREVIEW_WORKING_SIZE` của `tsuscan-ffi`). Build chéo + chạy qua adb (đổi thuật toán dò ⇒ chạy lại):
//!
//! ```sh
//! NDK=~/Android/Sdk/ndk/<bản>/toolchains/llvm/prebuilt/linux-x86_64/bin
//! CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=$NDK/aarch64-linux-android26-clang \
//!   cargo build --release -p tsuscan-imaging --example phone_bench --features synth --target aarch64-linux-android
//! adb push target/aarch64-linux-android/release/examples/phone_bench /data/local/tmp/
//! adb shell /data/local/tmp/phone_bench 30
//! ```
#![allow(missing_docs, clippy::unwrap_used)]
use image::{DynamicImage, imageops::FilterType};
use std::time::Instant;
use tsuscan_imaging::synth::{SynthConfig, generate};
use tsuscan_imaging::{DetectOptions, detect_luma, iou};

fn main() {
    let n: u64 = std::env::args().nth(1).and_then(|v| v.parse().ok()).unwrap_or(30);
    let frames: Vec<_> = (0..n)
        .map(|seed| {
            let s = generate(seed, &SynthConfig::default()).unwrap();
            let (w, h) = s.image.dimensions();
            let g = DynamicImage::ImageRgb8(s.image)
                .resize_exact(1440, 1080, FilterType::Triangle)
                .to_luma8();
            let k = (1440.0 / f64::from(w), 1080.0 / f64::from(h));
            let truth = tsuscan_imaging::Quad {
                pts: s
                    .truth
                    .pts
                    .map(|p| tsuscan_imaging::Point::new(p.x * k.0, p.y * k.1)),
            };
            (g, truth)
        })
        .collect();
    for ws in [320u32, 400, 480, 640] {
        let opts = DetectOptions {
            working_size: ws,
            ..DetectOptions::default()
        };
        let mut t = Vec::new();
        let mut ious = Vec::new();
        let mut miss = 0;
        for (g, truth) in &frames {
            let t0 = Instant::now();
            let d = detect_luma(g, opts).unwrap();
            t.push(t0.elapsed().as_secs_f64() * 1000.0);
            match d {
                Some(d) => ious.push(iou(&d.quad, truth)),
                None => miss += 1,
            }
        }
        t.sort_by(f64::total_cmp);
        let mean = t.iter().sum::<f64>() / t.len() as f64;
        let miou = ious.iter().sum::<f64>() / ious.len().max(1) as f64;
        println!(
            "ws={ws} mean={mean:.1}ms p50={:.1} p95={:.1} max={:.1} iou={miou:.4} miss={miss}",
            t[t.len() / 2],
            t[t.len() * 95 / 100 - 1],
            t[t.len() - 1]
        );
    }
    // Khung tối/đều (điện thoại úp): đo riêng.
    let dark = image::GrayImage::from_pixel(1440, 1080, image::Luma([12]));
    let t0 = Instant::now();
    for _ in 0..20 {
        let _ = detect_luma(
            &dark,
            DetectOptions {
                working_size: 480,
                ..DetectOptions::default()
            },
        );
    }
    println!("dark ws=480 mean={:.1}ms", t0.elapsed().as_secs_f64() * 50.0);
}

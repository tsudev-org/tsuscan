//! Chẩn đoán dò mép: `cargo run --release --example diag -- <seed đầu> <số ảnh> [thư mục vẽ khung]`.
//! In IoU + thời gian từng ảnh; có thư mục thì ghi ảnh có khung thật (xanh) và khung dò (đỏ).
#![allow(missing_docs, clippy::unwrap_used)]

use std::time::Instant;

use image::{DynamicImage, Rgb, RgbImage};
use tsuscan_imaging::synth::{SynthConfig, generate};
use tsuscan_imaging::{Quad, detect, iou};

fn draw(img: &mut RgbImage, q: &Quad, c: Rgb<u8>) {
    for i in 0..4 {
        let (a, b) = (q.pts[i], q.pts[(i + 1) % 4]);
        let n = a.dist(b).ceil() as u32;
        for t in 0..=n {
            let f = f64::from(t) / f64::from(n.max(1));
            let (x, y) = (a.x + (b.x - a.x) * f, a.y + (b.y - a.y) * f);
            for d in -1i32..=1 {
                let (px, py) = (x as i32 + d, y as i32);
                if px >= 0 && py >= 0 && (px as u32) < img.width() && (py as u32) < img.height() {
                    img.put_pixel(px as u32, py as u32, c);
                }
            }
        }
    }
}

/// Thời gian CPU của luồng hiện tại (ns), Linux: không tính thời gian chờ khi máy bận.
fn cpu_ns() -> Option<u64> {
    std::fs::read_to_string("/proc/thread-self/schedstat")
        .ok()?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let start: u64 = args.get(1).and_then(|v| v.parse().ok()).unwrap_or(0);
    let n: u64 = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(20);
    let out = args.get(3);
    let (mut tg, mut td, mut sum) = (0.0, 0.0, 0.0);
    for seed in start..start + n {
        let t0 = Instant::now();
        let s = generate(seed, &SynthConfig::default()).unwrap();
        let t1 = Instant::now();
        let dimg = DynamicImage::ImageRgb8(s.image.clone());
        let det = detect(&dimg).unwrap();
        let mut t2 = Instant::now();
        // Lấy lần nhanh nhất trong 5 lần: bớt nhiễu khi máy đang bận.
        let mut best = t2 - t1;
        for _ in 0..4 {
            let (t, c0) = (Instant::now(), cpu_ns());
            let _ = detect(&dimg).unwrap();
            let wall = t.elapsed();
            let cpu = cpu_ns()
                .zip(c0)
                .map(|(b, a)| std::time::Duration::from_nanos(b - a));
            best = best.min(cpu.unwrap_or(wall));
        }
        t2 = t1 + best;
        let score = det.map_or(0.0, |d| iou(&d.quad, &s.truth));
        sum += score;
        tg += (t1 - t0).as_secs_f64();
        td += (t2 - t1).as_secs_f64();
        let conf = det.map_or(0.0, |d| d.confidence);
        println!(
            "{seed:4} {:10} iou {score:.3} conf {conf:.2} det {:.1} ms",
            s.background.code(),
            (t2 - t1).as_secs_f64() * 1e3
        );
        if let Some(dir) = out {
            let mut img = s.image.clone();
            draw(&mut img, &s.truth, Rgb([0, 200, 0]));
            if let Some(d) = det {
                draw(&mut img, &d.quad, Rgb([230, 0, 0]));
            }
            img.save(format!("{dir}/{seed:04}.png")).unwrap();
        }
    }
    println!(
        "TB iou {:.4}; sinh {:.1} ms/ảnh; dò {:.1} ms/ảnh",
        sum / n as f64,
        tg * 1e3 / n as f64,
        td * 1e3 / n as f64
    );
}

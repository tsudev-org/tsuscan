//! Đo chất lượng dò mép trên bộ ảnh tổng hợp có nhãn (`testdata/edges`, sinh lúc test theo seed).
//!
//! Chỉ tiêu `docs/PLAN.md`: IoU trung bình ≥ 0,90 và ≥ 0,80 ở 95% ảnh. Không hạ ngưỡng khi đổi thuật toán.
//! Số mẫu: biến môi trường `TSUSCAN_EDGES_N` (mặc định 150, CI dùng mặc định).

use std::collections::BTreeMap;

use image::DynamicImage;
use tsuscan_imaging::synth::{Background, SynthConfig, generate};
use tsuscan_imaging::{detect, iou};

#[test]
fn edge_detection_meets_quality_targets() {
    let n: u64 = std::env::var("TSUSCAN_EDGES_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(150);
    let cfg = SynthConfig::default();
    let mut all = Vec::new();
    let mut by_bg: BTreeMap<Background, Vec<f64>> = BTreeMap::new();
    let mut worst = Vec::new();
    for seed in 0..n {
        let s = generate(seed, &cfg).unwrap();
        let det = detect(&DynamicImage::ImageRgb8(s.image)).unwrap();
        let score = det.map_or(0.0, |d| iou(&d.quad, &s.truth));
        all.push(score);
        by_bg.entry(s.background).or_default().push(score);
        worst.push((score, seed));
    }
    worst.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    let frac_ok = |v: &[f64]| v.iter().filter(|x| **x >= 0.8).count() as f64 / v.len() as f64;
    println!("\n| Nền | Số ảnh | IoU TB | ≥ 0,80 | Nhỏ nhất |");
    println!("| --- | ---: | ---: | ---: | ---: |");
    for (bg, v) in &by_bg {
        let min = v.iter().copied().fold(1.0, f64::min);
        println!(
            "| {} | {} | {:.4} | {:.1}% | {:.3} |",
            bg.code(),
            v.len(),
            mean(v),
            frac_ok(v) * 100.0,
            min
        );
    }
    println!(
        "| **Tất cả** | {} | {:.4} | {:.1}% | |",
        all.len(),
        mean(&all),
        frac_ok(&all) * 100.0
    );
    println!("Tệ nhất (IoU, seed): {:?}", &worst[..worst.len().min(8)]);

    assert!(mean(&all) >= 0.90, "IoU trung bình {:.4} < 0,90", mean(&all));
    assert!(
        frac_ok(&all) >= 0.95,
        "chỉ {:.1}% ảnh đạt IoU ≥ 0,80",
        frac_ok(&all) * 100.0
    );
}

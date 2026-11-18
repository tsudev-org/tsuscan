//! Benchmark dò mép trên khung 720p (chỉ tiêu < 40 ms trên x86).
#![allow(missing_docs, clippy::unwrap_used)]

use criterion::{Criterion, criterion_group, criterion_main};
use image::DynamicImage;
use tsuscan_imaging::synth::{SynthConfig, generate};
use tsuscan_imaging::{detect, filters, warp};

fn bench(c: &mut Criterion) {
    // Seed đầu tiên sinh khung 1280x720.
    let s = (0..1000)
        .map(|seed| generate(seed, &SynthConfig::default()).unwrap())
        .find(|s| s.image.dimensions() == (1280, 720))
        .unwrap();
    let img = DynamicImage::ImageRgb8(s.image.clone());
    c.bench_function("detect_720p", |b| {
        b.iter(|| detect(std::hint::black_box(&img)).unwrap());
    });
    let quad = s.truth;
    c.bench_function("warp_720p_bicubic", |b| {
        b.iter(|| warp::warp_rgb(&s.image, &quad, warp::WarpOptions::default()).unwrap());
    });
    let page = warp::warp_rgb(&s.image, &quad, warp::WarpOptions::default())
        .unwrap()
        .image;
    c.bench_function("filter_bw", |b| {
        b.iter(|| filters::apply(&page, filters::Filter::BlackWhite, None));
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);

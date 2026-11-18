//! Chẩn đoán tốc độ OCR theo thời gian CPU luồng (không bị nhiễu khi máy bận).
//! `cargo run --release -p tsuscan-ml --example ocr_diag -- [seed] [số trang]`

#![allow(clippy::unwrap_used, missing_docs)]

use std::path::Path;
use std::time::Instant;

use tsuscan_ml::ModelStore;
use tsuscan_ml::metrics::char_errors;
use tsuscan_ml::ocr::{OcrEngine, reading_order};
use tsuscan_ml::synth::{NamedFont, OcrSynthConfig, generate_page, parse_corpus};

fn cpu() -> f64 {
    let s = std::fs::read_to_string("/proc/thread-self/stat").unwrap_or_default();
    let f: Vec<&str> = s.rsplit(')').next().unwrap_or("").split_whitespace().collect();
    let t = |i: usize| f.get(i).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    (t(11) + t(12)) / 100.0
}

fn main() {
    let a: Vec<u64> = std::env::args().skip(1).filter_map(|v| v.parse().ok()).collect();
    let (seed, n) = (a.first().copied().unwrap_or(0), a.get(1).copied().unwrap_or(2));
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let corpus = parse_corpus(&std::fs::read_to_string(root.join("testdata/ocr-vi/corpus.txt")).unwrap());
    let fonts: Vec<NamedFont> = [
        "Tinos-Regular",
        "Tinos-Bold",
        "Arimo-wght",
        "BeVietnamPro-Regular",
    ]
    .iter()
    .map(|f| {
        NamedFont::from_bytes(
            f,
            std::fs::read(root.join(format!("testdata/fonts/{f}.ttf"))).unwrap(),
        )
        .unwrap()
    })
    .collect();
    let (c0, t0) = (cpu(), Instant::now());
    let engine = OcrEngine::from_store(&ModelStore::from_env().unwrap()).unwrap();
    println!(
        "nạp: {:.2} s CPU, {:.2} s tường; det ký hiệu={}, rec ký hiệu={}",
        cpu() - c0,
        t0.elapsed().as_secs_f64(),
        engine.detector().is_symbolic(),
        engine.recognizer().is_symbolic()
    );
    for s in seed..seed + n {
        let page = generate_page(s, &corpus, &fonts, &OcrSynthConfig::default()).unwrap();
        let c1 = cpu();
        let boxes = engine.detector().detect(&page.image).unwrap();
        let c2 = cpu();
        let ordered = reading_order(boxes.into_iter().map(|b| b.quad).collect());
        let mut text = Vec::new();
        for (q, _) in &ordered {
            text.push(engine.recognizer().recognize_quad(&page.image, q).unwrap().text);
        }
        let c3 = cpu();
        let (e, t) = char_errors(&page.text(), &text.join("\n"));
        println!(
            "trang {s}: det {:.2} s, rec {:.2} s ({} dòng, {:.0} ms/dòng) CPU; CER {:.2}%",
            c2 - c1,
            c3 - c2,
            ordered.len(),
            1000.0 * (c3 - c2) / ordered.len().max(1) as f64,
            100.0 * e as f64 / t.max(1) as f64
        );
    }
}

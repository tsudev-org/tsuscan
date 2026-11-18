//! Đo CER OCR tiếng Việt trên bộ trang sinh tất định (`testdata/ocr-vi`). Ngưỡng PLAN: CER ≤ 3%.
//!
//! Cần tệp mô hình (`scripts/fetch-models.sh`). Thiếu mô hình: bỏ qua có thông báo, trừ khi
//! `TSUSCAN_REQUIRE_MODELS=1` (CI) thì trượt. Số trang: `TSUSCAN_OCR_PAGES` (mặc định 12).

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use tsuscan_ml::ModelStore;
use tsuscan_ml::metrics::{char_errors, normalize_text, wer};
use tsuscan_ml::ocr::{DET_MODEL, OcrEngine, REC_MODEL};
use tsuscan_ml::synth::{NamedFont, OcrSynthConfig, generate_page, parse_corpus};

const MAX_CER: f64 = 0.03;

#[allow(clippy::unwrap_used)]
fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
}

#[allow(clippy::unwrap_used)]
fn fonts() -> Vec<NamedFont> {
    let dir = root().join("testdata/fonts");
    [
        "Tinos-Regular",
        "Tinos-Bold",
        "Arimo-wght",
        "BeVietnamPro-Regular",
    ]
    .iter()
    .map(|n| NamedFont::from_bytes(n, std::fs::read(dir.join(format!("{n}.ttf"))).unwrap()).unwrap())
    .collect()
}

#[allow(clippy::unwrap_used)]
fn engine() -> Option<OcrEngine> {
    let store = ModelStore::from_env().unwrap();
    if !(store.is_present(DET_MODEL) && store.is_present(REC_MODEL)) {
        assert!(
            std::env::var("TSUSCAN_REQUIRE_MODELS").as_deref() != Ok("1"),
            "thiếu mô hình OCR trong khi TSUSCAN_REQUIRE_MODELS=1"
        );
        eprintln!("BỎ QUA: chưa có mô hình OCR - chạy scripts/fetch-models.sh");
        return None;
    }
    Some(OcrEngine::from_store(&store).unwrap())
}

#[test]
#[allow(clippy::unwrap_used)]
fn cer_tieng_viet_tren_trang_sinh() {
    let Some(engine) = engine() else { return };
    let corpus = parse_corpus(&std::fs::read_to_string(root().join("testdata/ocr-vi/corpus.txt")).unwrap());
    let fonts = fonts();
    let pages: u64 = std::env::var("TSUSCAN_OCR_PAGES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(12);
    let cfg = OcrSynthConfig::default();
    let (mut errs, mut total, mut wer_sum) = (0usize, 0usize, 0.0f64);
    let mut by_font: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let t0 = Instant::now();
    for seed in 0..pages {
        let page = generate_page(seed, &corpus, &fonts, &cfg).unwrap();
        let res = engine.recognize(&page.image).unwrap();
        let (truth, hyp) = (page.text(), res.text());
        let (e, n) = char_errors(&truth, &hyp);
        let c = e as f64 / n.max(1) as f64;
        println!(
            "trang {seed:>3} {:<22} CER {:>6.2}% ({e}/{n})",
            page.font,
            100.0 * c
        );
        if c > 0.03 {
            for (t, h) in normalize_text(&truth).lines().zip(normalize_text(&hyp).lines()) {
                if t != h {
                    println!("    chuẩn: {t}\n    đọc  : {h}");
                }
            }
        }
        errs += e;
        total += n;
        wer_sum += wer(&truth, &hyp);
        let f = by_font.entry(page.font.clone()).or_default();
        f.0 += e;
        f.1 += n;
    }
    let cer = errs as f64 / total.max(1) as f64;
    println!("---");
    for (f, (e, n)) in &by_font {
        println!("{f:<22} CER {:>6.2}%", 100.0 * *e as f64 / (*n).max(1) as f64);
    }
    println!(
        "TỔNG {pages} trang: CER {:.2}% ({errs}/{total}), WER TB {:.2}%, {:.1} s/trang (tường)",
        100.0 * cer,
        100.0 * wer_sum / pages as f64,
        t0.elapsed().as_secs_f64() / pages as f64
    );
    assert!(
        cer <= MAX_CER,
        "CER {:.2}% > {:.0}%",
        100.0 * cer,
        100.0 * MAX_CER
    );
}

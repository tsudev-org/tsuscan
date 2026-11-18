//! Đầu-cuối có mô hình: trang tiếng Việt sinh tất định → OCR thật → PDF có lớp chữ → `pdftotext` trích lại.
//! Đo: chữ trích từ PDF so với chữ chuẩn (CER ≤ 3%, cùng ngưỡng PLAN) và khớp chữ OCR (lớp chữ không làm mất/đổi
//! ký tự). Thiếu mô hình/poppler: bỏ qua có thông báo, trừ khi `TSUSCAN_REQUIRE_MODELS=1` / `TSUSCAN_REQUIRE_PDF_TOOLS=1`.

// Hàm phụ của test (không gắn #[test]) cũng được phép unwrap: lỗi = test trượt.
#![allow(clippy::unwrap_used)]

use std::path::Path;
use std::process::Command;

use image::DynamicImage;
use tsuscan_doc::export::text;
use tsuscan_doc::{Document, Page, PdfOptions, write_pdf};
use tsuscan_ml::ModelStore;
use tsuscan_ml::metrics::char_errors;
use tsuscan_ml::ocr::{DET_MODEL, OcrEngine, REC_MODEL};
use tsuscan_ml::synth::{NamedFont, OcrSynthConfig, generate_page, parse_corpus};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
}

#[test]
fn ocr_text_layer_matches_truth() {
    let store = ModelStore::from_env().unwrap();
    if !(store.is_present(DET_MODEL) && store.is_present(REC_MODEL)) {
        assert!(
            std::env::var("TSUSCAN_REQUIRE_MODELS").as_deref() != Ok("1"),
            "thiếu mô hình OCR trong khi TSUSCAN_REQUIRE_MODELS=1"
        );
        eprintln!("BỎ QUA: chưa có mô hình OCR - chạy scripts/fetch-models.sh");
        return;
    }
    if Command::new("pdftotext").arg("-v").output().is_err() {
        assert!(
            std::env::var("TSUSCAN_REQUIRE_PDF_TOOLS").as_deref() != Ok("1"),
            "thiếu pdftotext"
        );
        eprintln!("BỎ QUA: thiếu pdftotext (poppler-utils)");
        return;
    }
    let engine = OcrEngine::from_store(&store).unwrap();
    let corpus = parse_corpus(&std::fs::read_to_string(root().join("testdata/ocr-vi/corpus.txt")).unwrap());
    let font = NamedFont::from_bytes(
        "Tinos-Regular",
        std::fs::read(root().join("testdata/fonts/Tinos-Regular.ttf")).unwrap(),
    )
    .unwrap();
    let synth = generate_page(3, &corpus, &[font], &OcrSynthConfig::default()).unwrap();
    let ocr = engine.recognize(&synth.image).unwrap();
    let doc = Document::new(vec![
        Page::new(DynamicImage::ImageRgb8(synth.image.clone()))
            .with_dpi(150.0)
            .with_ocr(ocr),
    ]);
    let pdf = write_pdf(&doc, &PdfOptions::default()).unwrap();
    let dir = std::env::temp_dir().join(format!("tsuscan-ocr-pdf-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("ocr.pdf");
    std::fs::write(&path, &pdf).unwrap();
    let out = Command::new("pdftotext").arg(&path).arg("-").output().unwrap();
    let extracted = String::from_utf8(out.stdout).unwrap();

    let truth = synth.text();
    let ocr_text = text::to_txt(&doc);
    let (e1, n1) = char_errors(&truth, &extracted);
    let (e2, n2) = char_errors(&ocr_text, &extracted);
    let cer_truth = e1 as f64 / n1.max(1) as f64;
    let cer_layer = e2 as f64 / n2.max(1) as f64;
    println!(
        "PDF {} KB; CER(chuẩn, chữ trích từ PDF) {:.2}%; CER(chữ OCR, chữ trích từ PDF) {:.2}%",
        pdf.len() / 1024,
        100.0 * cer_truth,
        100.0 * cer_layer
    );
    assert!(cer_truth <= 0.03, "CER so chuẩn {:.2}%", 100.0 * cer_truth);
    assert!(
        cer_layer <= 0.005,
        "lớp chữ lệch chữ OCR {:.2}%",
        100.0 * cer_layer
    );
}

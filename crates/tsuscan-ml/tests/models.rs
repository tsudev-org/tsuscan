//! Test tích hợp các mô hình phụ trợ: hướng trang (PP-LCNet doc_ori) và làm phẳng trang cong (UVDoc).
//! Thiếu mô hình: bỏ qua có thông báo, trừ khi `TSUSCAN_REQUIRE_MODELS=1`.

use std::path::Path;

use tsuscan_ml::ModelStore;
use tsuscan_ml::dewarp::{self, Dewarper};
use tsuscan_ml::metrics::cer;
use tsuscan_ml::ocr::OcrEngine;
use tsuscan_ml::orientation::{self, OrientationClassifier, Rotation};
use tsuscan_ml::synth::{NamedFont, OcrSynthConfig, curl, generate_page, parse_corpus};

#[allow(clippy::unwrap_used)]
fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
}

#[allow(clippy::unwrap_used)]
fn store_with(ids: &[&str]) -> Option<ModelStore> {
    let store = ModelStore::from_env().unwrap();
    if ids.iter().all(|id| store.is_present(id)) {
        return Some(store);
    }
    assert!(
        std::env::var("TSUSCAN_REQUIRE_MODELS").as_deref() != Ok("1"),
        "thiếu mô hình {ids:?}"
    );
    eprintln!("BỎ QUA: thiếu mô hình {ids:?} - chạy scripts/fetch-models.sh");
    None
}

#[allow(clippy::unwrap_used)]
fn page(seed: u64, degrade: bool) -> tsuscan_ml::synth::OcrPage {
    let corpus = parse_corpus(&std::fs::read_to_string(root().join("testdata/ocr-vi/corpus.txt")).unwrap());
    let fonts: Vec<NamedFont> = ["Tinos-Regular", "Arimo-wght", "BeVietnamPro-Regular"]
        .iter()
        .map(|n| {
            NamedFont::from_bytes(
                n,
                std::fs::read(root().join(format!("testdata/fonts/{n}.ttf"))).unwrap(),
            )
            .unwrap()
        })
        .collect();
    generate_page(
        seed,
        &corpus,
        &fonts,
        &OcrSynthConfig {
            degrade,
            ..OcrSynthConfig::default()
        },
    )
    .unwrap()
}

#[test]
#[allow(clippy::unwrap_used)]
fn huong_trang_dung_ca_bon_goc() {
    let Some(store) = store_with(&[orientation::MODEL]) else {
        return;
    };
    let c = OrientationClassifier::from_bytes(&store.load_verified(orientation::MODEL).unwrap()).unwrap();
    let mut ok = 0;
    let mut total = 0;
    for seed in 0..6 {
        let upright = page(seed, true).image;
        // Ảnh bị xoay theo chiều kim đồng hồ `r` ⇒ phép sửa đúng là xoay ngược lại.
        for (r, fix) in [
            (Rotation::None, Rotation::None),
            (Rotation::Cw90, Rotation::Cw270),
            (Rotation::Cw180, Rotation::Cw180),
            (Rotation::Cw270, Rotation::Cw90),
        ] {
            let got = c.classify(&r.apply(&upright)).unwrap();
            total += 1;
            if got.fix == fix {
                ok += 1;
            } else {
                println!(
                    "seed {seed} xoay {}°: đoán sửa {}° (cần {}°) p={:.2}",
                    r.degrees(),
                    got.fix.degrees(),
                    fix.degrees(),
                    got.confidence
                );
            }
        }
    }
    println!("hướng trang đúng {ok}/{total}");
    assert!(ok * 100 >= total * 95, "hướng trang đúng {ok}/{total} < 95%");
}

#[test]
#[allow(clippy::unwrap_used)]
fn lam_phang_giam_cer_trang_cong() {
    let Some(store) = store_with(&[dewarp::MODEL, "ocr-det", "ocr-rec-vi"]) else {
        return;
    };
    let d = Dewarper::from_bytes(&store.load_verified(dewarp::MODEL).unwrap()).unwrap();
    let engine = OcrEngine::from_store(&store).unwrap();
    let flat = page(3, false);
    let curled = curl(&flat.image, 45.0);
    let fixed = d.dewarp(&curled).unwrap();
    let truth = flat.text();
    let c_curl = cer(&truth, &engine.recognize(&curled).unwrap().text());
    let c_fixed = cer(&truth, &engine.recognize(&fixed).unwrap().text());
    println!(
        "CER trang cong {:.2}% → sau UVDoc {:.2}%",
        100.0 * c_curl,
        100.0 * c_fixed
    );
    if let Ok(dir) = std::env::var("TSUSCAN_DUMP") {
        curled.save(Path::new(&dir).join("cong.png")).unwrap();
        fixed.save(Path::new(&dir).join("phang.png")).unwrap();
    }
    assert!(
        c_fixed < c_curl,
        "làm phẳng không cải thiện: {c_curl} → {c_fixed}"
    );
    assert!(c_fixed <= 0.05, "CER sau làm phẳng {:.2}% > 5%", 100.0 * c_fixed);
}

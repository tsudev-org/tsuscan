//! `tsuscan`: CLI dev cho lõi ảnh và AI trên máy. Không phải sản phẩm phát hành (app nằm ở `apps/app`).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use image::DynamicImage;
use serde_json::json;
use tsuscan_imaging::filters::{self, Filter};
use tsuscan_imaging::synth::{SynthConfig, generate};
use tsuscan_imaging::warp::{WarpOptions, warp};
use tsuscan_imaging::{Quad, decode, detect};

#[derive(Parser)]
#[command(
    name = "tsuscan",
    version,
    about = "CLI dev của Tsu Scan (lõi ảnh + AI trên máy)"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Dò tứ giác tài liệu trong ảnh.
    Detect {
        /// Ảnh vào (JPEG/PNG/WebP).
        image: PathBuf,
        /// In kết quả dạng JSON.
        #[arg(long)]
        json: bool,
    },
    /// Dò mép + nắn + lọc, ghi ảnh trang.
    Scan {
        /// Ảnh vào.
        image: PathBuf,
        /// Bộ lọc: original, auto, color, gray, bw, shadow, whiteboard.
        #[arg(short, long, default_value = "auto")]
        filter: String,
        /// Tệp ra (PNG/JPEG theo đuôi).
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Sinh ảnh tổng hợp có nhãn (bộ mẫu `testdata/edges`).
    Synth {
        /// Seed đầu.
        #[arg(long, default_value_t = 0)]
        seed: u64,
        /// Số ảnh.
        #[arg(short = 'n', long, default_value_t = 10)]
        count: u64,
        /// Hệ số kích thước khung.
        #[arg(long, default_value_t = 1.0)]
        scale: f64,
        /// Thư mục ra (ghi `<seed>.jpg` + `labels.json`).
        #[arg(short, long)]
        out: PathBuf,
    },
    /// OCR một ảnh trang đã nắn (mô hình trong `TSUSCAN_MODELS` hoặc `models/`).
    Ocr {
        /// Ảnh trang.
        image: PathBuf,
        /// Định dạng ra: text, markdown, json.
        #[arg(short, long, default_value = "text")]
        format: String,
        /// Tự xoay trang về đúng chiều trước khi OCR.
        #[arg(long)]
        orient: bool,
    },
    /// Đoán hướng trang (0/90/180/270).
    Orient {
        /// Ảnh trang.
        image: PathBuf,
    },
    /// Làm phẳng trang cong (UVDoc).
    Dewarp {
        /// Ảnh trang.
        image: PathBuf,
        /// Tệp ra.
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Tách ảnh hai trang sách tại gáy (ghi `<tên>-trai` và `<tên>-phai`).
    Split {
        /// Ảnh hai trang.
        image: PathBuf,
        /// Tệp ra (đuôi quyết định định dạng).
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Xóa ngón tay chạm mép trang.
    Definger {
        /// Ảnh trang.
        image: PathBuf,
        /// Tệp ra.
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Sinh trang chữ tiếng Việt có nhãn (bộ mẫu `testdata/ocr-vi`).
    SynthOcr {
        /// Seed đầu.
        #[arg(long, default_value_t = 0)]
        seed: u64,
        /// Số trang.
        #[arg(short = 'n', long, default_value_t = 5)]
        count: u64,
        /// Thư mục ra (ghi `<seed>.jpg` + `<seed>.txt`).
        #[arg(short, long)]
        out: PathBuf,
    },
    /// Kiểm toàn vẹn các tệp mô hình theo manifest.
    Models,
}

fn quad_json(q: &Quad) -> serde_json::Value {
    json!(
        q.pts
            .iter()
            .map(|p| [(p.x * 100.0).round() / 100.0, (p.y * 100.0).round() / 100.0])
            .collect::<Vec<_>>()
    )
}

fn ml<T, E: std::fmt::Display>(r: Result<T, E>) -> Result<T, String> {
    r.map_err(|e| e.to_string())
}

fn repo_path(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn save(img: &image::RgbImage, path: &Path) -> Result<(), String> {
    img.save(path)
        .map_err(|e| format!("không ghi được {}: {e}", path.display()))
}

fn suffixed(path: &Path, suffix: &str) -> PathBuf {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("trang");
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("png");
    path.with_file_name(format!("{stem}-{suffix}.{ext}"))
}

fn check_models() -> Result<(), String> {
    let store = ml(tsuscan_ml::ModelStore::from_env())?;
    let mut bad = 0;
    for m in &store.manifest().models {
        let status = if store.is_present(&m.id) {
            match store.load_verified(&m.id) {
                Ok(_) => "đạt".to_owned(),
                Err(e) => {
                    bad += 1;
                    format!("HỎNG: {e}")
                }
            }
        } else {
            "CHƯA TẢI".to_owned()
        };
        println!("{:<18} {:<11} {:>10} B  {}", m.id, m.license, m.size, status);
    }
    if bad > 0 {
        return Err(format!("{bad} mô hình không qua kiểm toàn vẹn"));
    }
    Ok(())
}

fn run_ocr(image: &Path, format: &str, orient: bool) -> Result<(), String> {
    use tsuscan_ml::{ModelStore, layout, orientation};
    let store = ml(ModelStore::from_env())?;
    let mut page = load(image)?.to_rgb8();
    if orient {
        let c = ml(orientation::OrientationClassifier::from_bytes(&ml(
            store.load_verified(orientation::MODEL)
        )?))?;
        let (p, r) = ml(c.upright(&page, 0.6))?;
        eprintln!("Hướng: xoay {}°", r.degrees());
        page = p;
    }
    let engine = ml(tsuscan_ml::OcrEngine::from_store(&store))?;
    let res = ml(engine.recognize(&page))?;
    match format {
        "text" => println!("{}", res.text()),
        "markdown" => println!(
            "{}",
            layout::to_markdown(&layout::analyze(&res, f64::from(page.width())))
        ),
        "json" => {
            let lines: Vec<_> = res
                        .lines
                        .iter()
                        .map(|l| json!({ "text": l.text, "quad": quad_json(&l.quad), "confidence": l.confidence, "row": l.row }))
                        .collect();
            println!(
                "{}",
                json!({ "width": page.width(), "height": page.height(), "lines": lines })
            );
        }
        other => return Err(format!("định dạng `{other}` không hỗ trợ (text, markdown, json)")),
    }
    Ok(())
}

fn run_ml(cmd: Cmd) -> Result<(), String> {
    use tsuscan_ml::{ModelStore, dewarp, orientation, pages, synth};
    match cmd {
        Cmd::Ocr {
            image,
            format,
            orient,
        } => run_ocr(&image, &format, orient)?,
        Cmd::Orient { image } => {
            let store = ml(ModelStore::from_env())?;
            let c = ml(orientation::OrientationClassifier::from_bytes(&ml(
                store.load_verified(orientation::MODEL)
            )?))?;
            let r = ml(c.classify(&load(&image)?.to_rgb8()))?;
            println!(
                "Cần xoay {}° theo chiều kim đồng hồ - tin cậy {:.2}",
                r.fix.degrees(),
                r.confidence
            );
        }
        Cmd::Dewarp { image, output } => {
            let store = ml(ModelStore::from_env())?;
            let d = ml(dewarp::Dewarper::from_bytes(&ml(
                store.load_verified(dewarp::MODEL)
            )?))?;
            let out = ml(d.dewarp(&load(&image)?.to_rgb8()))?;
            save(&out, &output)?;
            println!("Đã ghi {}", output.display());
        }
        Cmd::Split { image, output } => {
            let img = load(&image)?.to_rgb8();
            let (l, r) = pages::split_spread(&img).ok_or("không tìm thấy gáy sách")?;
            let (lp, rp) = (suffixed(&output, "trai"), suffixed(&output, "phai"));
            save(&l, &lp)?;
            save(&r, &rp)?;
            println!("Đã ghi {} và {}", lp.display(), rp.display());
        }
        Cmd::Definger { image, output } => {
            let (out, frac) = pages::remove_fingers(&load(&image)?.to_rgb8());
            save(&out, &output)?;
            println!("Đã xóa {:.2}% diện tích, ghi {}", 100.0 * frac, output.display());
        }
        Cmd::SynthOcr { seed, count, out } => {
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            let corpus = synth::parse_corpus(
                &std::fs::read_to_string(repo_path("testdata/ocr-vi/corpus.txt"))
                    .map_err(|e| e.to_string())?,
            );
            let mut fonts = Vec::new();
            for n in [
                "Tinos-Regular",
                "Tinos-Bold",
                "Arimo-wght",
                "BeVietnamPro-Regular",
            ] {
                let bytes = std::fs::read(repo_path(&format!("testdata/fonts/{n}.ttf")))
                    .map_err(|e| e.to_string())?;
                fonts.push(ml(synth::NamedFont::from_bytes(n, bytes))?);
            }
            for s in seed..seed.saturating_add(count) {
                let p = ml(synth::generate_page(
                    s,
                    &corpus,
                    &fonts,
                    &synth::OcrSynthConfig::default(),
                ))?;
                save(&p.image, &out.join(format!("{s:04}.png")))?;
                std::fs::write(out.join(format!("{s:04}.txt")), p.text() + "\n")
                    .map_err(|e| e.to_string())?;
            }
            println!("Đã sinh {count} trang vào {}", out.display());
        }
        Cmd::Models => check_models()?,
        Cmd::Detect { .. } | Cmd::Scan { .. } | Cmd::Synth { .. } => {
            return Err("lệnh lõi ảnh xử lý ở run()".into());
        }
    }
    Ok(())
}

fn load(path: &Path) -> Result<DynamicImage, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("không đọc được {}: {e}", path.display()))?;
    decode(&bytes).map_err(|e| e.to_string())
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.cmd {
        Cmd::Detect { image, json } => {
            let img = load(&image)?;
            let det = detect(&img).map_err(|e| e.to_string())?;
            if json {
                let v = det.map_or(
                    serde_json::Value::Null,
                    |d| json!({ "quad": quad_json(&d.quad), "confidence": d.confidence }),
                );
                println!("{v}");
            } else if let Some(d) = det {
                let pts: Vec<String> = d
                    .quad
                    .pts
                    .iter()
                    .map(|p| format!("({:.1}, {:.1})", p.x, p.y))
                    .collect();
                println!("Tứ giác: {} - tin cậy {:.2}", pts.join(" "), d.confidence);
            } else {
                println!("Không tìm thấy tài liệu");
            }
        }
        Cmd::Scan {
            image,
            filter,
            output,
        } => {
            let filter = Filter::from_code(&filter).map_err(|e| e.to_string())?;
            let img = load(&image)?;
            let quad = detect(&img)
                .map_err(|e| e.to_string())?
                .map_or_else(|| Quad::full_frame(img.width(), img.height()), |d| d.quad);
            let page = warp(&img, &quad, WarpOptions::default()).map_err(|e| e.to_string())?;
            let out = filters::apply(&page.image, filter, None);
            out.save(&output)
                .map_err(|e| format!("không ghi được {}: {e}", output.display()))?;
            println!("Đã ghi {} ({}x{})", output.display(), out.width(), out.height());
        }
        Cmd::Synth {
            seed,
            count,
            scale,
            out,
        } => {
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            let cfg = SynthConfig { scale, effects: true };
            let mut labels = serde_json::Map::new();
            for s in seed..seed.saturating_add(count) {
                let sample = generate(s, &cfg).map_err(|e| e.to_string())?;
                let name = format!("{s:04}.jpg");
                let mut f = std::fs::File::create(out.join(&name)).map_err(|e| e.to_string())?;
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut f, 80)
                    .encode_image(&sample.image)
                    .map_err(|e| e.to_string())?;
                labels.insert(name, json!({ "seed": s, "background": sample.background.code(), "quad": quad_json(&sample.truth) }));
            }
            let text = serde_json::to_string_pretty(&labels).map_err(|e| e.to_string())?;
            std::fs::write(out.join("labels.json"), text + "\n").map_err(|e| e.to_string())?;
            println!("Đã sinh {count} ảnh vào {}", out.display());
        }
        other => run_ml(other)?,
    }
    Ok(())
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("lỗi: {e}");
            ExitCode::FAILURE
        }
    }
}

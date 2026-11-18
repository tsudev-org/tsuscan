//! `tsuscan`: CLI dev cho lõi ảnh. Không phải sản phẩm phát hành (app nằm ở `apps/app`).

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
#[command(name = "tsuscan", version, about = "CLI dev của Tsu Scan (lõi ảnh)")]
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
}

fn quad_json(q: &Quad) -> serde_json::Value {
    json!(
        q.pts
            .iter()
            .map(|p| [(p.x * 100.0).round() / 100.0, (p.y * 100.0).round() / 100.0])
            .collect::<Vec<_>>()
    )
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

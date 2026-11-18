//! Lệnh tầng tài liệu (P3): ghi PDF, xuất DOCX/XLSX/TXT/Markdown/ảnh, nhập PDF, che thông tin, đọc mã vạch.

use std::path::{Path, PathBuf};

use clap::Subcommand;
use serde_json::json;
use tsuscan_doc::export::image::{ImageFormat, export_image};
use tsuscan_doc::export::{docx, text, xlsx};
use tsuscan_doc::pdf::read::ImportOptions;
use tsuscan_doc::redact::{self, Rect};
use tsuscan_doc::{
    ColorMode, Document, Metadata, Page, PdfOptions, Permissions, Protection, Quality, Timestamp, barcode,
    import, write_pdf,
};

/// Lệnh tài liệu.
#[derive(Subcommand)]
pub enum DocCmd {
    /// Ghép ảnh/PDF thành một PDF (tùy chọn OCR lớp chữ ẩn, PDF/A-2b, mật khẩu AES-256).
    Pdf {
        /// Tệp vào (JPEG/PNG/WebP/PDF), theo thứ tự trang.
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
        /// Tệp PDF ra.
        #[arg(short, long)]
        output: PathBuf,
        /// Chạy OCR và thêm lớp chữ tìm được.
        #[arg(long)]
        ocr: bool,
        /// Ghi chuẩn lưu trữ PDF/A-2b.
        #[arg(long)]
        pdfa: bool,
        /// Tên BIẾN MÔI TRƯỜNG chứa mật khẩu mở tệp (không nhận mật khẩu trên dòng lệnh: lộ qua lịch sử/ps).
        #[arg(long)]
        password_env: Option<String>,
        /// Chất lượng: high, balanced, small.
        #[arg(short, long, default_value = "balanced")]
        quality: String,
        /// Màu: auto, color, gray, bw.
        #[arg(short, long, default_value = "auto")]
        color: String,
        /// Tiêu đề tài liệu.
        #[arg(long, default_value = "")]
        title: String,
        /// DPI của ảnh vào (quyết định khổ giấy).
        #[arg(long, default_value_t = 300.0)]
        dpi: f32,
    },
    /// Xuất ảnh/PDF sang DOCX, XLSX, TXT, Markdown (theo đuôi tệp ra) - luôn OCR.
    Export {
        /// Tệp vào.
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
        /// Tệp ra: .docx .xlsx .txt .md
        #[arg(short, long)]
        output: PathBuf,
        /// Tiêu đề.
        #[arg(long, default_value = "")]
        title: String,
    },
    /// Nhập PDF/ảnh, ghi từng trang thành ảnh (JPEG/PNG/WebP theo `--format`).
    Import {
        /// Tệp vào.
        input: PathBuf,
        /// Thư mục ra.
        #[arg(short, long)]
        out: PathBuf,
        /// DPI vẽ trang PDF.
        #[arg(long, default_value_t = 200.0)]
        dpi: f32,
        /// Biến môi trường chứa mật khẩu PDF.
        #[arg(long)]
        password_env: Option<String>,
        /// Định dạng ảnh ra: jpg, png, webp.
        #[arg(short, long, default_value = "png")]
        format: String,
    },
    /// Che thông tin (xóa hẳn) rồi ghi PDF: theo chuỗi tìm trong chữ OCR và/hoặc vùng x,y,w,h.
    Redact {
        /// Ảnh trang.
        image: PathBuf,
        /// Chuỗi cần che (lặp lại được).
        #[arg(long)]
        find: Vec<String>,
        /// Vùng `x,y,w,h` theo điểm ảnh (lặp lại được).
        #[arg(long)]
        rect: Vec<String>,
        /// PDF ra.
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Đọc QR/mã vạch trên ảnh, in JSON.
    Barcode {
        /// Ảnh.
        image: PathBuf,
    },
}

fn err<T, E: std::fmt::Display>(r: Result<T, E>) -> Result<T, String> {
    r.map_err(|e| e.to_string())
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("không đọc được {}: {e}", path.display()))
}

fn write(path: &Path, data: &[u8]) -> Result<(), String> {
    std::fs::write(path, data).map_err(|e| format!("không ghi được {}: {e}", path.display()))
}

fn password(var: Option<&String>) -> Result<String, String> {
    match var {
        None => Ok(String::new()),
        Some(v) => std::env::var(v).map_err(|_| format!("biến môi trường `{v}` chưa đặt")),
    }
}

fn load_pages(inputs: &[PathBuf], dpi: f32) -> Result<Vec<Page>, String> {
    let mut pages = Vec::new();
    for p in inputs {
        let bytes = read(p)?;
        let mut got = err(import::import(&bytes, &ImportOptions::default()))?;
        // Trang PDF đã mang DPI vẽ thật; ảnh thì dùng DPI người dùng khai.
        if import::sniff(&bytes) == import::InputKind::Image {
            for page in &mut got {
                page.dpi = dpi;
            }
        }
        pages.extend(got);
    }
    Ok(pages)
}

fn ocr_pages(pages: &mut [Page]) -> Result<(), String> {
    let store = err(tsuscan_ml::ModelStore::from_env())?;
    let engine = err(tsuscan_ml::OcrEngine::from_store(&store))?;
    for (i, p) in pages.iter_mut().enumerate() {
        let res = err(engine.recognize(&p.image.to_rgb8()))?;
        eprintln!("OCR trang {}: {} dòng", i + 1, res.lines.len());
        p.ocr = Some(res);
    }
    Ok(())
}

fn color_mode(code: &str) -> Result<ColorMode, String> {
    Ok(match code {
        "auto" => ColorMode::Auto,
        "color" => ColorMode::Color,
        "gray" => ColorMode::Gray,
        "bw" => ColorMode::Bilevel,
        _ => return Err("màu: auto | color | gray | bw".into()),
    })
}

fn image_format(code: &str) -> Result<ImageFormat, String> {
    Ok(match code {
        "jpg" | "jpeg" => ImageFormat::Jpeg,
        "png" => ImageFormat::Png,
        "webp" => ImageFormat::WebP,
        _ => return Err("định dạng ảnh: jpg | png | webp".into()),
    })
}

fn document(pages: Vec<Page>, title: String) -> Document {
    Document {
        pages,
        meta: Metadata {
            title,
            created: Some(Timestamp::now(420)),
            ..Metadata::default()
        },
    }
}

fn parse_rect(s: &str) -> Result<Rect, String> {
    let v: Vec<f64> = s
        .split(',')
        .map(|x| {
            x.trim()
                .parse::<f64>()
                .map_err(|_| format!("vùng `{s}` sai dạng x,y,w,h"))
        })
        .collect::<Result<_, _>>()?;
    match v.as_slice() {
        [x, y, w, h] => Ok(Rect::new(*x, *y, *w, *h)),
        _ => Err(format!("vùng `{s}` cần 4 số x,y,w,h")),
    }
}

/// Chạy một lệnh tài liệu.
///
/// # Errors
/// Thông báo lỗi tiếng Việt cho người dùng CLI.
#[allow(clippy::too_many_lines)] // Một nhánh match mỗi lệnh: tách hàm không làm rõ hơn.
pub fn run(cmd: DocCmd) -> Result<(), String> {
    match cmd {
        DocCmd::Pdf {
            inputs,
            output,
            ocr,
            pdfa,
            password_env,
            quality,
            color,
            title,
            dpi,
        } => {
            let mut pages = load_pages(&inputs, dpi)?;
            if ocr {
                ocr_pages(&mut pages)?;
            }
            let user = password(password_env.as_ref())?;
            let opts = PdfOptions {
                pdfa,
                protection: (!user.is_empty()).then(|| Protection {
                    user_password: user,
                    owner_password: String::new(),
                    permissions: Permissions::default(),
                }),
                color: color_mode(&color)?,
                quality: err(Quality::from_code(&quality))?,
                no_text_layer: false,
            };
            let n = pages.len();
            let bytes = err(write_pdf(&document(pages, title), &opts))?;
            write(&output, &bytes)?;
            println!(
                "Đã ghi {} ({n} trang, {} KB)",
                output.display(),
                bytes.len() / 1024
            );
        }
        DocCmd::Export {
            inputs,
            output,
            title,
        } => {
            let mut pages = load_pages(&inputs, 300.0)?;
            ocr_pages(&mut pages)?;
            let doc = document(pages, title);
            let ext = output
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            let bytes = match ext.as_str() {
                "docx" => err(docx::to_docx(&doc))?,
                "xlsx" => err(xlsx::to_xlsx(&doc))?,
                "txt" => text::to_txt(&doc).into_bytes(),
                "md" => text::to_markdown(&doc).into_bytes(),
                _ => return Err("đuôi tệp ra: .docx .xlsx .txt .md".into()),
            };
            write(&output, &bytes)?;
            println!("Đã ghi {}", output.display());
        }
        DocCmd::Import {
            input,
            out,
            dpi,
            password_env,
            format,
        } => {
            let fmt = image_format(&format)?;
            let opts = ImportOptions {
                dpi,
                password: password(password_env.as_ref())?,
                pages: Vec::new(),
            };
            let pages = err(import::import(&read(&input)?, &opts))?;
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            for (i, p) in pages.iter().enumerate() {
                let data = err(export_image(&p.image, fmt, ColorMode::Color, Quality::High))?;
                write(
                    &out.join(format!("trang-{:04}.{}", i + 1, fmt.extension())),
                    &data,
                )?;
            }
            println!("Đã nhập {} trang vào {}", pages.len(), out.display());
        }
        DocCmd::Redact {
            image,
            find,
            rect,
            output,
        } => {
            let mut pages = load_pages(std::slice::from_ref(&image), 300.0)?;
            ocr_pages(&mut pages)?;
            let mut page = pages.into_iter().next().ok_or("không có trang")?;
            let mut rects: Vec<Rect> = rect.iter().map(|s| parse_rect(s)).collect::<Result<_, _>>()?;
            for f in &find {
                let found = redact::find_text(&page, f);
                eprintln!("`{f}`: {} vị trí", found.len());
                rects.extend(found);
            }
            let rep = redact::redact(&mut page, &rects);
            eprintln!(
                "Đã che {} vùng; bỏ {} dòng, cắt {} dòng chữ OCR",
                rep.regions, rep.lines_removed, rep.lines_trimmed
            );
            let bytes = err(write_pdf(
                &document(vec![page], String::new()),
                &PdfOptions::default(),
            ))?;
            write(&output, &bytes)?;
            println!("Đã ghi {}", output.display());
        }
        DocCmd::Barcode { image } => {
            let img = err(tsuscan_imaging::decode(&read(&image)?))?;
            let codes: Vec<_> = barcode::scan(&img)
                .into_iter()
                .map(|b| json!({ "format": b.format, "text": b.text, "points": b.points }))
                .collect();
            println!("{}", json!(codes));
        }
    }
    Ok(())
}

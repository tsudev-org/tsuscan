//! Ghi bộ PDF/DOCX/XLSX mẫu ra thư mục để đối chiếu bằng công cụ ngoài (poppler, qpdf, veraPDF, LibreOffice).
//! Chạy: cargo run -p tsuscan-doc --example doc_probe -- <thư mục>

use image::{DynamicImage, GrayImage, Luma, Rgb, RgbImage};
use tsuscan_doc::export::{docx, xlsx};
use tsuscan_doc::{Document, Metadata, Page, PdfOptions, Permissions, Protection, Timestamp, write_pdf};
use tsuscan_imaging::{Point, Quad};
use tsuscan_ml::OcrPageResult;
use tsuscan_ml::ocr::OcrLine;

fn line(text: &str, x: f64, y: f64, row: usize) -> OcrLine {
    let w = text.chars().count() as f64 * 26.0;
    OcrLine {
        text: text.into(),
        quad: Quad {
            pts: [
                Point::new(x, y),
                Point::new(x + w, y),
                Point::new(x + w, y + 48.0),
                Point::new(x, y + 48.0),
            ],
        },
        confidence: 0.9,
        row,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::args().nth(1).ok_or("thiếu thư mục ra")?;
    let bw = GrayImage::from_fn(2480, 3508, |x, y| {
        Luma([
            if (300..1200).contains(&y) && (200..2200).contains(&x) && (x / 9 + y / 13) % 4 == 0 {
                0
            } else {
                255
            },
        ])
    });
    let ocr = OcrPageResult {
        lines: vec![
            line("CỘNG HÒA XÃ HỘI CHỦ NGHĨA VIỆT NAM", 790.0, 300.0, 0),
            line("Hợp đồng thuê nhà - Bên A: Nguyễn Văn Ấn", 200.0, 500.0, 1),
            line("Tên hàng", 200.0, 700.0, 2),
            line("SL", 1100.0, 700.0, 2),
            line("Giá", 1800.0, 700.0, 2),
            line("Gạo ST25", 200.0, 780.0, 3),
            line("2", 1100.0, 780.0, 3),
            line("1.250.000", 1800.0, 780.0, 3),
        ],
    };
    let color = RgbImage::from_fn(1240, 1754, |x, y| {
        if (x / 40 + y / 40) % 2 == 0 {
            Rgb([200, 40, 40])
        } else {
            Rgb([250, 248, 240])
        }
    });
    let doc = Document {
        pages: vec![
            Page::new(DynamicImage::ImageLuma8(bw)).with_ocr(ocr),
            Page::new(DynamicImage::ImageRgb8(color)).with_dpi(150.0),
        ],
        meta: Metadata {
            title: "Hợp đồng thuê nhà".into(),
            author: "Tsu Scan thử".into(),
            subject: "Mẫu đối chiếu".into(),
            keywords: vec!["hợp đồng".into(), "mẫu".into()],
            created: Some(Timestamp::now(420)),
        },
    };
    std::fs::write(
        format!("{dir}/plain.pdf"),
        write_pdf(&doc, &PdfOptions::default())?,
    )?;
    std::fs::write(
        format!("{dir}/pdfa.pdf"),
        write_pdf(
            &doc,
            &PdfOptions {
                pdfa: true,
                ..PdfOptions::default()
            },
        )?,
    )?;
    let protection = Protection {
        user_password: "mở-123".into(),
        owner_password: "chủ-456".into(),
        permissions: Permissions {
            copy: false,
            ..Permissions::default()
        },
    };
    std::fs::write(
        format!("{dir}/aes.pdf"),
        write_pdf(
            &doc,
            &PdfOptions {
                protection: Some(protection),
                ..PdfOptions::default()
            },
        )?,
    )?;
    std::fs::write(format!("{dir}/mau.docx"), docx::to_docx(&doc)?)?;
    std::fs::write(format!("{dir}/mau.xlsx"), xlsx::to_xlsx(&doc)?)?;
    println!("đã ghi vào {dir}");
    Ok(())
}

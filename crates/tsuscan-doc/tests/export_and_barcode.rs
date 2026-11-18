//! Xuất DOCX/XLSX/TXT/Markdown và đọc mã vạch, nhập ảnh/PDF.

// Hàm phụ của test (không gắn #[test]) cũng được phép unwrap: lỗi = test trượt.
#![allow(clippy::unwrap_used)]

use image::{DynamicImage, GrayImage, Luma, Rgb, RgbImage};
use rxing::{BarcodeFormat, MultiFormatWriter, Writer};
use tsuscan_doc::export::{docx, text, xlsx};
use tsuscan_doc::pdf::read::ImportOptions;
use tsuscan_doc::{Document, Metadata, Page, PdfOptions, barcode, import, write_pdf};
use tsuscan_imaging::{Point, Quad};
use tsuscan_ml::OcrPageResult;
use tsuscan_ml::ocr::OcrLine;

fn cell(text: &str, x: f64, y: f64, row: usize) -> OcrLine {
    let w = text.chars().count() as f64 * 20.0;
    OcrLine {
        text: text.into(),
        quad: Quad {
            pts: [
                Point::new(x, y),
                Point::new(x + w, y),
                Point::new(x + w, y + 30.0),
                Point::new(x, y + 30.0),
            ],
        },
        confidence: 0.9,
        row,
    }
}

fn invoice_page() -> Page {
    let lines = vec![
        cell("HÓA ĐƠN BÁN HÀNG", 840.0, 100.0, 0),
        cell(
            "Khách hàng mua hàng tại cửa hàng & thanh toán <tiền mặt>.",
            100.0,
            200.0,
            1,
        ),
        cell("Tên hàng", 100.0, 300.0, 2),
        cell("SL", 900.0, 300.0, 2),
        cell("Thành tiền", 1400.0, 300.0, 2),
        cell("Gạo ST25", 100.0, 350.0, 3),
        cell("2", 900.0, 350.0, 3),
        cell("1.250.000", 1400.0, 350.0, 3),
        cell("Mã KH", 100.0, 400.0, 4),
        cell("1", 900.0, 400.0, 4),
        cell("0912345678", 1400.0, 400.0, 4),
    ];
    let img = RgbImage::from_pixel(2000, 1000, Rgb([255, 255, 255]));
    Page::new(DynamicImage::ImageRgb8(img)).with_ocr(OcrPageResult { lines })
}

fn doc() -> Document {
    Document {
        pages: vec![invoice_page(), Page::new(DynamicImage::new_rgb8(300, 400))],
        meta: Metadata {
            title: "Hóa đơn tháng 11".into(),
            ..Metadata::default()
        },
    }
}

/// Đọc một mục trong ZIP do bộ ghi của crate tạo (Deflate hoặc lưu thẳng) - đủ cho kiểm nội dung.
fn zip_entry(zip: &[u8], name: &str) -> Option<Vec<u8>> {
    let mut i = 0;
    while i + 30 <= zip.len() && zip[i..i + 4] == *b"PK\x03\x04" {
        let method = u16::from_le_bytes([zip[i + 8], zip[i + 9]]);
        let clen = u32::from_le_bytes([zip[i + 18], zip[i + 19], zip[i + 20], zip[i + 21]]) as usize;
        let nlen = u16::from_le_bytes([zip[i + 26], zip[i + 27]]) as usize;
        let fname = std::str::from_utf8(&zip[i + 30..i + 30 + nlen]).ok()?;
        let body = &zip[i + 30 + nlen..i + 30 + nlen + clen];
        if fname == name {
            return Some(if method == 8 {
                miniz_oxide::inflate::decompress_to_vec(body).ok()?
            } else {
                body.to_vec()
            });
        }
        i += 30 + nlen + clen;
    }
    None
}

#[test]
fn docx_contains_heading_paragraph_table_and_image() {
    let z = docx::to_docx(&doc()).unwrap();
    let d = String::from_utf8(zip_entry(&z, "word/document.xml").unwrap()).unwrap();
    assert!(
        d.contains("<w:pStyle w:val=\"Title\"/></w:pPr><w:r><w:t xml:space=\"preserve\">Hóa đơn tháng 11")
    );
    assert!(d.contains("Heading1") && d.contains("HÓA ĐƠN BÁN HÀNG"));
    assert!(d.contains("&amp; thanh toán &lt;tiền mặt&gt;."));
    assert!(d.contains("<w:tbl>") && d.contains("1.250.000"));
    assert!(d.contains("<w:br w:type=\"page\"/>") && d.contains("r:embed=\"rIdImg1\""));
    assert!(
        zip_entry(&z, "word/media/trang1.jpg")
            .unwrap()
            .starts_with(&[0xFF, 0xD8])
    );
    for part in [
        "[Content_Types].xml",
        "_rels/.rels",
        "word/styles.xml",
        "word/_rels/document.xml.rels",
        "docProps/core.xml",
    ] {
        assert!(zip_entry(&z, part).is_some(), "thiếu {part}");
    }
}

#[test]
fn xlsx_has_table_sheet_with_numbers_and_text_ids() {
    let z = xlsx::to_xlsx(&doc()).unwrap();
    let wb = String::from_utf8(zip_entry(&z, "xl/workbook.xml").unwrap()).unwrap();
    assert!(wb.contains("name=\"Bảng 1\"") && wb.contains("name=\"Văn bản\""));
    let s1 = String::from_utf8(zip_entry(&z, "xl/worksheets/sheet1.xml").unwrap()).unwrap();
    assert!(s1.contains("<v>1250000</v>"), "{s1}");
    assert!(
        s1.contains("<t xml:space=\"preserve\">0912345678</t>"),
        "số điện thoại phải giữ số 0: {s1}"
    );
    assert!(s1.contains("state=\"frozen\""));
    let s2 = String::from_utf8(zip_entry(&z, "xl/worksheets/sheet2.xml").unwrap()).unwrap();
    assert!(s2.contains("HÓA ĐƠN BÁN HÀNG"));
}

#[test]
fn txt_and_markdown() {
    let d = doc();
    let t = text::to_txt(&d);
    assert!(t.contains("HÓA ĐƠN BÁN HÀNG") && t.contains(text::PAGE_BREAK));
    let m = text::to_markdown(&d);
    assert!(m.starts_with("# Hóa đơn tháng 11\n"));
    assert!(m.contains("| Tên hàng | SL | Thành tiền |"), "{m}");
    assert!(m.contains("\n---\n"));
}

fn render(matrix: &rxing::common::BitMatrix, module: u32) -> DynamicImage {
    let (w, h) = (matrix.getWidth(), matrix.getHeight());
    let pad = 4 * module;
    let img = GrayImage::from_fn(w * module + 2 * pad, h * module + 2 * pad, |x, y| {
        let inside = x >= pad && y >= pad && x < pad + w * module && y < pad + h * module;
        let black = inside && matrix.get((x - pad) / module, (y - pad) / module);
        Luma([if black { 0 } else { 255 }])
    });
    DynamicImage::ImageLuma8(img)
}

#[test]
fn reads_qr_and_code128_on_a_page() {
    let vietqr =
        "00020101021238570010A00000072701270006970436011300123456789010208QRIBFTTA53037045802VN6304ABCD";
    let qr = MultiFormatWriter
        .encode(vietqr, &BarcodeFormat::QR_CODE, 0, 0)
        .unwrap();
    let bar = MultiFormatWriter
        .encode("HD-2026-000123", &BarcodeFormat::CODE_128, 0, 0)
        .unwrap();
    let qr_img = render(&qr, 6).to_luma8();
    let bar_img = render(&bar, 3).to_luma8();
    let mut page = GrayImage::from_pixel(1600, 1200, Luma([255]));
    image::imageops::overlay(&mut page, &qr_img, 100, 100);
    // Mã 1D kéo cao để giống mã thật.
    let tall = image::imageops::resize(
        &bar_img,
        bar_img.width(),
        120,
        image::imageops::FilterType::Nearest,
    );
    image::imageops::overlay(&mut page, &tall, 700, 800);
    let found = barcode::scan(&DynamicImage::ImageLuma8(page));
    let texts: Vec<&str> = found.iter().map(|b| b.text.as_str()).collect();
    assert!(texts.contains(&vietqr), "{found:?}");
    assert!(texts.contains(&"HD-2026-000123"), "{found:?}");
    let q = found.iter().find(|b| b.text == vietqr).unwrap();
    assert!(q.format.to_lowercase().contains("qr"));
    assert!(
        q.points
            .iter()
            .all(|(x, y)| *x >= 90.0 && *y >= 90.0 && *x < 500.0 && *y < 500.0)
    );
    assert_eq!(barcode::scan(&DynamicImage::new_rgb8(0, 0)), Vec::new());
    assert_eq!(barcode::scan(&DynamicImage::new_rgb8(64, 64)), Vec::new());
}

#[test]
fn import_images_and_own_pdf() {
    let mut png = Vec::new();
    DynamicImage::ImageRgb8(RgbImage::from_pixel(40, 30, Rgb([1, 2, 3])))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    let pages = import::import(&png, &ImportOptions::default()).unwrap();
    assert_eq!(pages[0].image.width(), 40);
    let pdf = write_pdf(&doc(), &PdfOptions::default()).unwrap();
    assert_eq!(import::sniff(&pdf), import::InputKind::Pdf);
    let pages = import::import(
        &pdf,
        &ImportOptions {
            dpi: 100.0,
            ..ImportOptions::default()
        },
    )
    .unwrap();
    assert_eq!(pages.len(), 2);
    // Trang 1: 2000 px ở 300 DPI = 6,67 inch ⇒ 667 px ở 100 DPI.
    assert!(
        (i64::from(pages[0].image.width()) - 667).abs() <= 1,
        "{}",
        pages[0].image.width()
    );
}

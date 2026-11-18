//! Vòng ghi → đọc PDF, đối chiếu bằng bộ đọc độc lập:
//! - `hayro` (thuần Rust, có giải mã AES-256) - luôn chạy;
//! - poppler (`pdftotext`, `pdfinfo`) và `qpdf --check` - chạy khi có; CI đặt `TSUSCAN_REQUIRE_PDF_TOOLS=1` để bắt buộc.

// Hàm phụ của test (không gắn #[test]) cũng được phép unwrap: lỗi = test trượt.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;
use std::process::Command;

use image::{DynamicImage, GrayImage, Luma, Rgb, RgbImage};
use tsuscan_doc::pdf::read::{ImportOptions, import_pdf, page_count};
use tsuscan_doc::redact::{self, Rect};
use tsuscan_doc::{
    ColorMode, DocError, Document, Metadata, Page, PdfOptions, Permissions, Protection, Quality, Timestamp,
    write_pdf,
};
use tsuscan_imaging::{Point, Quad};
use tsuscan_ml::OcrPageResult;
use tsuscan_ml::ocr::OcrLine;

const LINES: [&str; 3] = [
    "CỘNG HÒA XÃ HỘI CHỦ NGHĨA VIỆT NAM",
    "Hợp đồng thuê nhà số 12/2026",
    "Số CCCD: 001234567890",
];

fn line(text: &str, y: f64) -> OcrLine {
    let (x0, x1) = (150.0, 150.0 + text.chars().count() as f64 * 28.0);
    OcrLine {
        text: text.into(),
        quad: Quad {
            pts: [
                Point::new(x0, y),
                Point::new(x1, y),
                Point::new(x1, y + 50.0),
                Point::new(x0, y + 50.0),
            ],
        },
        confidence: 0.95,
        row: 0,
    }
}

fn ocr() -> OcrPageResult {
    let mut lines: Vec<OcrLine> = LINES
        .iter()
        .enumerate()
        .map(|(i, t)| line(t, 200.0 + 120.0 * i as f64))
        .collect();
    for (i, l) in lines.iter_mut().enumerate() {
        l.row = i;
    }
    OcrPageResult { lines }
}

/// Trang A4 300 DPI đen trắng (sọc chữ giả) - đi đường CCITT G4.
fn bw_page() -> Page {
    let img = GrayImage::from_fn(2480, 3508, |x, y| {
        let ink = (200..900).contains(&y) && (150..1400).contains(&x) && (x / 9 + y / 13) % 4 == 0;
        Luma([if ink { 0 } else { 255 }])
    });
    Page::new(DynamicImage::ImageLuma8(img)).with_ocr(ocr())
}

fn color_page() -> Page {
    let img = RgbImage::from_fn(1240, 1754, |x, y| {
        if (x / 40 + y / 40) % 2 == 0 {
            Rgb([200, 40, 40])
        } else {
            Rgb([250, 248, 240])
        }
    });
    Page::new(DynamicImage::ImageRgb8(img)).with_dpi(150.0)
}

fn doc(pages: Vec<Page>) -> Document {
    Document {
        pages,
        meta: Metadata {
            title: "Hợp đồng thuê nhà".into(),
            author: String::new(),
            subject: "Thử nghiệm".into(),
            keywords: vec!["hợp đồng".into()],
            created: Some(Timestamp {
                unix: 1_795_455_000,
                offset_minutes: 420,
            }),
        },
    }
}

fn tool(name: &str) -> bool {
    let ok = Command::new(name).arg("--version").output().is_ok();
    if !ok {
        assert!(
            std::env::var("TSUSCAN_REQUIRE_PDF_TOOLS").as_deref() != Ok("1"),
            "thiếu công cụ `{name}` mà TSUSCAN_REQUIRE_PDF_TOOLS=1"
        );
        eprintln!("bỏ qua kiểm bằng `{name}` (chưa cài)");
    }
    ok
}

fn tmp(name: &str, data: &[u8]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tsuscan-doc-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join(name);
    std::fs::write(&p, data).unwrap();
    p
}

fn pdftotext(path: &PathBuf, password: Option<&str>) -> String {
    let mut c = Command::new("pdftotext");
    if let Some(pw) = password {
        c.args(["-upw", pw]);
    }
    let out = c.arg(path).arg("-").output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).unwrap()
}

fn qpdf_check(path: &PathBuf, password: Option<&str>) {
    if !tool("qpdf") {
        return;
    }
    let mut c = Command::new("qpdf");
    if let Some(pw) = password {
        c.arg(format!("--password={pw}"));
    }
    let out = c.arg("--check").arg(path).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "qpdf --check: {text}");
    assert!(text.contains("No syntax or stream encoding errors"), "{text}");
}

fn norm(s: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    s.nfc().filter(|c| !c.is_whitespace()).collect()
}

#[test]
fn bilevel_g4_roundtrip_is_lossless_and_text_is_searchable() {
    let page = bw_page();
    let original = page.image.to_luma8();
    let bytes = write_pdf(&doc(vec![page]), &PdfOptions::default()).unwrap();
    assert!(bytes.windows(15).any(|w| w == b"/CCITTFaxDecode"));
    assert!(bytes.len() < 120_000, "G4 A4 phải nhỏ: {} B", bytes.len());

    // Vẽ lại bằng hayro ở đúng 300 DPI ⇒ từng điểm ảnh khớp (G4 không mất dữ liệu).
    let pages = import_pdf(
        &bytes,
        &ImportOptions {
            dpi: 300.0,
            ..ImportOptions::default()
        },
    )
    .unwrap();
    assert_eq!(pages.len(), 1);
    let back = pages[0].image.to_luma8();
    assert_eq!(back.dimensions(), original.dimensions());
    let diff = back
        .pixels()
        .zip(original.pixels())
        .filter(|(a, b)| (i16::from(a.0[0]) - i16::from(b.0[0])).abs() > 64)
        .count();
    assert!(
        diff * 1000 < (back.width() * back.height()) as usize,
        "lệch {diff} điểm"
    );

    let path = tmp("bw.pdf", &bytes);
    qpdf_check(&path, None);
    if tool("pdftotext") {
        let text = norm(&pdftotext(&path, None));
        for l in LINES {
            assert!(text.contains(&norm(l)), "thiếu `{l}` trong lớp chữ: {text}");
        }
    }
}

#[test]
fn pdfa_has_required_structures() {
    let bytes = write_pdf(
        &doc(vec![bw_page(), color_page()]),
        &PdfOptions {
            pdfa: true,
            ..PdfOptions::default()
        },
    )
    .unwrap();
    let s = String::from_utf8_lossy(&bytes);
    assert!(
        s.contains("<pdfaid:part>2</pdfaid:part>")
            && s.contains("<pdfaid:conformance>B</pdfaid:conformance>")
    );
    assert!(s.contains("/OutputIntents") && s.contains("/GTS_PDFA1") && s.contains("/DestOutputProfile"));
    assert!(s.contains("/ID [") && !s.contains("/Encrypt"));
    assert!(s.contains("/CIDToGIDMap") && s.contains("/FontFile2"));
    assert_eq!(page_count(&bytes, "").unwrap(), 2);
    let path = tmp("pdfa.pdf", &bytes);
    qpdf_check(&path, None);
    if tool("pdfinfo") {
        let out = Command::new("pdfinfo").arg(&path).output().unwrap();
        let info = String::from_utf8_lossy(&out.stdout);
        assert!(info.contains("Hợp đồng thuê nhà"), "{info}");
        assert!(info.contains("Pages:           2"), "{info}");
        assert!(info.contains("Encrypted:       no"), "{info}");
    }
    // PDF/A cùng mật khẩu: từ chối rõ ràng.
    let both = PdfOptions {
        pdfa: true,
        protection: Some(Protection {
            user_password: "a".into(),
            owner_password: String::new(),
            permissions: Permissions::default(),
        }),
        ..PdfOptions::default()
    };
    assert!(matches!(
        write_pdf(&doc(vec![color_page()]), &both),
        Err(DocError::InvalidOption(_))
    ));
}

#[test]
fn aes256_password_is_required_and_independent_readers_decrypt() {
    let pw = "Mật-khẩu 2026";
    let opts = PdfOptions {
        protection: Some(Protection {
            user_password: pw.into(),
            owner_password: "chủ sở hữu".into(),
            permissions: Permissions::default(),
        }),
        quality: Quality::Small,
        ..PdfOptions::default()
    };
    let bytes = write_pdf(&doc(vec![bw_page(), color_page()]), &opts).unwrap();
    let s = String::from_utf8_lossy(&bytes);
    assert!(s.contains("/Encrypt") && s.contains("/AESV3") && s.contains("/R 6"));
    // /ID ngẫu nhiên: hai lần ghi cùng nội dung cho ID khác nhau.
    let again = write_pdf(&doc(vec![bw_page(), color_page()]), &opts).unwrap();
    let id = |b: &[u8]| {
        let t = String::from_utf8_lossy(b).to_string();
        t[t.rfind("/ID [").unwrap()..].lines().next().unwrap().to_owned()
    };
    assert_ne!(id(&bytes), id(&again));
    // Tiêu đề không còn ở dạng rõ trong tệp.
    let title_utf16: Vec<u8> = "Hợp đồng".encode_utf16().flat_map(u16::to_be_bytes).collect();
    assert!(
        !bytes
            .windows(title_utf16.len())
            .any(|w| w == title_utf16.as_slice())
    );

    assert!(matches!(page_count(&bytes, ""), Err(DocError::PasswordRequired)));
    assert!(matches!(
        page_count(&bytes, "sai"),
        Err(DocError::PasswordRequired)
    ));
    assert_eq!(page_count(&bytes, pw).unwrap(), 2);
    assert_eq!(page_count(&bytes, "chủ sở hữu").unwrap(), 2);
    let pages = import_pdf(
        &bytes,
        &ImportOptions {
            dpi: 72.0,
            password: pw.into(),
            pages: vec![1],
        },
    )
    .unwrap();
    // Trang màu vẽ lại đúng màu (JPEG có sai số nhỏ).
    let px = pages[0].image.to_rgb8();
    let p = px.get_pixel(5, 5).0;
    assert!(p[0] > 150 && p[1] < 90, "{p:?}");

    let path = tmp("aes.pdf", &bytes);
    qpdf_check(&path, Some(pw));
    if tool("pdftotext") {
        let text = norm(&pdftotext(&path, Some(pw)));
        assert!(text.contains(&norm(LINES[1])), "{text}");
        let out = Command::new("pdftotext").arg(&path).arg("-").output().unwrap();
        assert!(!out.status.success(), "mở được khi không có mật khẩu");
    }
}

#[test]
fn redaction_removes_text_from_output() {
    let mut page = bw_page();
    let rects = redact::find_text(&page, "001234567890");
    assert_eq!(rects.len(), 1);
    redact::redact(&mut page, &rects);
    let bytes = write_pdf(&doc(vec![page]), &PdfOptions::default()).unwrap();
    let path = tmp("redact.pdf", &bytes);
    if tool("pdftotext") {
        let text = pdftotext(&path, None);
        assert!(!text.contains("1234"), "{text}");
        assert!(norm(&text).contains(&norm("Hợp đồng thuê nhà")));
    }
    // Điểm ảnh vùng che là đen đặc sau khi vẽ lại.
    let back = import_pdf(
        &bytes,
        &ImportOptions {
            dpi: 300.0,
            ..ImportOptions::default()
        },
    )
    .unwrap();
    let r: Rect = rects[0];
    let g = back[0].image.to_luma8();
    let mut dark = 0;
    let mut total = 0;
    for y in (r.y as u32 + 5)..(r.y + r.h) as u32 - 5 {
        for x in (r.x as u32 + 5)..(r.x + r.w) as u32 - 5 {
            total += 1;
            dark += u32::from(g.get_pixel(x, y).0[0] < 64);
        }
    }
    assert_eq!(dark, total);
}

#[test]
fn options_and_limits() {
    assert!(matches!(
        write_pdf(&Document::default(), &PdfOptions::default()),
        Err(DocError::Empty)
    ));
    let no_text = write_pdf(
        &doc(vec![bw_page()]),
        &PdfOptions {
            no_text_layer: true,
            color: ColorMode::Gray,
            quality: Quality::High,
            ..PdfOptions::default()
        },
    )
    .unwrap();
    let s = String::from_utf8_lossy(&no_text);
    assert!(!s.contains("/FontFile2") && s.contains("/DCTDecode"));
    // Rác / PDF cụt / quá lớn số trang yêu cầu: trả lỗi, không panic.
    assert!(matches!(
        page_count(b"%PDF-1.7\ngarbage", ""),
        Err(DocError::InvalidPdf)
    ));
    // PDF cụt: bộ đọc có thể khôi phục hoặc báo lỗi - chỉ yêu cầu không panic.
    let _ = page_count(&no_text[..no_text.len() / 2], "");
    assert!(matches!(
        import_pdf(
            &no_text,
            &ImportOptions {
                pages: vec![5],
                ..ImportOptions::default()
            }
        ),
        Err(DocError::InvalidArgument(_))
    ));
}

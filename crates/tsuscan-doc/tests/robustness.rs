//! Bộ đọc PDF với dữ liệu xấu: đột biến tất định (chạy trên stable, bổ sung cho libFuzzer `fuzz/pdf_import`) và
//! các tệp "bom" dựng tay. Yêu cầu: chỉ trả `Err`/`Ok`, không panic thoát ra ngoài, không ngốn bộ nhớ vô hạn.

// Hàm phụ của test (không gắn #[test]) cũng được phép unwrap: lỗi = test trượt.
#![allow(clippy::unwrap_used)]

use image::{DynamicImage, GrayImage, Luma};
use tsuscan_doc::pdf::read::{ImportOptions, import_pdf, page_count};
use tsuscan_doc::{Document, Page, PdfOptions, Permissions, Protection, write_pdf};
use tsuscan_imaging::synth::Rng;

fn seed_pdfs() -> Vec<(Vec<u8>, &'static str)> {
    let img = GrayImage::from_fn(300, 200, |x, y| {
        Luma([if (x / 5 + y / 7) % 3 == 0 { 0 } else { 255 }])
    });
    let doc = Document::new(vec![Page::new(DynamicImage::ImageLuma8(img)).with_dpi(72.0)]);
    let plain = write_pdf(&doc, &PdfOptions::default()).unwrap();
    let enc = write_pdf(
        &doc,
        &PdfOptions {
            protection: Some(Protection {
                user_password: "pw".into(),
                owner_password: String::new(),
                permissions: Permissions::default(),
            }),
            ..PdfOptions::default()
        },
    )
    .unwrap();
    vec![(plain, ""), (enc, "pw")]
}

const TOKENS: [&[u8]; 12] = [
    b"<<",
    b">>",
    b"/Length 999999999",
    b" 0 R",
    b"/Width 65535",
    b"stream\n",
    b"endobj",
    b"[[[[[[[[[[[[[[[[",
    b"/Kids [1 0 R]",
    b"(",
    b"/MediaBox [0 0 1e30 1e30]",
    b"/Filter /FlateDecode",
];

fn mutate(rng: &mut Rng, data: &[u8]) -> Vec<u8> {
    let mut d = data.to_vec();
    for _ in 0..=rng.index(4) {
        if d.is_empty() {
            break;
        }
        let at = rng.index(d.len());
        match rng.index(5) {
            0 => d[at] ^= 1 << rng.index(8),
            1 => d[at] = rng.next_u64() as u8,
            2 => d.truncate(at),
            3 => {
                let t = TOKENS[rng.index(TOKENS.len())];
                d.splice(at..at, t.iter().copied());
            }
            _ => {
                let end = (at + 1 + rng.index(64)).min(d.len());
                d.drain(at..end);
            }
        }
    }
    d
}

#[test]
fn mutated_pdfs_never_panic() {
    let seeds = seed_pdfs();
    let mut rng = Rng::new(0x05EE_DD0C);
    let mut ok = 0;
    let n = 300;
    for i in 0..n {
        let (seed, pw) = &seeds[i % seeds.len()];
        let data = mutate(&mut rng, seed);
        let _ = page_count(&data, pw);
        let opts = ImportOptions {
            dpi: 36.0,
            password: (*pw).into(),
            pages: vec![0],
        };
        if import_pdf(&data, &opts).is_ok() {
            ok += 1;
        }
    }
    eprintln!("{ok}/{n} tệp đột biến vẫn mở được");
}

/// PDF tối thiểu một trang vẽ một ảnh với từ điển ảnh tùy ý (dựng tay, offset xref đúng).
fn pdf_with_image(image_dict: &str, stream: &[u8]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offs = Vec::new();
    let content = b"q 100 0 0 100 0 0 cm /Im0 Do Q";
    let objs: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R /Resources << /XObject << /Im0 5 0 R >> >> >>"
            .to_vec(),
        [format!("<< /Length {} >>\nstream\n", content.len()).as_bytes(), content, b"\nendstream"].concat(),
        [
            format!("<< /Type /XObject /Subtype /Image {image_dict} /Length {} >>\nstream\n", stream.len()).as_bytes(),
            stream,
            b"\nendstream",
        ]
        .concat(),
    ];
    for (i, o) in objs.iter().enumerate() {
        offs.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(o);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for o in offs {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objs.len() + 1
        )
        .as_bytes(),
    );
    out
}

#[test]
fn hand_made_pdf_renders() {
    let pdf = pdf_with_image(
        "/Width 2 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8",
        &[0, 255],
    );
    let pages = import_pdf(
        &pdf,
        &ImportOptions {
            dpi: 72.0,
            ..ImportOptions::default()
        },
    )
    .unwrap();
    let g = pages[0].image.to_luma8();
    assert!(g.get_pixel(10, 50).0[0] < 50 && g.get_pixel(90, 50).0[0] > 200);
}

/// Nén Deflate (zlib hoặc thô) `n` MiB số 0 theo luồng (không giữ cả khối trong bộ nhớ).
fn deflate_zeros(mib: usize, format: miniz_oxide::DataFormat) -> Vec<u8> {
    use miniz_oxide::deflate::core::CompressorOxide;
    use miniz_oxide::deflate::stream::deflate;
    use miniz_oxide::{MZFlush, MZStatus};
    let mut c = Box::<CompressorOxide>::default();
    c.set_format_and_level(format, 9);
    let chunk = vec![0u8; 1 << 20];
    let mut buf = vec![0u8; 1 << 16];
    let mut out = Vec::new();
    for i in 0..mib {
        let flush = if i + 1 == mib {
            MZFlush::Finish
        } else {
            MZFlush::None
        };
        let mut input: &[u8] = &chunk;
        loop {
            let r = deflate(&mut c, input, &mut buf, flush);
            out.extend_from_slice(&buf[..r.bytes_written]);
            input = &input[r.bytes_consumed..];
            let ended = r.status == Ok(MZStatus::StreamEnd);
            if (input.is_empty() && flush == MZFlush::None && r.bytes_written < buf.len()) || ended {
                break;
            }
        }
    }
    out
}

/// Bộ nhớ thường trú đỉnh của tiến trình (KiB), chỉ Linux.
fn peak_rss_kib() -> Option<u64> {
    let s = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = s.lines().find(|l| l.starts_with("VmHWM:"))?;
    line.split_whitespace().nth(1)?.parse().ok()
}

#[test]
fn image_and_flate_bombs_are_bounded() {
    use miniz_oxide::DataFormat;
    // Ảnh khai báo 6.000 × 6.000 RGB, luồng Flate nở 1 GiB số 0 (tệp ≈ 1 MB). Hai biến thể:
    // - zlib: lớp quét thô chặn trước khi mở;
    // - deflate thô (không đầu zlib, hayro vẫn giải được): né lớp 1, lớp duyệt đối tượng phải chặn.
    for (format, name) in [(DataFormat::Zlib, "zlib"), (DataFormat::Raw, "deflate thô")] {
        let bomb = deflate_zeros(1024, format);
        assert!(bomb.len() < 2_000_000, "{}", bomb.len());
        let pdf = pdf_with_image(
            "/Width 6000 /Height 6000 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode",
            &bomb,
        );
        let before = peak_rss_kib();
        let t = std::time::Instant::now();
        let r = import_pdf(
            &pdf,
            &ImportOptions {
                dpi: 72.0,
                ..ImportOptions::default()
            },
        );
        let after = peak_rss_kib();
        eprintln!(
            "bom {name}: {:?} sau {:?}; RSS đỉnh {:?} → {:?} KiB",
            r.as_ref().map(Vec::len).map_err(ToString::to_string),
            t.elapsed(),
            before,
            after
        );
        // Bị từ chối TRƯỚC khi hayro giải nén (RSS đỉnh tăng < 200 MiB).
        assert!(
            matches!(r, Err(tsuscan_doc::DocError::LimitExceeded(_))),
            "{name}"
        );
        if let (Some(b), Some(a)) = (before, after) {
            assert!(a < b + 200_000, "{name}: RSS đỉnh tăng {} KiB", a - b);
        }
    }
    // Ảnh khai báo khổng lồ (10,8 GB) dù dữ liệu nhỏ: chặn theo kích thước khai báo.
    let pdf = pdf_with_image(
        "/Width 60000 /Height 60000 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode",
        &[0xFF, 0xD8, 0xFF, 0xD9],
    );
    assert!(matches!(
        import_pdf(&pdf, &ImportOptions::default()),
        Err(tsuscan_doc::DocError::LimitExceeded(_))
    ));
}

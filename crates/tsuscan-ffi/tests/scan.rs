//! Luồng quét qua đúng bề mặt API mà Dart gọi: khung xem trước YUV → góc; ảnh chụp → dò, nắn, lọc, lưu kho.
#![allow(clippy::expect_used)]

use std::io::Cursor;
use std::path::PathBuf;

use image::{DynamicImage, ImageFormat};
use tsuscan_ffi::api::error::{ApiErrorKind, ApiResult};
use tsuscan_ffi::api::library::{LibraryHandle, generate_db_key};
use tsuscan_ffi::api::scan::{CapturedImage, ScanQuad, detect_preview, preview_row_step, scan_filters};
use tsuscan_imaging::synth::{SynthConfig, generate};

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let p = std::env::temp_dir().join(format!("tsuscan-ffi-scan-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        Self(p)
    }
    fn path(&self) -> String {
        self.0.to_string_lossy().into_owned()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const CLEAN: SynthConfig = SynthConfig {
    scale: 1.0,
    effects: false,
};

/// Góc thật chuẩn hóa (TL, TR, BR, BL).
fn truth(seed: u64) -> (image::RgbImage, Vec<f64>) {
    let s = generate(seed, &CLEAN).expect("mẫu tổng hợp");
    let (w, h) = (f64::from(s.image.width()), f64::from(s.image.height()));
    let c = s.truth.pts.iter().flat_map(|p| [p.x / w, p.y / h]).collect();
    (s.image, c)
}

fn max_err(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0.0, f64::max)
}

fn jpeg_with_exif_marker(img: &image::RgbImage) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    assert!(
        DynamicImage::ImageRgb8(img.clone())
            .write_to(&mut out, ImageFormat::Jpeg)
            .is_ok()
    );
    let jpeg = out.into_inner();
    // Chèn APP1 "Exif" giả (chứa chuỗi GPS) ngay sau SOI để kiểm bị bỏ khi lưu.
    let payload = b"Exif\0\0GPS-21.0285N-105.8542E";
    let len = (payload.len() + 2) as u16;
    let mut with = vec![0xFF, 0xD8, 0xFF, 0xE1];
    with.extend_from_slice(&len.to_be_bytes());
    with.extend_from_slice(payload);
    with.extend_from_slice(&jpeg[2..]);
    with
}

#[test]
fn khung_xem_truoc_co_stride_va_xoay_90() -> ApiResult<()> {
    let mut hits = 0;
    for seed in 0..10u64 {
        let (img, want) = truth(seed);
        let gray = DynamicImage::ImageRgb8(img).to_luma8();
        let (w, h) = gray.dimensions();
        // Hàng có 32 byte đệm như ImageProxy của CameraX; hàng cuối không đệm.
        let stride = w + 32;
        let mut buf = vec![0u8; (stride * (h - 1) + w) as usize];
        for (y, row) in gray.rows().enumerate() {
            for (x, px) in row.enumerate() {
                buf[y * stride as usize + x] = px.0[0];
            }
        }
        let Some(q) = detect_preview(buf, w, h, stride, 1, 90)? else {
            continue;
        };
        // Góc thật xoay 90° thuận chiều kim đồng hồ: (x, y) → (1 - y, x), rồi TL của ảnh xoay là BL cũ.
        let rot: Vec<(f64, f64)> = want.chunks(2).map(|p| (1.0 - p[1], p[0])).collect();
        let want_rot: Vec<f64> = [3, 0, 1, 2].iter().flat_map(|&i| [rot[i].0, rot[i].1]).collect();
        assert_eq!(q.corners.len(), 8);
        if max_err(&q.corners, &want_rot) < 0.03 {
            hits += 1;
        }
    }
    assert!(hits >= 9, "chỉ {hits}/10 khung khớp góc thật");
    Ok(())
}

#[test]
fn buoc_hang_giu_du_hang_cho_anh_lam_viec() {
    // Khung phân tích CameraX 1440x1080 (đo trên Galaxy M32 5G) ⇒ ảnh làm việc 320x240, mỗi 4 hàng lấy 1.
    assert_eq!(preview_row_step(1440, 1080), 4);
    assert_eq!(preview_row_step(1080, 1440), 4);
    assert_eq!(preview_row_step(1920, 1080), 6);
    assert_eq!(preview_row_step(320, 240), 1);
    assert_eq!(preview_row_step(16, 4096), 12);
}

#[test]
fn khung_xem_truoc_gom_hang_van_khop_goc() -> ApiResult<()> {
    let mut hits = 0;
    for seed in 0..10u64 {
        let (img, want) = truth(seed);
        let gray = DynamicImage::ImageRgb8(img).to_luma8();
        let (w, h) = gray.dimensions();
        let step = preview_row_step(w, h);
        assert!(step > 1, "{w}x{h}");
        // Như Dart: chỉ chép hàng 0, step, 2·step... (có đệm cuối hàng, hàng cuối không đệm).
        let stride = w + 8;
        let rows = h.div_ceil(step);
        let mut buf = vec![0u8; (stride * (rows - 1) + w) as usize];
        for r in 0..rows {
            for x in 0..w {
                buf[(r * stride + x) as usize] = gray.get_pixel(x, r * step).0[0];
            }
        }
        let Some(q) = detect_preview(buf, w, h, stride, step, 0)? else {
            continue;
        };
        if max_err(&q.corners, &want) < 0.03 {
            hits += 1;
        }
    }
    assert!(hits >= 9, "chỉ {hits}/10 khung khớp góc thật");
    Ok(())
}

#[test]
fn khung_xem_truoc_sai_kich_thuoc_bi_tu_choi() {
    for step in [0, 17] {
        let e = detect_preview(vec![0; 640 * 480], 640, 480, 640, step, 0);
        assert!(
            matches!(e, Err(e) if e.kind == ApiErrorKind::InvalidInput),
            "bước {step}"
        );
    }
    // Khai bước 4 nhưng gửi thiếu hàng.
    let e = detect_preview(vec![0; 640 * 100], 640, 480, 640, 4, 0);
    assert!(matches!(e, Err(e) if e.kind == ApiErrorKind::InvalidInput));
    let e = detect_preview(vec![0; 100], 640, 480, 640, 1, 0);
    assert!(matches!(e, Err(e) if e.kind == ApiErrorKind::InvalidInput));
    let e = detect_preview(vec![0; 640 * 480], 640, 480, 640, 1, 45);
    assert!(matches!(e, Err(e) if e.kind == ApiErrorKind::InvalidInput));
    // Khung trơn: không có tài liệu, không lỗi.
    assert!(matches!(
        detect_preview(vec![128; 640 * 480], 640, 480, 640, 1, 0),
        Ok(None)
    ));
}

#[test]
fn anh_chup_do_nan_loc_va_luu_kho_bo_exif() -> ApiResult<()> {
    let (img, want) = truth(3);
    let cap = CapturedImage::decode(jpeg_with_exif_marker(&img))?;
    assert_eq!((cap.width(), cap.height()), img.dimensions());

    let ScanQuad { corners, confidence } = cap.detect()?.expect("phải dò được tài liệu");
    assert!(confidence > 0.35);
    assert!(
        max_err(&corners, &want) < 0.02,
        "lệch {}",
        max_err(&corners, &want)
    );

    for f in scan_filters() {
        let jpeg = cap.render(corners.clone(), f.clone(), 512)?;
        let out = image::load_from_memory(&jpeg).expect("JPEG xem trước");
        assert!(out.width().max(out.height()) <= 512, "{f}");
        if f == "bw" || f == "gray" {
            assert!(
                matches!(out, DynamicImage::ImageLuma8(_)),
                "{f} phải là JPEG xám một kênh"
            );
        }
    }
    let bad = cap.render(corners.clone(), "sepia".into(), 512);
    assert!(matches!(bad, Err(e) if e.kind == ApiErrorKind::InvalidInput));
    let bad = cap.render(vec![0.0, 0.0, 0.5, 0.0, 1.0, 0.0, 0.2, 0.0], "auto".into(), 512);
    assert!(matches!(bad, Err(e) if e.kind == ApiErrorKind::InvalidInput));

    let dir = TempDir::new("save");
    let lib = LibraryHandle::create(dir.path(), generate_db_key()?)?;
    let doc = lib.create_document("Quét thử".into(), None)?;
    let page = lib.add_scanned_page(doc.clone(), &cap, corners, "auto".into())?;
    let whole = lib.add_scanned_page(doc.clone(), &cap, vec![], "original".into())?;
    let pages = lib.list_pages(doc)?;
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[1].id, whole);
    assert_eq!((pages[1].width, pages[1].height), img.dimensions());
    let p = &pages[0];
    assert_eq!(p.id, page);
    // Trang đã nắn: tỉ lệ khớp khổ giấy thật (bộ sinh dùng A4/Letter/ID-1), không còn là khung camera.
    let ratio = f64::from(p.width.max(p.height)) / f64::from(p.width.min(p.height));
    assert!((1.25..1.6).contains(&ratio), "tỉ lệ {ratio}");

    let stored = lib.page_image(page)?;
    assert_eq!(image::guess_format(&stored).ok(), Some(ImageFormat::Jpeg));
    assert!(
        !stored.windows(4).any(|w| w == b"Exif"),
        "siêu dữ liệu gốc phải bị bỏ"
    );
    assert!(!stored.windows(3).any(|w| w == b"GPS"));
    Ok(())
}

#[test]
fn anh_chup_hong_bi_tu_choi() {
    let e = CapturedImage::decode(b"khong phai anh".to_vec());
    assert!(matches!(e, Err(e) if e.kind == ApiErrorKind::UnsupportedImage));
}

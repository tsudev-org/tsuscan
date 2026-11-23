//! Luồng thư viện qua đúng bề mặt API mà Dart gọi.

use std::io::Cursor;
use std::path::PathBuf;

use tsuscan_ffi::api::error::{ApiErrorKind, ApiResult};
use tsuscan_ffi::api::library::{LibraryHandle, generate_db_key, library_exists};

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let p = std::env::temp_dir().join(format!("tsuscan-ffi-{name}-{}", std::process::id()));
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

fn png(w: u32, h: u32) -> Vec<u8> {
    let img = image::RgbImage::from_fn(w, h, |x, y| image::Rgb([(x % 256) as u8, (y % 256) as u8, 200]));
    let mut out = Cursor::new(Vec::new());
    assert!(img.write_to(&mut out, image::ImageFormat::Png).is_ok());
    out.into_inner()
}

#[test]
fn tao_mo_lai_dung_khoa_va_tu_choi_khoa_sai() -> ApiResult<()> {
    let dir = TempDir::new("open");
    let key = generate_db_key()?;
    assert_eq!(key.len(), 32);
    assert!(!library_exists(dir.path()));
    drop(LibraryHandle::create(dir.path(), key.clone())?);
    assert!(library_exists(dir.path()));

    let again = LibraryHandle::create(dir.path(), key.clone());
    assert!(matches!(again, Err(e) if e.kind == ApiErrorKind::AlreadyExists));
    let wrong = LibraryHandle::open(dir.path(), generate_db_key()?);
    assert!(matches!(wrong, Err(e) if e.kind == ApiErrorKind::WrongKey));
    let short = LibraryHandle::open(dir.path(), vec![0; 16]);
    assert!(matches!(short, Err(e) if e.kind == ApiErrorKind::InvalidInput));
    LibraryHandle::open(dir.path(), key)?;
    Ok(())
}

#[test]
fn thu_muc_nhan_tai_lieu_va_tim_khong_dau() -> ApiResult<()> {
    let dir = TempDir::new("flow");
    let lib = LibraryHandle::create(dir.path(), generate_db_key()?)?;
    let folder = lib.create_folder("Giấy tờ".into(), None)?;
    let doc = lib.create_document("Hợp đồng thuê nhà".into(), Some(folder.clone()))?;
    let other = lib.create_document("Hóa đơn điện".into(), None)?;
    let tag = lib.create_tag("Quan trọng".into())?;
    assert_eq!(lib.create_tag("QUAN TRỌNG".into())?, tag);
    lib.set_tag(doc.clone(), tag.clone(), true)?;

    assert_eq!(lib.list_documents(None, false, None, 10, 0)?.total, 2);
    let second = lib.list_documents(None, false, None, 1, 1)?;
    assert_eq!((second.items.len(), second.total), (1, 2));
    let in_folder = lib
        .list_documents(Some(folder.clone()), false, None, 10, 0)?
        .items;
    assert_eq!(
        in_folder.iter().map(|d| d.id.as_str()).collect::<Vec<_>>(),
        [doc.as_str()]
    );
    let root = lib.list_documents(None, true, None, 10, 0)?.items;
    assert_eq!(root[0].id, other);
    let tagged = lib.list_documents(None, false, Some(tag.clone()), 10, 0)?;
    assert_eq!(tagged.total, 1);
    assert_eq!(lib.document_tags(doc.clone())?[0].name, "Quan trọng");

    let hits = lib.search("hop dong".into(), 20)?;
    assert_eq!(hits.first().map(|h| h.document_id.as_str()), Some(doc.as_str()));
    assert_eq!(lib.search("h".into(), 20)?.len(), 0);

    lib.rename_document(doc.clone(), "Hợp đồng lao động".into())?;
    lib.move_document(doc.clone(), None)?;
    assert_eq!(lib.document(doc.clone())?.folder_id, None);
    lib.delete_folder(folder)?;
    assert_eq!(lib.list_folders()?.len(), 0);
    lib.delete_document(doc.clone())?;
    assert_eq!(lib.search("lao dong".into(), 20)?.len(), 0);
    assert!(matches!(lib.document(doc), Err(e) if e.kind == ApiErrorKind::NotFound));
    Ok(())
}

#[test]
fn ten_rong_hoac_ky_tu_dieu_khien_bi_tu_choi() -> ApiResult<()> {
    let dir = TempDir::new("names");
    let lib = LibraryHandle::create(dir.path(), generate_db_key()?)?;
    for bad in ["", "   ", "a\u{0}b", &"x".repeat(201)] {
        let e = lib.create_document(bad.to_owned(), None);
        assert!(
            matches!(e, Err(e) if e.kind == ApiErrorKind::InvalidInput),
            "{bad:?}"
        );
    }
    assert_eq!(
        lib.document(lib.create_document("  Biên bản  ".into(), None)?)?
            .title,
        "Biên bản"
    );
    Ok(())
}

#[test]
fn nhap_anh_luu_jpeg_va_tra_anh_thu_nho() -> ApiResult<()> {
    let dir = TempDir::new("pages");
    let lib = LibraryHandle::create(dir.path(), generate_db_key()?)?;
    let doc = lib.create_document("Ảnh".into(), None)?;
    let page = lib.import_image_page(doc.clone(), png(1600, 1200))?;
    let pages = lib.list_pages(doc.clone())?;
    assert_eq!((pages[0].width, pages[0].height), (1600, 1200));
    assert_eq!(lib.document(doc.clone())?.page_count, 1);

    let full = lib.page_image(page.clone())?;
    assert_eq!(image::guess_format(&full).ok(), Some(image::ImageFormat::Jpeg));
    let thumb =
        image::load_from_memory(&lib.page_thumbnail(page.clone(), 256)?).map_err(|_| unreachable_err())?;
    assert_eq!((thumb.width(), thumb.height()), (256, 192));
    let tiny =
        image::load_from_memory(&lib.page_thumbnail(page.clone(), 1)?).map_err(|_| unreachable_err())?;
    assert_eq!(tiny.width(), 64);

    let bad = lib.import_image_page(doc.clone(), b"%PDF-1.7 not an image".to_vec());
    assert!(matches!(bad, Err(e) if e.kind == ApiErrorKind::UnsupportedImage));
    lib.delete_page(page)?;
    assert_eq!(lib.list_pages(doc)?.len(), 0);
    Ok(())
}

#[test]
fn xoay_sap_xep_chuyen_trang_va_thung_rac() -> ApiResult<()> {
    let dir = TempDir::new("organize");
    let lib = LibraryHandle::create(dir.path(), generate_db_key()?)?;
    let a = lib.create_document("A".into(), None)?;
    let b = lib.create_document("B".into(), None)?;
    let p1 = lib.import_image_page(a.clone(), png(300, 200))?;
    let p2 = lib.import_image_page(a.clone(), png(120, 80))?;

    lib.rotate_page(p1.clone(), 1)?;
    let full = image::load_from_memory(&lib.page_image(p1.clone())?).map_err(|_| unreachable_err())?;
    assert_eq!((full.width(), full.height()), (200, 300));
    assert_eq!(
        (
            lib.list_pages(a.clone())?[0].width,
            lib.list_pages(a.clone())?[0].height
        ),
        (200, 300)
    );
    lib.rotate_page(p1.clone(), -1)?;
    assert_eq!(lib.list_pages(a.clone())?[0].width, 300, "xoay trái = xoay -1");
    lib.rotate_page(p1.clone(), 4)?;
    assert_eq!(lib.list_pages(a.clone())?[0].width, 300);

    lib.reorder_pages(a.clone(), vec![p2.clone(), p1.clone()])?;
    assert_eq!(lib.list_pages(a.clone())?[0].id, p2);
    lib.move_pages(vec![p1.clone()], b.clone(), None)?;
    assert_eq!(lib.document(b.clone())?.page_count, 1);
    assert!(lib.page_image(p1).is_ok(), "đọc được bằng khóa tài liệu đích");

    lib.trash_document(a.clone())?;
    assert_eq!(lib.list_documents(None, false, None, 10, 0)?.total, 1);
    let trash = lib.list_trash()?;
    assert_eq!(trash.len(), 1);
    assert_eq!(
        trash[0].purge_at_ms - trash[0].trashed_ms,
        30 * 24 * 60 * 60 * 1000
    );
    assert_eq!(lib.purge_trash(true)?, 0, "chưa quá 30 ngày");
    lib.restore_document(a.clone())?;
    lib.trash_document(a)?;
    assert_eq!(lib.purge_trash(false)?, 1);
    assert_eq!(lib.list_trash()?.len(), 0);
    Ok(())
}

fn unreachable_err() -> tsuscan_ffi::api::error::ApiError {
    tsuscan_ffi::api::error::ApiError {
        kind: ApiErrorKind::Internal,
        detail: "ảnh thu nhỏ không giải mã được".into(),
    }
}

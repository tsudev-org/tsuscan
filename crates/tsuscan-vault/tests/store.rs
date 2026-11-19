//! Kho trên máy: mở/khóa, tài liệu/trang/tệp, thư mục/nhãn, tìm kiếm không dấu, xóa hẳn, chống tráo tệp.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::assert_is_empty)]

use std::path::{Path, PathBuf};

use tsuscan_vault::store::{DB_FILE, DocumentFilter, FILES_DIR, NewPage, Vault};
use tsuscan_vault::{Key32, VaultError};

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let p = std::env::temp_dir().join(format!("tsuscan-vault-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        Self(p)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn key(b: u8) -> Key32 {
    Key32::from_bytes([b; 32])
}

fn page<'a>(image: &'a [u8], text: &'a str) -> NewPage<'a> {
    NewPage {
        image,
        width: 100,
        height: 140,
        dpi: 300,
        ocr_text: text,
        ocr_blob: None,
    }
}

fn files_under(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.join(FILES_DIR)];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() { stack.push(p) } else { out.push(p) }
        }
    }
    out.sort();
    out
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn create_open_wrong_key_and_no_overwrite() {
    let t = TempDir::new("open");
    let doc = {
        let mut v = Vault::create(&t.0, &key(1)).unwrap();
        v.create_document("Giấy khai sinh", None).unwrap()
    };
    assert!(matches!(Vault::create(&t.0, &key(1)), Err(VaultError::Format(_))));
    assert!(matches!(Vault::open(&t.0, &key(2)), Err(VaultError::WrongDbKey)));
    let v = Vault::open(&t.0, &key(1)).unwrap();
    assert_eq!(v.document(&doc).unwrap().title, "Giấy khai sinh");
    let missing = TempDir::new("missing");
    assert!(matches!(
        Vault::open(&missing.0, &key(1)),
        Err(VaultError::NotFound(_))
    ));
    assert!(!missing.0.exists(), "mở kho không tồn tại không được tạo tệp");
}

#[test]
fn pages_roundtrip_encrypted_on_disk() {
    let t = TempDir::new("pages");
    let cccd_text = "SO-CCCD-079123456789";
    let image: Vec<u8> = b"JPEGDATA-chu-ky-mat"
        .iter()
        .copied()
        .cycle()
        .take(150_000)
        .collect();
    let mut v = Vault::create(&t.0, &key(3)).unwrap();
    let doc = v.create_document("Căn cước công dân", None).unwrap();
    let p1 = v
        .add_page(
            &doc,
            NewPage {
                ocr_blob: Some(b"{\"lines\":[]}"),
                ..page(&image, cccd_text)
            },
        )
        .unwrap();
    let p2 = v.add_page(&doc, page(b"trang hai", "trang hai")).unwrap();
    assert_eq!(v.page_image(&p1).unwrap().as_slice(), image.as_slice());
    assert_eq!(
        v.page_ocr_blob(&p1).unwrap().as_deref(),
        Some(&b"{\"lines\":[]}"[..])
    );
    let pages = v.list_pages(&doc).unwrap();
    assert_eq!(
        pages.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
        [p1.as_str(), p2.as_str()]
    );
    assert_eq!(v.document(&doc).unwrap().page_count, 2);
    drop(v);

    // Không tệp nào trên đĩa (CSDL, WAL, tệp trang) chứa bản rõ.
    let mut all = files_under(&t.0);
    for e in std::fs::read_dir(&t.0).unwrap() {
        let p = e.unwrap().path();
        if p.is_file() {
            all.push(p);
        }
    }
    assert!(all.len() >= 3);
    for f in &all {
        let raw = std::fs::read(f).unwrap();
        for needle in [
            cccd_text.as_bytes(),
            b"JPEGDATA",
            "Căn cước".as_bytes(),
            b"can cuoc",
            b"trang hai",
        ] {
            assert!(
                !contains(&raw, needle),
                "{} lộ {:?}",
                f.display(),
                String::from_utf8_lossy(needle)
            );
        }
    }
}

#[test]
fn tampered_or_swapped_page_files_are_rejected() {
    let t = TempDir::new("tamper");
    let mut v = Vault::create(&t.0, &key(4)).unwrap();
    let a = v.create_document("A", None).unwrap();
    let b = v.create_document("B", None).unwrap();
    let pa = v.add_page(&a, page(b"anh tai lieu A", "")).unwrap();
    let pb = v.add_page(&b, page(b"anh tai lieu B", "")).unwrap();
    let files = files_under(&t.0);
    assert_eq!(files.len(), 2);
    // Tráo nội dung hai tệp: khóa/ngữ cảnh khác ⇒ giải mã thất bại cả hai.
    let (fa, fb) = (
        std::fs::read(&files[0]).unwrap(),
        std::fs::read(&files[1]).unwrap(),
    );
    std::fs::write(&files[0], &fb).unwrap();
    std::fs::write(&files[1], &fa).unwrap();
    assert!(matches!(v.page_image(&pa), Err(VaultError::Decrypt)));
    assert!(matches!(v.page_image(&pb), Err(VaultError::Decrypt)));
    // Lật một bit.
    let mut bad = fa.clone();
    let last = bad.len() - 1;
    bad[last] ^= 1;
    std::fs::write(&files[0], &bad).unwrap();
    std::fs::write(&files[1], &bad).unwrap();
    assert!(v.page_image(&pa).is_err() && v.page_image(&pb).is_err());
}

#[test]
fn delete_removes_files_text_and_index() {
    let t = TempDir::new("delete");
    let mut v = Vault::create(&t.0, &key(5)).unwrap();
    let doc = v.create_document("Hợp đồng thuê nhà", None).unwrap();
    let p1 = v.add_page(&doc, page(b"img1", "bên cho thuê")).unwrap();
    let p2 = v.add_page(&doc, page(b"img2", "bên thuê")).unwrap();
    let p3 = v.add_page(&doc, page(b"img3", "điều khoản")).unwrap();
    assert_eq!(files_under(&t.0).len(), 3);

    v.delete_page(&p2).unwrap();
    assert_eq!(files_under(&t.0).len(), 2);
    let pos: Vec<i64> = v.list_pages(&doc).unwrap().iter().map(|p| p.position).collect();
    assert_eq!(pos, [0, 1]);
    assert!(v.page_image(&p2).is_err());

    v.reorder_pages(&doc, &[&p3, &p1]).unwrap();
    assert_eq!(v.list_pages(&doc).unwrap()[0].id, p3);
    assert!(v.reorder_pages(&doc, &[&p3]).is_err());

    assert_eq!(v.search("dieu khoan", 10).unwrap().len(), 1);
    v.delete_document(&doc).unwrap();
    assert!(files_under(&t.0).is_empty());
    assert!(v.search("dieu khoan", 10).unwrap().is_empty());
    assert!(v.search("hop dong", 10).unwrap().is_empty());
    assert!(matches!(v.document(&doc), Err(VaultError::NotFound(_))));
    assert!(v.list_documents(DocumentFilter::default()).unwrap().is_empty());
}

#[test]
fn vietnamese_search_accentless_prefix_and_ranking() {
    let t = TempDir::new("search");
    let mut v = Vault::create(&t.0, &key(6)).unwrap();
    let giao_duc = v.create_document("Bằng tốt nghiệp", None).unwrap();
    v.add_page(&giao_duc, page(b"x", "Bộ Giáo dục và Đào tạo cấp bằng"))
        .unwrap();
    let giao_duc_khong_dau = v.create_document("Bằng tốt nghiệp", None).unwrap();
    v.add_page(&giao_duc_khong_dau, page(b"x", "Bo Giao duc va Dao tao cap bang"))
        .unwrap();
    let dat = v.create_document("Sổ đỏ Đắk Lắk", None).unwrap();
    let hd = v.create_document("Hợp đồng lao động", None).unwrap();
    v.add_page(&hd, page(b"x", "Điều 1. Thời hạn hợp đồng")).unwrap();

    // Gõ không dấu ra kết quả có dấu; đ → d.
    let r = v.search("dak lak", 10).unwrap();
    assert_eq!(
        r.iter().map(|h| h.document_id.as_str()).collect::<Vec<_>>(),
        [dat.as_str()]
    );
    // Tiền tố + không phân biệt hoa thường.
    assert_eq!(v.search("HOP DO", 10).unwrap()[0].document_id, hd);
    // Gõ có dấu, mức khớp tương đương: tài liệu khớp đúng dấu xếp trên tài liệu chỉ khớp không dấu (×0,8).
    let r = v.search("giáo dục", 10).unwrap();
    assert_eq!(r.len(), 2);
    assert_eq!(r[0].document_id, giao_duc);
    assert!(r[0].accent_exact && !r[1].accent_exact);
    assert_eq!(r[1].document_id, giao_duc_khong_dau);
    // Tiêu đề nặng hơn nội dung.
    let in_title = v.create_document("Thời hạn visa", None).unwrap();
    assert_eq!(v.search("thoi han", 10).unwrap()[0].document_id, in_title);
    // Truy vấn quá ngắn / ký tự đặc biệt FTS5 không gây lỗi.
    assert!(v.search("a", 10).unwrap().is_empty());
    for q in [
        "\"",
        "*",
        "a OR b",
        "NEAR(x y)",
        "title_norm:abc",
        "(((",
        "-x",
        "^a",
    ] {
        assert!(v.search(q, 10).is_ok(), "{q}");
    }
}

#[test]
fn folders_tags_and_filters() {
    let t = TempDir::new("folders");
    let mut v = Vault::create(&t.0, &key(7)).unwrap();
    let giay_to = v.create_folder("Giấy tờ", None).unwrap();
    let ca_nhan = v.create_folder("Cá nhân", Some(&giay_to)).unwrap();
    assert!(v.create_folder("x", Some("khong-co")).is_err());
    let d1 = v.create_document("CCCD", Some(&ca_nhan)).unwrap();
    let d2 = v.create_document("Hóa đơn điện", None).unwrap();
    let quan_trong = v.create_tag("Quan trọng").unwrap();
    assert_eq!(v.create_tag("  quan TRỌNG ").unwrap(), quan_trong);
    v.set_tag(&d1, &quan_trong, true).unwrap();
    v.set_tag(&d2, &quan_trong, true).unwrap();
    v.set_tag(&d2, &quan_trong, false).unwrap();

    let ids = |f: DocumentFilter<'_>| -> Vec<String> {
        v.list_documents(f).unwrap().into_iter().map(|d| d.id).collect()
    };
    assert_eq!(
        ids(DocumentFilter {
            tag: Some(&quan_trong),
            ..Default::default()
        }),
        std::slice::from_ref(&d1)
    );
    assert_eq!(
        ids(DocumentFilter {
            folder: Some(None),
            ..Default::default()
        }),
        std::slice::from_ref(&d2)
    );
    assert_eq!(
        ids(DocumentFilter {
            folder: Some(Some(&ca_nhan)),
            ..Default::default()
        }),
        std::slice::from_ref(&d1)
    );
    assert_eq!(v.document_tags(&d1).unwrap()[0].name, "Quan trọng");

    // Xóa thư mục: tài liệu và thư mục con lên thư mục cha.
    v.delete_folder(&ca_nhan).unwrap();
    assert_eq!(
        v.document(&d1).unwrap().folder_id.as_deref(),
        Some(giay_to.as_str())
    );
    v.delete_folder(&giay_to).unwrap();
    assert_eq!(v.document(&d1).unwrap().folder_id, None);
    assert!(v.list_folders().unwrap().is_empty());

    v.update_document(&d2, Some("Hóa đơn tiền điện"), None).unwrap();
    assert_eq!(v.search("tien dien", 5).unwrap()[0].document_id, d2);
    v.delete_tag(&quan_trong).unwrap();
    assert!(v.document_tags(&d1).unwrap().is_empty());
}

#[test]
fn rekey_and_reopen() {
    let t = TempDir::new("rekey");
    let doc = {
        let mut v = Vault::create(&t.0, &key(8)).unwrap();
        let d = v.create_document("Hộ chiếu", None).unwrap();
        v.add_page(&d, page(b"anh ho chieu", "passport")).unwrap();
        v.rekey(&key(9)).unwrap();
        d
    };
    assert!(matches!(Vault::open(&t.0, &key(8)), Err(VaultError::WrongDbKey)));
    let v = Vault::open(&t.0, &key(9)).unwrap();
    let p = &v.list_pages(&doc).unwrap()[0];
    assert_eq!(v.page_image(&p.id).unwrap().as_slice(), b"anh ho chieu");
    assert!(t.0.join(DB_FILE).is_file());
}

#[test]
fn phan_trang_va_dem_tai_lieu_theo_bo_loc() {
    let dir = TempDir::new("page");
    let mut v = Vault::create(&dir.0, &key(7)).unwrap();
    let folder = v.create_folder("Hồ sơ", None).unwrap();
    let mut ids = Vec::new();
    for i in 0..25 {
        let f = (i % 5 == 0).then_some(folder.as_str());
        ids.push(v.create_document(&format!("Tài liệu {i:02}"), f).unwrap());
    }
    let all = v.list_documents(DocumentFilter::default()).unwrap();
    assert_eq!(v.count_documents(DocumentFilter::default()).unwrap(), 25);
    let page2 = v.list_documents_page(DocumentFilter::default(), 10, 10).unwrap();
    assert_eq!(page2, all[10..20]);
    let last = v.list_documents_page(DocumentFilter::default(), 10, 20).unwrap();
    assert_eq!(last.len(), 5);
    assert!(
        v.list_documents_page(DocumentFilter::default(), 10, 100)
            .unwrap()
            .is_empty()
    );
    let in_folder = DocumentFilter {
        folder: Some(Some(&folder)),
        ..Default::default()
    };
    assert_eq!(v.count_documents(in_folder).unwrap(), 5);
    assert_eq!(v.list_documents_page(in_folder, 3, 3).unwrap().len(), 2);
}

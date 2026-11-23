//! P7a: thùng rác 30 ngày, thay ảnh trang, chuyển trang giữa tài liệu (ghép/tách), nâng lược đồ v1 → v2.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::assert_is_empty)]

use std::path::{Path, PathBuf};

use tsuscan_vault::store::{DocumentFilter, FILES_DIR, NewPage, TRASH_RETENTION_MS, Vault};
use tsuscan_vault::{Key32, VaultError};

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let p = std::env::temp_dir().join(format!("tsuscan-org-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        Self(p)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn key() -> Key32 {
    Key32::from_bytes([7; 32])
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

fn file_count(root: &Path) -> usize {
    let mut n = 0;
    let mut stack = vec![root.join(FILES_DIR)];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(d) else { continue };
        for e in rd {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else {
                n += 1;
            }
        }
    }
    n
}

fn all() -> DocumentFilter<'static> {
    DocumentFilter::default()
}

#[test]
fn trash_hides_restores_and_purges_after_30_days() {
    let t = TempDir::new("trash");
    let mut v = Vault::create(&t.0, &key()).unwrap();
    let a = v.create_document("Hợp đồng thuê nhà", None).unwrap();
    let b = v.create_document("Hóa đơn điện", None).unwrap();
    v.add_page(&a, page(b"IMG-A", "tiền thuê tháng")).unwrap();

    v.trash_document(&a).unwrap();
    assert_eq!(v.count_documents(all()).unwrap(), 1);
    assert!(v.list_documents(all()).unwrap().iter().all(|d| d.id != a));
    assert!(matches!(v.document(&a), Err(VaultError::NotFound(_))));
    assert!(
        v.search("thue", 10).unwrap().is_empty(),
        "tài liệu trong thùng rác không được tìm thấy"
    );
    let trash = v.list_trash().unwrap();
    assert_eq!(trash.len(), 1);
    assert_eq!(trash[0].info.id, a);
    assert_eq!(trash[0].info.page_count, 1);
    assert_eq!(file_count(&t.0), 1, "thùng rác giữ tệp trang");

    v.restore_document(&a).unwrap();
    assert_eq!(v.count_documents(all()).unwrap(), 2);
    assert_eq!(v.search("thue", 10).unwrap().len(), 1);
    assert!(matches!(v.restore_document(&a), Err(VaultError::NotFound(_))));
    assert!(
        matches!(v.purge_document(&b), Err(VaultError::NotFound(_))),
        "chỉ xóa vĩnh viễn từ thùng rác"
    );

    v.trash_document(&a).unwrap();
    let trashed = v.list_trash().unwrap()[0].trashed_ms;
    // Chưa đủ 30 ngày ⇒ giữ.
    assert_eq!(
        v.purge_trash(trashed + TRASH_RETENTION_MS - 1, TRASH_RETENTION_MS)
            .unwrap(),
        0
    );
    assert_eq!(v.list_trash().unwrap().len(), 1);
    // Đủ 30 ngày ⇒ xóa hẳn cả tệp.
    assert_eq!(
        v.purge_trash(trashed + TRASH_RETENTION_MS, TRASH_RETENTION_MS)
            .unwrap(),
        1
    );
    assert!(v.list_trash().unwrap().is_empty());
    assert_eq!(file_count(&t.0), 0);
    assert!(matches!(v.trash_document(&a), Err(VaultError::NotFound(_))));
}

#[test]
fn empty_trash_and_purge_one() {
    let t = TempDir::new("empty");
    let mut v = Vault::create(&t.0, &key()).unwrap();
    let a = v.create_document("A", None).unwrap();
    let b = v.create_document("B", None).unwrap();
    v.trash_document(&a).unwrap();
    v.trash_document(&b).unwrap();
    v.purge_document(&a).unwrap();
    assert_eq!(v.list_trash().unwrap().len(), 1);
    assert_eq!(
        v.purge_trash(i64::MAX, 0).unwrap(),
        1,
        "older_than = 0 ⇒ dọn sạch"
    );
    assert!(v.list_trash().unwrap().is_empty());
}

#[test]
fn replace_page_image_swaps_file_and_keeps_ocr() {
    let t = TempDir::new("replace");
    let mut v = Vault::create(&t.0, &key()).unwrap();
    let d = v.create_document("Sổ hộ khẩu", None).unwrap();
    let p = v.add_page(&d, page(b"IMG-OLD", "chủ hộ")).unwrap();
    v.replace_page_image(&p, b"IMG-ROTATED", 140, 100, None).unwrap();
    assert_eq!(&*v.page_image(&p).unwrap(), b"IMG-ROTATED");
    let info = &v.list_pages(&d).unwrap()[0];
    assert_eq!((info.width, info.height), (140, 100));
    assert_eq!(info.ocr_text, "chủ hộ");
    assert_eq!(file_count(&t.0), 1, "tệp cũ bị xóa hẳn");
    v.replace_page_image(&p, b"IMG-3", 140, 100, Some(("thành viên", None)))
        .unwrap();
    assert_eq!(v.search("thanh vien", 5).unwrap().len(), 1);
    assert!(v.search("chu ho", 5).unwrap().is_empty());
}

#[test]
fn move_pages_merges_splits_and_reencrypts() {
    let t = TempDir::new("move");
    let mut v = Vault::create(&t.0, &key()).unwrap();
    let a = v.create_document("A", None).unwrap();
    let b = v.create_document("B", None).unwrap();
    let a1 = v.add_page(&a, page(b"A1", "alpha")).unwrap();
    let a2 = v.add_page(&a, page(b"A2", "beta")).unwrap();
    let a3 = v.add_page(&a, page(b"A3", "gamma")).unwrap();
    let b1 = v.add_page(&b, page(b"B1", "delta")).unwrap();

    // Tách: a3, a1 sang B, chèn trước b1.
    v.move_pages(&[&a3, &a1], &b, Some(0)).unwrap();
    let ids = |v: &Vault, d: &str| {
        v.list_pages(d)
            .unwrap()
            .into_iter()
            .map(|p| p.id)
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&v, &b), vec![a3.clone(), a1.clone(), b1.clone()]);
    assert_eq!(ids(&v, &a), vec![a2.clone()]);
    assert_eq!(
        v.list_pages(&a).unwrap()[0].position,
        0,
        "nguồn được đánh lại thứ tự"
    );
    assert_eq!(&*v.page_image(&a3).unwrap(), b"A3", "giải mã được bằng DK đích");
    assert_eq!(&*v.page_image(&a1).unwrap(), b"A1");
    assert_eq!(file_count(&t.0), 4, "không còn tệp cũ");
    let hits = v.search("gamma", 5).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].document_id, b, "chỉ mục đi theo trang");

    // Ghép: mọi trang của A vào cuối B.
    v.move_pages(&[&a2], &b, None).unwrap();
    assert_eq!(ids(&v, &b), vec![a3.clone(), a1.clone(), b1, a2.clone()]);
    assert_eq!(v.document(&a).unwrap().page_count, 0);

    assert!(matches!(v.move_pages(&[], &b, None), Err(VaultError::Format(_))));
    assert!(matches!(
        v.move_pages(&[&a2], &b, None),
        Err(VaultError::Format(_))
    ));
    assert!(matches!(
        v.move_pages(&[&a2, &a2], &a, None),
        Err(VaultError::Format(_))
    ));
    assert!(matches!(
        v.move_pages(&["khong-co"], &a, None),
        Err(VaultError::NotFound(_))
    ));
}

#[test]
fn reopen_keeps_trash_state() {
    let t = TempDir::new("reopen");
    let a = {
        let mut v = Vault::create(&t.0, &key()).unwrap();
        let a = v.create_document("A", None).unwrap();
        v.trash_document(&a).unwrap();
        a
    };
    let v = Vault::open(&t.0, &key()).unwrap();
    assert_eq!(v.list_trash().unwrap()[0].info.id, a);
}

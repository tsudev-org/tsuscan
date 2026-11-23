# PHIẾU BÀN GIAO - P7a Tổ chức trang + thùng rác (xong mã) → P7b

- **Mã phiếu**: 20261124-08
- **Từ**: agent-scan (phiên d786, 24/11/2026) - **Đến**: phiên P7a/P7b tiếp theo
- **Trạng thái**: MỞ
- **Nhánh git**: `feat/p7a-organize` (tách từ `feat/p6-camera` trước khi PR #9 gộp ⇒ sau khi #9 gộp squash phải
  `git rebase --onto origin/main feat/p6-camera feat/p7a-organize` hoặc cherry-pick commit P7a). PR vào `main`, squash.

## 1. Việc đã làm xong

- P7 chia 3 phiên trong `docs/PLAN.md`: P7a tổ chức + thùng rác, P7b chú thích/chữ ký/che, P7c OCR trong app
  (+ đặt tên thông minh, nhận diện giấy tờ, danh thiếp → vCard).
- Kho lược đồ **v2** (`documents.trashed_ms`, chỉ thêm bước): `trash_document`, `restore_document`, `list_trash`,
  `purge_document`, `purge_trash(now, older_than)`, `replace_page_image`, `move_pages` (mã hóa lại bằng FK mới/DK đích),
  `page_document_id`. Tài liệu trong thùng rác không có dòng FTS. Test `crates/tsuscan-vault/tests/organize.rs` (5).
- Cầu: `trash_document`, `restore_document`, `list_trash` (`TrashedDocumentDto`), `purge_document`, `purge_trash`,
  `rotate_page`, `reorder_pages`, `move_pages`. Test `tests/library.rs::xoay_sap_xep_chuyen_trang_va_thung_rac`.
- App: màn `TrashScreen` (nút thùng rác trên thanh thư viện), "Xóa" tài liệu ⇒ thùng rác + Hoàn tác, menu trang
  (xoay, lên trước/ra sau, chuyển sang tài liệu khác, tách từ trang này), menu tài liệu "Ghép tài liệu khác vào đây"
  (nguồn rỗng vào thùng rác), kính lúp khi kéo góc (`quad_view.dart`), tự dọn thùng rác > 30 ngày khi mở kho
  (`platform_native.dart`). Test `test/organize_test.dart` (4).
- Phiếu SEC1, AV1 ĐÓNG. Phiếu rà soát `docs/security-reviews/20261124-p7a-organize.md`.

## 2. Việc dang dở + bước tiếp theo CỤ THỂ

- [ ] Gộp PR #9 (P6) khi CI xanh, rebase P7a lên main, mở PR P7a, chờ CI, gộp.
- [ ] Chạy thật P7a trên Galaxy M32 (adb Windows `/mnt/c/Users/devng/AppData/Local/Android/Sdk/platform-tools/adb.exe`;
  Flutter trong WSL không thấy máy ⇒ `flutter build apk --profile --target-platform android-arm64` (~12 phút) rồi `adb.exe install -r`).
  24/11: APK profile P7a đã build sẵn ở `C:\Users\devng\AppData\Local\Temp\tsuscan\app-profile.apk`, nhưng điện thoại ngắt
  adb giữa phiên (👤 cắm lại). Cài đè `-r` để kiểm luôn nâng lược đồ v1 → v2 trên dữ liệu thật; kiểm: Xóa ⇒ thùng rác +
  Hoàn tác, khôi phục, xoay (ảnh thu nhỏ tải lại), chuyển/tách/ghép, kính lúp khi kéo góc.
- [ ] P7b, P7c theo `docs/PLAN.md`. Phiếu GP1 (Play Console) sau khi P6 gộp; PRO1/PRO2 sau P8.

## 5. Cảnh báo

- Migration v1 đã phát hành nội bộ ⇒ BẤT BIẾN; đổi lược đồ chỉ thêm `V3`.
- `move_pages` PHẢI mã hóa lại (không bọc lại FK cũ) - xem phiếu rà soát P7a mục 3.
- `organize_test.dart` dùng `pumpApp` từ `app_flow_test.dart`.

## 6. Trạng thái cổng kiểm

- [x] check-standards, `cargo fmt --check`, clippy `-D warnings` (vault, ffi), `cargo test` vault + ffi library
- [x] `flutter analyze --fatal-infos` 0, `flutter test` 47 đạt
- [ ] CI của PR P7a, chạy thật Android

## 7. Kết quả xử lý (agent nhận điền)

-

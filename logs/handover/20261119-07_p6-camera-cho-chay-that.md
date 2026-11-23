# PHIẾU BÀN GIAO - P6 Quét bằng camera: phần lõi xong, chờ chạy thật trên Android + chế độ Sách/QR

- **Mã phiếu**: 20261119-07
- **Từ**: agent-scan (phiên P6) - **Đến**: phiên P6 tiếp theo
- **Thời điểm**: 19/11/2026
- **Trạng thái**: MỞ
- **Nhánh git**: `feat/p6-camera` (tách từ `main` sau PR #8). PR vào `main`, squash.

## 1. Việc đã làm xong

- Đầu phiên (phiếu 06 mục 2): PR #8 (P5) gộp sau khi sửa CI (FRB `dart_format_line_length: 120`; Windows
  `_SILENCE_EXPERIMENTAL_COROUTINE_DEPRECATION_WARNINGS` cho `local_auth_windows` 2.0.2); ghi T3 vào phiếu điều phối;
  đóng 24 cảnh báo CodeQL (chủ dự án cho agent tự quyết).
- Rust `crates/tsuscan-ffi/src/api/scan.rs`: `detect_preview(luma, w, h, stride, rotation)` (dò trên kênh Y, ảnh làm
  việc 480 px, trả góc chuẩn hóa 0..1 đã xoay theo hướng hiển thị), `CapturedImage` opaque (giải mã 1 lần + EXIF,
  `detect`, `render` JPEG xem trước, xóa trắng khi drop), `scan_filters()`; `LibraryHandle::add_scanned_page`
  (nắn + lọc + JPEG q90 một lần, ≤ 4096 px). `encode_jpeg` giữ ảnh xám một kênh. Test `tests/scan.rs` (ảnh tổng hợp
  P1, stride + xoay 90°, bỏ EXIF/GPS, 7 bộ lọc).
- Dart: `data/scan_engine.dart` (Rust/Memory), `features/scan/` gồm `scan_session.dart` (chế độ, `AutoCaptureTracker`,
  phiên chụp loạt chỉ giữ JPEG gốc), `camera_scan_page.dart`, `scan_review_page.dart`, `page_editor_page.dart`,
  `quad_view.dart`, `scan_page.dart` (điểm vào; nhập ảnh cũng qua dò mép + duyệt).
- **Sửa lỗi bảo mật có từ P5**: `AppScope` + màn khóa chuyển lên `MaterialApp.builder` (`_LockGate` trong `app.dart`);
  trước đó hộp thoại/route ở Navigator gốc hiện đè lên màn khóa. Test hồi quy trong `app_flow_test.dart`.
- Quyền: Android chỉ thêm `CAMERA`, gỡ `RECORD_AUDIO`/`WRITE_EXTERNAL_STORAGE`, camera không bắt buộc; iOS
  `NSCameraUsageDescription`. CI chặn quyền Android ngoài danh sách đã rà soát. Phiếu rà soát `docs/security-reviews/20261119-p6-camera.md`.

## 2. Việc dang dở + bước tiếp theo CỤ THỂ

- [ ] **Chạy thật trên Android qua adb** (cần 👤 cắm điện thoại, bật gỡ lỗi USB/không dây; adb Windows:
  `/mnt/c/Users/devng/AppData/Local/Android/Sdk/platform-tools/adb.exe`). Kiểm: xin quyền camera; tứ giác khớp
  khung xem trước (hướng `sensorOrientation`); thời gian `detect_preview` ở `ResolutionPreset.max` (nếu khung > 1080p
  làm giật ⇒ đổi preset hoặc thu nhỏ trước khi qua cầu); tự chụp; đèn; tệp tạm trong `cache/` đã bị xóa
  (`adb shell run-as com.tsudev.tsuscan ls cache`); trang lưu đúng chiều.
- [ ] Chế độ **Sách** (tách 2 trang - `tsuscan-ml` có sẵn) và **QR/mã vạch** (`tsuscan_doc::barcode::scan`): cần thêm
  phụ thuộc `tsuscan-ml`/`tsuscan-doc` vào `tsuscan-ffi` ⇒ đo tăng kích thước `.so` arm64 trước (tract), cân nhắc gom
  cùng lúc đưa OCR vào app.
- [ ] Nhớ chế độ quét lần trước (SettingsStore) - nhỏ.

## 3. File liên quan / đang khóa

| Đường dẫn | Lý do | Còn khóa? |
| --- | --- | --- |
| (không) | | Không |

## 5. Cảnh báo và quyết định quan trọng

- Route toàn màn hình/hộp thoại đẩy lên Navigator gốc nay thấy `AppScope` và bị màn khóa che. KHÔNG đặt lại màn khóa
  vào trong route `home`.
- Camera chỉ chạy khi màn chụp ở tiền cảnh, app không khóa; mọi đường bật camera phải qua `_start()` (kiểm khóa).
- Ảnh từ `takePicture()` PHẢI `deleteTempFile` ngay sau khi đọc.
- `ScanFilter.code` phải khớp `tsuscan_imaging::Filter::code` (test `scan_test.dart` đọc thẳng `filters.rs`).
- `MemoryScanCapture.render` trả bản sao (dispose xóa trắng bộ đệm gốc).
- Các cảnh báo phiếu 01-06 vẫn hiệu lực.

## 6. Trạng thái cổng kiểm

- [x] `./scripts/check-standards.sh`
- [x] `cargo clippy -p tsuscan_ffi --all-targets -D warnings`, `cargo test -p tsuscan_ffi` (lõi + `tests/scan.rs`)
- [x] `flutter analyze --fatal-infos` 0 vấn đề, `flutter test` 41 đạt
- [ ] Chạy thật trên Android

## 7. Kết quả xử lý (agent nhận điền sau khi thực hiện)

-
- 24/11/2026 (agent-scan, phiên d786): phần mã P6 (chạy thật Galaxy M32, Sách/QR, nhớ chế độ) đã có sẵn từ phiên trước
  nhưng chưa commit ⇒ kiểm cổng: check-standards đạt, `cargo fmt --check` đạt, `cargo clippy -p tsuscan-imaging
  -p tsuscan-doc -p tsuscan-ml -p tsuscan_ffi --all-targets -D warnings` đạt, `flutter analyze --fatal-infos` 0 vấn
  đề, `flutter test` 43 đạt. `cargo test` chưa chạy xong (bị cắt vì hết hạn mức). Đã commit cục bộ, CHƯA push.
  Tiếp: chạy `cargo test -p tsuscan-imaging -p tsuscan-doc -p tsuscan_ffi` → `git rebase origin/main` (main có #10, #11,
  không trùng file) → push `feat/p6-camera`, đổi PR #9 khỏi nháp, chờ CI, gộp squash → xóa khóa `logs/LOCKS.md`,
  cập nhật STATE, ĐÓNG phiếu này → P7. Phiếu GP1 (trên main) làm sau khi P6 gộp. SEC1, AV1 đã ĐÓNG.

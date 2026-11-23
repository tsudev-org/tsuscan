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

- [x] PR #9 (P6) gộp `a7bee25`; PR #12 (P7a) gộp `35598d4` (iOS/macOS lần đầu hỏng vì GitHub hết máy macOS ⇒ chạy lại đạt).
- [ ] **Chạy thật P7a trên Galaxy M32 - AGENT TỰ LÀM** (mục 3). 24/11 cuối phiên: `adb.exe devices` đã thấy lại máy
  `RF8T80J6WVF` ⇒ phiên sau kiểm lại bằng lệnh mục 3.1, có máy là làm ngay, không cần hỏi.
- [ ] P7b (mục 4), rồi P7c theo `docs/PLAN.md`.
- [ ] Phiếu GP1 (`logs/handover/20261120-GP1_*`): P6 đã gộp ⇒ làm được, nhưng nên làm SAU P7c (quyền khai phải khớp AAB nộp;
  P7c thêm tải mô hình qua mạng ⇒ khai Data safety một lần). 👤 bấm gửi duyệt. PRO1/PRO2 sau P8.

## 3. Quy trình chạy thật trên điện thoại (agent tự làm, 👤 chỉ cắm cáp + bật gỡ lỗi USB)

Flutter trong WSL KHÔNG thấy máy ⇒ build trong WSL, cài + điều khiển bằng adb của Windows.

```bash
ADB=/mnt/c/Users/devng/AppData/Local/Android/Sdk/platform-tools/adb.exe
PKG=com.tsudev.tsuscan
OUT=/mnt/c/Users/devng/AppData/Local/Temp/tsuscan       # thư mục Windows để adb.exe đọc được APK
```

1. **Có máy chưa**: `timeout 10 $ADB devices` - dòng `<serial>\tdevice` là được. `unauthorized` ⇒ nhắn 👤 "Bấm Cho phép
   gỡ lỗi USB trên điện thoại". Trống ⇒ nhắn 👤 "Cắm lại cáp và bật Gỡ lỗi USB", rồi chờ bằng Monitor
   (`until $ADB devices | grep -q 'device$'; do sleep 5; done`, run_in_background) - KHÔNG chạy `adb install` khi chưa có
   máy (treo ở "waiting for device", lỗi 24/11).
2. **APK**: dùng sẵn `$OUT/app-profile.apk` nếu `git log -1 --format=%H -- apps/app crates` không đổi so với `35598d4`
   (APK build từ P7a). Mã đã đổi ⇒ build lại (~12 phút, chạy nền):
   `cd apps/app && flutter build apk --profile --target-platform android-arm64 && cp build/app/outputs/flutter-apk/app-profile.apk $OUT/`.
3. **Cài ĐÈ** (giữ dữ liệu ⇒ kiểm nâng lược đồ v1 → v2): `$ADB install -r "C:\\Users\\devng\\AppData\\Local\\Temp\\tsuscan\\app-profile.apk"`.
   KHÔNG `uninstall`/`pm clear` trước bước kiểm nâng cấp (mất dữ liệu P6 cũ = mất ca kiểm migration).
4. **Mở + theo dõi lỗi**: `$ADB logcat -c; $ADB shell am start -n $PKG/.MainActivity`, rồi
   `$ADB logcat --pid=$($ADB shell pidof $PKG | tr -d '\r') -v brief | grep -iE 'flutter|error|exception|panic'` (nền).
   Có dòng `purge trash`/`Exception`/`panic` ⇒ ghi lại nguyên văn.
5. **Nhìn + bấm**: chụp `$ADB exec-out screencap -p > $SCRATCH/s1.png` rồi Read ảnh; tìm toạ độ nút bằng
   `$ADB shell uiautomator dump /sdcard/ui.xml && $ADB exec-out cat /sdcard/ui.xml` (Flutter có `content-desc` từ
   tooltip/Semantics) rồi `$ADB shell input tap X Y`; kéo góc: `input swipe X1 Y1 X2 Y2 800`; nhập chữ: `input text`.
   Ảnh chụp màn hình để trong scratchpad, KHÔNG commit (có thể chứa ảnh tài liệu - D3).
6. **Khóa app**: nếu bật PIN, 👤 nhập PIN/vân tay (agent không giữ PIN). Không có tài liệu để thử ⇒ quét 2-3 trang giấy
   tự in (KHÔNG giấy tờ thật) hoặc nhập ảnh mẫu `testdata/` qua `$ADB push` + nút nhập ảnh.

**Danh sách kiểm P7a** (đánh dấu từng dòng vào mục 7):

- [ ] Mở được kho cũ sau khi cài đè (tài liệu P6 còn nguyên, tìm kiếm vẫn ra) = nâng lược đồ v2 đạt, logcat không lỗi.
- [ ] Xóa tài liệu ⇒ biến khỏi thư viện + SnackBar Hoàn tác; bấm Hoàn tác ⇒ trở lại.
- [ ] Xóa lần nữa ⇒ nút thùng rác trên thanh thư viện ⇒ thấy tài liệu + ngày tự xóa (≈ +30 ngày); tìm kiếm KHÔNG ra
  tài liệu đó; Khôi phục ⇒ về thư viện, tìm kiếm ra lại.
- [ ] Xóa vĩnh viễn 1 tài liệu + Dọn sạch (hộp xác nhận đỏ).
- [ ] Menu trang: xoay phải/trái ⇒ ảnh thu nhỏ tải lại đúng chiều; lên trước/ra sau ⇒ số trang đổi.
- [ ] Chuyển trang sang tài liệu khác; tách từ trang N ⇒ tài liệu mới; ghép tài liệu khác vào ⇒ nguồn rỗng vào thùng rác.
- [ ] Màn sửa góc (sau khi chụp): kéo góc ⇒ kính lúp hiện, không che ngón tay, biến mất khi thả.
- [ ] Đóng hẳn app (`$ADB shell am force-stop $PKG`) rồi mở lại ⇒ mọi thay đổi trên còn.
- [ ] Hiệu năng: xoay/chuyển trang ảnh 12 MP < 2 s (ghi số đo nếu chậm).

Lỗi tìm được ⇒ sửa trên nhánh `fix/p7a-device` (hoặc gộp vào đầu P7b nếu nhỏ), test hồi quy, PR riêng. Xong hết ⇒ đóng
phiếu này (mục 7 ghi kết quả + model máy/Android), STATE bỏ ghi chú "Chưa chạy thật".

## 4. P7b - Chú thích + chữ ký + che (gợi ý chia việc, nhánh `feat/p7b-annotate` đã có commit log 9c04511)

- Lõi đã có: `crates/tsuscan-doc/src/redact.rs` (che xóa hẳn điểm ảnh + chữ), `signature.rs` (chữ ký hình). Việc P7b là
  đưa vào kho + cầu + app, KHÔNG viết lại thuật toán.
- Kho: chữ ký lưu sẵn mã hóa (tệp `.tsf` riêng, khóa theo MK, không theo DK tài liệu) ⇒ lược đồ **V3** (chỉ thêm bước).
  Chú thích: nên lưu dạng lớp vector (JSON đã mã hóa) tách khỏi ảnh gốc để sửa/xóa được; "làm phẳng" vào ảnh khi xuất.
- Che: ghi lại ảnh + xóa chữ OCR/`ocr_blob` vùng che + reindex FTS; KHÔNG giữ bản gốc (đúng nghĩa "xóa hẳn") - hộp xác
  nhận nói rõ không hoàn tác được.
- App: màn vẽ (bút/đánh dấu/văn bản, hoàn tác/làm lại), quản lý chữ ký (vẽ, lưu, chèn, đổi cỡ), công cụ che hình chữ nhật.
- Bắt buộc phiếu rà soát `docs/security-reviews/2026MMDD-p7b-annotate.md` (D3, mật mã, che). Khóa `logs/LOCKS.md` trước khi sửa.
- Xong mã ⇒ chạy thật theo mục 3 (thay danh sách kiểm bằng của P7b) TRƯỚC khi đóng phiên.

## 5. Cảnh báo

- Migration v1 đã phát hành nội bộ ⇒ BẤT BIẾN; đổi lược đồ chỉ thêm `V3`.
- `move_pages` PHẢI mã hóa lại (không bọc lại FK cũ) - xem phiếu rà soát P7a mục 3.
- `organize_test.dart` dùng `pumpApp` từ `app_flow_test.dart`.

## 6. Trạng thái cổng kiểm

- [x] check-standards, `cargo fmt --check`, clippy `-D warnings` (vault, ffi), `cargo test` vault + ffi library
- [x] `flutter analyze --fatal-infos` 0, `flutter test` 47 đạt
- [x] CI của PR #12
- [ ] Chạy thật Android

## 7. Kết quả xử lý (agent nhận điền)

-

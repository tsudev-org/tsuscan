# ADR-0010: Khung app Flutter, cầu `tsuscan_ffi`, khóa CSDL trong kho bí mật của thiết bị, khóa app bằng PIN

- **Trạng thái**: ĐÃ CHẤP NHẬN
- **Ngày**: 19/11/2026
- **Người quyết định**: agent-scan
- **Liên quan**: `crates/tsuscan-ffi`, `apps/app`, ADR-0001, ADR-0006, ADR-0009, `docs/THREAT_MODEL.md`

## 1. Bối cảnh

P5 dựng app Flutter 6 đích trên lõi Rust. Cần chốt: cách đóng gói Rust vào app, hình dạng API cầu nối, nơi giữ khóa
CSDL của kho SQLite3MC (ADR-0009), khóa app, và bản web khi lõi WASM chưa có (P11).

## 2. Quyết định

1. **flutter_rust_bridge 2.13.0 + cargokit** (`apps/app/rust_builder`, trỏ `crates/tsuscan-ffi`). Phương án build
   hook native-assets của FRB mới hơn nhưng chưa được hệ tsudev dùng thật; cargokit đã chạy ở lotus-share/tsudev-lotus.
   Crate tên `tsuscan_ffi` (gạch dưới) vì cargokit lấy tên gói làm tên thư viện. Phiên bản FRB ghim **cùng một số** ở
   4 nơi (Cargo.toml `=2.13.0`, pubspec, mã sinh, `scripts/gen-bridge.sh`) và test `frb_version_test.dart` chặn lệch
   (sự cố lotus-share 26.11.1801); Dependabot bỏ qua gói này.
2. **API dạng DTO**: Dart chỉ giữ một đối tượng mờ `LibraryHandle` (bọc `Vault` trong `Mutex` - `rusqlite::Connection`
   không `Sync`); mọi thứ khác là chuỗi/số/byte. Ảnh thu nhỏ giải mã + thu nhỏ trong Rust, Dart chỉ nhận JPEG nhỏ.
   Lỗi đi qua một mã ổn định `ApiErrorKind` để giao diện dịch, không mang đường dẫn/SQL.
3. **Khóa CSDL**: Rust sinh 32 byte → Dart lưu base64 vào `flutter_secure_storage` (Android Keystore, iOS/macOS
   Keychain `first_unlock_this_device` không đồng bộ iCloud, Windows DPAPI, Linux libsecret) **trước** khi tạo kho; mỗi
   lần mở đọc ra, đưa qua cầu rồi xóa. Thiếu khóa mà còn kho ⇒ màn "khóa không còn" với lựa chọn tạo kho mới (xóa hẳn
   dữ liệu cũ) - không có cửa hậu khôi phục (khôi phục thật = 24 từ + đồng bộ, P9).
4. **Khóa app**: PIN 4-12 chữ số, bộ kiểm `pin1$muối$Argon2id(19 MiB, 2 lượt)` trong kho bí mật, so sánh thời gian
   hằng; sai 5 lần ⇒ chờ 30 s nhân đôi tới 1 giờ, bộ đếm cũng nằm trong kho bí mật. Sinh trắc học (`local_auth`,
   `biometricOnly`) chỉ thay PIN khi người dùng bật. Khóa khi mở app và khi quay lại sau ≥ 1 phút ở nền. PIN là lớp
   khóa GIAO DIỆN: dữ liệu trên đĩa đã mã hóa bằng khóa CSDL độc lập với PIN (xem mục 4).
5. **Web xem trước**: chưa có lõi WASM (P11) ⇒ kho bộ nhớ (`MemoryLibraryRepository`), không lưu gì, có băng thông báo.
   Không đưa SQLite thường/IndexedDB rõ vào web chỉ để "có chạy" - trái ADR-0002/0009.
6. **Phân trang tại CSDL**: `Vault::list_documents_page` + `count_documents` (LIMIT/OFFSET), chân vùng bản ghi theo
   `DATA_TABLE.md`, nhớ mốc theo bảng `tsudev.pagesize.<mã-bảng>`. Không đổi lược đồ/định dạng v1.
7. **Nhập ảnh**: giải mã có giới hạn (`tsuscan-imaging::decode`), xoay EXIF rồi mã hóa lại JPEG q88 ⇒ siêu dữ liệu
   gốc (GPS, máy chụp) bị bỏ trước khi vào kho.
8. **Mặc định theo hệ thống** (PRODUCT_POLICY mục 1, phiếu T3): ngôn ngữ hệ điều hành (vi/en, khác ⇒ en), giao diện
   Hệ thống; app mới nên không cần di trú máy đã cài.

## 3. Hệ quả

- Thêm đích mới chỉ thêm giao diện; logic + định dạng nằm trong Rust dùng chung.
- Android `minSdk 26`, `allowBackup=false` + `dataExtractionRules` loại mọi miền: kho D3 không lên sao lưu đám mây
  (vốn vô dụng vì khóa Keystore không chuyển được).
- `local_auth` chưa có bản Linux ⇒ Linux chỉ có PIN.
- Đổi API cầu nối ⇒ chạy `scripts/gen-bridge.sh`; CI sinh lại và so `git diff`.

## 4. Rủi ro đã chấp nhận

- PIN không mã hóa thêm dữ liệu: người có máy đã mở khóa + quyền root đọc được khóa CSDL từ Keystore/Keychain. Mã hóa
  khóa CSDL bằng khóa dẫn xuất từ PIN bị loại vì PIN 4-6 số bị vét cạn ngoại tuyến trong vài giây khi đã có tệp; hướng
  đúng (P13 xem xét) là khóa Keystore yêu cầu xác thực người dùng (`setUserAuthenticationRequired`).
- Bản web xem trước không lưu dữ liệu - người dùng được báo rõ.

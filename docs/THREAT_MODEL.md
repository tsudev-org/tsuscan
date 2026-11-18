# Mô hình đe dọa - Tsu Scan

> Cập nhật: 19/11/2026 (P4, kho trên máy + mật mã). Đồng bộ/máy chủ (P8-P9) bổ sung khi làm. Quyết định liên quan:
> ADR-0002 (xử lý trên máy), ADR-0003 (E2EE), ADR-0009 (kho + định dạng). Dữ liệu cao nhất: **D3**.

## 1. Tài sản

| Tài sản | Mức | Nằm ở đâu |
| --- | --- | --- |
| Ảnh trang (CCCD, hộ chiếu, hợp đồng, hồ sơ y tế) | D3 | Tệp `files/**/*.tsf` mã hóa luồng; RAM khi hiển thị |
| Chữ OCR, tiêu đề, chỉ mục tìm kiếm | D3 | CSDL SQLite3MC (mã hóa toàn trang, gồm WAL); RAM |
| MK, DK, FK, AK | D3 | MK trong CSDL đã mã hóa; DK/FK bọc; bản rõ chỉ trong RAM (tự xóa khi hủy) |
| Khóa CSDL | D3 | Keystore/Keychain/DPAPI/libsecret (app, P5) |
| Mật khẩu mã hóa, 24 từ khôi phục | D3 | Chỉ trong đầu người dùng/giấy; RAM khi nhập |
| Siêu dữ liệu (số tài liệu, cỡ tệp, thời điểm sửa) | D2 | Lộ một phần qua cỡ/tên tệp trên đĩa (xem mục 4) |

## 2. Kẻ tấn công và khả năng

| Mã | Kẻ tấn công | Khả năng | Mục tiêu |
| --- | --- | --- | --- |
| T1 | Trộm/nhặt được máy đã khóa màn hình | Đọc toàn bộ bộ nhớ trong (root/JTAG/sao lưu) | Đọc giấy tờ |
| T2 | App/phần mềm độc khác trên máy (không root) | Đọc thư mục dùng chung, ảnh chụp màn hình, bộ nhớ tạm | Đọc giấy tờ |
| T3 | Máy chủ đồng bộ bị chiếm / nhà vận hành tò mò (P8+) | Đọc/sửa/xóa/phát lại mọi thứ đã gửi lên, ép tham số | Đọc, sửa, tráo dữ liệu |
| T4 | Người biết mật khẩu yếu của người dùng | Dò mật khẩu ngoại tuyến từ gói khóa trên máy chủ | Lấy MK |
| T5 | Tệp độc (PDF/ảnh/phản hồi máy scan) | Dữ liệu không tin cậy vào bộ đọc | Chạy mã, treo app |

## 3. Biện pháp (P4)

| Đe dọa | Biện pháp | Kiểm chứng |
| --- | --- | --- |
| T1 đọc tệp CSDL/WAL/tệp tạm | SQLite3MC ChaCha20-Poly1305 toàn trang; `SQLITE_TEMP_STORE=3` (bảng tạm chỉ RAM); `secure_delete` | `tests/store.rs::pages_roundtrip_encrypted_on_disk` quét mọi tệp tìm bản rõ; `tests/sqlite3mc_probe.rs` |
| T1 đọc tệp trang | XChaCha20-Poly1305 luồng, mỗi tệp một FK, FK bọc DK, DK bọc MK | Như trên + vector `conformance/` |
| T1 lấy khóa CSDL | Khóa trong kho bí mật phần cứng của thiết bị (P5); không có khóa ⇒ CSDL vô dụng | P5/P13 |
| T1/T3 sửa, tráo, cắt, đảo khối tệp | AEAD + AAD buộc ngữ cảnh (`fk|<id>`), nonce chứa chỉ số khối + cờ cuối, kiểm độ dài khai báo | `stream::tests`, `tests/store.rs::tampered_or_swapped...`, `tests/robustness.rs` |
| T3 tráo khóa bọc giữa tài liệu/vai trò | Ngữ cảnh `mk|pw`, `mk|rec`, `dk|<id>`, `fk|<id>`, `ak` trong AAD | `keys::tests::*context*` |
| T3 ép Argon2id yếu / quá nặng | Sàn 2 lượt/19 MiB, trần 10 lượt/1 GiB/4 luồng, kiểm TRƯỚC khi dẫn xuất | `keys::tests::backup_unlock_paths...` |
| T3 lệch đồng hồ để thắng mọi xung đột | HLC từ chối dấu tương lai > 5 phút, không kéo đồng hồ | `hlc::tests` |
| T4 dò mật khẩu | Argon2id 3 lượt/64 MiB; mật khẩu ≥ 10 ký tự; 24 từ = 256 bit | P13 đo trên máy thật |
| T2 | Kho nằm trong thư mục riêng của app (P5); không ghi tệp rõ ra ngoài; xóa tài liệu xóa hẳn tệp ảnh + chữ OCR + chỉ mục | `tests/store.rs::delete_removes_files_text_and_index` |
| T5 | Bộ đọc Rust an toàn bộ nhớ, giới hạn kích thước, fuzz | P1-P3; `fuzz/fuzz_targets/vault_open.rs` |

## 4. Rủi ro còn lại / chấp nhận

1. **Siêu dữ liệu trên đĩa**: số tệp trang và cỡ (đã đệm Padmé, rò ≤ O(log log n) bit) nhìn thấy được. Tên tệp là
   id ngẫu nhiên. Chấp nhận.
2. **RAM**: bản rõ ảnh/chữ trong RAM khi app mở; `zeroize` chỉ xóa khóa và bộ đệm của crate, không xóa bản sao do
   Flutter/hệ điều hành giữ. Giảm thiểu ở P5 (khóa app khi ở nền, `FLAG_SECURE`).
3. **Xóa trên bộ nhớ flash**: `secure_delete` + xóa tệp không đảm bảo xóa vật lý trên SSD/eMMC (wear leveling); vì dữ
   liệu đã mã hóa nên chỉ khôi phục được khi kẻ tấn công có cả khóa. Chấp nhận.
4. **Khóa CSDL và MK cùng trên một máy**: ai có khóa CSDL (máy mở khóa + root) đọc được MK. Đúng thiết kế dùng ngoại
   tuyến không cần mật khẩu; người dùng muốn thêm lớp có thể bật khóa app bằng PIN/sinh trắc (P5, mẫu ADR-0008
   qulythuchi).
5. **Bản vá `libsqlite3-sys`** do repo tự giữ: nâng rusqlite phải áp lại (có test chặn: `PRAGMA cipher` phải là
   `chacha20`).
6. **Chưa chạy fuzz dài** cho `vault_open` (máy phát triển chưa có cargo-fuzz) - P13 chạy trong CI định kỳ.

# ADR-0009: Kho trên máy bằng SQLite3MC, định dạng phong bì/luồng v1, phân cấp khóa, HLC

- **Trạng thái**: ĐÃ CHẤP NHẬN (bổ sung ADR-0003, không thay thế)
- **Ngày**: 19/11/2026
- **Người quyết định**: agent-scan
- **Liên quan**: `crates/tsuscan-vault`, `third_party/libsqlite3-sys`, `conformance/vault-v1.json`, `docs/THREAT_MODEL.md`

## 1. Bối cảnh

P4 cần CSDL mã hóa có FTS5 trên chữ OCR (dữ liệu D3) chạy trên Android, desktop và (P11) WASM, cùng định dạng mã hóa
tệp trang và phân cấp khóa mà ADR-0003 mới mô tả ở mức ý tưởng. `rusqlite` không có feature SQLite3MC chính thức.

## 2. Các phương án và số đo

Đo 19/11/2026: `cdylib` mở CSDL + bảng FTS5, `aarch64-linux-android` (NDK 28.2, API 26), release `opt-level="s"`,
LTO, strip.

| Phương án | `.so` arm64 | Ghi chú |
| --- | --- | --- |
| SQLite thường (`rusqlite` `bundled`) | 1,50 MB | Không mã hóa - mốc so sánh |
| **SQLite3MC** (ChaCha20-Poly1305, `sqlite3mc-src` 2.6.1) | **1,37 MB** | Tắt thuật toán cũ, không cần OpenSSL |
| SQLCipher (`bundled-sqlcipher-vendored-openssl`) | 5,48 MB | +4 MB mỗi ABI, build OpenSSL chậm, AES-CBC+HMAC |
| Mã hóa tầng ứng dụng, SQLite thường | ≈ 1,50 MB | Chỉ mục FTS5 nằm rõ trên đĩa |

- Mã hóa tầng ứng dụng bị loại: chỉ mục FTS5 (và trang WAL/tạm) chứa nguyên chữ OCR của giấy tờ ⇒ lộ dữ liệu D3 khi
  mất máy. Chỉ mục mù (HMAC từng từ) mất tìm tiền tố, xếp hạng và tô sáng theo `SEARCH_AND_FILTER.md`.
- SQLCipher bị loại vì kích thước và phụ thuộc OpenSSL; SQLite3MC đã được qulythuchi dùng (ADR-0008 của repo đó).
- WASM (P11): `rusqlite` trên `wasm32-unknown-unknown` dùng `sqlite-wasm-rs`, crate này có sẵn feature `sqlite3mc`
  ⇒ cùng định dạng tệp CSDL, IndexedDB/OPFS chỉ thấy bản mã.

## 3. Quyết định

1. **CSDL**: SQLite3MultipleCiphers qua bản sao vá `third_party/libsqlite3-sys` (`[patch.crates-io]`, chi tiết
   `third_party/libsqlite3-sys/TSUSCAN.md`). Chỉ bật `CODEC_TYPE_CHACHA20`; `SQLITE_TEMP_STORE=3` (bảng tạm chỉ trong
   RAM), `SQLITE_SECURE_DELETE=1`, `SQLITE_DQS=0`, tắt nạp extension. Khóa CSDL 32 byte ngẫu nhiên do app giữ trong
   Keystore/Keychain/DPAPI/libsecret, đặt bằng `PRAGMA hexkey`; mở xong kiểm `PRAGMA cipher = chacha20` và đọc thử
   `sqlite_schema` - sai khóa trả lỗi, không tạo tệp mới đè lên.
2. **Phong bì v1** (bản ghi, khóa bọc): `'T' 'S' 01 loại ‖ nonce 24 ‖ XChaCha20-Poly1305(đệm(bản rõ, khối))`,
   AAD = 4 byte đầu ‖ ngữ cảnh (UTF-8, ví dụ `dk|<id tài liệu>`). Đệm ISO/IEC 7816-4 (`0x80` rồi `0x00`, như
   `sodium_pad`). Loại `01` bản ghi (khối 256 B), `02` khóa (khối 16 B).
3. **Luồng v1** (tệp trang): đầu tệp 24 B = `'T' 'S' 01 03 ‖ tiền tố nonce 19 ‖ log2(cỡ khối) = 16`. Khối thứ i:
   nonce = tiền tố ‖ i (u32 big-endian) ‖ cờ (`01` nếu là khối cuối, ngược lại `00`), AAD = đầu tệp ‖ ngữ cảnh, bản
   rõ 65 536 B (khối cuối ≤, luôn có đúng một khối cuối). Bản rõ logic = độ dài thật (u64 little-endian) ‖ dữ liệu ‖
   số 0 đến cỡ Padmé ⇒ giấu kích thước chính xác (dư ≤ 12 %). Đảo/cắt/ghép khối ⇒ sai nonce hoặc thiếu cờ cuối ⇒ lỗi.
4. **Phân cấp khóa**:
   - MK 32 B ngẫu nhiên. Trên máy MK nằm trong bảng `meta` của CSDL đã mã hóa (dùng ngoại tuyến không cần tài khoản).
   - KEK mật khẩu = Argon2id(NFKC(mật khẩu), muối 16 B), mặc định 3 lượt / 64 MiB / 1 luồng; mở khóa từ chối dưới
     2 lượt / 19 MiB (sàn OWASP) và trên 10 lượt / 1 GiB (chống bị ép tham số làm treo máy). Mật khẩu ≥ 10 ký tự sau
     chuẩn hóa.
   - KEK khôi phục = HKDF-SHA256(entropy 32 B của 24 từ BIP39 tiếng Anh có tổng kiểm, info `tsuscan/recovery/v1`).
   - DK 32 B mỗi tài liệu bọc bằng MK (ngữ cảnh `dk|<id>`); FK 32 B mỗi tệp trang bọc bằng DK (`fk|<id>`); khóa tài
     khoản AK bọc bằng MK (`ak`) cho bản ghi không thuộc tài liệu (thư mục, nhãn). MK bọc bằng KEK: `mk|pw`, `mk|rec`.
   - Mọi khóa trong bộ nhớ dùng kiểu tự xóa (`zeroize`) khi hủy.
5. **HLC**: 64 bit = mili giây Unix (48 bit) ‖ bộ đếm (16 bit), kèm id thiết bị u64 để phá hòa. Nhận dấu từ xa lệch
   tương lai > 5 phút ⇒ từ chối (không kéo đồng hồ cục bộ). Hợp nhất theo trường, ghi sau thắng (như ADR-0004 của
   qulythuchi).
6. **Tìm kiếm**: FTS5 `unicode61 remove_diacritics 0` trên cột đã chuẩn hóa sẵn lúc ghi bằng **một** hàm
   `normalize::vi_normalize` (NFC → chữ thường → bỏ dấu, `đ` → `d` → gộp khoảng trắng), song song cột có dấu để xếp
   hạng khớp đúng dấu cao hơn.
7. **Vector đối chiếu** `conformance/vault-v1.json` sinh bằng nonce/muối cố định qua API chỉ dành cho test; CI kiểm
   lõi Rust giải mã đúng từng vector.

## 4. Hệ quả

- Đổi bất kỳ điểm nào ở mục 3.2-3.5 ⇒ phiên bản định dạng mới + ADR mới + sinh lại `conformance/` CÙNG PR; bản cũ
  vẫn đọc được.
- Nâng `rusqlite`/`libsqlite3-sys` phải áp lại bản vá (3 điểm) và chạy `tests/sqlite3mc_probe.rs`.
- Chỉ mục FTS và bản rõ chỉ tồn tại trong tệp CSDL đã mã hóa và RAM; tệp WAL/journal cũng được SQLite3MC mã hóa.
- Thời gian Argon2id 64 MiB trên điện thoại thật đo ở P6/P14; nếu > 2 giây trên máy tầm trung xem lại ở ADR mới.

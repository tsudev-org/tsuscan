# KIẾN TRÚC - Tsu Scan

## 1. Tổng quan

```
┌──────────────────────── apps/app (Flutter: Android · iOS · Windows · macOS · Linux · Web) ────────────────────────┐
│ Giao diện, i18n vi/en, điều hướng, camera (plugin), kho bí mật HĐH, sinh trắc, chia sẻ, in                        │
└───────────────┬───────────────────────────────────────────────────────────────────────────────┬──────────────────┘
                │ flutter_rust_bridge (FFI; web: WASM)                                           │ HTTPS + ghim CA
┌───────────────▼───────────────────── crates/ (Rust, dùng chung mọi nền tảng) ─────────┐  ┌──────▼────────────────┐
│ tsuscan-imaging  dò mép, phối cảnh, lọc, xoay                                          │  │ services/sync-api     │
│ tsuscan-ml       suy luận ONNX (tract): phân đoạn, dewarp, ngón tay, OCR, bố cục       │  │ Workers + Hono        │
│ tsuscan-doc      PDF/PDF-A/DOCX/XLSX, nhập PDF/ảnh, che, ký, QR                        │  │ D1: siêu dữ liệu mã   │
│ tsuscan-vault    SQLite mã hóa, FTS5, tệp mã hóa luồng, khóa, HLC, phong bì E2EE      │  │ R2: khối tệp đã mã hóa│
│ tsuscan-scanner  eSCL · SANE · WIA · TWAIN · ICA (chỉ desktop + eSCL Android)          │  └───────────────────────┘
│ tsuscan-ffi      API ổn định cho Flutter; tsuscan-cli  công cụ dòng lệnh               │
└────────────────────────────────────────────────────────────────────────────────────────┘
apps/site (Astro, Cloudflare Pages): giới thiệu, tải về, hướng dẫn, riêng tư.
```

## 2. Cấu trúc thư mục (hình trạng B của `PROJECT_STRUCTURE.md`)

| Đường dẫn | Nội dung |
| --- | --- |
| `crates/*` | Workspace Rust (`Cargo.toml` gốc), MSRV ghim trong `rust-toolchain.toml` |
| `apps/app` | Flutter, `lib/l10n/*.arb`, `lib/src/{features,ui,bridge}` |
| `apps/site` | Astro |
| `services/sync-api` | TypeScript, Hono, migration D1 |
| `models/` | Chỉ **manifest** mô hình (URL, sha256, chữ ký, giấy phép); tệp mô hình tải lúc build/chạy |
| `testdata/` | Bộ ảnh mẫu có nhãn (ảnh tự chụp hoặc giấy phép mở, KHÔNG có giấy tờ thật của người thật) |
| `conformance/` | Vector đối chiếu định dạng mã hóa giữa Rust và TypeScript |
| `docs/adr`, `docs/security-reviews` | Quyết định, phiếu rà soát |

## 3. Luồng quét một trang

1. Camera đưa khung YUV (xem trước) → `tsuscan-imaging::detect` trả tứ giác + điểm tin cậy; app vẽ khung, tính độ ổn
   định qua N khung ⇒ tự chụp.
2. Ảnh độ phân giải đầy đủ → nắn phối cảnh → (chế độ Sách) dewarp + tách trang + xóa ngón tay (`tsuscan-ml`) → bộ lọc.
3. OCR chạy nền (hàng đợi ưu tiên thấp), kết quả: chữ + hộp bao theo dòng/từ → FTS5 + lớp chữ PDF.
4. Lưu: ảnh gốc (tùy chọn) + ảnh đã xử lý mã hóa luồng vào kho; bản ghi trang/tài liệu vào SQLite mã hóa.
5. Đồng bộ (nếu đăng nhập): bản ghi đã niêm phong + khối tệp đã mã hóa lên máy chủ.

Mọi bước 1-4 chạy **ngoại tuyến**, ảnh không bao giờ rời thiết bị ở dạng rõ.

## 4. Mô hình AI

- Định dạng ONNX, suy luận bằng `tract` (Rust thuần ⇒ build từ nguồn được cho F-Droid, chạy WASM cho web). Bản
  desktop có thể bật `ort` (ONNX Runtime) để tăng tốc GPU sau 1.0, sau cờ tính năng.
- Mô hình KHÔNG nằm trong git. `models/manifest.toml` ghi URL, sha256, kích thước, giấy phép; tải lúc build (APK nhúng
  mô hình nhỏ) hoặc tải thêm trong app có kiểm sha256 + chữ ký minisign của tsudev.
- Chỉ dùng mô hình có giấy phép cho phép phân phối lại (Apache-2.0, MIT, BSD); ghi vào `THIRD-PARTY-NOTICES.md`.

## 5. Bảo mật (chi tiết `docs/THREAT_MODEL.md` từ P4)

- Dữ liệu D3. Kho trên máy: SQLite3MultipleCiphers (ChaCha20-Poly1305), khóa CSDL trong Keystore/Keychain/DPAPI/libsecret.
- Tệp trang: XChaCha20-Poly1305 dạng luồng chia khối 64 KiB, mỗi tệp một khóa ngẫu nhiên bọc bằng khóa tài liệu.
- E2EE đồng bộ: phân cấp khóa MK/KEK như Quly Thu Chi (ADR-0003), mật khẩu Argon2id, 24 từ khôi phục.
- Bộ đọc dữ liệu không tin cậy (ảnh, PDF, phản hồi eSCL) viết bằng Rust an toàn bộ nhớ + `cargo fuzz` trong CI định kỳ.
- `#![forbid(unsafe_code)]` mọi crate trừ `tsuscan-ffi` và backend phần cứng (WIA/TWAIN/SANE), mỗi khối `unsafe` có
  chú thích `// SAFETY:`.
- Chuỗi cung ứng: `cargo deny` (giấy phép, lỗ hổng, nguồn), `cargo audit`, ghim action theo SHA, SBOM + chứng thực
  nguồn gốc khi phát hành.

## 6. Hiệu năng

- Xem trước: xử lý ở độ phân giải giảm (≈ 640 px cạnh dài) trên luồng riêng, bỏ khung khi bận.
- Ảnh lớn: xử lý theo ô (tile) để giới hạn bộ nhớ < 300 MB trên điện thoại.
- OCR: hàng đợi nền, có thể hủy, ưu tiên trang đang xem.

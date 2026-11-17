# ADR-0001: Monorepo, lõi Rust dùng chung + Flutter mọi đích + Cloudflare Workers

- **Trạng thái**: ĐÃ CHẤP NHẬN
- **Ngày**: 18/11/2026
- **Người quyết định**: agent-scan (chủ dự án ủy quyền chọn phương án tối ưu, 18/11/2026)

## 1. Bối cảnh
Tsu Scan cần cùng một thuật toán xử lý ảnh/OCR/mã hóa cho 6 đích (Android, iOS, Windows, macOS, Linux, web) để kết quả
và dữ liệu khớp nhau khi đồng bộ. Xử lý ảnh nặng tính toán, đọc dữ liệu không tin cậy (ảnh, PDF) nên cần an toàn bộ nhớ.

## 2. Phương án đã xét
| Phương án | Ưu | Nhược |
| --- | --- | --- |
| A. Rust lõi + Flutter + `flutter_rust_bridge` | Một mã nguồn thuật toán, nhanh như C++, an toàn bộ nhớ, chạy WASM; Flutter đã dùng ở qulythuchi/lotus | Cầu FFI cần build đa ABI |
| B. Kotlin Multiplatform + OpenCV | Native Android tốt | OpenCV C++ nặng, khó F-Droid, desktop yếu |
| C. .NET/Avalonia như tsufiles | Có sẵn kinh nghiệm desktop | Di động/camera yếu, không WASM nhẹ |
| D. Mỗi nền tảng một app native | Trải nghiệm tối đa | Gấp 5 lần công, thuật toán lệch nhau |

## 3. Quyết định
Chọn A. Workspace `crates/` (Rust stable, `rust-toolchain.toml` ghim), app `apps/app` (Flutter ghim phiên bản + commit
như qulythuchi), máy chủ `services/sync-api` (TypeScript + Hono trên Workers, D1, R2), site `apps/site` (Astro).
Từ tsufiles chỉ học quy trình phát hành (SHA256SUMS ký, chứng thực nguồn gốc, OpenSSF Scorecard, menu chuột phải,
CLI), không dùng chung mã.

## 4. Hệ quả
- CI có job Rust (fmt, clippy, test, deny, audit) từ P1, job Flutter từ P5.
- Mọi logic nghiệp vụ nằm trong Rust, Flutter chỉ là giao diện và cầu nối thiết bị.

## 5. Thứ tự nền tảng
Android trước (P6) → máy tính (P10) → web/PWA (P11) → iOS (mã build trên CI, phát hành khi có tài khoản Apple).

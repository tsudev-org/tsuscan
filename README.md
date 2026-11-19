# Tsu Scan

**Quét tài liệu chuyên nghiệp, xử lý hoàn toàn trên thiết bị, OCR tiếng Việt, đồng bộ mã hóa đầu-cuối.**
Một sản phẩm của [TsuDEV](https://tsudev.com). *English below.*

> Trạng thái: đang xây dựng (P5 - khung app). Kế hoạch: [`docs/PLAN.md`](docs/PLAN.md).

## Chạy thử

```bash
cargo test --workspace                               # lõi Rust
export PATH="$HOME/tools/flutter/bin:$PATH"          # Flutter 3.47.8
cargo install flutter_rust_bridge_codegen --version 2.13.0 --locked
./scripts/gen-bridge.sh                              # chỉ khi đổi API trong crates/tsuscan-ffi/src/api
cd apps/app && flutter test && flutter run           # Android/desktop; web là bản xem trước (kho bộ nhớ)
```

- Chụp tài liệu, sách, CCCD, danh thiếp, bảng trắng; dò mép, nắn phẳng, khử bóng, làm phẳng trang cong.
- OCR tiếng Việt + Anh ngay trên máy, PDF tìm kiếm được, xuất DOCX/XLSX.
- Điều khiển máy scan thật trên Windows, macOS, Linux.
- Đồng bộ Android ⇄ máy tính bằng tài khoản tsudev; máy chủ chỉ giữ bản mã.
- Miễn phí, không quảng cáo, mã nguồn mở (AGPL-3.0).

Nền tảng: Android, Windows, macOS, Linux, web (PWA); iOS/macOS dựng song song, phát hành đầu 2027.

## English

**Tsu Scan** is a free, open-source document scanner: on-device edge detection, dewarping, filters and Vietnamese +
English OCR, searchable PDF export, real scanner support on desktop, and end-to-end encrypted sync. Status: in
development, see [`docs/PLAN.md`](docs/PLAN.md) (Vietnamese).

## Giấy phép

[AGPL-3.0-only](LICENSE). Tên "Tsu Scan", "TsuDEV" và logo không thuộc phạm vi giấy phép, xem [`NOTICE`](NOTICE).

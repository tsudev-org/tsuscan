# Nhật ký thay đổi

Theo [Keep a Changelog](https://keepachangelog.com/vi/1.1.0/), phiên bản theo `.standards/docs/VERSIONING.md`.

## [Chưa phát hành]

### Thêm
- Lõi ảnh `crates/tsuscan-imaging` (P1): giải mã JPEG/PNG/WebP có giới hạn 100 MP/512 MiB + hướng EXIF; dò mép cổ
  điển (Canny tự động, contour, vùng sáng, Hough có hướng, chấm điểm cực tính, tinh chỉnh cạnh, lượt sắc độ cho giấy
  trắng trên bàn trắng); nắn phối cảnh song tuyến/bicubic, đoán khổ A4/Letter/ID-1; 7 bộ lọc (Gốc, Tự động, Màu, Xám,
  Đen trắng Sauvola, Khử bóng, Bảng trắng).
- Bộ sinh ảnh tổng hợp có nhãn (5 loại nền, bóng tay, lóa, mờ chuyển động, nhiễu) và test đo IoU trên 150 ảnh.
- CLI dev `tsuscan` (`detect`, `scan`, `synth`), benchmark criterion, đích fuzz `decode`/`detect`, test bền vững.
- CI job `Lõi Rust (crates)`: fmt, clippy pedantic, test + độ phủ, cargo-deny; Dependabot cho cargo.
- Nền móng dự án (P0): bộ quy ước tsudev v3.7.0, kế hoạch 15 phase, kiến trúc, ma trận tính năng, ADR 0001-0006, CI khung.

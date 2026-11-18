# Nhật ký thay đổi

Theo [Keep a Changelog](https://keepachangelog.com/vi/1.1.0/), phiên bản theo `.standards/docs/VERSIONING.md`.

## [Chưa phát hành]

### Số đo P2 (18/11/2026, x86_64 4 nhân, máy dùng chung tải 6-13)

| Chỉ tiêu | Ngưỡng | Đo được |
| --- | --- | --- |
| CER OCR tiếng Việt in, toàn trang (`ocr_cer`, 12 trang) | ≤ 3% | **0,51%** (82/16.006 ký tự; WER 2,63%; Tinos 0,56%, Tinos Bold 0,68%, Arimo, Be Vietnam Pro 0,46%) |
| Hướng trang đúng (6 trang × 4 góc) | ≥ 95% | 24/24 |
| CER trang cong 45 px trước → sau làm phẳng | giảm, ≤ 5% | 33,77% → 0,46% |
| Thời gian OCR (CPU luồng, tract) | - | dò dòng ≈ 1,6-2,2 s/trang (cạnh 1.280); nhận dạng ≈ 0,11 s (rộng 256) - 0,64 s (rộng 1.024) mỗi dòng sau khi biên dịch xô |
| Dò mép P1 `detect` (CPU luồng, `diag -- 0 50`, tải 2,9) | < 40 ms | **31,6 ms/khung** - chỉ tiêu P1 đã chứng minh |
| Thời gian nhận dạng hình dạng ký hiệu (đã bỏ) | - | 0,32 s (256) - 1,35 s (1.024) mỗi dòng ⇒ chuyển sang xô chiều rộng 128 px |

### Thêm
- AI trên máy `crates/tsuscan-ml` (P2), suy luận ONNX bằng tract (Rust thuần):
  - OCR tiếng Việt/Anh: dò dòng PP-OCRv5 mobile det + hậu xử lý DB, nhận dạng AsterNet (CTC, 227 lớp), thứ tự đọc
    theo hàng, tự đảo màu chữ sáng nền tối, chia đoạn dòng dài. Chọn bằng số đo (ADR-0007).
  - Hướng trang 0/90/180/270 (PP-LCNet doc_ori), làm phẳng trang cong (UVDoc; lưới thô từ tract, lấy mẫu bằng Rust).
  - Cổ điển: tách hai trang sách tại gáy, xóa ngón tay chạm mép, bố cục đoạn/tiêu đề/bảng + xuất Markdown.
  - `models/manifest.toml` (URL ghim commit, sha256, kích thước, giấy phép), `scripts/fetch-models.sh`, nạp mô hình chỉ
    sau khi khớp sha256 (+ minisign khi bật khóa), từ chối external data; vá đồ thị lúc nạp cho 2 lỗi tương thích tract
    (`If` từ `torch.squeeze`, `Pad` reflect có `constant_value`).
  - Bộ mẫu `testdata/ocr-vi` (kho câu tự soạn + bộ sinh trang tất định, font OFL), test CER/hướng trang/làm phẳng.
  - CLI: `ocr` (text/markdown/json, `--orient`), `orient`, `dewarp`, `split`, `definger`, `synth-ocr`, `models`.
  - CI job `AI trên máy (tsuscan-ml)`: tải mô hình (cache theo hash manifest), test bắt buộc có mô hình + độ phủ.
- Lõi ảnh `crates/tsuscan-imaging` (P1): giải mã JPEG/PNG/WebP có giới hạn 100 MP/512 MiB + hướng EXIF; dò mép cổ
  điển (Canny tự động, contour, vùng sáng, Hough có hướng, chấm điểm cực tính, tinh chỉnh cạnh, lượt sắc độ cho giấy
  trắng trên bàn trắng); nắn phối cảnh song tuyến/bicubic, đoán khổ A4/Letter/ID-1; 7 bộ lọc (Gốc, Tự động, Màu, Xám,
  Đen trắng Sauvola, Khử bóng, Bảng trắng).
- Bộ sinh ảnh tổng hợp có nhãn (5 loại nền, bóng tay, lóa, mờ chuyển động, nhiễu) và test đo IoU trên 150 ảnh.
- CLI dev `tsuscan` (`detect`, `scan`, `synth`), benchmark criterion, đích fuzz `decode`/`detect`, test bền vững.
- CI job `Lõi Rust (crates)`: fmt, clippy pedantic, test + độ phủ, cargo-deny; Dependabot cho cargo.
- Nền móng dự án (P0): bộ quy ước tsudev v3.7.0, kế hoạch 15 phase, kiến trúc, ma trận tính năng, ADR 0001-0006, CI khung.

# ADR-0002: Xử lý ảnh, AI và OCR hoàn toàn trên thiết bị, suy luận ONNX bằng tract

- **Trạng thái**: ĐÃ CHẤP NHẬN
- **Ngày**: 18/11/2026
- **Người quyết định**: agent-scan (chủ dự án chốt "E2EE, xử lý trên máy" 18/11/2026)

## 1. Bối cảnh
Giấy tờ quét là dữ liệu D3. App tham chiếu phần lớn OCR trên máy chủ. F-Droid không nhận thư viện độc quyền (ML Kit)
hay nhị phân dựng sẵn không build lại được.

## 2. Quyết định
- Thuật toán cổ điển (P1) là nền, luôn có; mô hình AI (P2) là lớp tăng cường, có thể tắt.
- Suy luận ONNX bằng `tract` (Rust thuần, Apache-2.0/MIT): build từ nguồn, chạy WASM. `ort` là tùy chọn desktop sau 1.0.
- Engine OCR chọn bằng **đo** CER/tốc độ/kích thước trên `testdata/ocr-vi` ở P2 (ứng viên PaddleOCR PP-OCR ONNX,
  VietOCR ONNX, Tesseract 5 `vie`), ghi kết quả vào ADR mới.
- Mô hình ngoài git, manifest có sha256 + chữ ký minisign, chỉ giấy phép cho phép phân phối lại.

## 3. Hệ quả
Không có bước nào gửi ảnh/chữ ra mạng. Kích thước app tăng ⇒ APK nhúng mô hình nhỏ nhất, phần còn lại tải thêm.

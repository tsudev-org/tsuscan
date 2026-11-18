# ADR-0007: Engine OCR tiếng Việt - PP-OCRv5 det + AsterNet rec trên tract; mô hình phụ trợ P2

- **Trạng thái**: ĐÃ CHẤP NHẬN
- **Ngày**: 18/11/2026
- **Người quyết định**: agent-scan (ADR-0002 giao chọn engine OCR bằng số đo ở P2)

## 1. Bối cảnh

ADR-0002 chốt: suy luận ONNX bằng `tract` (Rust thuần), xử lý trên máy, chỉ giấy phép trọng số Apache-2.0/MIT/BSD,
chọn OCR bằng đo CER trên `testdata/ocr-vi`. Ứng viên trong PLAN: PaddleOCR PP-OCR, VietOCR, Tesseract 5.

## 2. Số đo (18/11/2026, x86_64 4 nhân, 1 luồng)

**Lượt 1 - lọc ứng viên nhận dạng dòng** (onnxruntime chỉ để lọc nhanh; 119 dòng của `testdata/ocr-vi/corpus.txt`,
4 font Tinos/Tinos Bold/Arimo/Be Vietnam Pro, cỡ 28-40 px, nhòe ngẫu nhiên, JPEG q55-90):

| Ứng viên | Giấy phép | Kích thước | CER dòng | CPU/dòng | Kết luận |
| --- | --- | --- | --- | --- | --- |
| **AsterNet v1.0** (asterxlabs, CTC, 227 lớp) | MIT | 41 MB | **0,45%** | **239 ms** | **Chọn** |
| VietOCR vgg_transformer (bản ONNX vemines) | MIT/Apache-2.0 | 153 MB + external data | 0,92% | 1.762 ms | Loại: chậm 7×, nặng 4×, tự hồi quy |
| PP-OCRv5 mobile rec (zh/en/ja) | Apache-2.0 | 16 MB | - | - | Loại: bảng ký tự thiếu 92 chữ Việt có dấu thanh (ả, ạ, ằ, ...) |
| PP-OCRv5 latin mobile rec | Apache-2.0 | 8 MB | - | - | Loại: bảng ký tự thiếu toàn bộ nguyên âm có dấu nặng/hỏi/ngã của tiếng Việt |
| Tesseract 5 `vie` | Apache-2.0 | 4-12 MB | - | - | Loại theo tiêu chí kiến trúc: engine C++ không chạy được trên tract/WASM, trái ADR-0002 (không đo) |

Bảng ký tự PP-OCR được kiểm bằng script so với bảng chữ Việt đầy đủ (134 ký tự có dấu); không cần đo CER vì
mô hình về nguyên tắc không thể phát ra các ký tự thiếu.

**Lượt 2 - toàn trang trên tract (số chính thức, CI chạy lại):** `cargo test -p tsuscan-ml --test ocr_cer`, trang A4
≈ 150 DPI sinh tất định (`synth.rs`), 22 dòng/trang, nghiêng ±1,2°, nhòe, nhiễu, JPEG.

| Chỉ tiêu | Ngưỡng PLAN | Đo được |
| --- | --- | --- |
| CER tiếng Việt in, toàn trang (dò + nhận dạng + thứ tự đọc), 12 trang | ≤ 3% | **0,51%** (82/16.006 ký tự), WER 2,63% |

## 3. Quyết định

- **Dò dòng**: PP-OCRv5 mobile det (DB, 4,8 MB, Apache-2.0), hậu xử lý DB viết bằng Rust (`ocr/det.rs`).
- **Nhận dạng**: AsterNet v1.0 (MIT), giải mã CTC tham lam, ảnh dòng cao 32 px, chuẩn hóa `[0, 1]`, tự đảo màu chữ
  sáng nền tối, dòng > 1.600 px chia đoạn ở khoảng trắng.
- **Hướng trang**: PP-LCNet x1.0 doc_ori (6,8 MB, Apache-2.0) - 0/90/180/270.
- **Làm phẳng trang cong**: UVDoc (31,7 MB, Apache-2.0) - mô hình tự `GridSample` trên ảnh vào.
- **Tách 2 trang sách, xóa ngón tay, bố cục đoạn/bảng**: thuật toán cổ điển (`pages.rs`, `layout.rs`), không thêm
  mô hình - đủ cho ảnh đã nắn phối cảnh, không tăng kích thước app, chạy được mọi nền tảng.
- **Phân đoạn tài liệu bằng AI**: KHÔNG thêm ở P2. Dò mép cổ điển P1 đạt IoU 0,9977 trên bộ tổng hợp; chưa có ảnh chụp
  thật để chứng minh mô hình phân đoạn hơn được. Đánh giá lại ở P6 với ảnh camera thật (nếu IoU ảnh thật < 0,90 thì
  thêm mô hình phân đoạn làm lớp bổ trợ, P1 vẫn là nền).
- tract không nhận nút `If` của `torch.squeeze(dim)` (hai nhánh khác hạng) ⇒ `runtime.rs` thay bằng `Squeeze` lúc nạp,
  không sửa tệp mô hình (sha256 giữ nguyên với nguồn). Mô hình dùng external data bị từ chối (nằm ngoài vùng kiểm sha256).
- Gói cài nhúng `ocr-det`, `ocr-rec-vi`, `doc-orientation` (≈ 54 MB); `dewarp` tải thêm khi bật chế độ Sách.

## 4. Hệ quả và rủi ro

- AsterNet là mô hình mới (2026), ít người dùng, dữ liệu huấn luyện không công bố chi tiết. Giảm rủi ro: ghim sha256,
  CI đo CER mỗi PR; nếu CER tụt hoặc giấy phép đổi, thay bằng mô hình khác chỉ cần đổi `models/manifest.toml` +
  `ocr/rec.rs` (bảng ký tự). Ứng viên dự phòng: VietOCR (đã đo).
- tract chậm hơn onnxruntime với LSTM/attention (số đo tốc độ trong `CHANGELOG.md`); OCR chạy ở hàng đợi nền
  (`ARCHITECTURE.md` mục 3) nên chấp nhận được. Desktop có thể bật `ort` sau 1.0 (ADR-0002).
- Chữ viết tay, chữ dọc và ký tự ngoài bảng 227 lớp (ví dụ `²`, `€`) chưa được hỗ trợ.

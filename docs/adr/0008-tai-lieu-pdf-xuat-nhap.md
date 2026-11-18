# ADR-0008: Tầng tài liệu - ghi PDF tự làm trên pdf-writer, nhập PDF bằng hayro, nén G4 thay JBIG2

- **Trạng thái**: ĐÃ CHẤP NHẬN
- **Ngày**: 19/11/2026
- **Người quyết định**: agent-scan (P3, `docs/PLAN.md`)

## 1. Bối cảnh

P3 cần: PDF có lớp chữ OCR ẩn tìm được, PDF/A-2b, nén thông minh, mật khẩu AES-256; DOCX/XLSX/TXT/Markdown/ảnh;
nhập PDF/ảnh/HEIC; che thông tin xóa hẳn; chữ ký; QR/mã vạch; fuzz bộ đọc PDF. Ràng buộc từ ADR-0001/0002: lõi Rust
thuần (chạy Android, desktop, WASM), không C/C++ nặng, giấy phép tương thích AGPL, mọi dữ liệu xử lý trên máy.

## 2. Quyết định

| Việc | Chọn | Loại | Lý do |
| --- | --- | --- | --- |
| Ghi PDF | `pdf-writer` 0.15 (cú pháp mức thấp, MIT/Apache) + mã của crate | `lopdf`, `printpdf` | Cần tự kiểm soát từng chuỗi/luồng để mã hóa AES và khớp PDF/A; pdf-writer không tự nén/mã hóa nên không có gì "âm thầm" |
| Lớp chữ | Phông TrueType "không nét" tự sinh (2 glyph rỗng), Type0 Identity-H, CID = Unicode, ToUnicode đồng nhất, chế độ tô 3, giãn `Tz` theo bề rộng dòng | Nhúng phông Việt thật | Vài trăm byte thay vì vài MB; tìm kiếm/chép đúng dấu tiếng Việt; cách OCRmyPDF/Tesseract đã kiểm chứng |
| PDF/A-2b | XMP khớp Info, OutputIntent sRGB ICC v2 TỰ SINH từ số liệu IEC 61966-2-1, ID trailer, không mã hóa | Nhúng tệp ICC của bên thứ ba | Không phụ thuộc giấy phép tệp ICC |
| Mã hóa | Bộ xử lý chuẩn V5/R6 AES-256 (ISO 32000-2), tự cài trên RustCrypto `aes`/`cbc`/`sha2` | RC4/AES-128 (R2-R4) | R6 là mức duy nhất không dùng MD5/RC4; PDF/A + mật khẩu bị từ chối (chuẩn cấm) |
| Nén màu/xám | JPEG baseline xen kẽ `jpeg-encoder` (thuần Rust), bảng Huffman chuẩn, 4:2:0 cho màu; trang xám 1 kênh. Huffman tối ưu làm `jpeg-encoder` ghi lượt quét không xen kẽ mà bộ giải mã của hayro đọc sai màu (đo 19/11/2026) | MozJPEG (PLAN) | MozJPEG là C (mozjpeg-sys) ⇒ trái ADR-0001 cho WASM/Android build; chênh dung lượng ≈ 5-10% không đáng đổi |
| Nén đen trắng | CCITT G4 (`fax`), không mất dữ liệu | JBIG2 | Không có bộ mã hóa thuần Rust; chế độ ký hiệu có mất dữ liệu từng thay chữ số trên bản scan (Xerox 2013) - không chấp nhận được với hợp đồng/CCCD; chế độ chung không mất dữ liệu chỉ hơn G4 ≈ 20-30% |
| Tự nhận loại trang | Đếm điểm có màu (lệch kênh ≥ 24, > 0,2%) và điểm trung gian (< 1%) trên lưới mẫu | - | Rẻ, tất định, test được |
| Nhập PDF | `hayro` 0.9 (thuần Rust, crate gốc `forbid(unsafe_code)`, có giải mã RC4/AES), vẽ trang thành ảnh | `pdfium` (C++), `lopdf` (không vẽ được) | Một đường xử lý cho mọi trang (lọc, OCR, che, xuất lại); mọi thứ ẩn trong tệp gốc bị bỏ |
| HEIC/HEIF/AVIF | Lõi trả `NeedsPlatformDecoder`; app giải mã bằng API hệ điều hành rồi gọi `import::from_rgba` | `libheif` (C, LGPL) | HEVC vướng bằng sáng chế, không có bản thuần Rust đủ chín; mọi nền tảng đích đều có bộ giải mã sẵn |
| DOCX/XLSX | XML tự viết + ZIP tối giản tự viết (chỉ ghi) | crate `zip`, `rust_xlsxwriter`, `docx-rs` | Vài trăm dòng, không thêm bề mặt giải nén; tất định |
| Mã vạch | `rxing` 0.9 (cổng ZXing, Apache-2.0) chỉ bật bộ giải mã + QR/1D/DataMatrix/PDF417/Aztec | `rqrr` (chỉ QR) | Hóa đơn/giấy tờ Việt Nam dùng cả QR (VietQR), Code 128, PDF417 |
| Che thông tin | Tô đặc điểm ảnh + cắt chữ OCR theo ước lượng ký tự (nới 1 ký tự), dòng nghiêng > 5° bỏ cả dòng | Phủ hình chữ nhật trong PDF | Phủ hình để lại dữ liệu bên dưới - lỗi kinh điển làm lộ tài liệu |
| Chữ ký | Chữ ký HÌNH (tách nét mực → PNG trong suốt → đóng lên trang) | PAdES | Chữ ký số cần chứng thư số + TSA; xem xét sau 1.0 |

## 3. Hệ quả

- Nhập PDF vẽ thành ảnh ⇒ PDF "sinh số" (chữ máy) nhập vào mất lớp chữ gốc, cần OCR lại. Chấp nhận: app là máy
  quét; PDF gốc vẫn có thể giữ nguyên ở dạng tệp đính kèm (P7 quyết định).
- PDF tự mã hóa không nhập lại được nếu quên mật khẩu (đúng thiết kế R6).
- Bộ đọc PDF bên thứ ba (`hayro`) là bề mặt tấn công lớn nhất của P3: giới hạn dung lượng/trang/điểm ảnh, chặn panic
  tại biên, test đột biến + đích fuzz `pdf_import`. Rủi ro còn lại ghi ở `docs/security-reviews/20261119-p3-doc.md`.
- `hayro` kéo theo phông Foxit (BSD, từ PDFium) và CMap Adobe (BSD-3) để vẽ phông chuẩn không nhúng.

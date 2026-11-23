# Nhật ký thay đổi

Theo [Keep a Changelog](https://keepachangelog.com/vi/1.1.0/), phiên bản theo `.standards/docs/VERSIONING.md`.

## [Chưa phát hành]

### P7a (24/11/2026) - Tổ chức trang + thùng rác

- Thùng rác 30 ngày: "Xóa" tài liệu chuyển vào thùng rác (có Hoàn tác), khôi phục, xóa vĩnh viễn, dọn sạch; tự xóa hẳn
  tài liệu quá 30 ngày khi mở app. Tài liệu trong thùng rác không hiện trong danh sách và tìm kiếm.
- Thao tác trang: xoay trái/phải, đưa lên trước/ra sau, chuyển sang tài liệu khác, tách thành tài liệu mới từ một trang,
  ghép tài liệu khác vào. Trang chuyển đi được mã hóa lại bằng khóa của tài liệu đích.
- Kính lúp khi kéo góc trang để chỉnh chính xác.
- Kho lược đồ v2 (thêm cột `trashed_ms`, nâng cấp tự động từ v1).

### P6 (19/11/2026) - Quét bằng camera

- Màn chụp toàn màn hình (Android/iOS): xem trước có tứ giác dò mép thời gian thực trong lõi Rust (kênh Y, bỏ khung
  khi bận), tự chụp khi tài liệu đứng yên và chờ trang đổi mới chụp tiếp (chụp loạt), đèn tắt/tự động/luôn bật.
- Chế độ: Tài liệu, CCCD 2 mặt (nhắc mặt trước/sau), Danh thiếp, Bảng trắng, Ảnh (không cắt).
- Duyệt trang: đổi thứ tự, xóa, chỉnh 4 góc (kéo hoặc phím mũi tên), dò lại, giữ cả khung, 7 bộ lọc, xem kết quả;
  lưu thành tài liệu, mã hóa JPEG một lần, không EXIF/GPS.
- Nhập ảnh có sẵn cũng được dò mép + nắn + lọc qua cùng màn duyệt (mọi nền tảng).
- Cầu Rust: `detect_preview`, `CapturedImage` (giải mã một lần, xóa trắng khi giải phóng), `add_scanned_page`.
- Quyền tối thiểu: chỉ `CAMERA` (gỡ `RECORD_AUDIO`/`WRITE_EXTERNAL_STORAGE` plugin tự thêm); ảnh tạm của camera bị xóa
  ngay sau khi đọc; camera tắt khi app vào nền hoặc khóa.
- Chế độ Sách: tìm gáy và tách ảnh hai trang thành 2 trang (mỗi trang chỉnh góc riêng). Chế độ QR/mã vạch: đọc ngay
  trên khung xem trước hoặc trên ảnh nhập, hiện chữ thuần + sao chép (không tự mở liên kết). Nhớ chế độ quét lần trước.
- Chạy thật trên Galaxy M32 5G: dò mép xem trước nhanh hơn (ảnh làm việc 320 px, chỉ gửi 1/4 số hàng qua cầu, lấy khung
  camera theo nhịp thay vì nhận cả 30 khung/giây - trước đó app ăn > 300% CPU khi mở camera).
- Quyền camera bị từ chối: không còn tự hỏi lại liên tục; thêm nút "Mở Cài đặt".
- Thanh trạng thái trên màn chụp đổi sang biểu tượng sáng (trước đó chìm trên nền đen).

### Sửa lỗi

- Bản profile/release treo ở màn khởi động do `jni` 1.2.0 (đã bị tác giả rút lại) - hạ về 1.1.2; CI chặn gói Dart bị rút
  lại hoặc dính cảnh báo bảo mật.

### Sửa lỗi bảo mật

- Màn khóa app không che hộp thoại và route trên Navigator gốc (có từ P5): hộp thoại đang mở vẫn hiện và bấm được khi
  app khóa. Nay màn khóa phủ toàn bộ Navigator (`MaterialApp.builder`), có test hồi quy.

### P5 (19/11/2026) - Khung app Flutter

- App Flutter 6 đích (Android, iOS, macOS, Windows, Linux, web xem trước) trên lõi Rust qua `flutter_rust_bridge`
  2.13.0 + cargokit (`crates/tsuscan-ffi`, ADR-0010).
- Thư viện tài liệu: thư mục, nhãn, lọc, phân trang theo `DATA_TABLE.md`, nhập ảnh (bỏ EXIF/GPS), xem/xóa trang.
- Tìm không dấu trên tên + chữ OCR, tô sáng đúng chữ có dấu.
- Khóa app bằng PIN (Argon2id, chờ tăng dần) + sinh trắc học; khóa CSDL trong kho bí mật của hệ điều hành.
- i18n vi/en, giao diện Sáng/Tối/Hệ thống theo token TsuDEV, mặc định theo hệ thống (PRODUCT_POLICY mục 1).
- Kho: `list_documents_page` + `count_documents` (phân trang tại CSDL, không đổi định dạng).

### Số đo P4 (19/11/2026, kho + mật mã `tsuscan-vault`)

| Chỉ tiêu | Đo được |
| --- | --- |
| `.so` arm64 (NDK 28.2, `-Os`, LTO): SQLite thường / **SQLite3MC** / SQLCipher+OpenSSL | 1,50 MB / **1,37 MB** / 5,48 MB (ADR-0009) |
| Đối chiếu độc lập vector `conformance/vault-v1.json` | libsodium 1.0.18 (hệ điều hành) giải đúng 5 phong bì + 3 luồng, Argon2id khớp; HKDF khớp `hashlib`; BIP39 khớp vector Trezor |
| Rò bản rõ ra đĩa (CSDL, WAL, tệp trang) | 0 (quét mọi tệp trong thư mục kho) |
| Đột biến phong bì/luồng (3.300) + chuỗi ngẫu nhiên (5.000) | 0 chấp nhận nhầm, 0 panic |
| Độ phủ dòng `tsuscan-vault` | 93,02% |

### Số đo P3 (19/11/2026, x86_64 4 nhân, máy dùng chung tải 7-9)

| Chỉ tiêu | Đo được |
| --- | --- |
| OCR thật → PDF lớp chữ → `pdftotext` (trang sinh tất định) | CER so chữ chuẩn **0,66%**; lớp chữ so chữ OCR **0,00%** (không mất/đổi ký tự) |
| Trang A4 300 DPI đen trắng (CCITT G4) | 19,4 KB/trang; vẽ lại khớp từng điểm ảnh |
| Trang A4 150 DPI màu (JPEG q80) | 92 KB/trang |
| Đối chiếu độc lập | qpdf (libqpdf 12.4): cú pháp OK, R6/V5 AESV3 256 bit, mở được bằng mật khẩu người dùng và chủ, từ chối mật khẩu sai; pikepdf: PDF/A `2B`; poppler: phông nhúng + ToUnicode, chữ Việt có dấu đúng; python-docx/openpyxl đọc đúng DOCX/XLSX |
| Bom giải nén (PDF 1 MB nở 1 GiB) | Trước P3: hayro chiếm 1,07 GB. Sau lớp chặn: từ chối sau 0,04-0,13 s, RSS đỉnh tăng < 0,2 MiB |
| 300 PDF đột biến tất định | 0 panic thoát ra, 182 vẫn mở được |

### Số đo P2 (18/11/2026, x86_64 4 nhân, máy dùng chung tải 6-13)

| Chỉ tiêu | Ngưỡng | Đo được |
| --- | --- | --- |
| CER OCR tiếng Việt in, toàn trang (`ocr_cer`, 12 trang) | ≤ 3% | **0,51%** (82/16.006 ký tự; WER 2,63%; Tinos 0,56%, Tinos Bold 0,68%, Arimo, Be Vietnam Pro 0,46%) |
| Hướng trang đúng (6 trang × 4 góc) | ≥ 95% | 24/24 |
| CER trang cong 45 px trước → sau làm phẳng | giảm, ≤ 5% | 33,77% → 0,46% |
| Thời gian OCR toàn trang (runner CI, gồm biên dịch xô) | - | ≈ 4,1 s/trang; độ phủ `tsuscan-ml` 94,16% dòng |
| Thời gian OCR (CPU luồng, tract, máy dev) | - | dò dòng ≈ 1,6-2,2 s/trang (cạnh 1.280); nhận dạng ≈ 0,11 s (rộng 256) - 0,64 s (rộng 1.024) mỗi dòng sau khi biên dịch xô |
| Dò mép P1 `detect` (CPU luồng, `diag -- 0 50`, tải 2,9) | < 40 ms | **31,6 ms/khung** - chỉ tiêu P1 đã chứng minh |
| Thời gian nhận dạng hình dạng ký hiệu (đã bỏ) | - | 0,32 s (256) - 1,35 s (1.024) mỗi dòng ⇒ chuyển sang xô chiều rộng 128 px |

### Thêm
- Tầng tài liệu `crates/tsuscan-doc` (P3, ADR-0008):
  - Ghi PDF: ảnh trang nén thông minh (tự nhận màu/xám/đen trắng → JPEG màu, JPEG 1 kênh, CCITT G4 không mất dữ
    liệu; trần DPI theo mức chất lượng), lớp chữ OCR ẩn tìm/chép được (phông "không nét" tự sinh, Identity-H +
    ToUnicode, giãn theo bề rộng dòng, xoay theo dòng nghiêng).
  - PDF/A-2b (XMP khớp Info, OutputIntent sRGB ICC tự sinh, ID trailer) và PDF mật khẩu AES-256 (R6, quyền in/sửa/
    chép/chú thích, mật khẩu chủ); hai tùy chọn loại trừ nhau theo chuẩn.
  - Xuất DOCX (tiêu đề, đoạn, bảng có viền, ảnh trang khi chưa OCR), XLSX (mỗi bảng một trang tính, số kiểu Việt Nam
    thành số, giữ số 0 đầu của CCCD/điện thoại), TXT, Markdown, JPEG/PNG/WebP (không EXIF).
  - Nhập PDF (hayro, vẽ trang thành ảnh, mật khẩu, chọn trang), JPEG/PNG/WebP; HEIC/HEIF/AVIF qua bộ giải mã hệ điều
    hành (`import::from_rgba`).
  - Chặn bom giải nén hai lớp + trần kích thước ảnh/trang/số trang, `catch_unwind` tại biên bộ đọc bên thứ ba.
  - Che thông tin xóa hẳn (ghi đè điểm ảnh + cắt chữ OCR, tìm theo chuỗi), chữ ký hình (tách nét mực, đóng lên trang),
    đọc QR/mã vạch (QR/VietQR, Code 128, EAN, DataMatrix, PDF417, Aztec).
  - CLI: `pdf` (`--ocr --pdfa --password-env -q -c`), `export` (.docx .xlsx .txt .md), `import`, `redact`, `barcode`.
  - Test đối chiếu bằng hayro + poppler + qpdf (`TSUSCAN_REQUIRE_PDF_TOOLS=1` trên CI), test đột biến, đích fuzz
    `pdf_import`/`barcode`, phiếu rà soát `docs/security-reviews/20261119-p3-doc.md`; CI: độ phủ `tsuscan-doc` ≥ 85%,
    job AI chạy thêm OCR → PDF → pdftotext.
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

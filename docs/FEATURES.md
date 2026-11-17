# MA TRẬN TÍNH NĂNG - Tsu Scan

Chọn lọc từ 10 app tham chiếu (chỉ học **tính năng và cách làm**, không chép mã, giao diện, tên hay tài sản của họ):
AS Adobe Scan · ML Microsoft Lens · CS CamScanner · AN Apple Notes · GS Genius Scan · GD Google Drive ·
SS Samsung Notes · SB Scanbot · CS2 Clear Scan · VF vFlat.

Cột "Phase" là phase hiện thực. "+" = Tsu Scan làm hơn app tham chiếu.

## 1. Chụp

| Tính năng | Tham chiếu | Phase | Ghi chú |
| --- | --- | --- | --- |
| Dò mép thời gian thực, khung tứ giác bám theo | AS ML CS AN GS GD SB | P1, P6 | Cổ điển (P1) + mô hình phân đoạn (P2) khi ảnh khó |
| Tự chụp khi khung ổn định + gợi ý ("lại gần", "giữ yên") | AS GS SB VF | P6 | Đọc gợi ý cho trình đọc màn hình |
| Chụp loạt nhiều trang liên tục | Tất cả | P6 | |
| Chế độ Sách: tách 2 trang, làm phẳng trang cong, xóa ngón tay | VF CS | P2, P6 | Chụp sách bằng nút âm lượng hoặc tự chụp khi lật trang |
| Chế độ CCCD/thẻ: 2 mặt gộp 1 trang A4 đúng tỉ lệ | CS SB GS | P6 | Che số CCCD tùy chọn khi xuất |
| Chế độ Danh thiếp → tạo danh bạ (vCard) | ML | P7 | |
| Chế độ Bảng trắng: khử lóa, tăng nét chữ | ML | P1 | |
| Đọc QR/mã vạch | SB GS | P3, P6 | Không tự mở liên kết, hiện cảnh báo trước |
| Nhập ảnh/PDF có sẵn để xử lý | Tất cả | P3, P6 | |
| Máy scan thật (phẳng, ADF, 2 mặt) qua eSCL/SANE/WIA/TWAIN/ICA | + | P10 | eSCL chạy cả Android |
| Webcam, thư mục theo dõi trên máy tính | + | P10 | |

## 2. Xử lý ảnh

| Tính năng | Tham chiếu | Phase |
| --- | --- | --- |
| Nắn phối cảnh, chỉnh góc tay có kính lúp | Tất cả | P1, P7 |
| Bộ lọc Tự động / Màu / Xám / Đen trắng / Khử bóng / Ảnh gốc | AS GS CS SB | P1 |
| Tự xoay theo hướng chữ | AS GS | P2 |
| Khử bóng tay, khử lóa, cân trắng | AS CS2 VF | P1 |
| Làm phẳng trang cong (dewarp) | VF | P2 |

## 3. OCR + hiểu tài liệu

| Tính năng | Tham chiếu | Phase |
| --- | --- | --- |
| OCR trên thiết bị tiếng Việt + Anh | Tất cả (phần lớn chạy trên máy chủ) | P2 |
| PDF tìm kiếm được (lớp chữ ẩn) | AS GS ML | P3 |
| Chép chữ, sửa chữ OCR | ML CS AN | P7 |
| Tìm kiếm toàn văn không dấu trên mọi tài liệu | GD AN GS | P4, P5 |
| Bảng → XLSX, văn bản → DOCX | ML CS | P3 |
| Đặt tên thông minh (ngày, loại giấy tờ, từ khóa) | GS SB | P7 |

## 4. Sửa, ký, bảo mật

| Tính năng | Tham chiếu | Phase |
| --- | --- | --- |
| Sắp xếp/chèn/xóa/xoay trang, ghép/tách tài liệu | Tất cả | P7 |
| Chú thích, bút đánh dấu, văn bản | AS CS SS | P7 |
| Chữ ký tay lưu sẵn (mã hóa) | AS CS GS | P3, P7 |
| Che thông tin **xóa hẳn điểm ảnh và chữ OCR** bên dưới | + | P3, P7 |
| PDF đặt mật khẩu AES-256 | CS GS | P3 |
| Khóa app PIN/sinh trắc, chặn chụp màn hình tùy chọn | GS | P5 |
| Toàn bộ dữ liệu trên máy mã hóa | + | P4 |

## 5. Tổ chức, đồng bộ, chia sẻ

| Tính năng | Tham chiếu | Phase |
| --- | --- | --- |
| Thư mục, nhãn, sắp xếp, lọc, thùng rác 30 ngày | Tất cả | P5, P7 |
| Đồng bộ đa thiết bị | AS CS GS GD SS | P8, P9 (E2EE, +) |
| Chia sẻ liên kết có hạn, khóa nằm trong `#fragment` | AS CS | P9 (+) |
| Xuất PDF, PDF/A, JPEG, PNG, WebP, DOCX, XLSX, TXT, Markdown | Tất cả | P3 |
| Chia sẻ qua hệ điều hành, in | Tất cả | P6, P10 |

## 6. Không làm (có lý do)

| Tính năng | Lý do |
| --- | --- |
| OCR/AI trên đám mây | Trái nguyên tắc E2EE, tốn phí |
| Quảng cáo, gói trả phí, hình mờ | Sản phẩm 0 đồng, mã nguồn mở |
| Dịch máy trực tuyến | Gửi nội dung ra ngoài; có thể xét dịch trên máy sau 1.0 |
| Thư viện độc quyền (ML Kit, Firebase) | F-Droid từ chối, rò dữ liệu đo lường |

# ADR-0005: Hỗ trợ máy scan thật trên máy tính (và eSCL trên Android)

- **Trạng thái**: ĐÃ CHẤP NHẬN
- **Ngày**: 18/11/2026
- **Người quyết định**: agent-scan (chủ dự án ủy quyền tự chọn 18/11/2026)

## 1. Bối cảnh
Máy tính văn phòng thường nối máy scan phẳng/ADF hoặc máy in đa năng. App tham chiếu (trừ phần mềm hãng) không điều
khiển được máy scan thật; đây là điểm Tsu Scan làm hơn.

## 2. Quyết định
Crate `tsuscan-scanner` với một giao diện chung (liệt kê thiết bị, khả năng, quét trang/ADF/2 mặt, DPI, màu) và các
backend, làm theo thứ tự giá trị/rủi ro:
1. **eSCL/AirScan** (HTTP + mDNS, Rust thuần): chạy mọi OS kể cả Android; phần lớn máy in đa năng từ 2015 hỗ trợ.
2. **SANE** (Linux, macOS qua Homebrew): nạp `libsane` động lúc chạy, không bắt buộc cài.
3. **WIA 2.0** (Windows): crate `windows`.
4. **TWAIN 2.x 64-bit** (Windows, macOS): qua DSM chuẩn cho máy chỉ có driver TWAIN.
5. **ICA** (macOS): plugin Swift qua kênh nền tảng Flutter.
Ảnh từ máy scan đi qua cùng đường xử lý P1-P3 (bỏ bước dò mép nếu thiết bị đã cắt).

## 3. Hệ quả
P10 dài nhất; nếu vượt ngữ cảnh một phiên thì tách P10a (bố cục desktop + eSCL + SANE) và P10b (WIA + TWAIN + ICA).
Phản hồi eSCL là dữ liệu không tin cậy ⇒ fuzz bộ đọc XML.

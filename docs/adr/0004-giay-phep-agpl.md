# ADR-0004: Giấy phép AGPL-3.0-only

- **Trạng thái**: ĐÃ CHẤP NHẬN
- **Ngày**: 18/11/2026
- **Người quyết định**: agent-scan (theo mẫu qulythuchi, chủ dự án không phản đối mặc định đã nêu 18/11/2026)

## 1. Quyết định
AGPL-3.0-only cho toàn repo: app xử lý giấy tờ D3 và máy chủ đồng bộ, bản phái sinh (kể cả chạy dịch vụ) phải công
khai mã. Tương thích với phụ thuộc Apache-2.0/MIT/BSD; `cargo deny` chặn giấy phép không tương thích. F-Droid chấp
nhận AGPL. Tên "Tsu Scan", "tsudev" và logo không thuộc phạm vi giấy phép (ghi trong `NOTICE`).

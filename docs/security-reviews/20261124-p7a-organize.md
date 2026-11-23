# PHIẾU RÀ SOÁT BẢO MẬT - P7a Tổ chức trang + thùng rác

- **PR / nhánh**: `feat/p7a-organize`
- **Người thực hiện**: agent-scan (phiên P7a) - **Người rà soát**: chủ dự án (khi duyệt PR)
- **Ngày**: 24/11/2026

## 1. Thay đổi này đụng vào đâu

- [ ] Xác thực hoặc quản lý phiên
- [x] Phân quyền hoặc kiểm tra quyền sở hữu đối tượng - *chuyển trang giữa tài liệu: mỗi tài liệu có DK riêng*
- [ ] Tải tệp lên hoặc phục vụ tệp - *xoay trang giải mã ảnh trong kho bằng `tsuscan_imaging::decode` có giới hạn (P1)*
- [x] Truy vấn cơ sở dữ liệu có dữ liệu do người dùng nhập - *id trang/tài liệu, thứ tự trang; mọi câu SQL tham số hóa*
- [ ] Hiển thị nội dung do người dùng tạo ra HTML
- [ ] Gọi ra dịch vụ ngoài
- [x] Xử lý dữ liệu cá nhân (D2) hoặc bí mật (D3) - *ảnh giấy tờ, chữ OCR*
- [x] Mật mã - *mã hóa lại tệp trang bằng FK mới bọc DK đích khi chuyển trang*

## 2. Phân loại dữ liệu

| Dữ liệu | Mức | Ở đâu |
| --- | --- | --- |
| Ảnh trang, chữ OCR | D3 | Tệp `.tsf` mã hóa luồng (FK/DK/MK) + CSDL SQLite3MC |
| `trashed_ms` (lúc bỏ vào thùng rác) | D1 | Cột mới trong `documents` (CSDL đã mã hóa) |

## 3. Đối chiếu OWASP MASVS

- **MASVS-STORAGE-1/2**: tài liệu trong thùng rác vẫn mã hóa như cũ; xóa vĩnh viễn/tự dọn đi qua `delete_document` P4
  (xóa tệp, chữ OCR, chỉ mục, `secure_delete` của SQLite). Tài liệu trong thùng rác KHÔNG có dòng chỉ mục FTS (không lộ
  qua tìm kiếm). Lược đồ v2 chỉ thêm bước (`ALTER TABLE ... ADD COLUMN`), v1 bất biến.
- **MASVS-CRYPTO-1/2**: chuyển trang không bao giờ bọc lại FK cũ bằng DK đích - luôn giải mã rồi mã hóa lại bằng FK MỚI,
  id tệp mới (ngữ cảnh AAD `context::file(id)` mới); tệp + dòng `files` cũ bị xóa. Lý do: khi đồng bộ (P9) chia sẻ từng
  tài liệu, DK của tài liệu nguồn không được mở được trang đã chuyển đi.
- **MASVS-PRIVACY**: không thêm quyền, không gọi mạng.

## 4. Ca tấn công đã cân nhắc

| Ca | Xử lý |
| --- | --- |
| Tìm kiếm lộ nội dung tài liệu đã "xóa" (thùng rác) | `reindex` bỏ dòng khi `trashed_ms IS NOT NULL`; truy vấn tìm kiếm lọc thêm `trashed_ms IS NULL`; test `trash_hides_restores_and_purges_after_30_days` |
| Thùng rác giữ dữ liệu mãi (người dùng tưởng đã xóa) | Hộp xác nhận ghi rõ 30 ngày; tự dọn khi mở kho (`purge_trash(expired_only)`), lỗi dọn không chặn mở kho và thử lại lần sau; "Xóa vĩnh viễn"/"Dọn sạch" xóa ngay |
| Lỗi giữa chừng khi chuyển trang để lại tệp mồ côi hoặc trang trỏ tệp không tồn tại | Ghi mọi tệp mới trước, rồi một giao dịch CSDL; lỗi ⇒ xóa tệp mới, CSDL không đổi; chỉ xóa tệp cũ SAU khi commit |
| `order`/`page_ids` giả mạo (lặp, thiếu, trang của tài liệu khác) | `reorder_pages` đòi hoán vị đúng; `move_pages` từ chối rỗng/lặp/cùng tài liệu đích; id không tồn tại ⇒ `NotFound` |
| Ảnh trong kho bị sửa làm hỏng bộ giải mã khi xoay | Tệp có xác thực (AEAD luồng) ⇒ sửa tệp = lỗi giải mã trước khi tới bộ giải mã ảnh; `decode` P1 có giới hạn kích thước |

## 5. Kiểm tra bắt buộc

- [x] Đầu vào được kiểm tra tại biên (id, thứ tự, số lần xoay lấy mod 4).
- [x] Đầu ra được mã hóa đúng ngữ cảnh (không HTML).
- [ ] Quyền phía máy chủ - *không áp dụng*.
- [ ] Giới hạn tần suất - *không áp dụng*.
- [x] Giới hạn kích thước (ảnh trang ≤ 200 MiB khi ghi, giải mã có giới hạn P1).
- [x] Không secret trong mã, log, thông báo lỗi (lỗi dọn thùng rác chỉ log kiểu lỗi).
- [x] Thông báo lỗi không lộ chi tiết nội bộ.
- [x] `gitleaks` chạy trong CI.
- [x] Không thêm phụ thuộc mới.

## 6. Rủi ro còn lại

- Xoay trang mã hóa lại JPEG (q90) ⇒ mất chất lượng nhẹ mỗi lần xoay; chấp nhận được (P7b có thể lưu góc xoay thay vì
  ghi lại ảnh nếu cần).
- Hộp chữ OCR (`ocr_blob`) bị bỏ khi xoay; chữ thuần giữ nguyên. P7c chạy lại OCR.
- `trashed_ms` chưa đồng bộ giữa thiết bị (P9 định nghĩa hợp nhất theo HLC).

## 7. Kết luận của người rà soát

- [ ] Đạt - được merge
- [ ] Đạt có điều kiện
- [ ] Không đạt - lý do:

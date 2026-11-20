# 20261120-GP1 - tsuscan: đưa TsuScan lên Google Play Console (phần B của phiếu toàn hệ)

- **Người lập**: phiên điều phối `tsudev-62`, 20/11/2026.
- **Người làm**: phiên tsuscan. **Trạng thái**: MỞ.
- **Phiếu mẹ**: `~/projects/logs/handover/20261120-GP1_google-play-console-toan-he.md` - làm theo **mục 4 (checklist
  mỗi app)**; mục 2 (phần A dùng chung) ĐÃ XONG ở phiên tsudev, đừng làm lại.
- **Công cụ trình duyệt**: `~/projects/logs/tools/browser/README.md` (`tsb`, hồ sơ `play` = Edge `tsudev-play`, đã đăng
  nhập Play Console). Dùng tsb, KHÔNG viết script Playwright thô không rào chắn.

## 1. Hiện trạng (đo 20/11/2026)

| Package | Trên Play Console | Pro/ủng hộ qua Play |
|---|---|---|
| `com.tsudev.tsuscan` | ❌ chưa tạo | chưa |

## 2. Việc riêng của repo này

1. Quyền **camera/ảnh**: khai lý do; dùng Photo Picker thay quyền đọc toàn bộ ảnh nếu được (chính sách Google về `READ_MEDIA_IMAGES`) - kiểm manifest trước khi khai.
2. Tạo app mới (GP1 mục 4 bước 1) rồi đủ "Thiết lập ứng dụng".
3. Repo đang ở nhánh `feat/p6-camera` và có nhiều khoá trong `logs/LOCKS.md` - làm Play Console SAU khi nhánh camera ổn định (quyền khai phải khớp bản AAB nộp).

## 3. Ranh giới

- Nút gửi duyệt / xuất bản / thanh toán / xoá app: **👤 bấm** (tsb chặn sẵn) - agent điền rồi dừng, chụp màn hình.
- Danh sách ≥ 12 người thử kín, hồ sơ thanh toán, mật khẩu tài khoản thử cho người duyệt: 👤.
- Không tạo khoá tài khoản dịch vụ mới; `tsudev-play-verify` đã có quyền mức tài khoản (phủ app mới).
- Bằng chứng: ảnh "Tổng quan về việc xuất bản" vào `logs/trust/`, ghi `logs/trust/TRUST.md`.

## 4. Xong khi

GP1 mục 4 bước 1-7 đạt cho app này; ghi một dòng vào mục 5 phiếu này VÀ mục 6 phiếu mẹ.

## 5. Kết quả xử lý (mỗi phiên ghi một dòng)


## Bổ sung 20/11/2026 (tsudev điều phối) - điền biểu mẫu đứng tên chủ dự án

Theo tsudev-standards v4.2.0 `STORE_SUBMISSION.md` mục 0.1 (PR tsudev-standards #65): mọi trường Play Console điền như
chủ dự án tự điền, KHÔNG chọn/ghi "do AI thực hiện". Câu hỏi về tính năng thật của app (có tính năng AI tạo sinh
không...) trả lời đúng mã, ghi tệp mã làm nguồn trong `docs/store/google-play.md`.

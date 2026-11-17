# ADR-0003: Đồng bộ mã hóa đầu-cuối, đăng nhập tsudev qua máy chủ

- **Trạng thái**: ĐÃ CHẤP NHẬN
- **Ngày**: 18/11/2026
- **Người quyết định**: agent-scan (chủ dự án chốt E2EE 18/11/2026)

## 1. Bối cảnh
Đồng bộ tài liệu giữa điện thoại và máy tính cùng tài khoản tsudev; máy chủ không được đọc nội dung.

## 2. Quyết định
- **Đăng nhập**: dùng lại mẫu đã chạy ở sound-db/qulythuchi (ADR-0003 của qulythuchi): Worker là RP OIDC (client
  `tsuscan`), cấp mã một lần + PKCE S256 cho app qua trình duyệt hệ thống; access JWT 15 phút + refresh xoay vòng
  phát hiện dùng lại; thu hồi theo thiết bị; ghim CA.
- **Khóa**: mật khẩu mã hóa (riêng với mật khẩu tsudev) → Argon2id → KEK bọc khóa chính MK; 24 từ khôi phục bọc MK
  lần nữa. Mỗi tài liệu một khóa DK bọc bằng MK; mỗi tệp trang một khóa ngẫu nhiên bọc bằng DK.
- **Định dạng**: bản ghi niêm phong XChaCha20-Poly1305 + AAD (mã bản ghi, loại, phiên bản); tệp dạng luồng chia khối
  64 KiB (kiểu secretstream), đệm kích thước để giấu số trang. Vector `conformance/` để Worker và lõi Rust đối chiếu.
- **Đồng bộ**: bản ghi theo HLC, ghi sau thắng theo trường; tệp bất biến theo băm nội dung (đã mã hóa).
- **Chia sẻ liên kết**: khóa DK nằm ở `#fragment` (không gửi lên máy chủ), có hạn và thu hồi được.

## 3. Hệ quả
Quên cả mật khẩu mã hóa và 24 từ ⇒ mất dữ liệu trên máy chủ (dữ liệu trên máy vẫn còn); app cảnh báo rõ khi thiết lập.
Chủ dự án đăng ký client OIDC `tsuscan` ở P8.

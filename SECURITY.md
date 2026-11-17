# Chính sách bảo mật - Tsu Scan

## Báo lỗ hổng

**KHÔNG mở issue công khai.** Dùng [Báo cáo lỗ hổng riêng tư của GitHub](https://github.com/tsudev-org/tsuscan/security/advisories/new).
Gửi kèm: thành phần (app/máy chủ/web), phiên bản, cách tái hiện, tác động. Phản hồi trong 7 ngày, bản sửa lỗi
nghiêm trọng trong 30 ngày. Quy trình chung: [`.standards/SECURITY.md`](.standards/SECURITY.md).

## Cam kết bảo mật (dữ liệu mức D3)

- **Xử lý trên thiết bị**: dò mép, AI, OCR chạy trên máy; ảnh và chữ không gửi ra mạng ([ADR-0002](docs/adr/0002-xu-ly-tren-thiet-bi-tract-onnx.md)).
- **Mã hóa khi lưu**: CSDL và tệp trang trên máy đều mã hóa; khóa trong kho bí mật của hệ điều hành.
- **Mã hóa đầu-cuối khi đồng bộ**: máy chủ chỉ giữ bản mã ([ADR-0003](docs/adr/0003-e2ee-dang-nhap-tsudev.md)).
- **Mã an toàn bộ nhớ**: bộ đọc ảnh/PDF/dữ liệu máy scan viết bằng Rust, kiểm thử fuzz.

## Kiểm chứng bản tải về

Mỗi bản phát hành kèm `SHA256SUMS` và chứng thực nguồn gốc bản build:

```bash
gh attestation verify <tệp> --repo tsudev-org/tsuscan
```

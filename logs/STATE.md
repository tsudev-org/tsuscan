# STATE.md - Trạng thái project Tsu Scan

> Agent đọc file này ở đầu mỗi phiên và cập nhật ở cuối mỗi phiên.
> Quy trình: `.standards/docs/AGENT_PROTOCOL.md`. Kế hoạch tổng: `docs/PLAN.md`.

## Hàng đợi task (làm từ trên xuống)

- [ ] **P1 - Lõi ảnh `crates/tsuscan-imaging`** (phiếu `logs/handover/20261118-01_p0-xong-bat-dau-p1.md`)
- [ ] P2 - AI trên máy + OCR
- [ ] P3 - Tài liệu + xuất
- [ ] P4 - Kho dữ liệu + mật mã
- [ ] P5 - Khung app Flutter
- [ ] P6 - Quét bằng camera (Android)
- [ ] P7 - Công cụ sửa + tổ chức
- [ ] P8 - Máy chủ sync-api (cần 👤 token Cloudflare, client OIDC)
- [ ] P9 - Đồng bộ trong app
- [ ] P10 - Bản máy tính + máy scan thật
- [ ] P11 - Web/PWA + website
- [ ] P12 - Phát hành + kênh cài (cần 👤 khóa ký Android)
- [ ] P13 - Kiểm định bảo mật + 1.0
- [ ] P14 - Kiểm thử thật đầu-cuối qua adb + Edge (phiên mới)

## Đang thực hiện

| Task | Agent | Bắt đầu |
| --- | --- | --- |
| (trống) | | |

## Đã hoàn thành (mới nhất trên cùng)

- 18/11/2026 - P0 Nền móng (agent-scan): bộ quy ước v3.7.0, `docs/PLAN.md` (15 phase), `docs/ARCHITECTURE.md`,
  `docs/FEATURES.md` (chọn lọc từ 10 app tham chiếu), ADR 0001-0006, LICENSE AGPL-3.0, README/SECURITY/NOTICE/CHANGELOG,
  CI (quy ước + gitleaks), Dependabot.

## Quyết định quan trọng

- 18/11/2026 - Chủ dự án chốt: repo `tsudev-org/tsuscan` Public, E2EE + xử lý trên máy, máy scan thật (agent chọn: đầy
  đủ), iOS hoãn dùng PWA, vi mặc định + en, tên miền `tsuscan.tsudev.com`, mã app `com.tsudev.tsuscan` (`docs/PLAN.md`).
- 18/11/2026 - Lõi Rust + Flutter + Workers: `docs/adr/0001-*`
- 18/11/2026 - Xử lý trên thiết bị, ONNX bằng tract: `docs/adr/0002-*`
- 18/11/2026 - E2EE + đăng nhập tsudev qua máy chủ: `docs/adr/0003-*`
- 18/11/2026 - AGPL-3.0-only: `docs/adr/0004-*`
- 18/11/2026 - Máy scan thật eSCL/SANE/WIA/TWAIN/ICA: `docs/adr/0005-*`
- 18/11/2026 - i18n vi/en từ đầu: `docs/adr/0006-*`

## Sự cố bảo mật

> Ghi theo `.standards/docs/SECURITY_BASELINE.md` mục 9. Để trống nếu chưa có.

- (chưa có)

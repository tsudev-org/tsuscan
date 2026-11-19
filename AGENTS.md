# AGENTS.md - tsuscan (Tsu Scan)

> **ĐỌC FILE NÀY ĐẦU TIÊN trong mọi phiên làm việc mới.**

## Phần A - Quy ước chung của hệ sinh thái (KHÔNG SỬA Ở ĐÂY)

Toàn bộ quy ước chung nằm trong bản sao chỉ-đọc tại
[`.standards/AGENTS.md`](.standards/AGENTS.md). `MUST` đọc file đó trước.

Bản quy ước repo này đang dùng: xem [`.standards-version`](.standards-version).

| Cần gì | Đọc file nào |
| --- | --- |
| Điểm vào, nguyên tắc chung | `.standards/AGENTS.md` |
| Quy trình phiên, khóa file, bàn giao | `.standards/docs/AGENT_PROTOCOL.md` |
| Bảo mật bắt buộc | `.standards/docs/SECURITY_BASELINE.md` |
| Quy tắc `.gitignore` | `.standards/docs/GITIGNORE_POLICY.md` |
| Nhánh, commit, PR, phát hành | `.standards/docs/GIT_WORKFLOW.md` |
| Đăng nhập, đăng ký, xác minh tài khoản | `.standards/docs/AUTH_AND_ACCOUNT.md` |
| Giao diện và token | `.standards/docs/DESIGN_SYSTEM.md` |
| Bảng bản ghi, bộ chọn số bản ghi | `.standards/docs/DATA_TABLE.md` |
| Logo, favicon, icon ứng dụng | `.standards/docs/BRAND_ASSETS.md` |
| tsudev.com, ảnh đại diện, trang hồ sơ | `.standards/docs/ECOSYSTEM_IDENTITY.md` |
| Cấu trúc thư mục | `.standards/docs/PROJECT_STRUCTURE.md` |
| Chọn ngôn ngữ, framework | `.standards/docs/LANGUAGE_SELECTION.md` |
| Hạ tầng 0 đồng | `.standards/docs/FREE_TIER_STACK.md` |
| Trình soạn thảo nội dung | `.standards/docs/RICH_TEXT_EDITOR.md` |
| Tìm kiếm và lọc tiếng Việt | `.standards/docs/SEARCH_AND_FILTER.md` |
| Kiểm thử và chất lượng mã | `.standards/docs/TESTING_QUALITY.md` |
| Khả năng truy cập | `.standards/docs/ACCESSIBILITY.md` |

`MUST NOT` sửa bất kỳ file nào trong `.standards/`. Cần đổi quy ước thì mở đề
xuất tại repo `tsudev-standards` theo `.standards/docs/SYNC.md` mục 1.

## Phần B - Riêng của repo này

### B.1. Repo này là gì

- **Loại**: app di động + desktop + web app + website + dịch vụ (monorepo, hình trạng B của `PROJECT_STRUCTURE.md`)
- **Sản phẩm**: Tsu Scan - quét tài liệu xử lý trên thiết bị, OCR tiếng Việt, đồng bộ E2EE. Repo `tsudev-org/tsuscan`.
- **Stack**: Rust (`crates/*`, lõi dùng chung), Flutter (`apps/app`, 6 đích) qua `flutter_rust_bridge`, TypeScript +
  Cloudflare Workers + Hono + D1 + R2 (`services/sync-api`), Astro (`apps/site`). Lý do: `docs/adr/0001-*`.
- **Hạng xác thực**: B - app có kết nối; dùng ngoại tuyến không cần tài khoản; IdP tsudev.com qua máy chủ (ADR-0003).
- **Mức phân loại dữ liệu cao nhất**: **D3** (giấy tờ tùy thân, hợp đồng, hồ sơ) - mọi thay đổi đụng mã hóa/đồng
  bộ/xác thực/bộ đọc dữ liệu không tin cậy `MUST` có phiếu rà soát theo `.standards/docs/templates/SECURITY_REVIEW.md`.
- **Người liên hệ khi có sự cố**: chủ dự án tsudev (GitHub `tsudev-tsudev`).
- **Agent chính**: agent-scan. Kế hoạch theo phase: `docs/PLAN.md`. Tính năng: `docs/FEATURES.md`.

### B.2. Chạy trong 5 phút

```bash
./scripts/check-standards.sh
cargo test --workspace                                # lõi Rust (từ P1)
export PATH="$HOME/tools/flutter/bin:$PATH"           # Flutter 3.47.8 (từ P5)
cd apps/app && flutter test && flutter run -d linux
```

### B.3. Ranh giới sở hữu file

| Vùng | Chủ sở hữu | Ghi chú |
| --- | --- | --- |
| `crates/tsuscan-imaging`, `crates/tsuscan-ml` | agent-scan (ảnh/AI) | Đổi thuật toán ⇒ chạy lại bộ đo `testdata/`, không hạ ngưỡng |
| `crates/tsuscan-doc` | agent-scan (tài liệu) | |
| `crates/tsuscan-vault` | agent-scan (mật mã) | Đổi định dạng phong bì ⇒ tăng phiên bản + sinh lại `conformance/` CÙNG PR |
| `crates/tsuscan-scanner` | agent-scan (phần cứng) | |
| `apps/app` | agent-scan (app) | |
| `services/sync-api` | agent-scan (máy chủ) | Migration D1 đã áp dụng là BẤT BIẾN |
| `apps/site` | agent-scan (web) | |
| `docs/adr` | agent-scan | ADR đã chấp nhận chỉ được thay bằng ADR mới |

### B.4. Điểm lệch chuẩn đã biết

| Lệch ở đâu | Vì sao | Kế hoạch |
| --- | --- | --- |
| Tên repo `tsuscan` thay vì `tsudev-scan` (`ECOSYSTEM_IDENTITY.md` mục 3) | Theo cách đặt tên mới của hệ (tsufiles, tsucontact, tsuswico), chủ dự án chốt 18/11/2026 | Đề xuất cập nhật quy ước ở tsudev-standards |
| Logo: dùng logo tsudev chung | Chưa có logo riêng được duyệt (`BRAND_ASSETS.md` mục 15) | Có logo duyệt ⇒ thay `assets/brand/source/` |
| iOS phát triển song song, chưa phát hành | Chủ dự án xác nhận 19/11/2026 (khớp `PRODUCT_POLICY.md` v3.8.1): mọi phase làm đích iOS/macOS cùng Android/Windows để Mac/iPhone tải và cài được; chưa có máy Mac + tài khoản Apple Developer | Mã iOS/macOS build + test trên CI (runner macOS); đầu 2027 có máy Mac thì ký và phát hành; trước đó iPhone dùng thêm PWA |
| Giấy phép AGPL-3.0 | Dữ liệu D3 + máy chủ, bản phái sinh phải công khai mã | ADR-0004 |

### B.5. Việc riêng của repo cần biết trước khi sửa

- KHÔNG bao giờ commit giấy tờ thật của người thật vào `testdata/`. Ảnh mẫu tự tạo (giấy tờ giả in ra) hoặc giấy phép mở.
- KHÔNG gửi ảnh/chữ OCR ra mạng ở bất kỳ đâu ngoài đồng bộ E2EE (ADR-0002).
- Mô hình AI không vào git: chỉ sửa `models/manifest.toml`.
- `.gitignore` chuẩn có mẫu `*secret*`, `*password*`, `*.key`: đặt tên file mã nguồn tránh các từ này hoặc thêm ngoại lệ
  có lý do; luôn chạy `git check-ignore -v <file>` khi file mới "biến mất".
- gh: chạy `gh api user -q .login` trước gh/push (phải là `tsudev-tsudev`); không `gh auth switch`.
- KHÔNG tạo/đổi tên repo thành các tên cũ bị cấm trong `~/.claude/CLAUDE.md`.

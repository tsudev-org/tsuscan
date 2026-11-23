# KẾ HOẠCH TỔNG - Tsu Scan

> Mỗi phase = **một phiên làm việc**. Cuối phase: cập nhật `logs/STATE.md`, viết phiếu `logs/handover/`,
> nhả khóa `logs/LOCKS.md`, dừng mọi Monitor/tiến trình nền, cây làm việc sạch, rồi đóng phiên.
> Phiên mới mở bằng khối lệnh ở `.standards/docs/AGENT_PROTOCOL.md` mục 2, sau đó đọc `logs/STATE.md` và phiếu mới
> nhất trong `logs/handover/`. KHÔNG đọc lại toàn bộ tài liệu nếu phiếu đã chỉ rõ file cần đọc (tiết kiệm ngữ cảnh).

## Mục tiêu

Bộ phần mềm **quét tài liệu chuyên nghiệp, xử lý hoàn toàn trên thiết bị, mã hóa đầu-cuối (E2EE)** chạy trên Android,
iOS, Windows, macOS, Linux và web (PWA tại `https://tsuscan.tsudev.com/app`), đồng bộ giữa các thiết bị cùng tài khoản
TsuDEV. Chất lượng dò mép, làm phẳng trang, lọc ảnh và OCR tiếng Việt ngang hoặc hơn các app tham chiếu
(`docs/FEATURES.md`). Chi phí vận hành 0 đồng.

## Ràng buộc đã chốt với chủ dự án (18/11/2026)

| Mục | Quyết định |
| --- | --- |
| Tên sản phẩm / thư mục / repo | "Tsu Scan" / `~/projects/tsuscan` (đổi từ `tsu-scan` ngày 18/11/2026 cho khớp tên repo) / `tsudev-org/tsuscan` (Public) |
| Bảo vệ dữ liệu | E2EE, máy chủ chỉ giữ bản mã; OCR/AI chạy trên thiết bị, không gửi ảnh ra ngoài (ADR-0002, 0003) |
| Máy tính | Đầy đủ: nhập tệp, webcam, **điều khiển máy scan thật** eSCL/SANE/WIA/TWAIN/ICA (ADR-0005, agent tự chọn theo ủy quyền) |
| Phát hành di động | Android miễn phí (GitHub Releases + F-Droid); iOS viết sẵn mã, chờ tài khoản Apple; iPhone tạm dùng PWA |
| Ngôn ngữ | Tiếng Việt (mặc định) + tiếng Anh, i18n từ ngày đầu (ADR-0006) |
| Thứ tự nền tảng | Android trước → máy tính → web/PWA → iOS (ADR-0001 mục 5) |
| Giấy phép | AGPL-3.0-only (ADR-0004) |
| Tên miền / mã app | `tsuscan.tsudev.com` / `com.tsudev.tsuscan`; gói `tsudev-tsuscan`, winget `tsudev.TsuScan` |
| Phân loại dữ liệu | **D3** (CCCD, hợp đồng, giấy tờ nhà đất, hồ sơ y tế) ⇒ rà soát bảo mật bắt buộc mỗi phase đụng dữ liệu |
| Chi phí | 0 đồng: GitHub Public (Actions, CodeQL), Cloudflare Workers/D1/R2 gói miễn phí, hạn mức đồng bộ 1 GB/tài khoản |

## Các phase

| # | Phase | Kết quả nghiệm thu |
| --- | --- | --- |
| P0 | Nền móng | Bộ quy ước, PLAN, ARCHITECTURE, FEATURES, ADR 0001-0006, repo GitHub + ruleset + CI khung (quy ước, gitleaks) |
| P1 | Lõi ảnh `crates/tsuscan-imaging` (Rust) | Dò mép cổ điển + chấm điểm tứ giác, nắn phối cảnh, tự xoay, bộ lọc (Tự động, Màu, Xám, Đen trắng Sauvola, Khử bóng, Bảng trắng), bộ ảnh mẫu có nhãn, IoU tứ giác trung bình ≥ 0,90, benchmark < 40 ms/khung 720p trên x86; CLI dev `tsuscan`; fuzz bộ giải mã ảnh |
| P2 | AI trên máy + OCR `crates/tsuscan-ml` | Suy luận ONNX bằng `tract` (Rust thuần, hợp F-Droid): phân đoạn tài liệu, làm phẳng trang cong, tách 2 trang sách, xóa ngón tay; OCR vi+en chọn bằng đo CER trên bộ mẫu tiếng Việt (ứng viên: PaddleOCR PP-OCR, VietOCR, Tesseract 5); bố cục (đoạn, bảng), hướng chữ; mô hình tải về có checksum + chữ ký |
| P3 | Tài liệu + xuất `crates/tsuscan-doc` | PDF có lớp chữ ẩn tìm được, PDF/A-2b, nén thông minh (JPEG thuần Rust thay MozJPEG, đen trắng CCITT G4, không JBIG2 - ADR-0008), PDF mật khẩu AES-256, DOCX/XLSX/TXT/Markdown/JPEG/PNG/WebP; nhập PDF/ảnh/HEIC; che thông tin xóa hẳn dữ liệu; chữ ký; đọc QR/mã vạch; fuzz bộ đọc PDF |
| P4 | Kho dữ liệu + mật mã `crates/tsuscan-vault` | SQLite mã hóa (SQLite3MultipleCiphers), lược đồ tài liệu/trang/thư mục/nhãn, FTS5 không dấu trên chữ OCR, tệp trang mã hóa luồng XChaCha20-Poly1305, phân cấp khóa + 24 từ khôi phục, HLC, vector đối chiếu `conformance/`, `docs/THREAT_MODEL.md` |
| P5 | Khung app Flutter `apps/app` | Cầu `flutter_rust_bridge`, build 5 đích + web, i18n vi/en, token thiết kế TsuDEV Sáng/Tối, điều hướng (Tài liệu · Quét · Tìm kiếm · Cài đặt), khóa app PIN/sinh trắc, thư viện tài liệu, thư mục/nhãn, tìm không dấu |
| P6 | Quét bằng camera (Android trước) | Xem trước dò mép thời gian thực, tự chụp khi ổn định, chụp loạt, chế độ Tài liệu/Sách/CCCD 2 mặt/Danh thiếp/Bảng trắng/Ảnh/QR, đèn, nhập từ thư viện; quyền tối thiểu; chạy thật trên máy Android qua adb |
| P7 | Công cụ sửa + tổ chức | Chỉnh góc có kính lúp, xoay, lọc, sắp xếp/chèn/xóa trang, chú thích, chữ ký, che, ghép/tách tài liệu, xem/sửa/chép chữ OCR, đặt tên thông minh, thùng rác 30 ngày, nhận diện loại giấy tờ (CCCD, hóa đơn, danh thiếp → danh bạ) |
| P8 | Máy chủ `services/sync-api` | Workers + Hono + D1 + R2: RP OIDC tsudev.com, token app, thiết bị, đồng bộ bản mã theo `rev`, tệp chia khối mã hóa, hạn mức, giới hạn tần suất, xóa tài khoản; triển khai production (cần 👤 token Cloudflare, client OIDC) |
| P9 | Đồng bộ trong app | Đăng nhập, mật khẩu mã hóa + 24 từ, đồng bộ nền, tải trang theo nhu cầu, quản lý/thu hồi thiết bị, ghim chứng chỉ, liên kết chia sẻ E2EE có hạn (khóa nằm trong `#fragment`) |
| P10 | Bản máy tính Windows/Linux/macOS | Bố cục desktop (kéo thả, phím tắt, nhiều cột), webcam, thư mục theo dõi, máy scan thật `crates/tsuscan-scanner` (eSCL mọi OS + Android, SANE, WIA 2.0, TWAIN 2.x 64-bit, ICA), CLI `tsuscan ocr/pdf`, menu chuột phải "OCR bằng Tsu Scan" |
| P11 | Web/PWA + website | Flutter web + lõi Rust WASM tại `/app` (chỉ giữ bản mã trong IndexedDB), trang Astro giới thiệu/tải về/hướng dẫn/riêng tư, thẻ ở `tsudev.com/apps` |
| P12 | Phát hành + kênh cài | Release ký, SBOM, chứng thực nguồn gốc; APK (reproducible, metadata F-Droid), Setup/MSIX, dmg, deb/rpm/AppImage; Scoop, brew, APT/RPM, PR winget; trình cập nhật kiểm chữ ký; Release NHÁP + job VirusTotal dùng chung (`tsudev-org/.github` `virustotal.yml`), sạch mới công bố (phiếu AV1) (cần 👤 khóa ký Android) |
| P13 | Kiểm định bảo mật + 1.0 | Rà soát mô hình đe dọa, OWASP MASVS-L2, fuzz dài, ZAP cho API/web, đối chiếu Luật Bảo vệ dữ liệu cá nhân, Luật An ninh mạng và chính sách Google Play/Microsoft Store/App Store hiện hành; phát hành 1.0.0 |
| P14 | Kiểm thử thật đầu-cuối (phiên mới) | Agent điều khiển điện thoại qua adb + trình duyệt Edge/Playwright, chạy từng tính năng theo kịch bản `docs/E2E_SCENARIOS.md` (lập ở P13), ghi lỗi, sửa triệt để, lặp tới khi sạch |

Thứ tự phụ thuộc: P0 → P1 → P2 → P3 → P4 → P5 → P6 → P7 → P8 → P9 → P10 → P11 → P12 → P13 → P14.
Song song được (theo ranh giới `AGENTS.md` B.3): P2 ∥ P3 sau P1; P8 ∥ P7 sau P5.

Lý do thứ tự: toàn bộ thuật toán (P1-P4) nằm trong lõi Rust dùng chung, có test và bộ ảnh mẫu đo được, nên khi mở
rộng sang nền tảng mới chỉ thêm lớp giao diện/phần cứng, không viết lại logic ⇒ dữ liệu giữa các máy luôn khớp.
Android đi trước vì camera là luồng cốt lõi và phát hành miễn phí.

## Chỉ tiêu chất lượng (đo bằng CI, không ước lượng)

| Chỉ tiêu | Ngưỡng |
| --- | --- |
| IoU tứ giác dò mép (bộ mẫu `testdata/edges`) | ≥ 0,90 trung bình, ≥ 0,80 ở 95% ảnh |
| CER OCR tiếng Việt in (bộ mẫu `testdata/ocr-vi`) | ≤ 3% |
| Độ phủ test lõi Rust | ≥ 85% dòng |
| Thời gian dò mép xem trước trên Android tầm trung | ≤ 33 ms/khung (30 fps) |
| Kích thước APK (mỗi ABI, chưa gồm mô hình tải thêm) | ≤ 40 MB |
| Cảnh báo `cargo clippy -D warnings`, `flutter analyze --fatal-infos` | 0 |

## Việc cần chủ dự án (không tự làm được)

- Token Cloudflare (Workers, D1, R2, DNS cho `tsuscan.tsudev.com`, quyền tối thiểu) ở P8: agent nhận qua
  `tsu-nhan-bi-mat` (chủ dự án chỉ bấm Copy trên trang Cloudflare, KHÔNG dán vào chat/tệp/lệnh `!`), lưu
  `~/.config/tsudev/tsuscan-cloudflare.env` (chmod 600, ngoài repo) hoặc đẩy thẳng `--stdout | wrangler secret put`;
  thu hồi sau khi xong (phiếu SEC1).
- Đăng ký client OIDC `tsuscan` ở tsudev.com, redirect `https://tsuscan.tsudev.com/auth/callback` (P8).
- Khóa ký Android giữ ngoại tuyến → secret Environment `release` (P12). Duyệt Environment `packages` khi phát hành APT.
- Duyệt logo riêng (`BRAND_ASSETS.md` mục 15), tạm dùng logo TsuDEV.
- Điện thoại Android bật gỡ lỗi USB/không dây cho P6 và P14.
- Tùy chọn: tài khoản Apple Developer (99 USD/năm) nếu muốn lên App Store.

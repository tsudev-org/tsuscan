# STATE.md - Trạng thái project Tsu Scan

> Agent đọc file này ở đầu mỗi phiên và cập nhật ở cuối mỗi phiên.
> Quy trình: `.standards/docs/AGENT_PROTOCOL.md`. Kế hoạch tổng: `docs/PLAN.md`.

## Hàng đợi task (làm từ trên xuống)

- [ ] **P6 - Quét bằng camera (Android)** (phiếu `logs/handover/20261119-06_p5-xong-bat-dau-p6.md`)
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

- 19/11/2026 - P5 Khung app Flutter (agent-scan, PR #8 `feat/p5-app`): `apps/app` 6 đích + `crates/tsuscan-ffi` (crate
  `tsuscan_ffi`, flutter_rust_bridge 2.13.0 + cargokit); thư viện (thư mục, nhãn, lọc, phân trang `DATA_TABLE.md`,
  nhập ảnh bỏ EXIF), chi tiết tài liệu, tìm không dấu có tô sáng, khóa app PIN Argon2id + sinh trắc, khóa CSDL trong kho
  bí mật HĐH, i18n vi/en, token Sáng/Tối/Hệ thống, mặc định theo hệ thống (T3); web = bản xem trước kho bộ nhớ (P11);
  kho thêm `list_documents_page`/`count_documents`. ADR-0010, phiếu rà soát `docs/security-reviews/20261119-p5-app.md`.
  CI: job `App Flutter (Linux, Web, Android)` (chạy thật bản Linux, kiểm `vault.db` đã mã hóa) + workflow
  `app-platforms.yml` (iOS/macOS không ký, Windows). Test: Rust ffi 15, Dart 31.

- 19/11/2026 - P4 Kho dữ liệu + mật mã (agent-scan, PR `feat/p4-vault`): `crates/tsuscan-vault` - SQLite3MC
  (ChaCha20-Poly1305) qua bản vá `third_party/libsqlite3-sys`, lược đồ v1 tài liệu/trang/thư mục/nhãn + bia mộ/HLC,
  FTS5 không dấu (khớp đúng dấu xếp trên), tệp trang mã hóa luồng v1, phong bì v1, Argon2id + 24 từ BIP39, HLC;
  ADR-0009, `docs/THREAT_MODEL.md`, phiếu rà soát `docs/security-reviews/20261119-p4-vault.md`, vector
  `conformance/vault-v1.json` (libsodium độc lập kiểm đạt). Đo `.so` arm64: SQLite3MC 1,37 MB vs SQLCipher 5,48 MB;
  độ phủ 93,02%.

- 19/11/2026 - P3 Tài liệu + xuất (agent-scan, PR `feat/p3-doc`): `crates/tsuscan-doc` - PDF lớp chữ OCR ẩn (phông
  không nét tự sinh), PDF/A-2b, AES-256 R6, nén tự nhận màu/xám/đen trắng (JPEG/G4), DOCX/XLSX/TXT/MD/ảnh, nhập PDF
  (hayro) + ảnh, HEIC qua hệ điều hành, chặn bom giải nén 2 lớp, che thông tin xóa hẳn, chữ ký hình, QR/mã vạch (rxing);
  CLI `pdf/export/import/redact/barcode`; ADR-0008; phiếu rà soát `docs/security-reviews/20261119-p3-doc.md`. Đo:
  OCR → PDF → pdftotext CER 0,66% (lớp chữ 0,00% so chữ OCR); qpdf/pikepdf/poppler/python-docx/openpyxl đối chiếu đạt.

- 18/11/2026 - P2 AI trên máy + OCR (agent-scan, PR `feat/p2-ml`): `crates/tsuscan-ml` trên tract - OCR PP-OCRv5 det +
  AsterNet (CER 0,51% / 12 trang), hướng trang (24/24), làm phẳng UVDoc (CER trang cong 33,8% → 0,46%), tách 2 trang,
  xóa ngón tay, bố cục → Markdown; manifest mô hình kiểm sha256/minisign, `scripts/fetch-models.sh`, ADR-0007, phiếu
  rà soát `docs/security-reviews/20261118-p2-ml.md`, CI job `AI trên máy (tsuscan-ml)`. Benchmark P1 đã chứng minh
  (31,6 ms CPU/khung). Ruleset `main` bắt buộc PR (phiếu AN1R).

- 18/11/2026 - P1 Lõi ảnh (agent-scan, PR `feat/p1-imaging`): `crates/tsuscan-imaging` (giải mã có giới hạn, dò mép,
  nắn, 7 bộ lọc), `crates/tsuscan-cli`, bộ sinh ảnh có nhãn, IoU TB 0,9977 / 100% ≥ 0,80 trên 150 ảnh, ảnh vàng bộ lọc,
  fuzz + test bền vững, CI `Lõi Rust (crates)`, phiếu rà soát `docs/security-reviews/20261118-p1-imaging.md`.
  Mở: chứng minh benchmark < 40 ms trên máy rảnh (phiếu 02 mục 2).

- 18/11/2026 - P0 Nền móng (agent-scan): bộ quy ước v3.7.0, `docs/PLAN.md` (15 phase), `docs/ARCHITECTURE.md`,
  `docs/FEATURES.md` (chọn lọc từ 10 app tham chiếu), ADR 0001-0006, LICENSE AGPL-3.0, README/SECURITY/NOTICE/CHANGELOG,
  CI (quy ước + gitleaks), Dependabot.

## Quyết định quan trọng

- 19/11/2026 - App: FRB 2.13.0 + cargokit, ghim cùng bản 4 nơi + test chặn lệch; khóa CSDL trong kho bí mật HĐH; PIN
  là khóa giao diện (Argon2id, chờ tăng dần); web chưa lưu dữ liệu tới P11: `docs/adr/0010-*`.

- 19/11/2026 - Chủ dự án: iOS/macOS phát triển song song mọi phase (Mac/iPhone tải và cài được), phát hành đầu 2027
  khi có máy Mac (`AGENTS.md` B.4).
- 19/11/2026 - Kho: SQLite3MC qua bản vá libsqlite3-sys (loại SQLCipher +4 MB/ABI, loại mã hóa tầng ứng dụng vì chỉ
  mục FTS lộ chữ OCR); phong bì/luồng v1, Argon2id 3/64 MiB sàn 2/19 MiB, KEK khôi phục HKDF: `docs/adr/0009-*`.

- 19/11/2026 - Tầng tài liệu: ghi PDF tự làm trên pdf-writer (mã hóa R6 tự cài), nhập PDF bằng hayro vẽ thành ảnh,
  G4 thay JBIG2, JPEG thuần Rust thay MozJPEG, HEIC qua bộ giải mã hệ điều hành: `docs/adr/0008-*`.

- 18/11/2026 - OCR: PP-OCRv5 mobile det + AsterNet rec trên tract; hướng trang PP-LCNet; làm phẳng UVDoc; tách trang/
  ngón tay/bố cục cổ điển; chưa thêm AI phân đoạn (xem lại ở P6): `docs/adr/0007-*`.

- 18/11/2026 - Thư mục dự án đổi thành `~/projects/tsuscan` (khớp tên repo), chủ dự án yêu cầu.
- 18/11/2026 - Dò mép P1: nhiều bộ sinh ứng viên (contour, vùng sáng, Hough có hướng) + một bộ chấm điểm cực tính;
  lượt sắc độ R-B chỉ chạy khi lượt độ sáng < 0,8 (sắc độ cứu bàn trắng nhưng là nhiễu trên gỗ).

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

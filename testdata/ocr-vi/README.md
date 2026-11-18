# Bộ mẫu OCR tiếng Việt

- `corpus.txt`: kho câu TỰ SOẠN (họ tên, số giấy tờ, địa chỉ đều hư cấu) - nhãn chuẩn. Không chứa giấy tờ thật.
- Trang ảnh KHÔNG nằm trong git: sinh tất định từ seed bằng `tsuscan_ml::synth::generate_page` với font OFL ở
  `testdata/fonts/` (A4 ≈ 150 DPI, nghiêng ±1,2°, nhòe, nhiễu, JPEG q55-90).
- Xem trang: `cargo run -p tsuscan-cli -- synth-ocr -n 5 -o /tmp/ocr-vi`.
- Đo: `cargo test -p tsuscan-ml --test ocr_cer -- --nocapture` (`TSUSCAN_OCR_PAGES`, mặc định 12). Ngưỡng PLAN: CER ≤ 3%.
- Đổi `corpus.txt`, font hoặc `synth.rs` làm đổi toàn bộ bộ mẫu ⇒ ghi số đo mới vào `CHANGELOG.md`.

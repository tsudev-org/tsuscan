# Mô hình AI

Thư mục này chỉ có `manifest.toml` trong git. Tệp `.onnx` tải bằng:

```bash
./scripts/fetch-models.sh            # tất cả
./scripts/fetch-models.sh ocr-det    # theo mã
cargo run -p tsuscan-cli -- models   # kiểm kích thước + sha256 từng tệp
```

Thêm/đổi mô hình: sửa `manifest.toml` (URL ghim commit, sha256, kích thước, giấy phép Apache-2.0/MIT/BSD), ghi công ở
`THIRD-PARTY-NOTICES.md`, chạy lại `cargo test -p tsuscan-ml` với mô hình mới. Lựa chọn và số đo: `docs/adr/0007-*`.

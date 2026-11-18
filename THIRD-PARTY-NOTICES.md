# Thành phần bên thứ ba

Danh sách phụ thuộc **trực tiếp** của mã Tsu Scan. Phụ thuộc bắc cầu được `cargo deny` kiểm giấy phép và nguồn
trong CI (danh sách cho phép: `deny.toml`). Cập nhật file này trong cùng PR khi thêm/bỏ phụ thuộc trực tiếp.

## Rust (`Cargo.toml`)

| Crate | Phiên bản | Giấy phép | Dùng ở | Mục đích |
| --- | --- | --- | --- | --- |
| [image](https://github.com/image-rs/image) | 0.25.10 | MIT OR Apache-2.0 | `tsuscan-imaging` | Giải mã/mã hóa JPEG, PNG, WebP (bộ giải mã thuần Rust) |
| [imageproc](https://github.com/image-rs/imageproc) | 0.27.0 | MIT | `tsuscan-imaging` | Dò contour Suzuki-Abe, làm mờ Gaussian |
| [nalgebra](https://github.com/dimforge/nalgebra) | 0.35.0 | Apache-2.0 | `tsuscan-imaging` | Giải hệ 8x8 cho homography |
| [thiserror](https://github.com/dtolnay/thiserror) | 2.0.21 | MIT OR Apache-2.0 | `tsuscan-imaging` | Kiểu lỗi |
| [clap](https://github.com/clap-rs/clap) | 4.6.8 | MIT OR Apache-2.0 | `tsuscan-cli` (dev) | Phân tích tham số dòng lệnh |
| [serde_json](https://github.com/serde-rs/json) | 1.0.154 | MIT OR Apache-2.0 | `tsuscan-cli` (dev) | Xuất JSON |
| [criterion](https://github.com/criterion-rs/criterion.rs) | 0.8.2 | Apache-2.0 OR MIT | benchmark (dev) | Đo hiệu năng |
| [tract-onnx](https://github.com/sonos/tract) | 0.23.10 | MIT OR Apache-2.0 | `tsuscan-ml` | Suy luận ONNX thuần Rust (kéo theo `tract-*`, `prost`, `ndarray`) |
| [sha2](https://github.com/RustCrypto/hashes) | 0.11.1 | MIT OR Apache-2.0 | `tsuscan-ml` | sha256 tệp mô hình |
| [minisign-verify](https://github.com/jedisct1/rust-minisign-verify) | 0.3.0 | MIT | `tsuscan-ml` | Kiểm chữ ký minisign (Ed25519) của mô hình |
| [serde](https://github.com/serde-rs/serde) + [toml](https://github.com/toml-rs/toml) | 1.0.229 / 1.2.1 | MIT OR Apache-2.0 | `tsuscan-ml` | Đọc `models/manifest.toml` |
| [unicode-normalization](https://github.com/unicode-rs/unicode-normalization) | 0.1.26 | (MIT OR Apache-2.0) AND Unicode-3.0 | `tsuscan-ml` | Chuẩn hóa NFC chữ OCR |
| [ab_glyph](https://github.com/alexheretic/ab-glyph) | 0.2.32 | Apache-2.0 | `tsuscan-ml` (feature `synth`, dev) | Vẽ chữ cho bộ trang mẫu OCR |
| [libfuzzer-sys](https://github.com/rust-fuzz/libfuzzer) | 0.4 | MIT OR Apache-2.0 | `fuzz/` (ngoài workspace) | Đích fuzz |

## Mô hình AI (`models/manifest.toml`, không nằm trong git)

| Mô hình | Giấy phép | Tác giả / nguồn | Dùng cho |
| --- | --- | --- | --- |
| PP-OCRv5 mobile det | Apache-2.0 | PaddlePaddle - huggingface.co/PaddlePaddle/PP-OCRv5_mobile_det_onnx | Dò dòng chữ |
| AsterNet v1.0 | MIT | asterxlabs - huggingface.co/asterxlabs/asternet-vietnamese-ocr | Nhận dạng chữ Việt (bảng ký tự nhúng ở `ocr/vocab.rs`) |
| PP-LCNet x1.0 doc_ori | Apache-2.0 | PaddlePaddle - huggingface.co/PaddlePaddle/PP-LCNet_x1_0_doc_ori_onnx | Hướng trang |
| UVDoc | Apache-2.0 | PaddlePaddle (mô hình UVDoc của Verhoeven và cộng sự) - huggingface.co/PaddlePaddle/UVDoc_onnx | Làm phẳng trang cong |

## Font trong `testdata/fonts/` (chỉ dùng sinh bộ mẫu test, không đóng gói vào app)

| Font | Giấy phép | Bản văn |
| --- | --- | --- |
| Tinos Regular/Bold (Steve Matteson, Google) | SIL OFL 1.1 | `testdata/fonts/OFL-Tinos.txt` |
| Arimo (biến thiên độ đậm) | SIL OFL 1.1 | `testdata/fonts/OFL-Arimo.txt` |
| Be Vietnam Pro Regular | SIL OFL 1.1 | `testdata/fonts/OFL-BeVietnamPro.txt` |

Bản văn giấy phép MIT/Apache-2.0 đi kèm mã nguồn từng crate trong kho crates.io; bản phát hành (P12) sẽ đóng gói
bản văn đầy đủ sinh tự động từ `Cargo.lock`.

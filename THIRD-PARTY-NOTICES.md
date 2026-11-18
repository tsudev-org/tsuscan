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
| [libfuzzer-sys](https://github.com/rust-fuzz/libfuzzer) | 0.4 | MIT OR Apache-2.0 | `fuzz/` (ngoài workspace) | Đích fuzz |

Bản văn giấy phép MIT/Apache-2.0 đi kèm mã nguồn từng crate trong kho crates.io; bản phát hành (P12) sẽ đóng gói
bản văn đầy đủ sinh tự động từ `Cargo.lock`.

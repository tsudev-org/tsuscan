//! Kho dữ liệu + mật mã của Tsu Scan (P4). Quyết định thiết kế: `docs/adr/0009-*`.
//!
//! - [`envelope`]: phong bì v1 cho bản ghi và khóa bọc; [`stream`]: tệp trang mã hóa luồng v1.
//! - [`key`]: khóa 32 byte tự xóa; [`keys`]: Argon2id, 24 từ, bọc MK/DK/FK/AK.
//! - [`store`]: kho SQLite3MC + tệp trang mã hóa, thư mục/nhãn, tìm kiếm FTS5 không dấu.
//! - [`hlc`]: đồng hồ logic lai; [`normalize`]: chuẩn hóa tiếng Việt dùng chung.
#![forbid(unsafe_code)]

pub mod envelope;
pub mod error;
pub mod hlc;
pub mod key;
pub mod keys;
pub mod normalize;
pub mod store;
pub mod stream;

pub use error::{Result, VaultError};
pub use key::Key32;
pub use store::Vault;

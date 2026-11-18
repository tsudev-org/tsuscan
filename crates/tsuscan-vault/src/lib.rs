//! Kho dữ liệu + mật mã của Tsu Scan (P4). Quyết định thiết kế: `docs/adr/0009-*`.
//!
//! - [`envelope`]: phong bì v1 cho bản ghi và khóa bọc; [`stream`]: tệp trang mã hóa luồng v1.
//! - [`secret`]: khóa 32 byte tự xóa.
#![forbid(unsafe_code)]

pub mod envelope;
pub mod error;
pub mod hlc;
pub mod keys;
pub mod normalize;
pub mod secret;
pub mod stream;

pub use error::{Result, VaultError};
pub use secret::Key32;

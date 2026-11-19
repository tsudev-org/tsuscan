//! Bề mặt API cho Dart. Mọi kiểu ở đây là DTO thuần (chuỗi, số, byte) để Dart không giữ con trỏ vào kho ngoài
//! [`library::LibraryHandle`].
#![deny(unsafe_code)]

pub mod error;
pub mod library;
pub mod pin;
pub mod text;

/// Khởi tạo cầu nối (flutter_rust_bridge gọi một lần khi Dart nạp thư viện).
#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

/// Phiên bản lõi Rust (hiển thị ở màn Giới thiệu).
#[flutter_rust_bridge::frb(sync)]
#[must_use]
pub fn core_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

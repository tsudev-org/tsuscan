//! Cầu Rust ↔ Flutter của Tsu Scan.
//!
//! Chỉ `api` là bề mặt công khai cho Dart (flutter_rust_bridge quét module này). `frb_generated` là mã sinh tự động
//! bởi `scripts/gen-bridge.sh` - KHÔNG sửa tay; CI sinh lại và so `git diff`.

pub mod api;
#[allow(
    missing_docs,
    unsafe_code,
    clippy::all,
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    unused_qualifications,
    unexpected_cfgs
)]
mod frb_generated;

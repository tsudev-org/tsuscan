//! Lõi xử lý ảnh Tsu Scan.
//!
//! - [`decode`]: giải mã JPEG/PNG/WebP có giới hạn tài nguyên + hướng EXIF.
//! - [`geometry`]: tứ giác, homography, IoU.
//! - [`detect`]: dò mép tài liệu cổ điển.
//! - [`warp`]: nắn phối cảnh.
//! - [`filters`]: bộ lọc trang.
//!
//! Không I/O mạng, không `unsafe`, không panic với dữ liệu xấu: lỗi trả về [`ImagingError`].

pub mod decode;
pub mod detect;
pub mod error;
pub mod filters;
pub mod geometry;
#[cfg(feature = "synth")]
pub mod synth;
pub mod warp;

pub use decode::decode;
pub use detect::{DetectOptions, Detection, detect, detect_luma, detect_with};
pub use error::{ImagingError, Result};
pub use filters::Filter;
pub use geometry::{Homography, Point, Quad, iou};
pub use warp::{Interpolation, PaperSize, WarpOptions, Warped, warp};

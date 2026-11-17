//! Lỗi của lõi ảnh. Mọi dữ liệu xấu trả `Err`, không bao giờ panic.

use thiserror::Error;

/// Lỗi trả về từ mọi hàm công khai của `tsuscan-imaging`.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ImagingError {
    /// Định dạng không nằm trong danh sách hỗ trợ (JPEG, PNG, WebP).
    #[error("định dạng ảnh không được hỗ trợ")]
    UnsupportedFormat,
    /// Ảnh vượt giới hạn điểm ảnh hoặc bộ nhớ.
    #[error("ảnh quá lớn: {width}x{height} vượt giới hạn {max_pixels} điểm ảnh")]
    TooLarge {
        /// Chiều rộng đọc được.
        width: u32,
        /// Chiều cao đọc được.
        height: u32,
        /// Giới hạn số điểm ảnh.
        max_pixels: u64,
    },
    /// Ảnh quá nhỏ để xử lý.
    #[error("ảnh quá nhỏ: {width}x{height}")]
    TooSmall {
        /// Chiều rộng.
        width: u32,
        /// Chiều cao.
        height: u32,
    },
    /// Tứ giác suy biến (diện tích bằng 0, điểm trùng, không lồi...).
    #[error("tứ giác không hợp lệ: {0}")]
    InvalidQuad(&'static str),
    /// Tham số không hợp lệ.
    #[error("tham số không hợp lệ: {0}")]
    InvalidArgument(&'static str),
    /// Lỗi giải mã/mã hóa từ crate `image`.
    #[error("lỗi giải mã ảnh: {0}")]
    Image(#[from] image::ImageError),
}

/// Kiểu kết quả rút gọn.
pub type Result<T> = std::result::Result<T, ImagingError>;

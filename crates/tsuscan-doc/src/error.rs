//! Lỗi của tầng tài liệu. Mọi dữ liệu xấu (PDF hỏng, mật khẩu sai, tham số sai) trả `Err`, không panic.

use thiserror::Error;

/// Lỗi trả về từ mọi hàm công khai của `tsuscan-doc`.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DocError {
    /// Tài liệu không có trang nào.
    #[error("tài liệu không có trang")]
    Empty,
    /// Tổ hợp tùy chọn không hợp lệ (ví dụ PDF/A cùng mật khẩu).
    #[error("tùy chọn không hợp lệ: {0}")]
    InvalidOption(&'static str),
    /// Tham số không hợp lệ.
    #[error("tham số không hợp lệ: {0}")]
    InvalidArgument(&'static str),
    /// Định dạng tệp nhập không nhận ra hoặc không hỗ trợ.
    #[error("định dạng tệp không được hỗ trợ")]
    UnsupportedFormat,
    /// Ảnh HEIC/HEIF: lõi Rust không giải mã HEVC, app phải giải mã bằng bộ giải mã của hệ điều hành.
    #[error("ảnh HEIC/HEIF cần bộ giải mã của hệ điều hành")]
    NeedsPlatformDecoder,
    /// PDF hỏng hoặc không đọc được.
    #[error("PDF không đọc được")]
    InvalidPdf,
    /// PDF có mật khẩu: chưa nhập hoặc nhập sai.
    #[error("PDF có mật khẩu: mật khẩu chưa đúng")]
    PasswordRequired,
    /// PDF dùng kiểu mã hóa không hỗ trợ.
    #[error("PDF dùng kiểu mã hóa không hỗ trợ")]
    UnsupportedEncryption,
    /// Vượt giới hạn tài nguyên (số trang, kích thước trang, dung lượng).
    #[error("vượt giới hạn: {0}")]
    LimitExceeded(&'static str),
    /// Không lấy được số ngẫu nhiên từ hệ điều hành (cần cho muối/khóa mã hóa).
    #[error("không lấy được số ngẫu nhiên từ hệ điều hành")]
    Random,
    /// Lỗi mã hóa/giải mã ảnh.
    #[error("lỗi ảnh: {0}")]
    Image(#[from] image::ImageError),
    /// Lỗi từ lõi ảnh.
    #[error("lỗi lõi ảnh: {0}")]
    Imaging(#[from] tsuscan_imaging::ImagingError),
    /// Lỗi bộ nén (JPEG/CCITT).
    #[error("lỗi nén: {0}")]
    Encode(String),
}

/// Kết quả rút gọn.
pub type Result<T> = std::result::Result<T, DocError>;

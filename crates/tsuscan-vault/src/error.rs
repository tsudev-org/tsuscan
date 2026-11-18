//! Lỗi của kho. Không bao giờ chứa bản rõ hay khóa.

/// Lỗi của `tsuscan-vault`.
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    /// Bản mã sai khóa, sai ngữ cảnh, bị sửa, cắt hoặc đảo khối.
    #[error("giải mã thất bại: sai khóa hoặc dữ liệu bị sửa")]
    Decrypt,
    /// Dữ liệu không đúng định dạng (đầu tệp, phiên bản, độ dài, đệm).
    #[error("định dạng không hợp lệ: {0}")]
    Format(&'static str),
    /// Mật khẩu không đạt yêu cầu tối thiểu.
    #[error("mật khẩu quá ngắn (cần ít nhất {0} ký tự)")]
    WeakPassword(usize),
    /// Tham số Argon2id ngoài khoảng cho phép (có thể do máy chủ độc ép tham số yếu/quá nặng).
    #[error("tham số dẫn xuất khóa ngoài khoảng cho phép")]
    KdfParams,
    /// 24 từ khôi phục sai (từ lạ, sai số từ, sai tổng kiểm).
    #[error("cụm từ khôi phục không hợp lệ")]
    RecoveryPhrase,
    /// Dấu thời gian HLC từ thiết bị khác lệch tương lai quá ngưỡng.
    #[error("đồng hồ thiết bị khác lệch quá xa")]
    ClockDrift,
    /// Khóa CSDL sai hoặc tệp không phải kho Tsu Scan.
    #[error("không mở được kho: sai khóa hoặc tệp hỏng")]
    WrongDbKey,
    /// Không tìm thấy bản ghi.
    #[error("không tìm thấy {0}")]
    NotFound(&'static str),
    /// Bộ sinh số ngẫu nhiên của hệ điều hành lỗi.
    #[error("bộ sinh số ngẫu nhiên lỗi")]
    Random,
    /// Lỗi SQLite.
    #[error("lỗi CSDL: {0}")]
    Db(#[from] rusqlite::Error),
    /// Lỗi vào/ra.
    #[error("lỗi vào/ra: {0}")]
    Io(#[from] std::io::Error),
}

/// Kết quả của kho.
pub type Result<T> = std::result::Result<T, VaultError>;

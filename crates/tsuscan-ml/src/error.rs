//! Lỗi của tsuscan-ml.

use thiserror::Error;

/// Lỗi suy luận, nạp mô hình hoặc dữ liệu vào.
#[derive(Debug, Error)]
pub enum MlError {
    /// Tệp mô hình sai kích thước, sai sha256 hoặc sai chữ ký: KHÔNG được nạp.
    #[error("mô hình `{id}` không qua kiểm tra toàn vẹn: {reason}")]
    Integrity {
        /// Mã mô hình trong manifest.
        id: String,
        /// Lý do.
        reason: String,
    },
    /// Manifest hỏng hoặc thiếu mục.
    #[error("manifest mô hình: {0}")]
    Manifest(String),
    /// Dữ liệu vào không hợp lệ (ảnh rỗng, kích thước quá giới hạn...).
    #[error("dữ liệu vào không hợp lệ: {0}")]
    InvalidInput(String),
    /// Lỗi từ engine suy luận tract.
    #[error("suy luận ONNX: {0}")]
    Inference(String),
    /// Lỗi đọc tệp.
    #[error("vào/ra: {0}")]
    Io(#[from] std::io::Error),
    /// Lỗi từ lõi ảnh.
    #[error(transparent)]
    Imaging(#[from] tsuscan_imaging::ImagingError),
}

/// Kết quả của tsuscan-ml.
pub type Result<T> = std::result::Result<T, MlError>;

impl From<tract_onnx::prelude::TractError> for MlError {
    fn from(e: tract_onnx::prelude::TractError) -> Self {
        Self::Inference(format!("{e:#}"))
    }
}

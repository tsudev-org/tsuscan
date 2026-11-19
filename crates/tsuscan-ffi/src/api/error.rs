//! Lỗi trả về Dart: một mã ổn định để giao diện dịch sang vi/en, không lộ chi tiết nội bộ (đường dẫn, SQL).

use tsuscan_imaging::ImagingError;
use tsuscan_vault::VaultError;

/// Loại lỗi (Dart ánh xạ sang thông báo đã dịch).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiErrorKind {
    /// Khóa CSDL sai (kho bí mật của máy trả khóa khác, hoặc tệp bị thay).
    WrongKey,
    /// Không tìm thấy đối tượng (đã xóa ở nơi khác).
    NotFound,
    /// Đầu vào không hợp lệ (tên rỗng, khóa sai độ dài...).
    InvalidInput,
    /// Ảnh không đọc được hoặc định dạng không hỗ trợ.
    UnsupportedImage,
    /// Ảnh vượt giới hạn kích thước.
    ImageTooLarge,
    /// Đã có kho ở thư mục này.
    AlreadyExists,
    /// Lỗi lưu trữ (đầy đĩa, quyền...).
    Storage,
    /// Lỗi khác (lỗi lập trình) - ghi nhận để sửa.
    Internal,
}

/// Lỗi trả về Dart.
#[derive(Debug, Clone)]
pub struct ApiError {
    /// Loại lỗi.
    pub kind: ApiErrorKind,
    /// Mô tả ngắn tiếng Việt cho nhật ký gỡ lỗi; KHÔNG chứa nội dung tài liệu, đường dẫn hay khóa.
    pub detail: String,
}

impl ApiError {
    pub(crate) fn new(kind: ApiErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.detail)
    }
}

impl std::error::Error for ApiError {}

impl From<VaultError> for ApiError {
    fn from(e: VaultError) -> Self {
        let kind = match &e {
            VaultError::WrongDbKey => ApiErrorKind::WrongKey,
            VaultError::NotFound(_) => ApiErrorKind::NotFound,
            VaultError::Format("thư mục đã có kho") => ApiErrorKind::AlreadyExists,
            VaultError::Format(_) | VaultError::WeakPassword(_) | VaultError::RecoveryPhrase => {
                ApiErrorKind::InvalidInput
            }
            VaultError::Io(_) | VaultError::Db(_) => ApiErrorKind::Storage,
            _ => ApiErrorKind::Internal,
        };
        // Lỗi SQLite/vào-ra có thể kèm đường dẫn ⇒ chỉ giữ loại lỗi.
        let detail = match &e {
            VaultError::Io(io) => format!("vào/ra: {:?}", io.kind()),
            VaultError::Db(_) => "lỗi CSDL".to_owned(),
            other => other.to_string(),
        };
        Self { kind, detail }
    }
}

impl From<ImagingError> for ApiError {
    fn from(e: ImagingError) -> Self {
        let kind = match e {
            ImagingError::TooLarge { .. } => ApiErrorKind::ImageTooLarge,
            ImagingError::InvalidQuad(_) | ImagingError::InvalidArgument(_) => ApiErrorKind::InvalidInput,
            _ => ApiErrorKind::UnsupportedImage,
        };
        Self::new(kind, e.to_string())
    }
}

/// Kết quả API.
pub type ApiResult<T> = Result<T, ApiError>;

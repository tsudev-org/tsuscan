//! Manifest mô hình (`models/manifest.toml`) và nạp tệp mô hình có kiểm toàn vẹn.
//!
//! Tệp mô hình tải từ mạng là **dữ liệu không tin cậy**. Trước khi đưa cho tract, mọi tệp `MUST` khớp kích thước và
//! sha256 ghi trong manifest. Manifest được nhúng vào app lúc build ([`Manifest::embedded`]) nên là mỏ neo tin cậy
//! ngang với chính bản app đã ký. Khi `signing.public_key` có giá trị, tệp còn `MUST` kèm chữ ký minisign `<tệp>.minisig`
//! hợp lệ (dùng cho mô hình phát hành lại từ Release TsuDEV, P12).

use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::error::{MlError, Result};

/// Kích thước tối đa một tệp mô hình được phép nạp (chặn tệp phình bất thường).
pub const MAX_MODEL_BYTES: u64 = 256 * 1024 * 1024;

/// Manifest nhúng sẵn trong bản build.
const EMBEDDED: &str = include_str!("../../../models/manifest.toml");

/// Toàn bộ manifest.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Phiên bản lược đồ manifest.
    pub schema: u32,
    /// Cấu hình chữ ký.
    #[serde(default)]
    pub signing: Signing,
    /// Danh sách mô hình.
    #[serde(rename = "model", default)]
    pub models: Vec<ModelSpec>,
}

/// Cấu hình chữ ký minisign.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Signing {
    /// Khóa công khai minisign (dạng base64 một dòng, `RW...`). Trống ⇒ chỉ kiểm sha256.
    #[serde(default)]
    pub public_key: String,
}

/// Một mô hình.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelSpec {
    /// Mã ổn định, dùng trong mã nguồn (`ocr-det`, `ocr-rec-vi`...).
    pub id: String,
    /// Tên mô tả.
    pub name: String,
    /// Tên tệp trong thư mục mô hình.
    pub file: String,
    /// URL tải, ghim theo commit/phiên bản.
    pub url: String,
    /// sha256 hex chữ thường.
    pub sha256: String,
    /// Kích thước byte.
    pub size: u64,
    /// SPDX giấy phép của trọng số.
    pub license: String,
    /// Trang nguồn (ghi công).
    pub source: String,
    /// Nhúng vào gói cài (true) hay tải thêm khi cần (false).
    #[serde(default)]
    pub bundled: bool,
}

/// Giấy phép trọng số được chấp nhận (ADR-0002, `docs/ARCHITECTURE.md` mục 4).
pub const ALLOWED_LICENSES: &[&str] = &["Apache-2.0", "MIT", "BSD-2-Clause", "BSD-3-Clause"];

impl Manifest {
    /// Phân tích manifest và kiểm tính hợp lệ.
    ///
    /// # Errors
    /// TOML hỏng, mã trùng, sha256 sai dạng, giấy phép ngoài danh sách cho phép, URL không phải HTTPS.
    pub fn parse(text: &str) -> Result<Self> {
        let m: Self = toml::from_str(text).map_err(|e| MlError::Manifest(e.to_string()))?;
        if m.schema != 1 {
            return Err(MlError::Manifest(format!("lược đồ {} chưa hỗ trợ", m.schema)));
        }
        let mut ids = std::collections::HashSet::new();
        for s in &m.models {
            if !ids.insert(s.id.as_str()) {
                return Err(MlError::Manifest(format!("mã trùng: {}", s.id)));
            }
            if s.sha256.len() != 64 || !s.sha256.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
                return Err(MlError::Manifest(format!(
                    "{}: sha256 phải là 64 ký tự hex thường",
                    s.id
                )));
            }
            if !ALLOWED_LICENSES.contains(&s.license.as_str()) {
                return Err(MlError::Manifest(format!(
                    "{}: giấy phép {} không được phép",
                    s.id, s.license
                )));
            }
            if !s.url.starts_with("https://") {
                return Err(MlError::Manifest(format!("{}: URL phải dùng HTTPS", s.id)));
            }
            if s.size == 0 || s.size > MAX_MODEL_BYTES {
                return Err(MlError::Manifest(format!(
                    "{}: kích thước {} ngoài giới hạn",
                    s.id, s.size
                )));
            }
            if s.file.contains(['/', '\\']) || s.file.starts_with('.') {
                return Err(MlError::Manifest(format!(
                    "{}: tên tệp không được chứa đường dẫn",
                    s.id
                )));
            }
        }
        Ok(m)
    }

    /// Manifest nhúng trong bản build.
    ///
    /// # Errors
    /// Chỉ khi manifest trong repo hỏng (test `manifest_nhung_hop_le` chặn từ CI).
    pub fn embedded() -> Result<Self> {
        Self::parse(EMBEDDED)
    }

    /// Tìm mô hình theo mã.
    ///
    /// # Errors
    /// Không có mã này.
    pub fn get(&self, id: &str) -> Result<&ModelSpec> {
        self.models
            .iter()
            .find(|m| m.id == id)
            .ok_or_else(|| MlError::Manifest(format!("không có mô hình `{id}`")))
    }
}

/// sha256 hex chữ thường.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let d = Sha256::digest(bytes);
    let mut s = String::with_capacity(64);
    for b in d {
        s.push(char::from(b"0123456789abcdef"[usize::from(b >> 4)]));
        s.push(char::from(b"0123456789abcdef"[usize::from(b & 15)]));
    }
    s
}

/// Kiểm byte mô hình theo manifest (kích thước, sha256, chữ ký nếu bật).
///
/// # Errors
/// [`MlError::Integrity`] khi bất kỳ kiểm tra nào trượt.
pub fn verify_bytes(spec: &ModelSpec, signing: &Signing, bytes: &[u8], minisig: Option<&str>) -> Result<()> {
    let fail = |reason: String| MlError::Integrity {
        id: spec.id.clone(),
        reason,
    };
    if bytes.len() as u64 != spec.size {
        return Err(fail(format!(
            "kích thước {} khác manifest {}",
            bytes.len(),
            spec.size
        )));
    }
    let got = sha256_hex(bytes);
    if got != spec.sha256 {
        return Err(fail(format!("sha256 {got} khác manifest")));
    }
    if !signing.public_key.is_empty() {
        let sig = minisig.ok_or_else(|| fail("thiếu chữ ký .minisig".into()))?;
        let pk = minisign_verify::PublicKey::from_base64(signing.public_key.trim())
            .map_err(|e| fail(format!("khóa công khai hỏng: {e}")))?;
        let sig = minisign_verify::Signature::decode(sig).map_err(|e| fail(format!("chữ ký hỏng: {e}")))?;
        pk.verify(bytes, &sig, false)
            .map_err(|e| fail(format!("chữ ký không hợp lệ: {e}")))?;
    }
    Ok(())
}

/// Thư mục chứa tệp mô hình đã tải.
#[derive(Debug, Clone)]
pub struct ModelStore {
    dir: PathBuf,
    manifest: Manifest,
}

impl ModelStore {
    /// Kho mô hình tại `dir` với manifest cho trước.
    #[must_use]
    pub fn new(dir: impl Into<PathBuf>, manifest: Manifest) -> Self {
        Self {
            dir: dir.into(),
            manifest,
        }
    }

    /// Kho mô hình theo biến môi trường `TSUSCAN_MODELS` (mặc định `models/` của repo khi chạy test/CLI dev).
    ///
    /// # Errors
    /// Manifest nhúng hỏng.
    pub fn from_env() -> Result<Self> {
        let dir = std::env::var_os("TSUSCAN_MODELS").map_or_else(
            || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../models"),
            PathBuf::from,
        );
        Ok(Self::new(dir, Manifest::embedded()?))
    }

    /// Manifest đang dùng.
    #[must_use]
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// Đường dẫn tệp của mô hình.
    ///
    /// # Errors
    /// Mã không có trong manifest.
    pub fn path(&self, id: &str) -> Result<PathBuf> {
        Ok(self.dir.join(&self.manifest.get(id)?.file))
    }

    /// Có tệp (chưa kiểm) cho mô hình này không.
    #[must_use]
    pub fn is_present(&self, id: &str) -> bool {
        self.path(id).is_ok_and(|p| p.is_file())
    }

    /// Đọc và kiểm toàn vẹn tệp mô hình. Chỉ byte đã qua kiểm mới được trả ra.
    ///
    /// # Errors
    /// Thiếu tệp, quá lớn, sai kích thước/sha256/chữ ký.
    pub fn load_verified(&self, id: &str) -> Result<Vec<u8>> {
        let spec = self.manifest.get(id)?;
        let path = self.dir.join(&spec.file);
        let meta = std::fs::metadata(&path)?;
        if meta.len() != spec.size {
            return Err(MlError::Integrity {
                id: id.to_owned(),
                reason: format!("kích thước {} khác manifest {}", meta.len(), spec.size),
            });
        }
        let bytes = std::fs::read(&path)?;
        let sig_path = self.dir.join(format!("{}.minisig", spec.file));
        let sig = std::fs::read_to_string(sig_path).ok();
        verify_bytes(spec, &self.manifest.signing, &bytes, sig.as_deref())?;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(bytes: &[u8]) -> ModelSpec {
        ModelSpec {
            id: "t".into(),
            name: "t".into(),
            file: "t.onnx".into(),
            url: "https://x".into(),
            sha256: sha256_hex(bytes),
            size: bytes.len() as u64,
            license: "MIT".into(),
            source: "https://x".into(),
            bundled: false,
        }
    }

    #[test]
    fn manifest_nhung_hop_le() {
        let m = Manifest::embedded().unwrap();
        assert!(!m.models.is_empty());
        for s in &m.models {
            assert!(m.get(&s.id).is_ok());
        }
    }

    #[test]
    fn sha256_vector_chuan() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn tu_choi_sai_kich_thuoc_hoac_sha() {
        let s = spec(b"hello");
        let none = Signing::default();
        assert!(verify_bytes(&s, &none, b"hello", None).is_ok());
        assert!(matches!(
            verify_bytes(&s, &none, b"hellO", None),
            Err(MlError::Integrity { .. })
        ));
        assert!(matches!(
            verify_bytes(&s, &none, b"hello!", None),
            Err(MlError::Integrity { .. })
        ));
    }

    #[test]
    fn bat_chu_ky_thi_thieu_chu_ky_bi_tu_choi() {
        let s = spec(b"hello");
        // Khóa công khai mẫu trong tài liệu minisign-verify.
        let signing = Signing {
            public_key: "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3".into(),
        };
        assert!(matches!(
            verify_bytes(&s, &signing, b"hello", None),
            Err(MlError::Integrity { .. })
        ));
        let bogus = "untrusted comment: x\nRWQf6LRCGA9i59SLOFxz6NxvASXDJeRtuZykwQepbDEGt87ig1BNpWaVWuNrm73YiIiJbq71Wi+dP9eKL8OC351vwIasSSbXxwA=\ntrusted comment: x\nAAAA\n";
        assert!(matches!(
            verify_bytes(&s, &signing, b"hello", Some(bogus)),
            Err(MlError::Integrity { .. })
        ));
    }

    #[test]
    fn manifest_tu_choi_giay_phep_va_url_xau() {
        let base = |lic: &str, url: &str| {
            format!(
                "schema = 1\n[[model]]\nid='a'\nname='a'\nfile='a.onnx'\nurl='{url}'\nsha256='{}'\nsize=1\nlicense='{lic}'\nsource='s'\n",
                "0".repeat(64)
            )
        };
        assert!(Manifest::parse(&base("MIT", "https://x")).is_ok());
        assert!(Manifest::parse(&base("CC-BY-NC-4.0", "https://x")).is_err());
        assert!(Manifest::parse(&base("MIT", "http://x")).is_err());
        assert!(Manifest::parse("schema = 2").is_err());
        let path = base("MIT", "https://x").replace("file='a.onnx'", "file='../a.onnx'");
        assert!(Manifest::parse(&path).is_err());
    }
}

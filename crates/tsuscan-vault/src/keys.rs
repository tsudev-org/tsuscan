//! Phân cấp khóa (ADR-0003, ADR-0009 mục 3.4).
//!
//! ```text
//! mật khẩu ─NFKC→ Argon2id ─→ KEK_pw ─┐
//! 24 từ BIP39 ─→ HKDF-SHA256 ─→ KEK_rec ┴─ bọc → MK ─ bọc → DK (mỗi tài liệu) ─ bọc → FK (mỗi tệp trang)
//!                                               └─ bọc → AK (bản ghi cấp tài khoản: thư mục, nhãn)
//! ```

use argon2::{Algorithm, Argon2, Params, Version};
use bip39::Mnemonic;
use hkdf::Hkdf;
use sha2::Sha256;
use unicode_normalization::UnicodeNormalization;
use zeroize::Zeroizing;

use crate::envelope::{self, Kind};
use crate::error::{Result, VaultError};
use crate::secret::{Key32, fill_random};

/// Số ký tự tối thiểu của mật khẩu mã hóa sau chuẩn hóa NFKC.
pub const MIN_PASSWORD_CHARS: usize = 10;
/// Độ dài muối Argon2id.
pub const SALT_LEN: usize = 16;
/// Info HKDF cho KEK khôi phục.
pub const RECOVERY_INFO: &[u8] = b"tsuscan/recovery/v1";

/// Tham số Argon2id (bộ nhớ KiB, số lượt, số luồng).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KdfParams {
    /// Bộ nhớ (KiB).
    pub m_kib: u32,
    /// Số lượt.
    pub t: u32,
    /// Số luồng.
    pub p: u32,
}

impl KdfParams {
    /// Mặc định khi tạo mới: 3 lượt / 64 MiB / 1 luồng.
    pub const DEFAULT: Self = Self {
        m_kib: 64 * 1024,
        t: 3,
        p: 1,
    };
    /// Sàn OWASP: 2 lượt / 19 MiB.
    pub const FLOOR: Self = Self {
        m_kib: 19 * 1024,
        t: 2,
        p: 1,
    };
    /// Trần chống bị ép tham số làm treo máy: 10 lượt / 1 GiB / 4 luồng.
    pub const CEILING: Self = Self {
        m_kib: 1024 * 1024,
        t: 10,
        p: 4,
    };

    /// Kiểm tham số nằm trong [sàn, trần]. Gọi trước MỌI lần dẫn xuất với tham số nhận từ ngoài.
    ///
    /// # Errors
    /// [`VaultError::KdfParams`] khi ngoài khoảng.
    pub fn validate(self) -> Result<Self> {
        let (lo, hi) = (Self::FLOOR, Self::CEILING);
        let ok = (lo.m_kib..=hi.m_kib).contains(&self.m_kib)
            && (lo.t..=hi.t).contains(&self.t)
            && (1..=hi.p).contains(&self.p);
        if ok { Ok(self) } else { Err(VaultError::KdfParams) }
    }
}

/// Chuẩn hóa mật khẩu NFKC (chữ Việt gõ trên iOS/Windows/Linux có thể khác dạng tổ hợp) và kiểm độ dài.
///
/// # Errors
/// [`VaultError::WeakPassword`] khi ít hơn [`MIN_PASSWORD_CHARS`] ký tự.
pub fn normalize_password(password: &str) -> Result<Zeroizing<String>> {
    let n = Zeroizing::new(password.nfkc().collect::<String>());
    if n.chars().count() < MIN_PASSWORD_CHARS {
        return Err(VaultError::WeakPassword(MIN_PASSWORD_CHARS));
    }
    Ok(n)
}

/// KEK từ mật khẩu: Argon2id(NFKC(mật khẩu), muối).
///
/// # Errors
/// [`VaultError::WeakPassword`], [`VaultError::KdfParams`].
pub fn password_kek(password: &str, salt: &[u8; SALT_LEN], params: KdfParams) -> Result<Key32> {
    let params = params.validate()?;
    let pw = normalize_password(password)?;
    let p = Params::new(params.m_kib, params.t, params.p, Some(32)).map_err(|_| VaultError::KdfParams)?;
    let mut out = Zeroizing::new([0u8; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, p)
        .hash_password_into(pw.as_bytes(), salt, out.as_mut())
        .map_err(|_| VaultError::KdfParams)?;
    Ok(Key32::from_bytes(*out))
}

/// Cụm 24 từ khôi phục (BIP39 tiếng Anh, 256 bit entropy + tổng kiểm). Tự xóa khi hủy.
pub struct RecoveryPhrase(Zeroizing<String>);

impl RecoveryPhrase {
    /// Sinh cụm mới từ 32 byte ngẫu nhiên.
    ///
    /// # Errors
    /// [`VaultError::Random`].
    pub fn generate() -> Result<Self> {
        let mut entropy = Zeroizing::new([0u8; 32]);
        fill_random(entropy.as_mut())?;
        Self::from_entropy(&entropy)
    }

    fn from_entropy(entropy: &[u8; 32]) -> Result<Self> {
        let m = Mnemonic::from_entropy(entropy).map_err(|_| VaultError::RecoveryPhrase)?;
        Ok(Self(Zeroizing::new(m.words().collect::<Vec<_>>().join(" "))))
    }

    /// Đọc cụm người dùng nhập: bỏ hoa/thường và khoảng trắng thừa, kiểm 24 từ + tổng kiểm.
    ///
    /// # Errors
    /// [`VaultError::RecoveryPhrase`].
    pub fn parse(input: &str) -> Result<Self> {
        let norm = Zeroizing::new(
            input
                .split_whitespace()
                .map(str::to_lowercase)
                .collect::<Vec<_>>()
                .join(" "),
        );
        let m = Mnemonic::parse_normalized(&norm).map_err(|_| VaultError::RecoveryPhrase)?;
        if m.word_count() != 24 {
            return Err(VaultError::RecoveryPhrase);
        }
        Ok(Self(norm))
    }

    /// Chuỗi 24 từ cách nhau một dấu cách (để hiển thị cho người dùng chép lại).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// KEK khôi phục = HKDF-SHA256(entropy, info [`RECOVERY_INFO`]).
    ///
    /// # Errors
    /// [`VaultError::RecoveryPhrase`] (không xảy ra với cụm đã kiểm).
    pub fn kek(&self) -> Result<Key32> {
        let m = Mnemonic::parse_normalized(&self.0).map_err(|_| VaultError::RecoveryPhrase)?;
        let (arr, n) = m.to_entropy_array();
        let entropy = Zeroizing::new(arr);
        let mut out = Zeroizing::new([0u8; 32]);
        Hkdf::<Sha256>::new(None, &entropy[..n])
            .expand(RECOVERY_INFO, out.as_mut())
            .map_err(|_| VaultError::RecoveryPhrase)?;
        Ok(Key32::from_bytes(*out))
    }
}

impl std::fmt::Debug for RecoveryPhrase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RecoveryPhrase(***)")
    }
}

/// Ngữ cảnh bọc khóa (AAD). Mỗi loại khóa một ngữ cảnh ⇒ không tráo được bản bọc giữa các chỗ dùng.
pub mod context {
    /// MK bọc bằng KEK mật khẩu.
    pub const MK_PASSWORD: &str = "mk|pw";
    /// MK bọc bằng KEK khôi phục.
    pub const MK_RECOVERY: &str = "mk|rec";
    /// AK bọc bằng MK.
    pub const ACCOUNT: &str = "ak";

    /// DK của tài liệu `doc_id` bọc bằng MK.
    #[must_use]
    pub fn document(doc_id: &str) -> String {
        format!("dk|{doc_id}")
    }

    /// FK của tệp `file_id` bọc bằng DK.
    #[must_use]
    pub fn file(file_id: &str) -> String {
        format!("fk|{file_id}")
    }
}

/// Bọc `key` bằng `kek` trong ngữ cảnh `ctx`.
///
/// # Errors
/// [`VaultError::Random`].
pub fn wrap(kek: &Key32, ctx: &str, key: &Key32) -> Result<Vec<u8>> {
    envelope::seal(kek, Kind::Key, ctx.as_bytes(), key.as_bytes())
}

/// Mở khóa đã bọc.
///
/// # Errors
/// [`VaultError::Decrypt`] khi sai KEK/ngữ cảnh; [`VaultError::Format`] khi bản bọc hỏng.
pub fn unwrap(kek: &Key32, ctx: &str, wrapped: &[u8]) -> Result<Key32> {
    let pt = envelope::open(kek, Kind::Key, ctx.as_bytes(), wrapped)?;
    Key32::from_slice(&pt)
}

/// Gói khóa đưa lên máy chủ (P8/P9): MK bọc bằng mật khẩu và bằng 24 từ. Máy chủ không mở được.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyBackup {
    /// Tham số Argon2id.
    pub kdf: KdfParams,
    /// Muối Argon2id.
    pub salt: [u8; SALT_LEN],
    /// MK bọc bằng KEK mật khẩu (ngữ cảnh `mk|pw`).
    pub mk_password: Vec<u8>,
    /// MK bọc bằng KEK khôi phục (ngữ cảnh `mk|rec`).
    pub mk_recovery: Vec<u8>,
}

impl KeyBackup {
    /// Tạo gói cho `mk` với mật khẩu và cụm khôi phục.
    ///
    /// # Errors
    /// Như [`password_kek`] và [`wrap`].
    pub fn create(mk: &Key32, password: &str, phrase: &RecoveryPhrase, kdf: KdfParams) -> Result<Self> {
        let mut salt = [0u8; SALT_LEN];
        fill_random(&mut salt)?;
        let kek = password_kek(password, &salt, kdf)?;
        Ok(Self {
            kdf,
            salt,
            mk_password: wrap(&kek, context::MK_PASSWORD, mk)?,
            mk_recovery: wrap(&phrase.kek()?, context::MK_RECOVERY, mk)?,
        })
    }

    /// Mở MK bằng mật khẩu. Tham số Argon2id từ máy chủ được kiểm sàn/trần trước.
    ///
    /// # Errors
    /// [`VaultError::KdfParams`], [`VaultError::WeakPassword`], [`VaultError::Decrypt`] (sai mật khẩu).
    pub fn unlock_with_password(&self, password: &str) -> Result<Key32> {
        let kek = password_kek(password, &self.salt, self.kdf)?;
        unwrap(&kek, context::MK_PASSWORD, &self.mk_password)
    }

    /// Mở MK bằng 24 từ.
    ///
    /// # Errors
    /// [`VaultError::Decrypt`] khi cụm từ đúng định dạng nhưng không phải của tài khoản này.
    pub fn unlock_with_recovery(&self, phrase: &RecoveryPhrase) -> Result<Key32> {
        unwrap(&phrase.kek()?, context::MK_RECOVERY, &self.mk_recovery)
    }

    /// Đổi mật khẩu: bọc lại MK bằng mật khẩu mới + muối mới; bản bọc khôi phục giữ nguyên.
    ///
    /// # Errors
    /// Như [`password_kek`].
    pub fn rewrap_password(&self, mk: &Key32, new_password: &str, kdf: KdfParams) -> Result<Self> {
        let mut salt = [0u8; SALT_LEN];
        fill_random(&mut salt)?;
        let kek = password_kek(new_password, &salt, kdf)?;
        Ok(Self {
            kdf,
            salt,
            mk_password: wrap(&kek, context::MK_PASSWORD, mk)?,
            mk_recovery: self.mk_recovery.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PW: &str = "mật khẩu thử nghiệm";

    #[test]
    fn params_floor_and_ceiling() {
        assert!(KdfParams::DEFAULT.validate().is_ok());
        assert!(
            KdfParams {
                m_kib: 8 * 1024,
                t: 3,
                p: 1
            }
            .validate()
            .is_err()
        );
        assert!(
            KdfParams {
                m_kib: 64 * 1024,
                t: 1,
                p: 1
            }
            .validate()
            .is_err()
        );
        assert!(
            KdfParams {
                m_kib: 4 * 1024 * 1024,
                t: 3,
                p: 1
            }
            .validate()
            .is_err()
        );
        assert!(
            KdfParams {
                m_kib: 64 * 1024,
                t: 3,
                p: 0
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn password_nfc_and_nfd_give_same_kek_and_short_rejected() {
        let salt = [1u8; SALT_LEN];
        let nfd: String = PW.nfd().collect();
        assert_ne!(nfd, PW);
        let a = password_kek(PW, &salt, KdfParams::FLOOR);
        let b = password_kek(&nfd, &salt, KdfParams::FLOOR);
        assert!(matches!((a, b), (Ok(a), Ok(b)) if a.as_bytes() == b.as_bytes()));
        assert!(matches!(
            password_kek("ngắn", &salt, KdfParams::FLOOR),
            Err(VaultError::WeakPassword(10))
        ));
    }

    #[test]
    fn recovery_phrase_roundtrip_and_checksum() {
        let p = RecoveryPhrase::generate().unwrap_or_else(|_| unreachable!());
        assert_eq!(p.as_str().split(' ').count(), 24);
        let messy = format!("  {}  ", p.as_str().to_uppercase().replace(' ', "\n "));
        let q = RecoveryPhrase::parse(&messy).unwrap_or_else(|_| unreachable!());
        assert!(matches!((p.kek(), q.kek()), (Ok(a), Ok(b)) if a.as_bytes() == b.as_bytes()));
        // Đổi một từ ⇒ sai tổng kiểm (gần như chắc chắn) hoặc khác KEK.
        let mut words: Vec<&str> = p.as_str().split(' ').collect();
        words[5] = if words[5] == "abandon" {
            "ability"
        } else {
            "abandon"
        };
        if let Ok(r) = RecoveryPhrase::parse(&words.join(" ")) {
            assert!(matches!((p.kek(), r.kek()), (Ok(a), Ok(b)) if a.as_bytes() != b.as_bytes()));
        }
        assert!(RecoveryPhrase::parse("abandon abandon").is_err());
        // Vector BIP39 12 từ hợp lệ (tổng kiểm đúng) nhưng không đủ 24 từ ⇒ phải bị từ chối.
        let twelve = format!("{}about", "abandon ".repeat(11));
        assert!(RecoveryPhrase::parse(&twelve).is_err());
    }

    #[test]
    fn backup_unlock_paths_and_context_binding() {
        let mk = Key32::random().unwrap_or_else(|_| unreachable!());
        let phrase = RecoveryPhrase::generate().unwrap_or_else(|_| unreachable!());
        let b = KeyBackup::create(&mk, PW, &phrase, KdfParams::FLOOR).unwrap_or_else(|_| unreachable!());
        assert!(matches!(b.unlock_with_password(PW), Ok(k) if k.as_bytes() == mk.as_bytes()));
        assert!(matches!(b.unlock_with_recovery(&phrase), Ok(k) if k.as_bytes() == mk.as_bytes()));
        assert!(matches!(
            b.unlock_with_password("sai mật khẩu rồi"),
            Err(VaultError::Decrypt)
        ));
        // Máy chủ độc hạ tham số ⇒ từ chối trước khi dẫn xuất.
        let weak = KeyBackup {
            kdf: KdfParams {
                m_kib: 1024,
                t: 1,
                p: 1,
            },
            ..b.clone()
        };
        assert!(matches!(
            weak.unlock_with_password(PW),
            Err(VaultError::KdfParams)
        ));
        // Tráo bản bọc mật khẩu sang ô khôi phục ⇒ sai ngữ cảnh.
        let swapped = KeyBackup {
            mk_recovery: b.mk_password.clone(),
            ..b.clone()
        };
        assert!(swapped.unlock_with_recovery(&phrase).is_err());
        let b2 = b
            .rewrap_password(&mk, "mật khẩu mới dài hơn", KdfParams::FLOOR)
            .unwrap_or_else(|_| unreachable!());
        assert!(b2.unlock_with_password(PW).is_err());
        assert!(
            matches!(b2.unlock_with_password("mật khẩu mới dài hơn"), Ok(k) if k.as_bytes() == mk.as_bytes())
        );
        assert!(matches!(b2.unlock_with_recovery(&phrase), Ok(k) if k.as_bytes() == mk.as_bytes()));
    }

    #[test]
    fn dk_fk_wrapping_is_context_bound() {
        let mk = Key32::from_bytes([1; 32]);
        let dk = Key32::from_bytes([2; 32]);
        let w = wrap(&mk, &context::document("a"), &dk).unwrap_or_default();
        assert!(matches!(unwrap(&mk, &context::document("a"), &w), Ok(k) if k.as_bytes() == dk.as_bytes()));
        assert!(unwrap(&mk, &context::document("b"), &w).is_err());
        assert!(unwrap(&mk, &context::file("a"), &w).is_err());
    }
}

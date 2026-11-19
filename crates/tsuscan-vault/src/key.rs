//! Khóa 32 byte tự xóa khỏi bộ nhớ khi hủy.

use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::{Result, VaultError};

/// Khóa đối xứng 32 byte (MK, KEK, DK, FK, AK, khóa CSDL). `Debug` không in nội dung.
// Không derive `PartialEq`: so sánh khóa không được phép rò thời gian; test so qua `as_bytes`.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct Key32([u8; 32]);

impl Key32 {
    /// Khóa ngẫu nhiên từ bộ sinh của hệ điều hành.
    ///
    /// # Errors
    /// [`VaultError::Random`] khi hệ điều hành không cấp được số ngẫu nhiên.
    pub fn random() -> Result<Self> {
        let mut k = [0u8; 32];
        fill_random(&mut k)?;
        Ok(Self(k))
    }

    /// Dựng từ byte có sẵn (khóa CSDL lấy từ kho bí mật của thiết bị, vector test).
    #[must_use]
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Dựng từ lát cắt đúng 32 byte.
    ///
    /// # Errors
    /// [`VaultError::Format`] khi độ dài khác 32.
    pub fn from_slice(bytes: &[u8]) -> Result<Self> {
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| VaultError::Format("khóa phải 32 byte"))?;
        Ok(Self(arr))
    }

    /// Byte của khóa. Chỉ dùng để đưa vào thuật toán, không ghi log.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Dạng hex (cho `PRAGMA hexkey`). Chuỗi trả về cũng tự xóa khi hủy.
    #[must_use]
    pub fn to_hex(&self) -> zeroize::Zeroizing<String> {
        zeroize::Zeroizing::new(Self::hex_of(&self.0))
    }
}

impl Key32 {
    /// Hex thường của byte bất kỳ (id ngẫu nhiên, dạng lưu trữ).
    #[must_use]
    pub fn hex_of(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            s.push(char::from(HEX[usize::from(b >> 4)]));
            s.push(char::from(HEX[usize::from(b & 0x0f)]));
        }
        s
    }
}

const HEX: &[u8; 16] = b"0123456789abcdef";

impl std::fmt::Debug for Key32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Key32(***)")
    }
}

/// Điền số ngẫu nhiên an toàn mật mã.
///
/// # Errors
/// [`VaultError::Random`] khi hệ điều hành không cấp được.
pub fn fill_random(buf: &mut [u8]) -> Result<()> {
    getrandom::fill(buf).map_err(|_| VaultError::Random)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_hides_key_and_hex_is_lowercase() {
        let k = Key32::from_bytes([0xab; 32]);
        assert_eq!(format!("{k:?}"), "Key32(***)");
        assert_eq!(&k.to_hex()[..4], "abab");
        assert_eq!(k.to_hex().len(), 64);
    }

    #[test]
    fn random_keys_differ() {
        let (a, b) = (Key32::random(), Key32::random());
        assert!(matches!((a, b), (Ok(a), Ok(b)) if a.as_bytes() != b.as_bytes()));
    }
}

//! Phong bì v1 (ADR-0009 mục 3.2): `'T' 'S' 01 loại ‖ nonce 24 ‖ XChaCha20-Poly1305(đệm(bản rõ))`.
//!
//! AAD = 4 byte đầu ‖ ngữ cảnh. Ngữ cảnh buộc bản mã vào đúng chỗ dùng (ví dụ `dk|<id tài liệu>`): đem phong bì của
//! tài liệu A gắn sang tài liệu B sẽ giải mã thất bại.

use chacha20poly1305::XChaCha20Poly1305;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use zeroize::Zeroizing;

use crate::error::{Result, VaultError};
use crate::key::{Key32, fill_random};

/// Byte nhận dạng định dạng.
pub const MAGIC: [u8; 2] = *b"TS";
/// Phiên bản định dạng phong bì.
pub const VERSION: u8 = 1;
/// Độ dài đầu phong bì (magic + phiên bản + loại).
pub const HEADER_LEN: usize = 4;
/// Độ dài nonce XChaCha20.
pub const NONCE_LEN: usize = 24;
/// Độ dài thẻ xác thực Poly1305.
pub const TAG_LEN: usize = 16;
/// Phần thêm tối thiểu so với bản rõ đã đệm.
pub const OVERHEAD: usize = HEADER_LEN + NONCE_LEN + TAG_LEN;

/// Loại phong bì; quyết định cỡ khối đệm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Bản ghi đồng bộ (tài liệu, trang, thư mục, nhãn), đệm khối 256 B.
    Record = 1,
    /// Khóa được bọc (MK, DK, FK, AK), đệm khối 16 B.
    Key = 2,
}

impl Kind {
    /// Cỡ khối đệm.
    #[must_use]
    pub const fn block(self) -> usize {
        match self {
            Self::Record => 256,
            Self::Key => 16,
        }
    }

    fn from_byte(b: u8) -> Option<Self> {
        match b {
            1 => Some(Self::Record),
            2 => Some(Self::Key),
            _ => None,
        }
    }
}

/// Niêm phong `plaintext` bằng nonce ngẫu nhiên.
///
/// # Errors
/// [`VaultError::Random`] khi không lấy được nonce.
pub fn seal(key: &Key32, kind: Kind, context: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
    let mut nonce = [0u8; NONCE_LEN];
    fill_random(&mut nonce)?;
    seal_with_nonce(key, kind, context, plaintext, &nonce)
}

/// Niêm phong với nonce cho trước. CHỈ dùng cho vector đối chiếu; dùng lại nonce với cùng khóa phá vỡ bảo mật.
///
/// # Errors
/// [`VaultError::Format`] khi bản rõ vượt giới hạn của AEAD (không xảy ra với dữ liệu thực).
#[doc(hidden)]
pub fn seal_with_nonce(
    key: &Key32,
    kind: Kind,
    context: &[u8],
    plaintext: &[u8],
    nonce: &[u8; NONCE_LEN],
) -> Result<Vec<u8>> {
    let header = [MAGIC[0], MAGIC[1], VERSION, kind as u8];
    let padded = Zeroizing::new(pad(plaintext, kind.block()));
    let aad = [header.as_slice(), context].concat();
    let ct = cipher(key)
        .encrypt(
            &(*nonce).into(),
            Payload {
                msg: &padded,
                aad: &aad,
            },
        )
        .map_err(|_| VaultError::Format("bản rõ quá dài"))?;
    let mut out = Vec::with_capacity(HEADER_LEN + NONCE_LEN + ct.len());
    out.extend_from_slice(&header);
    out.extend_from_slice(nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

/// Mở phong bì. Kiểm magic, phiên bản, loại, thẻ xác thực rồi bỏ đệm.
///
/// # Errors
/// [`VaultError::Format`] khi sai đầu phong bì/đệm; [`VaultError::Decrypt`] khi sai khóa, ngữ cảnh hoặc bị sửa.
pub fn open(key: &Key32, kind: Kind, context: &[u8], data: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    if data.len() < OVERHEAD + kind.block() {
        return Err(VaultError::Format("phong bì quá ngắn"));
    }
    if data[..2] != MAGIC {
        return Err(VaultError::Format("không phải phong bì Tsu Scan"));
    }
    if data[2] != VERSION {
        return Err(VaultError::Format("phiên bản phong bì không hỗ trợ"));
    }
    if Kind::from_byte(data[3]) != Some(kind) {
        return Err(VaultError::Format("sai loại phong bì"));
    }
    let (header, rest) = data.split_at(HEADER_LEN);
    let (nonce, ct) = rest.split_at(NONCE_LEN);
    let nonce: [u8; NONCE_LEN] = nonce.try_into().map_err(|_| VaultError::Format("nonce"))?;
    let aad = [header, context].concat();
    let padded = Zeroizing::new(
        cipher(key)
            .decrypt(&nonce.into(), Payload { msg: ct, aad: &aad })
            .map_err(|_| VaultError::Decrypt)?,
    );
    let len = unpad_len(&padded, kind.block())?;
    Ok(Zeroizing::new(padded[..len].to_vec()))
}

pub(crate) fn cipher(key: &Key32) -> XChaCha20Poly1305 {
    XChaCha20Poly1305::new(&(*key.as_bytes()).into())
}

/// Đệm ISO/IEC 7816-4 tới bội số của `block` (luôn thêm ít nhất 1 byte `0x80`).
fn pad(data: &[u8], block: usize) -> Vec<u8> {
    let total = (data.len() / block + 1) * block;
    let mut v = Vec::with_capacity(total);
    v.extend_from_slice(data);
    v.push(0x80);
    v.resize(total, 0);
    v
}

/// Độ dài bản rõ sau khi bỏ đệm.
fn unpad_len(padded: &[u8], block: usize) -> Result<usize> {
    if padded.is_empty() || !padded.len().is_multiple_of(block) {
        return Err(VaultError::Format("độ dài đệm sai"));
    }
    // Đệm dài tối đa một khối; dữ liệu đã được xác thực nên không cần thời gian hằng.
    let start = padded.len() - block;
    for i in (start..padded.len()).rev() {
        match padded[i] {
            0 => {}
            0x80 => return Ok(i),
            _ => break,
        }
    }
    Err(VaultError::Format("đệm sai"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(b: u8) -> Key32 {
        Key32::from_bytes([b; 32])
    }

    #[test]
    fn roundtrip_and_padding_sizes() {
        for n in [0usize, 1, 15, 16, 17, 255, 256, 1000] {
            let pt: Vec<u8> = (0..n).map(|i| i as u8).collect();
            for kind in [Kind::Record, Kind::Key] {
                let env = seal(&key(1), kind, b"ctx", &pt).unwrap_or_default();
                assert_eq!((env.len() - OVERHEAD) % kind.block(), 0);
                assert_eq!(
                    open(&key(1), kind, b"ctx", &env).map(|p| p.to_vec()).ok(),
                    Some(pt.clone())
                );
            }
        }
    }

    #[test]
    fn rejects_wrong_key_context_kind_and_tamper() {
        let env = seal(&key(1), Kind::Record, b"dk|a", b"hello").unwrap_or_default();
        assert!(matches!(
            open(&key(2), Kind::Record, b"dk|a", &env),
            Err(VaultError::Decrypt)
        ));
        assert!(matches!(
            open(&key(1), Kind::Record, b"dk|b", &env),
            Err(VaultError::Decrypt)
        ));
        assert!(matches!(
            open(&key(1), Kind::Key, b"dk|a", &env),
            Err(VaultError::Format(_))
        ));
        for i in 0..env.len() {
            let mut t = env.clone();
            t[i] ^= 1;
            assert!(open(&key(1), Kind::Record, b"dk|a", &t).is_err(), "byte {i}");
        }
        assert!(open(&key(1), Kind::Record, b"dk|a", &env[..env.len() - 1]).is_err());
    }

    #[test]
    fn unpad_rejects_bad_padding() {
        assert!(unpad_len(&[0u8; 16], 16).is_err());
        let mut p = vec![0u8; 16];
        p[3] = 0x80;
        p[10] = 1;
        assert!(unpad_len(&p, 16).is_err());
        assert!(unpad_len(&[0x80; 15], 16).is_err());
    }
}

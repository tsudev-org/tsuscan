//! Mã hóa PDF AES-256: bộ xử lý bảo mật chuẩn V5/R6 (ISO 32000-2 §7.6.4.3.3-4, thuật toán 2.A/2.B/8/9/10).
//!
//! - Khóa tệp 256 bit ngẫu nhiên từ hệ điều hành; mọi chuỗi và luồng mã hóa AES-256-CBC, IV ngẫu nhiên 16 byte
//!   đứng đầu, đệm PKCS#7.
//! - Mật khẩu chuẩn hóa NFKC (gần SASLprep mà chuẩn yêu cầu; khác biệt chỉ ở ký tự cấm/ánh xạ hiếm) rồi cắt 127 byte.
//! - R6 không dùng MD5/RC4; mật khẩu sai không giải mã được nội dung (khác "mật khẩu chủ" kiểu R2-R4).

use aes::cipher::{BlockModeEncrypt, KeyIvInit, block_padding::NoPadding, block_padding::Pkcs7};
use sha2::{Digest, Sha256, Sha384, Sha512};
use unicode_normalization::UnicodeNormalization;

use crate::error::{DocError, Result};

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;
type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;

/// Quyền cho người mở bằng mật khẩu người dùng (bit P, ISO 32000-2 Bảng 22).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // Mỗi cờ là một quyền độc lập của chuẩn PDF.
pub struct Permissions {
    /// In (cả chất lượng cao).
    pub print: bool,
    /// Sửa nội dung, ghép/sắp trang.
    pub modify: bool,
    /// Chép chữ/ảnh.
    pub copy: bool,
    /// Chú thích, điền biểu mẫu.
    pub annotate: bool,
}

impl Default for Permissions {
    fn default() -> Self {
        Self {
            print: true,
            modify: true,
            copy: true,
            annotate: true,
        }
    }
}

impl Permissions {
    /// Giá trị P (số nguyên 32 bit có dấu). Bit 7-8 và 13-32 luôn bật; bit 10 (trích cho trợ năng) luôn bật.
    #[must_use]
    pub fn p_value(self) -> i32 {
        let mut p: u32 = 0xFFFF_F0C0 | (1 << 9);
        if self.print {
            p |= (1 << 2) | (1 << 11);
        }
        if self.modify {
            p |= (1 << 3) | (1 << 10);
        }
        if self.copy {
            p |= 1 << 4;
        }
        if self.annotate {
            p |= (1 << 5) | (1 << 8);
        }
        p as i32
    }
}

/// Lấy byte ngẫu nhiên từ hệ điều hành.
///
/// # Errors
/// [`DocError::Random`] khi hệ điều hành không cấp được.
pub fn random_bytes<const N: usize>() -> Result<[u8; N]> {
    let mut b = [0u8; N];
    getrandom::fill(&mut b).map_err(|_| DocError::Random)?;
    Ok(b)
}

fn prep_password(pw: &str) -> Vec<u8> {
    let mut b: Vec<u8> = pw.nfkc().collect::<String>().into_bytes();
    b.truncate(127);
    b
}

fn cbc_no_pad<C>(enc: C, data: &[u8]) -> Vec<u8>
where
    C: BlockModeEncrypt,
{
    let mut buf = data.to_vec();
    let len = buf.len();
    // Đầu vào luôn là bội 16 byte (gọi nội bộ) ⇒ NoPadding không thể lỗi.
    match enc.encrypt_padded::<NoPadding>(&mut buf, len) {
        Ok(ct) => ct.to_vec(),
        Err(_) => Vec::new(),
    }
}

/// Thuật toán 2.B: băm mật khẩu cho R6.
fn hash_2b(password: &[u8], salt: &[u8], udata: &[u8]) -> [u8; 32] {
    let mut k: Vec<u8> = Sha256::new()
        .chain_update(password)
        .chain_update(salt)
        .chain_update(udata)
        .finalize()
        .to_vec();
    let mut round: u32 = 0;
    loop {
        let mut k1 = Vec::with_capacity(64 * (password.len() + k.len() + udata.len()));
        for _ in 0..64 {
            k1.extend_from_slice(password);
            k1.extend_from_slice(&k);
            k1.extend_from_slice(udata);
        }
        let (key, iv) = (&k[..16], &k[16..32]);
        let Ok(enc) = Aes128CbcEnc::new_from_slices(key, iv) else {
            return [0; 32];
        };
        let e = cbc_no_pad(enc, &k1);
        let modulo: u32 = e[..16].iter().map(|&b| u32::from(b)).sum::<u32>() % 3;
        k = match modulo {
            0 => Sha256::digest(&e).to_vec(),
            1 => Sha384::digest(&e).to_vec(),
            _ => Sha512::digest(&e).to_vec(),
        };
        round += 1;
        let last = u32::from(e.last().copied().unwrap_or(0));
        if round >= 64 && last <= round - 32 {
            break;
        }
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&k[..32]);
    out
}

fn aes256_cbc_zero_iv(key: &[u8; 32], data: &[u8]) -> Vec<u8> {
    match Aes256CbcEnc::new_from_slices(key, &[0u8; 16]) {
        Ok(enc) => cbc_no_pad(enc, data),
        Err(_) => Vec::new(),
    }
}

/// Bộ mã hóa của một tệp PDF: giữ khóa tệp và các giá trị cho từ điển `/Encrypt`.
#[derive(Clone)]
pub struct Crypt {
    key: [u8; 32],
    /// Giá trị `/U` (48 byte).
    pub u: Vec<u8>,
    /// Giá trị `/UE` (32 byte).
    pub ue: Vec<u8>,
    /// Giá trị `/O` (48 byte).
    pub o: Vec<u8>,
    /// Giá trị `/OE` (32 byte).
    pub oe: Vec<u8>,
    /// Giá trị `/Perms` (16 byte).
    pub perms: Vec<u8>,
    /// Giá trị `/P`.
    pub p: i32,
}

impl std::fmt::Debug for Crypt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Không bao giờ in khóa tệp.
        f.debug_struct("Crypt")
            .field("p", &self.p)
            .finish_non_exhaustive()
    }
}

impl Crypt {
    /// Tạo bộ mã hóa mới. `owner` rỗng ⇒ mật khẩu chủ ngẫu nhiên 256 bit (không ai mở khóa quyền được).
    ///
    /// # Errors
    /// [`DocError::InvalidArgument`] khi mật khẩu người dùng rỗng; [`DocError::Random`] khi thiếu số ngẫu nhiên.
    pub fn new(user: &str, owner: &str, perms: Permissions) -> Result<Self> {
        if user.is_empty() {
            return Err(DocError::InvalidArgument("mật khẩu mở tệp không được rỗng"));
        }
        let key: [u8; 32] = random_bytes()?;
        let salts: [u8; 32] = random_bytes()?;
        let owner_bytes = if owner.is_empty() {
            random_bytes::<32>()?.to_vec()
        } else {
            prep_password(owner)
        };
        Self::with_material(
            &prep_password(user),
            &owner_bytes,
            perms,
            key,
            salts,
            random_bytes()?,
        )
    }

    fn with_material(
        user: &[u8],
        owner: &[u8],
        perms: Permissions,
        key: [u8; 32],
        salts: [u8; 32],
        perms_tail: [u8; 4],
    ) -> Result<Self> {
        let (uvs, uks, ovs, oks) = (&salts[0..8], &salts[8..16], &salts[16..24], &salts[24..32]);
        let mut u = hash_2b(user, uvs, &[]).to_vec();
        u.extend_from_slice(uvs);
        u.extend_from_slice(uks);
        let ue = aes256_cbc_zero_iv(&hash_2b(user, uks, &[]), &key);
        let mut o = hash_2b(owner, ovs, &u).to_vec();
        o.extend_from_slice(ovs);
        o.extend_from_slice(oks);
        let oe = aes256_cbc_zero_iv(&hash_2b(owner, oks, &u), &key);
        let p = perms.p_value();
        let mut block = [0u8; 16];
        block[..4].copy_from_slice(&p.to_le_bytes());
        block[4..8].copy_from_slice(&[0xFF; 4]);
        block[8..12].copy_from_slice(b"Tadb");
        block[12..].copy_from_slice(&perms_tail);
        let perms_enc = aes256_cbc_zero_iv(&key, &block);
        if ue.len() != 32 || oe.len() != 32 || perms_enc.len() != 16 {
            return Err(DocError::Encode("AES lỗi khi dựng khóa".into()));
        }
        Ok(Self {
            key,
            u,
            ue,
            o,
            oe,
            perms: perms_enc,
            p,
        })
    }

    /// Mã hóa một chuỗi hoặc luồng: IV ngẫu nhiên ‖ AES-256-CBC(PKCS#7).
    ///
    /// # Errors
    /// [`DocError::Random`] khi thiếu số ngẫu nhiên.
    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        let iv: [u8; 16] = random_bytes()?;
        let enc = Aes256CbcEnc::new_from_slices(&self.key, &iv)
            .map_err(|_| DocError::Encode("khóa AES sai độ dài".into()))?;
        let mut buf = data.to_vec();
        let len = buf.len();
        buf.resize(len + 16 - len % 16, 0);
        let ct = enc
            .encrypt_padded::<Pkcs7>(&mut buf, len)
            .map_err(|_| DocError::Encode("đệm AES lỗi".into()))?;
        let mut out = iv.to_vec();
        out.extend_from_slice(ct);
        Ok(out)
    }

    /// Kiểm mật khẩu người dùng theo thuật toán 11 (dùng cho test và kiểm tra sau khi ghi).
    #[must_use]
    pub fn check_user_password(&self, user: &str) -> bool {
        let pw = prep_password(user);
        hash_2b(&pw, &self.u[32..40], &[]).as_slice() == &self.u[..32]
    }

    /// Kiểm mật khẩu chủ theo thuật toán 12.
    #[must_use]
    pub fn check_owner_password(&self, owner: &str) -> bool {
        let pw = prep_password(owner);
        hash_2b(&pw, &self.o[32..40], &self.u).as_slice() == &self.o[..32]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes::cipher::{BlockModeDecrypt, KeyIvInit};

    type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

    #[test]
    fn passwords_roundtrip_and_ue_recovers_key() {
        let c = Crypt::new("mật khẩu", "chủ", Permissions::default()).unwrap();
        assert_eq!(
            (c.u.len(), c.ue.len(), c.o.len(), c.oe.len(), c.perms.len()),
            (48, 32, 48, 32, 16)
        );
        assert!(c.check_user_password("mật khẩu"));
        // NFD của cùng chuỗi ⇒ NFKC giống nhau ⇒ vẫn đúng.
        assert!(c.check_user_password("ma\u{0302}\u{0323}t kha\u{0302}\u{0309}u"));
        assert!(!c.check_user_password("sai"));
        assert!(c.check_owner_password("chủ"));
        assert!(!c.check_owner_password("mật khẩu"));
        // Thuật toán 2.A: UE giải bằng hash(mật khẩu, salt khóa) ra đúng khóa tệp.
        let ik = hash_2b(&prep_password("mật khẩu"), &c.u[40..48], &[]);
        let mut buf = c.ue.clone();
        let dec = Aes256CbcDec::new_from_slices(&ik, &[0; 16]).unwrap();
        let key = dec.decrypt_padded::<NoPadding>(&mut buf).unwrap();
        assert_eq!(key, &c.key);
        assert!(Crypt::new("", "", Permissions::default()).is_err());
        assert!(format!("{c:?}").contains("Crypt"));
    }

    #[test]
    fn encrypt_layout_and_decrypt() {
        let c = Crypt::new("a", "", Permissions::default()).unwrap();
        for n in [0usize, 1, 15, 16, 17, 1000] {
            let data = vec![7u8; n];
            let ct = c.encrypt(&data).unwrap();
            assert_eq!(ct.len(), 16 + (n / 16 + 1) * 16);
            let mut body = ct[16..].to_vec();
            let pt = Aes256CbcDec::new_from_slices(&c.key, &ct[..16])
                .unwrap()
                .decrypt_padded::<Pkcs7>(&mut body)
                .unwrap();
            assert_eq!(pt, &data[..]);
        }
    }

    #[test]
    fn permission_bits() {
        assert_eq!(Permissions::default().p_value(), -4);
        let none = Permissions {
            print: false,
            modify: false,
            copy: false,
            annotate: false,
        };
        let p = none.p_value() as u32;
        assert_eq!(p & 0b11_1111_1100, 0b10_1100_0000);
        assert_eq!(p >> 12, 0xF_FFFF);
    }
}

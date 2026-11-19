//! Mã PIN khóa app: lưu bộ kiểm Argon2id trong kho bí mật của thiết bị, so sánh thời gian hằng.
//!
//! PIN chỉ là lớp khóa GIAO DIỆN (người khác cầm máy đang mở khóa). Dữ liệu trên đĩa đã được mã hóa bằng khóa CSDL
//! trong Keystore/Keychain/DPAPI/libsecret, không phụ thuộc PIN (ADR-0010).

use argon2::{Algorithm, Argon2, Params, Version};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

use super::error::{ApiError, ApiErrorKind, ApiResult};

/// Tiền tố định dạng bộ kiểm (đổi tham số ⇒ tăng phiên bản, vẫn đọc được bản cũ).
const PREFIX: &str = "pin1";
/// Argon2id 19 MiB / 2 lượt (sàn OWASP): mở khóa < 0,3 s trên điện thoại tầm trung.
const M_KIB: u32 = 19 * 1024;
const T_COST: u32 = 2;
const SALT_LEN: usize = 16;
/// Độ dài PIN cho phép.
pub const PIN_MIN: usize = 4;
/// Độ dài PIN tối đa.
pub const PIN_MAX: usize = 12;

fn check_pin(pin: &str) -> ApiResult<()> {
    let n = pin.chars().count();
    if !(PIN_MIN..=PIN_MAX).contains(&n) || !pin.chars().all(|c| c.is_ascii_digit()) {
        return Err(ApiError::new(ApiErrorKind::InvalidInput, "PIN gồm 4-12 chữ số"));
    }
    Ok(())
}

fn derive(pin: &str, salt: &[u8]) -> ApiResult<Zeroizing<[u8; 32]>> {
    let params = Params::new(M_KIB, T_COST, 1, Some(32))
        .map_err(|_| ApiError::new(ApiErrorKind::Internal, "tham số Argon2"))?;
    let mut out = Zeroizing::new([0u8; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(pin.as_bytes(), salt, out.as_mut())
        .map_err(|_| ApiError::new(ApiErrorKind::Internal, "Argon2"))?;
    Ok(out)
}

fn hex(bytes: &[u8]) -> String {
    tsuscan_vault::Key32::hex_of(bytes)
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}

/// Tạo bộ kiểm cho PIN mới (`pin1$<muối hex>$<băm hex>`).
///
/// # Errors
/// `InvalidInput` khi PIN không phải 4-12 chữ số.
pub fn pin_create_verifier(pin: String) -> ApiResult<String> {
    let pin = Zeroizing::new(pin);
    check_pin(&pin)?;
    let mut salt = [0u8; SALT_LEN];
    getrandom::fill(&mut salt).map_err(|_| ApiError::new(ApiErrorKind::Internal, "số ngẫu nhiên"))?;
    let h = derive(&pin, &salt)?;
    Ok(format!("{PREFIX}${}${}", hex(&salt), hex(h.as_ref())))
}

/// Kiểm PIN với bộ kiểm đã lưu. Bộ kiểm hỏng ⇒ `InvalidInput` (app yêu cầu đặt lại bằng sinh trắc/xóa dữ liệu).
///
/// # Errors
/// `InvalidInput` khi bộ kiểm không đúng định dạng.
pub fn pin_verify(pin: String, verifier: String) -> ApiResult<bool> {
    let pin = Zeroizing::new(pin);
    let mut parts = verifier.split('$');
    let (Some(PREFIX), Some(salt), Some(hash), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(ApiError::new(ApiErrorKind::InvalidInput, "bộ kiểm PIN hỏng"));
    };
    let (Some(salt), Some(hash)) = (unhex(salt), unhex(hash)) else {
        return Err(ApiError::new(ApiErrorKind::InvalidInput, "bộ kiểm PIN hỏng"));
    };
    if salt.len() != SALT_LEN || hash.len() != 32 {
        return Err(ApiError::new(ApiErrorKind::InvalidInput, "bộ kiểm PIN hỏng"));
    }
    // PIN sai định dạng vẫn chạy Argon2 để thời gian trả lời không lộ lý do.
    let got = derive(&pin, &salt)?;
    let well_formed = check_pin(&pin).is_ok();
    Ok(bool::from(got.as_ref().ct_eq(hash.as_slice())) && well_formed)
}

/// Thời gian chờ (giây) sau `failures` lần nhập sai liên tiếp: 5 lần đầu không chờ, sau đó 30 s nhân đôi, trần 1 giờ.
#[flutter_rust_bridge::frb(sync)]
#[must_use]
pub fn pin_lockout_seconds(failures: u32) -> u32 {
    if failures < 5 {
        return 0;
    }
    let exp = (failures - 5).min(7);
    (30u32 << exp).min(3600)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_dung_thi_mo_khoa_pin_sai_thi_khong() -> ApiResult<()> {
        let v = pin_create_verifier("2468".into())?;
        assert!(v.starts_with("pin1$"));
        assert!(pin_verify("2468".into(), v.clone())?);
        assert!(!pin_verify("2469".into(), v.clone())?);
        assert!(!pin_verify("24680".into(), v)?);
        Ok(())
    }

    #[test]
    fn hai_lan_tao_cung_pin_cho_bo_kiem_khac_nhau() -> ApiResult<()> {
        let a = pin_create_verifier("123456".into())?;
        let b = pin_create_verifier("123456".into())?;
        assert_ne!(a, b);
        Ok(())
    }

    #[test]
    fn tu_choi_pin_khong_phai_so_hoac_sai_do_dai() {
        for bad in ["123", "1234567890123", "12a4", "١٢٣٤", ""] {
            assert!(pin_create_verifier(bad.into()).is_err(), "{bad}");
        }
    }

    #[test]
    fn bo_kiem_hong_bao_loi_dau_vao() {
        for bad in ["", "pin1$zz$00", "pin0$00$00", "pin1$00$00$00", "pin1$0011$0011"] {
            let e = pin_verify("1234".into(), bad.into());
            assert!(
                matches!(
                    e,
                    Err(ApiError {
                        kind: ApiErrorKind::InvalidInput,
                        ..
                    })
                ),
                "{bad}"
            );
        }
    }

    #[test]
    fn thoi_gian_cho_tang_dan_va_co_tran() {
        assert_eq!(pin_lockout_seconds(0), 0);
        assert_eq!(pin_lockout_seconds(4), 0);
        assert_eq!(pin_lockout_seconds(5), 30);
        assert_eq!(pin_lockout_seconds(6), 60);
        assert_eq!(pin_lockout_seconds(11), 1920);
        assert_eq!(pin_lockout_seconds(12), 3600);
        assert_eq!(pin_lockout_seconds(u32::MAX), 3600);
    }
}

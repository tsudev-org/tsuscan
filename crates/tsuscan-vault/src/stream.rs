//! Mã hóa luồng v1 cho tệp trang (ADR-0009 mục 3.3), kiểu STREAM của Hoang-Reyhanitabar-Rogaway-Vizár.
//!
//! Đầu tệp 24 B = `'T' 'S' 01 03 ‖ tiền tố nonce 19 ‖ 16`. Khối i: nonce = tiền tố ‖ i (u32 BE) ‖ cờ cuối, AAD = đầu
//! tệp ‖ ngữ cảnh. Bản rõ logic = độ dài thật (u64 LE) ‖ dữ liệu ‖ số 0 tới cỡ Padmé.
//!
//! Đảo hai khối ⇒ sai nonce; cắt ở ranh giới khối ⇒ khối cuối còn lại mang cờ 0 ⇒ lỗi; ghép thêm ⇒ khối sau khối có
//! cờ 1 không tồn tại hợp lệ. Bên giải mã ghi dữ liệu ra dần: gặp lỗi thì BỎ TOÀN BỘ đầu ra.

use std::io::{Read, Write};

use chacha20poly1305::aead::{Aead, Payload};
use zeroize::Zeroizing;

use crate::envelope::{MAGIC, TAG_LEN, cipher};
use crate::error::{Result, VaultError};
use crate::key::{Key32, fill_random};

/// Phiên bản định dạng luồng.
pub const VERSION: u8 = 1;
/// Loại "tệp luồng" trong byte thứ 4 của đầu tệp.
pub const KIND_STREAM: u8 = 3;
/// Độ dài đầu tệp.
pub const HEADER_LEN: usize = 24;
/// log2 cỡ khối bản rõ.
pub const CHUNK_LOG2: u8 = 16;
/// Cỡ khối bản rõ (64 KiB).
pub const CHUNK: usize = 1 << CHUNK_LOG2;
/// Độ dài tiền tố nonce.
pub const PREFIX_LEN: usize = 19;
const CT_CHUNK: usize = CHUNK + TAG_LEN;
const LEN_FIELD: u64 = 8;

/// Cỡ sau đệm Padmé (Nikitin và cộng sự 2019): rò rỉ tối đa O(log log L) bit, dư ≤ 12 %.
#[must_use]
pub fn padme(len: u64) -> u64 {
    if len < 2 {
        return len;
    }
    let e = 63 - u64::from(len.leading_zeros()); // floor(log2 len)
    let s = 64 - u64::from(e.leading_zeros()); // floor(log2 e) + 1
    let mask = (1u64 << (e - s)) - 1;
    (len + mask) & !mask
}

/// Độ dài tệp mã hóa cho `len` byte dữ liệu.
#[must_use]
pub fn encrypted_len(len: u64) -> u64 {
    let logical = padme(len + LEN_FIELD);
    let chunks = logical.div_ceil(CHUNK as u64).max(1);
    HEADER_LEN as u64 + logical + chunks * TAG_LEN as u64
}

/// Mã hóa đúng `len` byte đọc từ `reader`, ghi tệp mã hóa vào `writer`. Trả số byte đã ghi.
///
/// # Errors
/// [`VaultError::Io`] khi đọc thiếu/ghi lỗi; [`VaultError::Format`] khi tệp quá lớn (> 2^32 khối).
pub fn encrypt<R: Read, W: Write>(
    key: &Key32,
    context: &[u8],
    len: u64,
    reader: R,
    writer: W,
) -> Result<u64> {
    let mut prefix = [0u8; PREFIX_LEN];
    fill_random(&mut prefix)?;
    encrypt_with_prefix(key, context, len, reader, writer, &prefix)
}

/// Như [`encrypt`] với tiền tố nonce cho trước. CHỈ dùng cho vector đối chiếu.
///
/// # Errors
/// Như [`encrypt`].
#[doc(hidden)]
pub fn encrypt_with_prefix<R: Read, W: Write>(
    key: &Key32,
    context: &[u8],
    len: u64,
    mut reader: R,
    mut writer: W,
    prefix: &[u8; PREFIX_LEN],
) -> Result<u64> {
    let logical = padme(
        len.checked_add(LEN_FIELD)
            .ok_or(VaultError::Format("tệp quá lớn"))?,
    );
    let chunks = logical.div_ceil(CHUNK as u64).max(1);
    if chunks > u64::from(u32::MAX) + 1 {
        return Err(VaultError::Format("tệp quá lớn"));
    }
    let header = header(prefix);
    writer.write_all(&header)?;
    let aad = [header.as_slice(), context].concat();
    let aead = cipher(key);
    let mut buf = Zeroizing::new(vec![0u8; CHUNK]);
    let mut pos = 0u64; // vị trí trong bản rõ logic
    let mut written = HEADER_LEN as u64;
    for i in 0..chunks {
        let n = (logical - pos).min(CHUNK as u64) as usize;
        let chunk = &mut buf[..n];
        chunk.fill(0);
        let mut off = 0usize;
        if pos == 0 {
            chunk[..8].copy_from_slice(&len.to_le_bytes());
            off = 8;
        }
        // Phần dữ liệu thật nằm trong [8, 8 + len) của bản rõ logic.
        let data_end = (len + LEN_FIELD).saturating_sub(pos).min(n as u64) as usize;
        if data_end > off {
            reader.read_exact(&mut chunk[off..data_end])?;
        }
        let nonce = nonce(prefix, i as u32, i + 1 == chunks);
        let ct = aead
            .encrypt(
                &nonce.into(),
                Payload {
                    msg: chunk,
                    aad: &aad,
                },
            )
            .map_err(|_| VaultError::Format("khối quá dài"))?;
        writer.write_all(&ct)?;
        written += ct.len() as u64;
        pos += n as u64;
    }
    writer.flush()?;
    Ok(written)
}

/// Giải mã tệp luồng, ghi dữ liệu thật ra `writer`, trả độ dài dữ liệu.
///
/// Đầu ra được ghi dần: khi trả `Err`, phần đã ghi KHÔNG đáng tin và phải bỏ.
///
/// # Errors
/// [`VaultError::Format`] khi sai đầu tệp; [`VaultError::Decrypt`] khi sai khóa/ngữ cảnh, bị sửa, cắt, đảo khối.
pub fn decrypt<R: Read, W: Write>(key: &Key32, context: &[u8], mut reader: R, mut writer: W) -> Result<u64> {
    let mut header = [0u8; HEADER_LEN];
    reader
        .read_exact(&mut header)
        .map_err(|_| VaultError::Format("tệp mã hóa quá ngắn"))?;
    if header[..2] != MAGIC || header[3] != KIND_STREAM {
        return Err(VaultError::Format("không phải tệp mã hóa Tsu Scan"));
    }
    if header[2] != VERSION || header[HEADER_LEN - 1] != CHUNK_LOG2 {
        return Err(VaultError::Format("phiên bản tệp mã hóa không hỗ trợ"));
    }
    let mut prefix = [0u8; PREFIX_LEN];
    prefix.copy_from_slice(&header[4..4 + PREFIX_LEN]);
    let aad = [header.as_slice(), context].concat();
    let aead = cipher(key);

    let mut cur = vec![0u8; CT_CHUNK];
    let mut next = vec![0u8; CT_CHUNK];
    let mut cur_len = read_full(&mut reader, &mut cur)?;
    let mut index: u64 = 0;
    let mut pos: u64 = 0; // vị trí trong bản rõ logic
    let mut data_len: Option<u64> = None;
    loop {
        let (last, next_len) = if cur_len < CT_CHUNK {
            (true, 0)
        } else {
            let n = read_full(&mut reader, &mut next)?;
            (n == 0, n)
        };
        if cur_len <= TAG_LEN || index > u64::from(u32::MAX) {
            return Err(VaultError::Decrypt);
        }
        let nonce = nonce(&prefix, index as u32, last);
        let pt = Zeroizing::new(
            aead.decrypt(
                &nonce.into(),
                Payload {
                    msg: &cur[..cur_len],
                    aad: &aad,
                },
            )
            .map_err(|_| VaultError::Decrypt)?,
        );
        let mut off = 0usize;
        if index == 0 {
            if pt.len() < 8 {
                return Err(VaultError::Format("thiếu trường độ dài"));
            }
            let mut l = [0u8; 8];
            l.copy_from_slice(&pt[..8]);
            data_len = Some(u64::from_le_bytes(l));
            off = 8;
        }
        let len = data_len.ok_or(VaultError::Format("thiếu trường độ dài"))?;
        let end = (len + LEN_FIELD).saturating_sub(pos).min(pt.len() as u64) as usize;
        if end > off {
            writer.write_all(&pt[off..end])?;
        }
        pos += pt.len() as u64;
        if last {
            break;
        }
        std::mem::swap(&mut cur, &mut next);
        cur_len = next_len;
        index += 1;
    }
    let len = data_len.ok_or(VaultError::Format("thiếu trường độ dài"))?;
    if len.checked_add(LEN_FIELD).map(padme) != Some(pos) {
        return Err(VaultError::Format("độ dài khai báo không khớp"));
    }
    writer.flush()?;
    Ok(len)
}

fn header(prefix: &[u8; PREFIX_LEN]) -> [u8; HEADER_LEN] {
    let mut h = [0u8; HEADER_LEN];
    h[..2].copy_from_slice(&MAGIC);
    h[2] = VERSION;
    h[3] = KIND_STREAM;
    h[4..4 + PREFIX_LEN].copy_from_slice(prefix);
    h[HEADER_LEN - 1] = CHUNK_LOG2;
    h
}

fn nonce(prefix: &[u8; PREFIX_LEN], index: u32, last: bool) -> [u8; 24] {
    let mut n = [0u8; 24];
    n[..PREFIX_LEN].copy_from_slice(prefix);
    n[PREFIX_LEN..PREFIX_LEN + 4].copy_from_slice(&index.to_be_bytes());
    n[23] = u8::from(last);
    n
}

/// Đọc tới khi đầy bộ đệm hoặc hết luồng.
fn read_full<R: Read>(r: &mut R, buf: &mut [u8]) -> Result<usize> {
    let mut n = 0;
    while n < buf.len() {
        match r.read(&mut buf[n..]) {
            Ok(0) => break,
            Ok(k) => n += k,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> Key32 {
        Key32::from_bytes([7; 32])
    }

    fn enc(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        encrypt(&key(), b"fk|1", data.len() as u64, data, &mut out).unwrap_or_default();
        out
    }

    fn dec(ct: &[u8], ctx: &[u8]) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        decrypt(&key(), ctx, ct, &mut out)?;
        Ok(out)
    }

    #[test]
    fn padme_matches_paper_examples() {
        assert_eq!(padme(0), 0);
        assert_eq!(padme(9), 10);
        assert_eq!(padme(100), 104);
        assert_eq!(padme(1000), 1024);
        assert_eq!(padme(65_537), 67_584);
        for l in 2..20_000u64 {
            let p = padme(l);
            assert!(p >= l && (p - l) * 100 <= l * 12 + 100, "{l} -> {p}");
        }
    }

    #[test]
    fn roundtrip_many_sizes_and_length_formula() {
        for n in [
            0usize,
            1,
            8,
            65_527,
            65_528,
            65_536,
            65_537,
            200_000,
            3 * CHUNK + 5,
        ] {
            let data: Vec<u8> = (0..n).map(|i| (i % 251) as u8).collect();
            let ct = enc(&data);
            assert_eq!(ct.len() as u64, encrypted_len(n as u64), "n={n}");
            assert_eq!(dec(&ct, b"fk|1").ok(), Some(data), "n={n}");
        }
    }

    #[test]
    fn rejects_truncation_reorder_append_and_wrong_context() {
        let data: Vec<u8> = (0..(3 * CHUNK)).map(|i| (i % 13) as u8).collect();
        let ct = enc(&data);
        assert!(dec(&ct, b"fk|2").is_err());
        // Cắt đúng ranh giới khối.
        assert!(dec(&ct[..HEADER_LEN + 2 * CT_CHUNK], b"fk|1").is_err());
        assert!(dec(&ct[..HEADER_LEN + CT_CHUNK], b"fk|1").is_err());
        // Đảo khối 0 và 1.
        let mut sw = ct.clone();
        let (a, b) = (HEADER_LEN, HEADER_LEN + CT_CHUNK);
        let k0 = ct[a..b].to_vec();
        sw[a..b].copy_from_slice(&ct[b..b + CT_CHUNK]);
        sw[b..b + CT_CHUNK].copy_from_slice(&k0);
        assert!(dec(&sw, b"fk|1").is_err());
        // Ghép thêm khối cuối lần nữa.
        let mut ap = ct.clone();
        ap.extend_from_slice(&ct[ct.len() - 100..]);
        assert!(dec(&ap, b"fk|1").is_err());
        // Lật một bit đầu tệp/khối.
        for i in [0, 3, 5, HEADER_LEN - 1, HEADER_LEN, ct.len() - 1] {
            let mut t = ct.clone();
            t[i] ^= 1;
            assert!(dec(&t, b"fk|1").is_err(), "byte {i}");
        }
        assert!(dec(&ct[..10], b"fk|1").is_err());
    }

    #[test]
    fn short_reader_is_error() {
        let mut out = Vec::new();
        assert!(encrypt(&key(), b"x", 10, &b"abc"[..], &mut out).is_err());
    }
}

//! Chặn "bom giải nén" trước khi giao PDF cho `hayro` (hayro 0.9 giải nén trọn luồng vào bộ nhớ, không có trần:
//! đo 19/11/2026 - tệp 1 MB làm tiến trình chiếm 1,07 GB).
//!
//! Hai lớp:
//! 1. [`scan_raw`] trên byte thô, TRƯỚC khi mở: mọi đoạn `stream … endstream` có đầu zlib được giải nén thử có trần.
//!    Bắt cả luồng đối tượng nén (ObjStm/XRef) mà hayro phải giải nén ngay lúc mở. Ranh giới lấy theo `endstream`
//!    gần nhất nên có thể bị né bằng cách cài chữ `endstream` vào giữa dữ liệu ⇒ chỉ là lớp ngoài.
//! 2. [`check_objects`] sau khi mở: duyệt mọi đối tượng luồng qua hayro (đúng `/Length`, đã giải mã AES/RC4 kể cả
//!    PDF mật khẩu rỗng), giải nén thử Flate có trần, chặn trên LZW/RunLength theo tỉ lệ nén tối đa của thuật toán,
//!    và kiểm kích thước ảnh khai báo trước khi bộ giải mã ảnh cấp phát.
//!
//! Còn hở (ghi ở phiếu rà soát P3): luồng đối tượng nén BÊN TRONG tệp mã hóa vượt lớp 1 (bản mã không có đầu zlib) và
//! được hayro giải nén trước lớp 2.

use hayro::hayro_syntax::object::{Object, Stream};
use hayro::hayro_syntax::{Filter, Pdf};
use miniz_oxide::inflate::stream::{InflateState, inflate};
use miniz_oxide::{DataFormat, MZError, MZFlush, MZStatus};

use crate::error::{DocError, Result};

/// Dữ liệu giải nén tối đa của MỘT luồng (ảnh 8.000 × 8.000 RGB ≈ 192 MiB vẫn qua).
pub const MAX_STREAM_DECODED: u64 = 256 * 1024 * 1024;
/// Tổng dữ liệu giải nén tối đa của cả tệp.
pub const MAX_TOTAL_DECODED: u64 = 2 * 1024 * 1024 * 1024;
/// Điểm ảnh tối đa của một ảnh trong PDF (bằng giới hạn ảnh của lõi P1).
pub const MAX_IMAGE_PIXELS: f64 = 100_000_000.0;

const LIMIT_STREAM: DocError = DocError::LimitExceeded("PDF có luồng giải nén quá lớn");
const LIMIT_TOTAL: DocError = DocError::LimitExceeded("PDF giải nén vượt tổng giới hạn");

/// Giải nén thử, chỉ đếm byte ra. `Some(n)` = n byte (dừng sớm nếu dữ liệu hỏng); `None` = vượt `cap`.
fn inflate_len(data: &[u8], format: DataFormat, cap: u64) -> Option<u64> {
    let mut state = InflateState::new_boxed(format);
    let mut buf = vec![0u8; 1 << 16];
    let mut input = data;
    let mut total = 0u64;
    loop {
        let r = inflate(&mut state, input, &mut buf, MZFlush::None);
        total += r.bytes_written as u64;
        if total > cap {
            return None;
        }
        input = &input[r.bytes_consumed..];
        match r.status {
            Ok(MZStatus::StreamEnd) | Err(_) => return Some(total),
            Ok(_) if r.bytes_consumed == 0 && r.bytes_written == 0 => return Some(total),
            Ok(_) => {}
        }
        if matches!(r.status, Err(MZError::Buf)) {
            return Some(total);
        }
    }
}

/// Byte giải nén Flate (thử zlib rồi deflate thô như hayro), `None` = vượt trần.
fn flate_len(data: &[u8], cap: u64) -> Option<u64> {
    let z = inflate_len(data, DataFormat::Zlib, cap)?;
    if z > 0 {
        return Some(z);
    }
    inflate_len(data, DataFormat::Raw, cap)
}

fn has_zlib_header(d: &[u8]) -> bool {
    d.len() >= 2 && d[0] & 0x0F == 8 && (u16::from(d[0]) << 8 | u16::from(d[1])) % 31 == 0
}

fn find(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    hay.get(from..)?
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

/// Lớp 1: quét byte thô.
///
/// # Errors
/// [`DocError::LimitExceeded`] khi một luồng hoặc tổng vượt trần.
pub fn scan_raw(bytes: &[u8]) -> Result<()> {
    let mut total = 0u64;
    let mut at = 0;
    while let Some(k) = find(bytes, b"stream", at) {
        let mut start = k + 6;
        // Bỏ qua "endstream" (cũng chứa chữ "stream").
        if k >= 3 && &bytes[k - 3..k] == b"end" {
            at = start;
            continue;
        }
        if bytes.get(start) == Some(&b'\r') {
            start += 1;
        }
        if bytes.get(start) == Some(&b'\n') {
            start += 1;
        }
        let end = find(bytes, b"endstream", start).unwrap_or(bytes.len());
        let data = &bytes[start..end];
        if has_zlib_header(data) {
            let n = flate_len(data, MAX_STREAM_DECODED).ok_or(LIMIT_STREAM)?;
            total += n;
            if total > MAX_TOTAL_DECODED {
                return Err(LIMIT_TOTAL);
            }
        }
        at = end.max(start);
    }
    Ok(())
}

/// Chặn trên kích thước giải mã của một luồng theo chuỗi bộ lọc (byte), `None` = vượt trần.
fn stream_bound(s: &Stream<'_>) -> Option<u64> {
    let raw = s.raw_data();
    let mut size = raw.len() as u64;
    let filters = s.filters();
    for (i, f) in filters.iter().enumerate() {
        size = match f {
            // Flate đầu chuỗi: đo thật. Sau bộ lọc khác: không có dữ liệu trung gian ⇒ chặn theo tỉ lệ tối đa
            // của deflate (1032:1).
            Filter::FlateDecode if i == 0 => flate_len(&raw, MAX_STREAM_DECODED)?,
            Filter::FlateDecode => size.saturating_mul(1032),
            // LZW mã 12 bit: một mã ra tối đa 4.096 byte ⇒ ≤ 4.096 byte / 1,5 byte vào.
            Filter::LzwDecode => size.saturating_mul(2731),
            Filter::RunLengthDecode => size.saturating_mul(64),
            // ASCIIHex/85 chỉ thu nhỏ; bộ giải mã ảnh cấp phát theo kích thước khai báo, kiểm ở `check_image`.
            _ => size,
        };
        if size > MAX_STREAM_DECODED {
            return None;
        }
    }
    Some(size)
}

fn check_image(s: &Stream<'_>) -> Result<()> {
    let d = s.dict();
    let is_image = d
        .get::<hayro::hayro_syntax::object::Name<'_>>(b"Subtype")
        .is_some_and(|n| n.as_ref() == b"Image");
    if !is_image {
        return Ok(());
    }
    let w = d.get::<f64>(b"Width").unwrap_or(0.0);
    let h = d.get::<f64>(b"Height").unwrap_or(0.0);
    if !(w.is_finite() && h.is_finite()) || w < 0.0 || h < 0.0 || w * h > MAX_IMAGE_PIXELS {
        return Err(DocError::LimitExceeded("PDF có ảnh quá lớn"));
    }
    Ok(())
}

/// Lớp 2: duyệt mọi đối tượng luồng của tệp đã mở.
///
/// # Errors
/// [`DocError::LimitExceeded`] khi một luồng/ảnh hoặc tổng vượt trần.
pub fn check_objects(pdf: &Pdf) -> Result<()> {
    let mut total = 0u64;
    for obj in pdf.objects() {
        let Object::Stream(s) = obj else { continue };
        check_image(&s)?;
        total = total.saturating_add(stream_bound(&s).ok_or(LIMIT_STREAM)?);
        if total > MAX_TOTAL_DECODED {
            return Err(LIMIT_TOTAL);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inflate_counts_and_caps() {
        let data = miniz_oxide::deflate::compress_to_vec_zlib(&vec![0u8; 300_000], 9);
        assert_eq!(flate_len(&data, 1 << 20), Some(300_000));
        assert_eq!(flate_len(&data, 1000), None);
        let raw = miniz_oxide::deflate::compress_to_vec(&[7u8; 5000], 6);
        assert_eq!(flate_len(&raw, 1 << 20), Some(5000));
        assert_eq!(flate_len(b"\xff\xfe\xfd rubbish", 1 << 20), Some(0));
        assert!(has_zlib_header(&data) && !has_zlib_header(b"ab"));
    }

    #[test]
    fn raw_scan_finds_bombs() {
        let bomb = miniz_oxide::deflate::compress_to_vec_zlib(&vec![0u8; 300 * 1024 * 1024], 9);
        let mut pdf = b"%PDF-1.7\n1 0 obj\n<< /Length 9 >>\nstream\r\n".to_vec();
        pdf.extend_from_slice(&bomb);
        pdf.extend_from_slice(b"\nendstream\nendobj\n");
        assert!(matches!(scan_raw(&pdf), Err(DocError::LimitExceeded(_))));
        assert!(scan_raw(b"%PDF-1.7 stream\nabc endstream stream").is_ok());
    }
}

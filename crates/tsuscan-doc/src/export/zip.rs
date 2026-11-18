//! Bộ ghi ZIP tối giản cho OOXML (DOCX/XLSX): nén Deflate, ngày cố định 1980-01-01 (tệp tất định, không lộ giờ
//! quét). Không đọc ZIP ⇒ không có bề mặt tấn công giải nén.

use crate::error::{DocError, Result};

/// Một mục trong tệp ZIP.
struct Entry {
    name: String,
    crc: u32,
    compressed: u32,
    size: u32,
    offset: u32,
    method: u16,
}

/// Bộ ghi ZIP trong bộ nhớ.
#[derive(Default)]
pub struct ZipWriter {
    out: Vec<u8>,
    entries: Vec<Entry>,
}

fn u16le(o: &mut Vec<u8>, v: u16) {
    o.extend_from_slice(&v.to_le_bytes());
}
fn u32le(o: &mut Vec<u8>, v: u32) {
    o.extend_from_slice(&v.to_le_bytes());
}

/// Ngày DOS 1980-01-01, giờ 00:00.
const DOS_DATE: u16 = (1 << 5) | 1;

impl ZipWriter {
    /// Bộ ghi rỗng.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Thêm tệp (nén Deflate; tệp đã nén sẵn như JPEG/PNG thì lưu nguyên).
    ///
    /// # Errors
    /// [`DocError::LimitExceeded`] khi tệp/kho vượt 4 GiB (không dùng ZIP64).
    pub fn add(&mut self, name: &str, data: &[u8]) -> Result<()> {
        let precompressed = [".png", ".jpg", ".jpeg", ".webp"]
            .iter()
            .any(|e| name.to_ascii_lowercase().ends_with(e));
        let (method, body) = if precompressed {
            (0u16, data.to_vec())
        } else {
            (8u16, miniz_oxide::deflate::compress_to_vec(data, 6))
        };
        let too_big = || DocError::LimitExceeded("ZIP vượt 4 GiB");
        let size = u32::try_from(data.len()).map_err(|_| too_big())?;
        let compressed = u32::try_from(body.len()).map_err(|_| too_big())?;
        let offset = u32::try_from(self.out.len()).map_err(|_| too_big())?;
        let name_len = u16::try_from(name.len()).map_err(|_| DocError::InvalidArgument("tên tệp quá dài"))?;
        let crc = crc32fast::hash(data);
        let o = &mut self.out;
        u32le(o, 0x0403_4b50);
        u16le(o, 20); // phiên bản cần
        u16le(o, 1 << 11); // cờ: tên UTF-8
        u16le(o, method);
        u16le(o, 0); // giờ
        u16le(o, DOS_DATE);
        u32le(o, crc);
        u32le(o, compressed);
        u32le(o, size);
        u16le(o, name_len);
        u16le(o, 0);
        o.extend_from_slice(name.as_bytes());
        o.extend_from_slice(&body);
        self.entries.push(Entry {
            name: name.to_owned(),
            crc,
            compressed,
            size,
            offset,
            method,
        });
        Ok(())
    }

    /// Ghi thư mục trung tâm và trả toàn bộ tệp.
    ///
    /// # Errors
    /// [`DocError::LimitExceeded`] khi quá 65.535 mục hoặc vượt 4 GiB.
    pub fn finish(mut self) -> Result<Vec<u8>> {
        let too_big = || DocError::LimitExceeded("ZIP vượt giới hạn");
        let cd_start = u32::try_from(self.out.len()).map_err(|_| too_big())?;
        let count = u16::try_from(self.entries.len()).map_err(|_| too_big())?;
        for e in &self.entries {
            let o = &mut self.out;
            u32le(o, 0x0201_4b50);
            u16le(o, 20); // tạo bởi
            u16le(o, 20); // cần
            u16le(o, 1 << 11);
            u16le(o, e.method);
            u16le(o, 0);
            u16le(o, DOS_DATE);
            u32le(o, e.crc);
            u32le(o, e.compressed);
            u32le(o, e.size);
            u16le(o, e.name.len() as u16);
            u16le(o, 0); // extra
            u16le(o, 0); // comment
            u16le(o, 0); // đĩa
            u16le(o, 0); // thuộc tính trong
            u32le(o, 0); // thuộc tính ngoài
            u32le(o, e.offset);
            o.extend_from_slice(e.name.as_bytes());
        }
        let cd_len = u32::try_from(self.out.len()).map_err(|_| too_big())? - cd_start;
        let o = &mut self.out;
        u32le(o, 0x0605_4b50);
        u16le(o, 0);
        u16le(o, 0);
        u16le(o, count);
        u16le(o, count);
        u32le(o, cd_len);
        u32le(o, cd_start);
        u16le(o, 0);
        Ok(self.out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_valid_structure() {
        let mut z = ZipWriter::new();
        z.add("a.xml", b"<a/>").unwrap();
        z.add("media/x.png", &[1, 2, 3]).unwrap();
        let b = z.finish().unwrap();
        assert_eq!(&b[..4], b"PK\x03\x04");
        let eocd = b.len() - 22;
        assert_eq!(&b[eocd..eocd + 4], b"PK\x05\x06");
        assert_eq!(u16::from_le_bytes([b[eocd + 10], b[eocd + 11]]), 2);
        // Dữ liệu deflate thô giải nén lại đúng.
        let body_start = 30 + 5;
        let clen = u32::from_le_bytes([b[18], b[19], b[20], b[21]]) as usize;
        let raw = miniz_oxide::inflate::decompress_to_vec(&b[body_start..body_start + clen]).unwrap();
        assert_eq!(raw, b"<a/>");
    }
}

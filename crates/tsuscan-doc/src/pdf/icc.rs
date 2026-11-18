//! Hồ sơ màu ICC v2 sRGB tự sinh (OutputIntent bắt buộc của PDF/A khi dùng DeviceRGB/DeviceGray).
//!
//! Dựng từ số liệu chuẩn IEC 61966-2-1 (cơ bản đã thích nghi D50, đường cong 1.024 điểm), không nhúng tệp
//! hồ sơ của bên thứ ba.

fn be16(o: &mut Vec<u8>, v: u16) {
    o.extend_from_slice(&v.to_be_bytes());
}
fn be32(o: &mut Vec<u8>, v: u32) {
    o.extend_from_slice(&v.to_be_bytes());
}
fn s15f16(o: &mut Vec<u8>, v: f64) {
    be32(o, (v * 65_536.0).round() as i32 as u32);
}

fn xyz(x: f64, y: f64, z: f64) -> Vec<u8> {
    let mut t = b"XYZ \0\0\0\0".to_vec();
    s15f16(&mut t, x);
    s15f16(&mut t, y);
    s15f16(&mut t, z);
    t
}

fn desc(text: &str) -> Vec<u8> {
    let mut t = b"desc\0\0\0\0".to_vec();
    be32(&mut t, text.len() as u32 + 1);
    t.extend_from_slice(text.as_bytes());
    t.push(0);
    be32(&mut t, 0); // Unicode language
    be32(&mut t, 0); // Unicode count
    be16(&mut t, 0); // ScriptCode code
    t.push(0); // ScriptCode count
    t.extend_from_slice(&[0; 67]);
    t
}

fn text(s: &str) -> Vec<u8> {
    let mut t = b"text\0\0\0\0".to_vec();
    t.extend_from_slice(s.as_bytes());
    t.push(0);
    t
}

fn srgb_curve() -> Vec<u8> {
    const N: u32 = 1024;
    let mut t = b"curv\0\0\0\0".to_vec();
    be32(&mut t, N);
    for i in 0..N {
        let v = f64::from(i) / f64::from(N - 1);
        let lin = if v <= 0.040_45 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        };
        be16(&mut t, (lin * 65_535.0).round() as u16);
    }
    t
}

/// Sinh hồ sơ sRGB ICC v2.1 (lớp màn hình, không gian RGB, PCS XYZ).
#[must_use]
pub fn srgb_profile() -> Vec<u8> {
    let trc = srgb_curve();
    // (thẻ, dữ liệu); ba TRC dùng chung một vùng dữ liệu.
    let elements: Vec<(&[u8; 4], Vec<u8>)> = vec![
        (b"desc", desc("sRGB IEC61966-2.1 (Tsu Scan)")),
        (b"cprt", text("No copyright, use freely")),
        (b"wtpt", xyz(0.9505, 1.0, 1.0891)),
        (b"rXYZ", xyz(0.436_07, 0.222_49, 0.013_92)),
        (b"gXYZ", xyz(0.385_15, 0.716_87, 0.097_08)),
        (b"bXYZ", xyz(0.143_07, 0.060_61, 0.714_10)),
        (b"rTRC", trc),
    ];
    let tag_count = elements.len() + 2; // + gTRC, bTRC trỏ chung
    let mut data = Vec::new();
    let mut table = Vec::new();
    let base = 128 + 4 + 12 * tag_count;
    let mut trc_entry = (0u32, 0u32);
    for (tag, el) in &elements {
        let off = (base + data.len()) as u32;
        let len = el.len() as u32;
        table.push((**tag, off, len));
        if *tag == b"rTRC" {
            trc_entry = (off, len);
        }
        data.extend_from_slice(el);
        while data.len() % 4 != 0 {
            data.push(0);
        }
    }
    table.push((*b"gTRC", trc_entry.0, trc_entry.1));
    table.push((*b"bTRC", trc_entry.0, trc_entry.1));

    let total = (base + data.len()) as u32;
    let mut p = Vec::with_capacity(total as usize);
    be32(&mut p, total);
    be32(&mut p, 0); // CMM: không chỉ định
    be32(&mut p, 0x0210_0000); // phiên bản 2.1
    p.extend_from_slice(b"mntrRGB XYZ ");
    for v in [2026u16, 1, 1, 0, 0, 0] {
        be16(&mut p, v);
    }
    p.extend_from_slice(b"acsp");
    be32(&mut p, 0); // nền tảng
    be32(&mut p, 0); // cờ
    be32(&mut p, 0); // nhà sản xuất thiết bị
    be32(&mut p, 0); // mẫu thiết bị
    p.extend_from_slice(&[0; 8]); // thuộc tính
    be32(&mut p, 0); // ý định: perceptual
    s15f16(&mut p, 0.9642);
    s15f16(&mut p, 1.0);
    s15f16(&mut p, 0.8249);
    be32(&mut p, 0); // người tạo
    p.extend_from_slice(&[0; 44]);
    be32(&mut p, tag_count as u32);
    for (tag, off, len) in table {
        p.extend_from_slice(&tag);
        be32(&mut p, off);
        be32(&mut p, len);
    }
    p.extend_from_slice(&data);
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_header() {
        let p = srgb_profile();
        assert_eq!(u32::from_be_bytes([p[0], p[1], p[2], p[3]]) as usize, p.len());
        assert_eq!(&p[4..8], &[0, 0, 0, 0]);
        assert_eq!(&p[12..24], b"mntrRGB XYZ ");
        assert_eq!(&p[36..40], b"acsp");
        assert_eq!(u32::from_be_bytes([p[128], p[129], p[130], p[131]]), 9);
        assert_eq!(p.len() % 4, 0);
    }
}

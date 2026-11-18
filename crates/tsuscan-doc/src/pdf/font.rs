//! Phông "không nét" cho lớp chữ ẩn: TrueType 2 glyph rỗng, mọi mã CID → glyph 1 (rộng 500/1000 em).
//!
//! Cách làm giống OCRmyPDF/Tesseract: chữ OCR vẽ ở chế độ tô 3 (không hiện) bằng phông Type0 Identity-H, mã CID =
//! điểm mã Unicode (BMP), bản đồ ToUnicode đồng nhất ⇒ tìm kiếm/chép chữ đúng tiếng Việt có dấu mà không cần nhúng
//! phông thật (vài trăm byte thay vì vài MB). Phông tự sinh ở đây, không lấy tệp của bên thứ ba.

/// Độ rộng mọi glyph theo đơn vị 1/1000 em (khớp `DW` trong PDF).
pub const GLYPH_WIDTH: u16 = 500;
/// Tên phông (PostScript).
pub const FONT_NAME: &str = "TsuScanGlyphless";

fn be16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_be_bytes());
}
fn be32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_be_bytes());
}
fn bei16(out: &mut Vec<u8>, v: i16) {
    out.extend_from_slice(&v.to_be_bytes());
}

fn checksum(data: &[u8]) -> u32 {
    data.chunks(4).fold(0u32, |acc, c| {
        let mut w = [0u8; 4];
        w[..c.len()].copy_from_slice(c);
        acc.wrapping_add(u32::from_be_bytes(w))
    })
}

fn head() -> Vec<u8> {
    let mut t = Vec::with_capacity(54);
    be32(&mut t, 0x0001_0000); // version
    be32(&mut t, 0x0001_0000); // fontRevision
    be32(&mut t, 0); // checkSumAdjustment (điền sau)
    be32(&mut t, 0x5F0F_3CF5); // magic
    be16(&mut t, 0b1011); // flags: baseline y=0, lsb x=0, ppem nguyên
    be16(&mut t, 1000); // unitsPerEm
    t.extend_from_slice(&[0; 16]); // created + modified
    bei16(&mut t, 0); // xMin
    bei16(&mut t, 0); // yMin
    bei16(&mut t, GLYPH_WIDTH as i16); // xMax
    bei16(&mut t, 1000); // yMax
    be16(&mut t, 0); // macStyle
    be16(&mut t, 3); // lowestRecPPEM
    bei16(&mut t, 2); // fontDirectionHint
    bei16(&mut t, 0); // indexToLocFormat: short
    bei16(&mut t, 0); // glyphDataFormat
    t
}

fn hhea() -> Vec<u8> {
    let mut t = Vec::with_capacity(36);
    be32(&mut t, 0x0001_0000);
    bei16(&mut t, 1000); // ascender
    bei16(&mut t, 0); // descender
    bei16(&mut t, 0); // lineGap
    be16(&mut t, GLYPH_WIDTH); // advanceWidthMax
    bei16(&mut t, 0); // minLeftSideBearing
    bei16(&mut t, 0); // minRightSideBearing
    bei16(&mut t, 0); // xMaxExtent
    bei16(&mut t, 1); // caretSlopeRise
    bei16(&mut t, 0); // caretSlopeRun
    bei16(&mut t, 0); // caretOffset
    t.extend_from_slice(&[0; 8]); // reserved
    bei16(&mut t, 0); // metricDataFormat
    be16(&mut t, 2); // numberOfHMetrics
    t
}

fn maxp() -> Vec<u8> {
    let mut t = Vec::with_capacity(32);
    be32(&mut t, 0x0001_0000);
    be16(&mut t, 2); // numGlyphs
    for v in [0u16, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0] {
        be16(&mut t, v);
    }
    t
}

fn os2() -> Vec<u8> {
    let mut t = Vec::with_capacity(96);
    be16(&mut t, 4); // version
    bei16(&mut t, GLYPH_WIDTH as i16); // xAvgCharWidth
    be16(&mut t, 400); // usWeightClass
    be16(&mut t, 5); // usWidthClass
    be16(&mut t, 0); // fsType: cho phép nhúng (PDF/A yêu cầu nhúng hợp lệ)
    for v in [650i16, 600, 0, 75, 650, 600, 0, 350, 50, 250] {
        bei16(&mut t, v); // sub/superscript, strikeout
    }
    bei16(&mut t, 0); // sFamilyClass
    t.extend_from_slice(&[0; 10]); // panose
    t.extend_from_slice(&[0; 16]); // ulUnicodeRange1-4
    t.extend_from_slice(b"TSUD"); // achVendID
    be16(&mut t, 0x0040); // fsSelection: REGULAR
    be16(&mut t, 0x0020); // usFirstCharIndex
    be16(&mut t, 0xFFFD); // usLastCharIndex
    bei16(&mut t, 1000); // sTypoAscender
    bei16(&mut t, 0); // sTypoDescender
    bei16(&mut t, 0); // sTypoLineGap
    be16(&mut t, 1000); // usWinAscent
    be16(&mut t, 0); // usWinDescent
    be32(&mut t, 1); // ulCodePageRange1: Latin 1
    be32(&mut t, 0); // ulCodePageRange2
    bei16(&mut t, 500); // sxHeight
    bei16(&mut t, 1000); // sCapHeight
    be16(&mut t, 0); // usDefaultChar
    be16(&mut t, 0x20); // usBreakChar
    be16(&mut t, 0); // usMaxContext
    t
}

fn hmtx() -> Vec<u8> {
    let mut t = Vec::with_capacity(8);
    for _ in 0..2 {
        be16(&mut t, GLYPH_WIDTH);
        bei16(&mut t, 0);
    }
    t
}

/// cmap định dạng 4 (3,1) chỉ có đoạn kết thúc bắt buộc: phông CID dùng CIDToGIDMap, không tra cmap.
fn cmap() -> Vec<u8> {
    let mut t = Vec::new();
    be16(&mut t, 0); // version
    be16(&mut t, 1); // numTables
    be16(&mut t, 3); // platform Windows
    be16(&mut t, 1); // encoding Unicode BMP
    be32(&mut t, 12); // offset
    // Bảng con định dạng 4, 1 đoạn.
    be16(&mut t, 4); // format
    be16(&mut t, 24); // length
    be16(&mut t, 0); // language
    be16(&mut t, 2); // segCountX2
    be16(&mut t, 2); // searchRange
    be16(&mut t, 0); // entrySelector
    be16(&mut t, 0); // rangeShift
    be16(&mut t, 0xFFFF); // endCode
    be16(&mut t, 0); // reservedPad
    be16(&mut t, 0xFFFF); // startCode
    bei16(&mut t, 1); // idDelta
    be16(&mut t, 0); // idRangeOffset
    t
}

fn post() -> Vec<u8> {
    let mut t = Vec::with_capacity(32);
    be32(&mut t, 0x0003_0000); // version 3: không tên glyph
    be32(&mut t, 0); // italicAngle
    bei16(&mut t, -100); // underlinePosition
    bei16(&mut t, 50); // underlineThickness
    be32(&mut t, 1); // isFixedPitch
    t.extend_from_slice(&[0; 16]);
    t
}

fn name() -> Vec<u8> {
    let records: [(u16, &str); 6] = [
        (1, FONT_NAME),
        (2, "Regular"),
        (3, "TsuScan:Glyphless:1.0"),
        (4, FONT_NAME),
        (5, "Version 1.0"),
        (6, FONT_NAME),
    ];
    let strings: Vec<Vec<u8>> = records
        .iter()
        .map(|(_, s)| s.encode_utf16().flat_map(u16::to_be_bytes).collect())
        .collect();
    let mut t = Vec::new();
    be16(&mut t, 0); // format
    be16(&mut t, records.len() as u16);
    be16(&mut t, (6 + 12 * records.len()) as u16); // stringOffset
    let mut off = 0u16;
    for ((id, _), s) in records.iter().zip(&strings) {
        be16(&mut t, 3); // Windows
        be16(&mut t, 1); // Unicode BMP
        be16(&mut t, 0x0409); // en-US
        be16(&mut t, *id);
        be16(&mut t, s.len() as u16);
        be16(&mut t, off);
        off += s.len() as u16;
    }
    for s in strings {
        t.extend_from_slice(&s);
    }
    t
}

/// Sinh tệp TrueType phông không nét (thứ tự bảng theo thẻ, căn 4 byte, checksum đúng chuẩn).
#[must_use]
pub fn glyphless_ttf() -> Vec<u8> {
    // glyf rỗng (2 glyph không đường nét), loca ngắn: 3 offset 0.
    let tables: [(&[u8; 4], Vec<u8>); 10] = [
        (b"OS/2", os2()),
        (b"cmap", cmap()),
        (b"glyf", Vec::new()),
        (b"head", head()),
        (b"hhea", hhea()),
        (b"hmtx", hmtx()),
        (b"loca", vec![0; 6]),
        (b"maxp", maxp()),
        (b"name", name()),
        (b"post", post()),
    ];
    let n = tables.len() as u16;
    let entry_selector = 15 - n.leading_zeros() as u16; // log2 sàn
    let search_range = (1u16 << entry_selector) * 16;
    let mut out = Vec::new();
    be32(&mut out, 0x0001_0000);
    be16(&mut out, n);
    be16(&mut out, search_range);
    be16(&mut out, entry_selector);
    be16(&mut out, n * 16 - search_range);
    let mut offset = 12 + 16 * u32::from(n);
    let mut body = Vec::new();
    let mut head_offset = 0usize;
    for (tag, data) in &tables {
        out.extend_from_slice(*tag);
        be32(&mut out, checksum(data));
        be32(&mut out, offset);
        be32(&mut out, data.len() as u32);
        if *tag == b"head" {
            head_offset = offset as usize;
        }
        body.extend_from_slice(data);
        while body.len() % 4 != 0 {
            body.push(0);
        }
        offset = 12 + 16 * u32::from(n) + body.len() as u32;
    }
    out.extend_from_slice(&body);
    let adj = 0xB1B0_AFBAu32.wrapping_sub(checksum(&out));
    out[head_offset + 8..head_offset + 12].copy_from_slice(&adj.to_be_bytes());
    out
}

/// Bảng CIDToGIDMap: 65.536 mã × 2 byte, mọi mã khác 0 → glyph 1 (nén Flate còn vài trăm byte).
#[must_use]
pub fn cid_to_gid_map() -> Vec<u8> {
    let mut m = vec![0u8; 65_536 * 2];
    for pair in m.as_chunks_mut::<2>().0.iter_mut().skip(1) {
        pair[1] = 1;
    }
    m
}

/// CMap ToUnicode đồng nhất cho mã 2 byte (bỏ vùng surrogate D800-DFFF).
#[must_use]
pub fn to_unicode_cmap() -> Vec<u8> {
    let mut s = String::from(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
         /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
         /CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n\
         1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n",
    );
    let hi: Vec<u32> = (0u32..=0xFF).filter(|h| !(0xD8..=0xDF).contains(h)).collect();
    // Mỗi khối beginbfrange tối đa 100 mục (giới hạn của PostScript CMap).
    for chunk in hi.chunks(100) {
        s.push_str(&format!("{} beginbfrange\n", chunk.len()));
        for h in chunk {
            s.push_str(&format!("<{h:02X}00> <{h:02X}FF> <{h:02X}00>\n"));
        }
        s.push_str("endbfrange\n");
    }
    s.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
    s.into_bytes()
}

/// Mã hóa chữ thành mã CID 2 byte (UTF-16BE, ký tự ngoài BMP → U+FFFD).
#[must_use]
pub fn encode_text(text: &str) -> Vec<u8> {
    text.chars()
        .flat_map(|c| {
            let u = u32::from(c);
            let v = if u <= 0xFFFF { u as u16 } else { 0xFFFD };
            v.to_be_bytes()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ttf_structure_and_checksum() {
        let f = glyphless_ttf();
        assert_eq!(&f[..4], &[0, 1, 0, 0]);
        assert_eq!(u16::from_be_bytes([f[4], f[5]]), 10);
        assert_eq!(f.len() % 4, 0);
        // Tổng checksum cả tệp với adjustment đúng = 0xB1B0AFBA.
        assert_eq!(checksum(&f), 0xB1B0_AFBA);
        // Thẻ bảng tăng dần (yêu cầu tìm nhị phân).
        let tags: Vec<&[u8]> = (0..10).map(|i| &f[12 + 16 * i..16 + 16 * i]).collect();
        assert!(tags.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn cmaps_and_text() {
        let m = cid_to_gid_map();
        assert_eq!(m.len(), 131_072);
        assert_eq!(&m[..4], &[0, 0, 0, 1]);
        let c = String::from_utf8(to_unicode_cmap()).unwrap();
        assert!(c.contains("<1E00> <1EFF> <1E00>"));
        assert!(!c.contains("<D800>"));
        assert_eq!(encode_text("Việt"), vec![0, 0x56, 0, 0x69, 0x1E, 0xC7, 0, 0x74]);
        assert_eq!(encode_text("😀"), vec![0xFF, 0xFD]);
    }
}

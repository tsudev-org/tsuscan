//! Xuất XLSX (SpreadsheetML): mỗi bảng nhận ra trên các trang là một trang tính "Bảng N". Tài liệu không có bảng
//! ⇒ một trang tính "Văn bản" (cột Trang, Nội dung) để vẫn lọc/tìm được trong Excel.
//!
//! Ô dạng số kiểu Việt Nam ("1.234.567", "12,5", "-3") ghi thành SỐ để cộng trừ được; chuỗi số có 0 đứng đầu
//! (số CCCD, điện thoại, mã bưu chính) giữ nguyên dạng CHỮ để không mất số 0.

use tsuscan_ml::layout::Block;

use super::docx::{doc_props, root_rels};
use super::page_blocks;
use super::zip::ZipWriter;
use crate::error::Result;
use crate::model::Document;
use crate::pdf::xmp::xml_escape;

/// Đọc số theo cách viết Việt Nam (dấu chấm ngăn nghìn, dấu phẩy thập phân). `None` nếu không phải số.
#[must_use]
pub fn parse_vi_number(s: &str) -> Option<f64> {
    let t = s.trim();
    let (neg, body) = t.strip_prefix('-').map_or((false, t), |r| (true, r));
    if body.is_empty() || body.len() > 24 {
        return None;
    }
    let (int_part, frac) = match body.split_once(',') {
        Some((i, f)) => (i, Some(f)),
        None => (body, None),
    };
    if let Some(f) = frac
        && (f.is_empty() || !f.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    let groups: Vec<&str> = int_part.split('.').collect();
    let valid_groups = !groups[0].is_empty()
        && groups
            .iter()
            .all(|g| !g.is_empty() && g.bytes().all(|b| b.is_ascii_digit()))
        && (groups.len() == 1 || (groups[0].len() <= 3 && groups[1..].iter().all(|g| g.len() == 3)));
    if !valid_groups {
        return None;
    }
    let digits: String = groups.concat();
    // Số 0 đứng đầu (trừ "0" và "0,5") là mã định danh, không phải số.
    if digits.len() > 1 && digits.starts_with('0') {
        return None;
    }
    // Quá 15 chữ số có nghĩa thì Excel làm tròn ⇒ giữ dạng chữ.
    if digits.len() + frac.map_or(0, str::len) > 15 {
        return None;
    }
    let v: f64 = format!("{digits}.{}", frac.unwrap_or("0")).parse().ok()?;
    Some(if neg { -v } else { v })
}

/// Tên cột Excel từ chỉ số 0 (A, B, ..., Z, AA, ...).
#[must_use]
pub fn column_name(mut i: usize) -> String {
    let mut s = Vec::new();
    loop {
        s.push(b'A' + (i % 26) as u8);
        if i < 26 {
            break;
        }
        i = i / 26 - 1;
    }
    s.reverse();
    String::from_utf8(s).unwrap_or_default()
}

fn cell(r: usize, c: usize, v: &str, style: u8) -> String {
    let rf = format!("{}{}", column_name(c), r + 1);
    if let Some(n) = parse_vi_number(v) {
        format!("<c r=\"{rf}\" s=\"{style}\"><v>{n}</v></c>")
    } else if v.is_empty() {
        String::new()
    } else {
        format!(
            "<c r=\"{rf}\" s=\"{style}\" t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>",
            xml_escape(v)
        )
    }
}

fn sheet(rows: &[Vec<String>], header: bool) -> String {
    let cols = rows.iter().map(Vec::len).max().unwrap_or(1).max(1);
    let mut s = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<worksheet \
xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">",
    );
    if header {
        s.push_str("<sheetViews><sheetView workbookViewId=\"0\"><pane ySplit=\"1\" topLeftCell=\"A2\" activePane=\"bottomLeft\" state=\"frozen\"/></sheetView></sheetViews>");
    }
    s.push_str("<cols>");
    for c in 0..cols {
        let width = rows
            .iter()
            .filter_map(|r| r.get(c))
            .map(|v| v.chars().count())
            .max()
            .unwrap_or(8)
            .clamp(6, 80);
        s.push_str(&format!(
            "<col min=\"{n}\" max=\"{n}\" width=\"{}\" customWidth=\"1\"/>",
            width + 2,
            n = c + 1
        ));
    }
    s.push_str("</cols><sheetData>");
    for (ri, r) in rows.iter().enumerate() {
        s.push_str(&format!("<row r=\"{}\">", ri + 1));
        let style = u8::from(header && ri == 0);
        for (ci, v) in r.iter().enumerate() {
            s.push_str(&cell(ri, ci, v, style));
        }
        s.push_str("</row>");
    }
    s.push_str("</sheetData></worksheet>");
    s
}

const STYLES: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<styleSheet \
xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">\
<fonts count=\"2\"><font><sz val=\"11\"/><name val=\"Calibri\"/></font><font><b/><sz val=\"11\"/><name val=\"Calibri\"/></font></fonts>\
<fills count=\"2\"><fill><patternFill patternType=\"none\"/></fill><fill><patternFill patternType=\"gray125\"/></fill></fills>\
<borders count=\"1\"><border><left/><right/><top/><bottom/><diagonal/></border></borders>\
<cellStyleXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/></cellStyleXfs>\
<cellXfs count=\"2\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\"/>\
<xf numFmtId=\"0\" fontId=\"1\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyFont=\"1\"/></cellXfs>\
<cellStyles count=\"1\"><cellStyle name=\"Normal\" xfId=\"0\" builtinId=\"0\"/></cellStyles></styleSheet>";

/// Xuất XLSX.
///
/// # Errors
/// [`crate::DocError::Empty`] khi không có trang; lỗi ZIP vượt giới hạn.
pub fn to_xlsx(doc: &Document) -> Result<Vec<u8>> {
    doc.ensure_pages()?;
    let mut sheets: Vec<(String, String)> = Vec::new();
    let mut text_rows: Vec<Vec<String>> = vec![vec!["Trang".into(), "Nội dung".into()]];
    for (pi, page) in doc.pages.iter().enumerate() {
        for b in page_blocks(page) {
            match b {
                Block::Table(rows) => {
                    let name = format!("Bảng {}", sheets.len() + 1);
                    sheets.push((name, sheet(&rows, true)));
                }
                Block::Heading(t) | Block::Paragraph(t) => {
                    text_rows.push(vec![(pi + 1).to_string(), t]);
                }
            }
        }
    }
    if sheets.is_empty() || text_rows.len() > 1 {
        sheets.push(("Văn bản".into(), sheet(&text_rows, true)));
    }
    let mut wb = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<workbook \
xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" \
xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><sheets>",
    );
    let mut rels = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships \
xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">",
    );
    let mut types = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types \
xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
<Default Extension=\"xml\" ContentType=\"application/xml\"/>\
<Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/>\
<Override PartName=\"/xl/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml\"/>\
<Override PartName=\"/docProps/core.xml\" ContentType=\"application/vnd.openxmlformats-package.core-properties+xml\"/>\
<Override PartName=\"/docProps/app.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.extended-properties+xml\"/>",
    );
    for (i, (name, _)) in sheets.iter().enumerate() {
        let n = i + 1;
        wb.push_str(&format!(
            "<sheet name=\"{}\" sheetId=\"{n}\" r:id=\"rId{n}\"/>",
            xml_escape(name)
        ));
        rels.push_str(&format!(
            "<Relationship Id=\"rId{n}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet{n}.xml\"/>"
        ));
        types.push_str(&format!(
            "<Override PartName=\"/xl/worksheets/sheet{n}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>"
        ));
    }
    wb.push_str("</sheets></workbook>");
    rels.push_str(&format!(
        "<Relationship Id=\"rId{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/></Relationships>",
        sheets.len() + 1
    ));
    types.push_str("</Types>");
    let (core, app) = doc_props(doc, "16.0300");
    let mut z = ZipWriter::new();
    z.add("[Content_Types].xml", types.as_bytes())?;
    z.add("_rels/.rels", root_rels("xl/workbook.xml").as_bytes())?;
    z.add("xl/workbook.xml", wb.as_bytes())?;
    z.add("xl/_rels/workbook.xml.rels", rels.as_bytes())?;
    z.add("xl/styles.xml", STYLES.as_bytes())?;
    for (i, (_, xml)) in sheets.iter().enumerate() {
        z.add(&format!("xl/worksheets/sheet{}.xml", i + 1), xml.as_bytes())?;
    }
    z.add("docProps/core.xml", core.as_bytes())?;
    z.add("docProps/app.xml", app.as_bytes())?;
    z.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vi_numbers() {
        assert_eq!(parse_vi_number("1.234.567"), Some(1_234_567.0));
        assert_eq!(parse_vi_number("12,5"), Some(12.5));
        assert_eq!(parse_vi_number("-3"), Some(-3.0));
        assert_eq!(parse_vi_number("0"), Some(0.0));
        assert_eq!(parse_vi_number("0,25"), Some(0.25));
        assert_eq!(parse_vi_number("001234567890"), None); // CCCD
        assert_eq!(parse_vi_number("0912345678"), None); // điện thoại
        assert_eq!(parse_vi_number("1.23"), None);
        assert_eq!(parse_vi_number("12.34.567"), None);
        assert_eq!(parse_vi_number("1,"), None);
        assert_eq!(parse_vi_number("abc"), None);
        assert_eq!(parse_vi_number(""), None);
        assert_eq!(parse_vi_number("1234567890123456"), None);
    }

    #[test]
    fn columns() {
        assert_eq!(column_name(0), "A");
        assert_eq!(column_name(25), "Z");
        assert_eq!(column_name(26), "AA");
        assert_eq!(column_name(701), "ZZ");
        assert_eq!(column_name(702), "AAA");
    }
}

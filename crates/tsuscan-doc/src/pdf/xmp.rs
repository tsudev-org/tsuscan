//! Gói XMP cho PDF/A-2b: khớp từng trường với từ điển Info (veraPDF kiểm khớp).

use crate::model::{Metadata, PRODUCER, Timestamp};

/// Thoát ký tự XML (kể cả nháy) và bỏ ký tự điều khiển không hợp lệ trong XML 1.0.
#[must_use]
pub fn xml_escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&apos;"),
            '\t' | '\n' | '\r' => o.push(c),
            c if (c as u32) < 0x20 || c == '\u{FFFE}' || c == '\u{FFFF}' => {}
            c => o.push(c),
        }
    }
    o
}

/// Sinh gói XMP. `pdfa` = thêm khai báo `pdfaid` phần 2 mức B.
#[must_use]
pub fn packet(meta: &Metadata, created: Timestamp, pdfa: bool) -> Vec<u8> {
    let date = created.iso8601();
    let mut s = String::from(
        "<?xpacket begin=\"\u{FEFF}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
         <x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n\
         <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
         <rdf:Description rdf:about=\"\"\n \
         xmlns:dc=\"http://purl.org/dc/elements/1.1/\"\n \
         xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\"\n \
         xmlns:pdf=\"http://ns.adobe.com/pdf/1.3/\"\n \
         xmlns:pdfaid=\"http://www.aiim.org/pdfa/ns/id/\">\n",
    );
    s.push_str("<dc:format>application/pdf</dc:format>\n");
    if !meta.title.is_empty() {
        s.push_str(&format!(
            "<dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">{}</rdf:li></rdf:Alt></dc:title>\n",
            xml_escape(&meta.title)
        ));
    }
    if !meta.author.is_empty() {
        s.push_str(&format!(
            "<dc:creator><rdf:Seq><rdf:li>{}</rdf:li></rdf:Seq></dc:creator>\n",
            xml_escape(&meta.author)
        ));
    }
    if !meta.subject.is_empty() {
        s.push_str(&format!(
            "<dc:description><rdf:Alt><rdf:li xml:lang=\"x-default\">{}</rdf:li></rdf:Alt></dc:description>\n",
            xml_escape(&meta.subject)
        ));
    }
    if !meta.keywords.is_empty() {
        s.push_str(&format!(
            "<pdf:Keywords>{}</pdf:Keywords>\n",
            xml_escape(&meta.keywords.join(", "))
        ));
    }
    s.push_str(&format!(
        "<pdf:Producer>{p}</pdf:Producer>\n<xmp:CreatorTool>{p}</xmp:CreatorTool>\n\
         <xmp:CreateDate>{date}</xmp:CreateDate>\n<xmp:ModifyDate>{date}</xmp:ModifyDate>\n\
         <xmp:MetadataDate>{date}</xmp:MetadataDate>\n",
        p = xml_escape(PRODUCER)
    ));
    if pdfa {
        s.push_str("<pdfaid:part>2</pdfaid:part>\n<pdfaid:conformance>B</pdfaid:conformance>\n");
    }
    s.push_str("</rdf:Description>\n</rdf:RDF>\n</x:xmpmeta>\n");
    // Đệm cho phép sửa tại chỗ (khuyến nghị của chuẩn XMP).
    for _ in 0..20 {
        s.push_str(&" ".repeat(99));
        s.push('\n');
    }
    s.push_str("<?xpacket end=\"w\"?>");
    s.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_and_fields() {
        assert_eq!(xml_escape("a<b>&\"'\u{1}c"), "a&lt;b&gt;&amp;&quot;&apos;c");
        let meta = Metadata {
            title: "Hợp đồng <số 1>".into(),
            author: "Tsu".into(),
            subject: "Thuê nhà".into(),
            keywords: vec!["hợp đồng".into(), "2026".into()],
            created: None,
        };
        let t = Timestamp {
            unix: 1_763_487_000,
            offset_minutes: 420,
        };
        let x = String::from_utf8(packet(&meta, t, true)).unwrap();
        assert!(x.contains("Hợp đồng &lt;số 1&gt;"));
        assert!(x.contains("<pdfaid:part>2</pdfaid:part>"));
        assert!(x.contains("<xmp:CreateDate>2025-11-19T00:30:00+07:00</xmp:CreateDate>"));
        assert!(x.contains("<pdf:Keywords>hợp đồng, 2026</pdf:Keywords>"));
        let y = String::from_utf8(packet(&Metadata::default(), t, false)).unwrap();
        assert!(!y.contains("pdfaid:part>") && !y.contains("dc:title"));
    }
}

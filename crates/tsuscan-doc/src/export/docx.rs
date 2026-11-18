//! Xuất DOCX (Office Open XML, ECMA-376): tiêu đề → Heading 1, đoạn văn → Normal, bảng → bảng có viền.
//! Trang chưa có chữ OCR được chèn ảnh trang (vừa khổ A4 dọc) để không mất nội dung. Ngắt trang giữa các trang.

use tsuscan_ml::layout::Block;

use super::image::{ImageFormat, export_image};
use super::page_blocks;
use super::zip::ZipWriter;
use crate::compress::{ColorMode, Quality};
use crate::error::Result;
use crate::model::{Document, PRODUCER, Timestamp};
use crate::pdf::xmp::xml_escape;

const NS_W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const NS_R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
/// EMU mỗi inch.
const EMU_PER_INCH: f64 = 914_400.0;
/// Vùng in tối đa của A4 lề 1 inch: 6,27 × 9,69 inch.
const MAX_W_IN: f64 = 6.27;
const MAX_H_IN: f64 = 9.69;

fn run(text: &str) -> String {
    format!(
        "<w:r><w:t xml:space=\"preserve\">{}</w:t></w:r>",
        xml_escape(text)
    )
}

fn paragraph(style: Option<&str>, text: &str) -> String {
    let ppr = style.map_or(String::new(), |s| {
        format!("<w:pPr><w:pStyle w:val=\"{s}\"/></w:pPr>")
    });
    format!("<w:p>{ppr}{}</w:p>", run(text))
}

fn table(rows: &[Vec<String>]) -> String {
    let cols = rows.iter().map(Vec::len).max().unwrap_or(0).max(1);
    let mut s = String::from(
        "<w:tbl><w:tblPr><w:tblStyle w:val=\"TableGrid\"/><w:tblW w:w=\"5000\" w:type=\"pct\"/></w:tblPr><w:tblGrid>",
    );
    for _ in 0..cols {
        s.push_str(&format!("<w:gridCol w:w=\"{}\"/>", 9000 / cols));
    }
    s.push_str("</w:tblGrid>");
    for r in rows {
        s.push_str("<w:tr>");
        for c in 0..cols {
            let cell = r.get(c).map_or("", String::as_str);
            s.push_str(&format!(
                "<w:tc><w:tcPr><w:tcW w:w=\"0\" w:type=\"auto\"/></w:tcPr>{}</w:tc>",
                paragraph(None, cell)
            ));
        }
        s.push_str("</w:tr>");
    }
    s.push_str("</w:tbl>");
    s
}

fn image_paragraph(rid: &str, id: usize, w_px: u32, h_px: u32, dpi: f32) -> String {
    let (mut w_in, mut h_in) = (f64::from(w_px) / f64::from(dpi), f64::from(h_px) / f64::from(dpi));
    let k = (MAX_W_IN / w_in).min(MAX_H_IN / h_in).min(1.0);
    w_in *= k;
    h_in *= k;
    let (cx, cy) = ((w_in * EMU_PER_INCH) as u64, (h_in * EMU_PER_INCH) as u64);
    format!(
        "<w:p><w:r><w:drawing><wp:inline distT=\"0\" distB=\"0\" distL=\"0\" distR=\"0\">\
<wp:extent cx=\"{cx}\" cy=\"{cy}\"/><wp:docPr id=\"{id}\" name=\"Trang {id}\"/>\
<a:graphic xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\">\
<a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">\
<pic:pic xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">\
<pic:nvPicPr><pic:cNvPr id=\"{id}\" name=\"trang{id}\"/><pic:cNvPicPr/></pic:nvPicPr>\
<pic:blipFill><a:blip r:embed=\"{rid}\"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill>\
<pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"{cx}\" cy=\"{cy}\"/></a:xfrm>\
<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic>\
</wp:inline></w:drawing></w:r></w:p>"
    )
}

const PAGE_BREAK: &str = "<w:p><w:r><w:br w:type=\"page\"/></w:r></w:p>";

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Times New Roman" w:hAnsi="Times New Roman" w:eastAsia="Times New Roman" w:cs="Times New Roman"/><w:sz w:val="26"/><w:szCs w:val="26"/><w:lang w:val="vi-VN"/></w:rPr></w:rPrDefault>
<w:pPrDefault><w:pPr><w:spacing w:after="120" w:line="276" w:lineRule="auto"/></w:pPr></w:pPrDefault></w:docDefaults>
<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/></w:style>
<w:style w:type="paragraph" w:styleId="Title"><w:name w:val="Title"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:qFormat/><w:pPr><w:jc w:val="center"/><w:spacing w:after="240"/></w:pPr><w:rPr><w:b/><w:sz w:val="36"/><w:szCs w:val="36"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:qFormat/><w:pPr><w:keepNext/><w:jc w:val="center"/><w:spacing w:before="240" w:after="120"/><w:outlineLvl w:val="0"/></w:pPr><w:rPr><w:b/><w:sz w:val="30"/><w:szCs w:val="30"/></w:rPr></w:style>
<w:style w:type="table" w:default="1" w:styleId="TableNormal"><w:name w:val="Normal Table"/><w:tblPr><w:tblInd w:w="0" w:type="dxa"/><w:tblCellMar><w:top w:w="0" w:type="dxa"/><w:left w:w="108" w:type="dxa"/><w:bottom w:w="0" w:type="dxa"/><w:right w:w="108" w:type="dxa"/></w:tblCellMar></w:tblPr></w:style>
<w:style w:type="table" w:styleId="TableGrid"><w:name w:val="Table Grid"/><w:basedOn w:val="TableNormal"/><w:pPr><w:spacing w:after="0"/></w:pPr><w:tblPr><w:tblBorders><w:top w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:left w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:bottom w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:right w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:insideH w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:insideV w:val="single" w:sz="4" w:space="0" w:color="auto"/></w:tblBorders></w:tblPr></w:style>
</w:styles>"#;

/// Phần `docProps/core.xml` + `app.xml` dùng chung cho DOCX/XLSX.
pub(crate) fn doc_props(doc: &Document, app: &str) -> (String, String) {
    let m = &doc.meta;
    let mut core = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<cp:coreProperties \
xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" \
xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:dcterms=\"http://purl.org/dc/terms/\" \
xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">",
    );
    if !m.title.is_empty() {
        core.push_str(&format!("<dc:title>{}</dc:title>", xml_escape(&m.title)));
    }
    if !m.subject.is_empty() {
        core.push_str(&format!("<dc:subject>{}</dc:subject>", xml_escape(&m.subject)));
    }
    if !m.author.is_empty() {
        core.push_str(&format!("<dc:creator>{}</dc:creator>", xml_escape(&m.author)));
    }
    if !m.keywords.is_empty() {
        core.push_str(&format!(
            "<cp:keywords>{}</cp:keywords>",
            xml_escape(&m.keywords.join(", "))
        ));
    }
    if let Some(t) = m.created {
        core.push_str(&format!(
            "<dcterms:created xsi:type=\"dcterms:W3CDTF\">{}</dcterms:created>",
            Timestamp::iso8601_utc(t)
        ));
    }
    core.push_str("</cp:coreProperties>");
    let app = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Properties \
xmlns=\"http://schemas.openxmlformats.org/officeDocument/2006/extended-properties\">\
<Application>{}</Application><AppVersion>{app}</AppVersion></Properties>",
        xml_escape(PRODUCER)
    );
    (core, app)
}

pub(crate) const ROOT_RELS_DOCX: &str = "word/document.xml";

pub(crate) fn root_rels(main: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships \
xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rId1\" Type=\"{REL}/officeDocument\" Target=\"{main}\"/>\
<Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties\" Target=\"docProps/core.xml\"/>\
<Relationship Id=\"rId3\" Type=\"{REL}/extended-properties\" Target=\"docProps/app.xml\"/>\
</Relationships>"
    )
}

/// Xuất DOCX.
///
/// # Errors
/// Lỗi mã hóa ảnh trang (trang chưa OCR) hoặc ZIP vượt giới hạn.
pub fn to_docx(doc: &Document) -> Result<Vec<u8>> {
    doc.ensure_pages()?;
    let mut body = String::new();
    let mut images: Vec<(String, Vec<u8>)> = Vec::new();
    if !doc.meta.title.trim().is_empty() {
        body.push_str(&paragraph(Some("Title"), doc.meta.title.trim()));
    }
    for (i, page) in doc.pages.iter().enumerate() {
        if i > 0 {
            body.push_str(PAGE_BREAK);
        }
        let blocks = page_blocks(page);
        if blocks.is_empty() {
            let png = export_image(&page.image, ImageFormat::Jpeg, ColorMode::Auto, Quality::Balanced)?;
            let rid = format!("rIdImg{}", images.len() + 1);
            body.push_str(&image_paragraph(
                &rid,
                images.len() + 1,
                page.image.width(),
                page.image.height(),
                page.effective_dpi(),
            ));
            images.push((rid, png));
            continue;
        }
        for b in blocks {
            match b {
                Block::Heading(t) => body.push_str(&paragraph(Some("Heading1"), &t)),
                Block::Paragraph(t) => body.push_str(&paragraph(None, &t)),
                Block::Table(rows) => {
                    body.push_str(&table(&rows));
                    // Word bắt buộc có đoạn sau bảng ở cuối thân.
                    body.push_str("<w:p/>");
                }
            }
        }
    }
    let document = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:document xmlns:w=\"{NS_W}\" \
xmlns:r=\"{NS_R}\" xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\">\
<w:body>{body}<w:sectPr><w:pgSz w:w=\"11906\" w:h=\"16838\"/>\
<w:pgMar w:top=\"1440\" w:right=\"1440\" w:bottom=\"1440\" w:left=\"1440\" w:header=\"708\" w:footer=\"708\" w:gutter=\"0\"/>\
</w:sectPr></w:body></w:document>"
    );
    let mut rels = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships \
xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rIdStyles\" Type=\"{REL}/styles\" Target=\"styles.xml\"/>"
    );
    for (i, (rid, _)) in images.iter().enumerate() {
        rels.push_str(&format!(
            "<Relationship Id=\"{rid}\" Type=\"{REL}/image\" Target=\"media/trang{}.jpg\"/>",
            i + 1
        ));
    }
    rels.push_str("</Relationships>");
    let content_types = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types \
xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
<Default Extension=\"xml\" ContentType=\"application/xml\"/>\
<Default Extension=\"jpg\" ContentType=\"image/jpeg\"/>\
<Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/>\
<Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/>\
<Override PartName=\"/docProps/core.xml\" ContentType=\"application/vnd.openxmlformats-package.core-properties+xml\"/>\
<Override PartName=\"/docProps/app.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.extended-properties+xml\"/>\
</Types>";
    let (core, app) = doc_props(doc, "1.0000");
    let mut z = ZipWriter::new();
    z.add("[Content_Types].xml", content_types.as_bytes())?;
    z.add("_rels/.rels", root_rels(ROOT_RELS_DOCX).as_bytes())?;
    z.add("word/document.xml", document.as_bytes())?;
    z.add("word/styles.xml", STYLES.as_bytes())?;
    z.add("word/_rels/document.xml.rels", rels.as_bytes())?;
    for (i, (_, data)) in images.iter().enumerate() {
        z.add(&format!("word/media/trang{}.jpg", i + 1), data)?;
    }
    z.add("docProps/core.xml", core.as_bytes())?;
    z.add("docProps/app.xml", app.as_bytes())?;
    z.finish()
}

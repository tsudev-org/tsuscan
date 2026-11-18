//! Che thông tin XÓA HẲN dữ liệu (không chỉ phủ hình):
//!
//! 1. Điểm ảnh trong vùng bị ghi đè bằng màu đặc ngay trên ảnh trang (không có lớp dưới để gỡ).
//! 2. Chữ OCR giao với vùng bị cắt khỏi lớp chữ: ký tự nằm trong vùng (nới thêm 1 ký tự mỗi bên, vì vị trí ký tự
//!    chỉ ước lượng theo bề rộng trung bình) bị bỏ, phần còn lại tách thành dòng mới. Dòng nghiêng > 5° giao vùng
//!    thì bỏ cả dòng (ước lượng ký tự kém tin cậy ⇒ chọn che thừa thay vì lộ).
//! 3. Bộ xuất PDF ghi lại ảnh + chữ từ mô hình đã che, không sao chép đối tượng PDF gốc nào ⇒ không còn bản cũ.

use image::{DynamicImage, GenericImage, Rgba};
use tsuscan_imaging::{Point, Quad};
use tsuscan_ml::ocr::OcrLine;

use crate::model::Page;

/// Hình chữ nhật trục thẳng theo điểm ảnh của ảnh trang.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Trái.
    pub x: f64,
    /// Trên.
    pub y: f64,
    /// Rộng.
    pub w: f64,
    /// Cao.
    pub h: f64,
}

impl Rect {
    /// Tạo hình chữ nhật.
    #[must_use]
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }

    fn valid(&self) -> bool {
        [self.x, self.y, self.w, self.h].iter().all(|v| v.is_finite()) && self.w > 0.0 && self.h > 0.0
    }

    fn overlaps_y(&self, y0: f64, y1: f64) -> bool {
        y0 < self.y + self.h && y1 > self.y
    }
}

/// Báo cáo sau khi che.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RedactReport {
    /// Số vùng hợp lệ đã tô.
    pub regions: usize,
    /// Số dòng OCR bị bỏ hẳn.
    pub lines_removed: usize,
    /// Số dòng OCR bị cắt bớt ký tự.
    pub lines_trimmed: usize,
}

fn bounds(q: &Quad) -> (f64, f64, f64, f64) {
    let xs = q.pts.map(|p| p.x);
    let ys = q.pts.map(|p| p.y);
    (
        xs.iter().copied().fold(f64::INFINITY, f64::min),
        ys.iter().copied().fold(f64::INFINITY, f64::min),
        xs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        ys.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    )
}

fn lerp(a: Point, b: Point, t: f64) -> Point {
    Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

/// Tứ giác con theo tỉ lệ [t0, t1] dọc chiều dòng.
fn sub_quad(q: &Quad, t0: f64, t1: f64) -> Quad {
    let [tl, tr, br, bl] = q.pts;
    Quad {
        pts: [
            lerp(tl, tr, t0),
            lerp(tl, tr, t1),
            lerp(bl, br, t1),
            lerp(bl, br, t0),
        ],
    }
}

fn tilt_deg(q: &Quad) -> f64 {
    let [_, _, br, bl] = q.pts;
    (br.y - bl.y).atan2(br.x - bl.x).to_degrees().abs()
}

/// Cắt một dòng theo các vùng che. Trả (các dòng còn lại, có bị đụng không).
fn cut_line(line: &OcrLine, rects: &[Rect]) -> (Vec<OcrLine>, bool) {
    let (x0, y0, x1, y1) = bounds(&line.quad);
    let hits: Vec<&Rect> = rects
        .iter()
        .filter(|r| r.overlaps_y(y0, y1) && x0 < r.x + r.w && x1 > r.x)
        .collect();
    if hits.is_empty() {
        return (vec![line.clone()], false);
    }
    let chars: Vec<char> = line.text.chars().collect();
    if chars.is_empty() || tilt_deg(&line.quad) > 5.0 || x1 - x0 < 1.0 {
        return (Vec::new(), true);
    }
    let n = chars.len() as f64;
    let cw = (x1 - x0) / n;
    let hidden: Vec<bool> = (0..chars.len())
        .map(|i| {
            let (cx0, cx1) = (x0 + i as f64 * cw, x0 + (i + 1) as f64 * cw);
            hits.iter().any(|r| cx0 < r.x + r.w + cw && cx1 > r.x - cw)
        })
        .collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if hidden[i] {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && !hidden[i] {
            i += 1;
        }
        let seg: String = chars[start..i].iter().collect();
        let trimmed = seg.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Thu tứ giác theo phần chữ còn lại (bỏ khoảng trắng hai đầu).
        let lead = seg.chars().take_while(|c| c.is_whitespace()).count();
        let tail = seg.chars().rev().take_while(|c| c.is_whitespace()).count();
        let (s, e) = ((start + lead) as f64 / n, (i - tail) as f64 / n);
        out.push(OcrLine {
            text: trimmed.to_owned(),
            quad: sub_quad(&line.quad, s, e),
            confidence: line.confidence,
            row: line.row,
        });
    }
    (out, true)
}

fn paint(img: &mut DynamicImage, r: &Rect) {
    let (w, h) = (f64::from(img.width()), f64::from(img.height()));
    // Nới 1 điểm ảnh để không sót viền khử răng cưa.
    let xa = (r.x - 1.0).floor().clamp(0.0, w) as u32;
    let ya = (r.y - 1.0).floor().clamp(0.0, h) as u32;
    let xb = (r.x + r.w + 1.0).ceil().clamp(0.0, w) as u32;
    let yb = (r.y + r.h + 1.0).ceil().clamp(0.0, h) as u32;
    for y in ya..yb {
        for x in xa..xb {
            img.put_pixel(x, y, Rgba([0, 0, 0, 255]));
        }
    }
}

/// Che các vùng trên trang: tô đen điểm ảnh và cắt chữ OCR tương ứng. Vùng không hợp lệ bị bỏ qua.
pub fn redact(page: &mut Page, rects: &[Rect]) -> RedactReport {
    let rects: Vec<Rect> = rects.iter().copied().filter(Rect::valid).collect();
    let mut report = RedactReport {
        regions: rects.len(),
        ..RedactReport::default()
    };
    if rects.is_empty() {
        return report;
    }
    // Ảnh 16 bit/float hiếm gặp: quy về RGB8 trước khi tô để chắc chắn ghi đè được.
    if !matches!(
        page.image,
        DynamicImage::ImageLuma8(_) | DynamicImage::ImageRgb8(_) | DynamicImage::ImageRgba8(_)
    ) {
        page.image = DynamicImage::ImageRgb8(page.image.to_rgb8());
    }
    for r in &rects {
        paint(&mut page.image, r);
    }
    if let Some(ocr) = page.ocr.as_mut() {
        let mut lines = Vec::with_capacity(ocr.lines.len());
        for l in &ocr.lines {
            let (rest, touched) = cut_line(l, &rects);
            if touched {
                if rest.is_empty() {
                    report.lines_removed += 1;
                } else {
                    report.lines_trimmed += 1;
                }
            }
            lines.extend(rest);
        }
        ocr.lines = lines;
    }
    report
}

/// Tìm vùng chứa chuỗi (không phân biệt hoa thường) trong chữ OCR - gợi ý che nhanh số CCCD, số tài khoản...
#[must_use]
pub fn find_text(page: &Page, needle: &str) -> Vec<Rect> {
    let needle: Vec<char> = needle.trim().to_lowercase().chars().collect();
    let Some(ocr) = &page.ocr else {
        return Vec::new();
    };
    if needle.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for l in &ocr.lines {
        let hay: Vec<char> = l.text.to_lowercase().chars().collect();
        // Chữ thường hóa có thể đổi số ký tự (hiếm): khi đó dùng cả dòng.
        let same_len = hay.len() == l.text.chars().count();
        let (x0, y0, x1, y1) = bounds(&l.quad);
        let cw = (x1 - x0) / hay.len().max(1) as f64;
        let mut i = 0;
        while i + needle.len() <= hay.len() {
            if hay[i..i + needle.len()] == needle[..] {
                if same_len {
                    out.push(Rect::new(
                        x0 + i as f64 * cw,
                        y0,
                        needle.len() as f64 * cw,
                        y1 - y0,
                    ));
                } else {
                    out.push(Rect::new(x0, y0, x1 - x0, y1 - y0));
                }
                i += needle.len();
            } else {
                i += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tsuscan_ml::OcrPageResult;

    fn line(text: &str, x0: f64, y0: f64, x1: f64, y1: f64) -> OcrLine {
        OcrLine {
            text: text.into(),
            quad: Quad {
                pts: [
                    Point::new(x0, y0),
                    Point::new(x1, y0),
                    Point::new(x1, y1),
                    Point::new(x0, y1),
                ],
            },
            confidence: 0.9,
            row: 0,
        }
    }

    fn page() -> Page {
        let white = image::RgbImage::from_pixel(400, 100, image::Rgb([255, 255, 255]));
        Page::new(DynamicImage::ImageRgb8(white)).with_ocr(OcrPageResult {
            lines: vec![
                line("Số CCCD: 001234567890 hết", 0.0, 10.0, 250.0, 30.0),
                line("Họ tên: Nguyễn Văn A", 0.0, 50.0, 200.0, 70.0),
            ],
        })
    }

    #[test]
    fn removes_pixels_and_text() {
        let mut p = page();
        let rects = find_text(&p, "001234567890");
        assert_eq!(rects.len(), 1);
        let rep = redact(&mut p, &rects);
        assert_eq!(
            rep,
            RedactReport {
                regions: 1,
                lines_removed: 0,
                lines_trimmed: 1
            }
        );
        let text = p.ocr.as_ref().unwrap().text();
        assert!(!text.contains("1234"), "{text}");
        assert!(text.contains("Số CCCD"));
        assert!(text.contains("Nguyễn Văn A"));
        let r = rects[0];
        let px = p.image.to_rgb8();
        let c = px.get_pixel((r.x + r.w / 2.0) as u32, 20);
        assert_eq!(c.0, [0, 0, 0]);
    }

    #[test]
    fn tilted_or_full_lines_are_dropped() {
        let mut p = page();
        let rep = redact(&mut p, &[Rect::new(0.0, 45.0, 400.0, 30.0)]);
        assert_eq!(rep.lines_removed, 1);
        let mut q = page();
        if let Some(o) = q.ocr.as_mut() {
            o.lines[0].quad.pts[2].y = 60.0; // nghiêng mạnh
        }
        let rep = redact(&mut q, &[Rect::new(100.0, 15.0, 5.0, 5.0)]);
        assert_eq!(rep.lines_removed, 1);
        let mut z = page();
        let rep = redact(
            &mut z,
            &[Rect::new(f64::NAN, 0.0, 1.0, 1.0), Rect::new(0.0, 0.0, 0.0, 5.0)],
        );
        assert_eq!(rep, RedactReport::default());
        assert_eq!(find_text(&z, "  "), Vec::new());
        assert_eq!(
            find_text(&Page::new(DynamicImage::new_rgb8(2, 2)), "a"),
            Vec::new()
        );
    }

    #[test]
    fn paints_gray_and_wide_images() {
        let mut p = Page::new(DynamicImage::new_luma16(20, 20));
        redact(&mut p, &[Rect::new(-5.0, -5.0, 100.0, 3.0)]);
        assert!(matches!(p.image, DynamicImage::ImageRgb8(_)));
        let mut g = Page::new(DynamicImage::ImageLuma8(image::GrayImage::from_pixel(
            10,
            10,
            image::Luma([200]),
        )));
        redact(&mut g, &[Rect::new(2.0, 2.0, 2.0, 2.0)]);
        assert_eq!(g.image.to_luma8().get_pixel(3, 3).0, [0]);
        assert_eq!(g.image.to_luma8().get_pixel(9, 9).0, [200]);
    }
}

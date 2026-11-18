//! Bộ sinh trang chữ in có nhãn để đo CER OCR (`testdata/ocr-vi`).
//!
//! Tất định theo `seed`: cùng seed + cùng font + cùng kho câu ⇒ cùng ảnh, nên bộ mẫu không cần nằm trong git.
//! Trang mô phỏng ảnh đầu ra của bước nắn phối cảnh (P1) ở ≈ 200 DPI: nền giấy hơi ngả màu, chữ đen/xám đậm,
//! nhòe nhẹ, nhiễu, nghiêng nhỏ và nén JPEG.

use ab_glyph::{Font, FontVec, PxScale, ScaleFont};
use image::{ImageEncoder, Rgb, RgbImage, codecs::jpeg::JpegEncoder};
use tsuscan_imaging::synth::Rng;

use crate::error::{MlError, Result};

/// Một dòng chữ chuẩn trên trang sinh.
#[derive(Debug, Clone, PartialEq)]
pub struct TruthLine {
    /// Chữ chuẩn (NFC, một khoảng trắng giữa các từ).
    pub text: String,
    /// Hộp bao (x0, y0, x1, y1) trên ảnh trước khi xoay nhiễu.
    pub bbox: [f32; 4],
}

/// Trang sinh kèm nhãn.
#[derive(Debug, Clone)]
pub struct OcrPage {
    /// Ảnh trang.
    pub image: RgbImage,
    /// Các dòng theo thứ tự đọc.
    pub lines: Vec<TruthLine>,
    /// Tên font đã dùng.
    pub font: String,
}

impl OcrPage {
    /// Chữ chuẩn của cả trang, mỗi dòng một `\n`.
    #[must_use]
    pub fn text(&self) -> String {
        self.lines
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Font có tên.
pub struct NamedFont {
    /// Tên hiển thị (ghi vào báo cáo).
    pub name: String,
    font: FontVec,
}

impl NamedFont {
    /// Nạp font từ byte TTF/OTF.
    ///
    /// # Errors
    /// Font hỏng.
    pub fn from_bytes(name: &str, bytes: Vec<u8>) -> Result<Self> {
        let font =
            FontVec::try_from_vec(bytes).map_err(|_| MlError::InvalidInput(format!("font hỏng: {name}")))?;
        Ok(Self {
            name: name.to_owned(),
            font,
        })
    }

    /// Font có đủ glyph cho mọi ký tự (trừ khoảng trắng) của `text` không.
    #[must_use]
    pub fn covers(&self, text: &str) -> bool {
        text.chars()
            .filter(|c| !c.is_whitespace())
            .all(|c| self.font.glyph_id(c).0 != 0)
    }
}

/// Cấu hình bộ sinh trang.
#[derive(Debug, Clone, Copy)]
pub struct OcrSynthConfig {
    /// Rộng trang (px).
    pub width: u32,
    /// Cao trang (px).
    pub height: u32,
    /// Cỡ chữ nhỏ nhất/lớn nhất (px chiều cao em).
    pub font_px: (f32, f32),
    /// Số dòng mỗi trang tối đa.
    pub max_lines: usize,
    /// Bật hiệu ứng suy giảm (nhòe, nhiễu, nghiêng, JPEG).
    pub degrade: bool,
}

impl Default for OcrSynthConfig {
    fn default() -> Self {
        // A4 ở ≈ 150 DPI; chữ 10-14 pt ⇒ 21-30 px em.
        Self {
            width: 1240,
            height: 1754,
            font_px: (24.0, 34.0),
            max_lines: 22,
            degrade: true,
        }
    }
}

/// Đọc kho câu: bỏ dòng trống và dòng `#`, chuẩn hóa khoảng trắng.
#[must_use]
pub fn parse_corpus(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(crate::metrics::normalize_line)
        .collect()
}

/// Sinh một trang.
///
/// # Errors
/// Kho câu rỗng, không có font, hoặc không font nào phủ được kho câu.
pub fn generate_page(
    seed: u64,
    corpus: &[String],
    fonts: &[NamedFont],
    cfg: &OcrSynthConfig,
) -> Result<OcrPage> {
    if corpus.is_empty() || fonts.is_empty() {
        return Err(MlError::InvalidInput("kho câu hoặc danh sách font rỗng".into()));
    }
    let mut rng = Rng::new(seed ^ 0x6f63_725f_7669);
    let font = &fonts[rng.index(fonts.len())];
    let px = rng.range(f64::from(cfg.font_px.0), f64::from(cfg.font_px.1)) as f32;
    let scale = PxScale::from(px);
    let sf = font.font.as_scaled(scale);
    let line_h = (sf.height() + sf.line_gap()) * rng.range(1.15, 1.45) as f32;

    let margin_x = cfg.width as f32 * rng.range(0.07, 0.11) as f32;
    let max_w = cfg.width as f32 - 2.0 * margin_x;
    let paper = {
        let base = rng.range(232.0, 250.0);
        [base, base - rng.range(0.0, 6.0), base - rng.range(2.0, 14.0)]
    };
    let ink = rng.range(10.0, 60.0) as f32;
    let mut img = RgbImage::from_fn(cfg.width, cfg.height, |_, _| Rgb(paper.map(|v| v as u8)));

    let mut y = cfg.height as f32 * rng.range(0.06, 0.09) as f32;
    let mut lines = Vec::new();
    let start = rng.index(corpus.len());
    let mut idx = start;
    while lines.len() < cfg.max_lines {
        let src = &corpus[idx % corpus.len()];
        idx += 1;
        if idx - start > corpus.len() {
            break;
        }
        if !font.covers(src) {
            continue;
        }
        for piece in wrap(src, &sf, max_w) {
            if y + line_h > cfg.height as f32 * 0.94 || lines.len() >= cfg.max_lines {
                break;
            }
            let w = text_width(&piece, &sf);
            let centered = piece
                .chars()
                .filter(|c| c.is_alphabetic())
                .all(char::is_uppercase)
                && rng.chance(0.7);
            let x0 = if centered {
                (cfg.width as f32 - w) / 2.0
            } else {
                margin_x
            };
            let baseline = y + sf.ascent();
            draw_text(&mut img, &piece, &sf, &font.font, scale, x0, baseline, ink, paper);
            lines.push(TruthLine {
                text: piece,
                bbox: [x0, y, x0 + w, y + sf.height()],
            });
            y += line_h * if rng.chance(0.15) { 1.6 } else { 1.0 };
        }
    }
    if cfg.degrade {
        img = degrade(img, &mut rng)?;
    }
    Ok(OcrPage {
        image: img,
        lines,
        font: font.name.clone(),
    })
}

/// Uốn cong trang như trang sách mở (mặt trụ): dòng chữ võng theo hình sin với biên độ `amp` px (tăng dần về
/// cuối trang) và nén ngang nhẹ gần gáy. Phần lộ ra ngoài trang tô màu nền tối (mặt bàn).
#[must_use]
pub fn curl(img: &RgbImage, amp: f64) -> RgbImage {
    let (w, h) = img.dimensions();
    let (wf, hf) = (f64::from(w), f64::from(h));
    let margin = amp.abs().ceil() as u32 + 8;
    let out_h = h + 2 * margin;
    RgbImage::from_fn(w, out_h, |x, y| {
        let xf = f64::from(x) + 0.5;
        let yf = f64::from(y) + 0.5 - f64::from(margin);
        let t = xf / wf;
        let a = amp * (0.6 + 0.4 * (yf / hf).clamp(0.0, 1.0));
        let sx = xf + 0.02 * wf * (2.0 * std::f64::consts::PI * t).sin() / (2.0 * std::f64::consts::PI);
        let sy = yf - a * (std::f64::consts::PI * t).sin();
        if sx < 0.0 || sy < 0.0 || sx >= wf || sy >= hf {
            Rgb([70, 58, 46])
        } else {
            tsuscan_imaging::warp::sample_bilinear(img, sx - 0.5, sy - 0.5)
        }
    })
}

fn text_width<F: Font, S: ScaleFont<F>>(text: &str, sf: &S) -> f32 {
    let mut w = 0.0;
    let mut prev = None;
    for c in text.chars() {
        let id = sf.glyph_id(c);
        if let Some(p) = prev {
            w += sf.kern(p, id);
        }
        w += sf.h_advance(id);
        prev = Some(id);
    }
    w
}

/// Ngắt dòng theo từ cho vừa `max_w`.
fn wrap<F: Font, S: ScaleFont<F>>(text: &str, sf: &S, max_w: f32) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for word in text.split(' ') {
        let cand = if cur.is_empty() {
            word.to_owned()
        } else {
            format!("{cur} {word}")
        };
        if text_width(&cand, sf) <= max_w || cur.is_empty() {
            cur = cand;
        } else {
            out.push(std::mem::take(&mut cur));
            word.clone_into(&mut cur);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn draw_text<F: Font, S: ScaleFont<F>, G: Font>(
    img: &mut RgbImage,
    text: &str,
    sf: &S,
    font: &G,
    scale: PxScale,
    x0: f32,
    baseline: f32,
    ink: f32,
    paper: [f64; 3],
) {
    let mut x = x0;
    let mut prev = None;
    let (w, h) = img.dimensions();
    for c in text.chars() {
        let id = sf.glyph_id(c);
        if let Some(p) = prev {
            x += sf.kern(p, id);
        }
        let glyph = id.with_scale_and_position(scale, ab_glyph::point(x, baseline));
        if let Some(outlined) = font.outline_glyph(glyph) {
            let b = outlined.px_bounds();
            outlined.draw(|gx, gy, cov| {
                let px = b.min.x as i64 + i64::from(gx);
                let py = b.min.y as i64 + i64::from(gy);
                if px < 0 || py < 0 || px >= i64::from(w) || py >= i64::from(h) {
                    return;
                }
                let p = img.get_pixel_mut(px as u32, py as u32);
                for (ch, bg) in p.0.iter_mut().zip(paper) {
                    let cur = f32::from(*ch);
                    let target = ink.min(bg as f32);
                    *ch = (cur + (target - cur) * cov.clamp(0.0, 1.0)).round() as u8;
                }
            });
        }
        x += sf.h_advance(id);
        prev = Some(id);
    }
}

fn degrade(img: RgbImage, rng: &mut Rng) -> Result<RgbImage> {
    let mut img = img;
    if rng.chance(0.6) {
        let angle = rng.range(-1.2, 1.2).to_radians() as f32;
        let bg = *img.get_pixel(2, 2);
        img = imageproc::geometric_transformations::rotate_about_center(
            &img,
            angle,
            imageproc::geometric_transformations::Interpolation::Bilinear,
            imageproc::geometric_transformations::Border::Constant(bg),
        );
    }
    let sigma = rng.range(0.0, 1.1) as f32;
    if sigma > 0.3 {
        img = imageproc::filter::gaussian_blur_f32(&img, sigma);
    }
    let noise = rng.range(0.0, 7.0);
    let gain = rng.range(0.85, 1.0);
    for p in img.pixels_mut() {
        let n = rng.fast_normal() * noise;
        for c in &mut p.0 {
            *c = (f64::from(*c) * gain + (1.0 - gain) * 128.0 + n).clamp(0.0, 255.0) as u8;
        }
    }
    let quality = 55 + rng.index(36) as u8;
    let mut buf = Vec::new();
    JpegEncoder::new_with_quality(&mut buf, quality)
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|e| MlError::InvalidInput(format!("mã hóa JPEG: {e}")))?;
    let decoded = image::load_from_memory_with_format(&buf, image::ImageFormat::Jpeg)
        .map_err(|e| MlError::InvalidInput(format!("giải mã JPEG: {e}")))?;
    Ok(decoded.to_rgb8())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpus_bo_dong_chu_thich_va_chuan_hoa() {
        let c = parse_corpus("# x\n\n  a   b \nc\n");
        assert_eq!(c, vec!["a b".to_owned(), "c".to_owned()]);
    }
}

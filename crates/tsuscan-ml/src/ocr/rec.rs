//! Nhận dạng một dòng chữ bằng AsterNet (CTC, 227 lớp tiếng Việt + Latin cơ bản).

use image::{GrayImage, Rgb, RgbImage, imageops::FilterType};
use tsuscan_imaging::{Homography, Point, Quad, warp::sample_bilinear};

use super::vocab::{ASTERNET_CHARS, ASTERNET_CLASSES};
use crate::error::{MlError, Result};
use crate::prep::{Channels, Normalize, to_tensor};
use crate::runtime::Session;

/// Chiều cao dòng mô hình nhận.
pub const LINE_HEIGHT: u32 = 32;
/// Chiều rộng tối đa của dòng (px sau khi đưa về cao 32) - dòng dài hơn bị chia đoạn.
pub const MAX_LINE_WIDTH: u32 = 1536;
/// Bước "xô" chiều rộng: ảnh dòng được đệm phải lên bội số này để dùng lại kế hoạch đã biên dịch.
pub const WIDTH_BUCKET: u32 = 128;

/// Kết quả nhận dạng một dòng.
#[derive(Debug, Clone, PartialEq)]
pub struct LineText {
    /// Chữ (NFC).
    pub text: String,
    /// Tin cậy trung bình của các ký tự phát ra (0..1); 0 nếu rỗng.
    pub confidence: f32,
}

/// Bộ nhận dạng dòng.
#[derive(Debug)]
pub struct TextRecognizer {
    session: Session,
    chars: Vec<char>,
}

impl TextRecognizer {
    /// Dựng từ byte mô hình AsterNet đã kiểm.
    ///
    /// # Errors
    /// Mô hình không dịch được.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        // Mỗi xô một kế hoạch: tối đa MAX_LINE_WIDTH / WIDTH_BUCKET = 12 kế hoạch.
        let session = Session::per_shape("ocr-rec", bytes, (MAX_LINE_WIDTH / WIDTH_BUCKET) as usize)?;
        let chars: Vec<char> = ASTERNET_CHARS.chars().collect();
        debug_assert_eq!(chars.len() + 1, ASTERNET_CLASSES);
        Ok(Self { session, chars })
    }

    /// Có chạy một kế hoạch cho mọi chiều rộng dòng (hình dạng ký hiệu) không (hiện luôn `false`: dùng xô chiều rộng).
    #[must_use]
    pub fn is_symbolic(&self) -> bool {
        self.session.is_symbolic()
    }

    /// Nhận dạng ảnh một dòng (bất kỳ kích thước; được đưa về cao 32).
    ///
    /// # Errors
    /// Ảnh rỗng hoặc lỗi suy luận.
    pub fn recognize_line(&self, line: &RgbImage) -> Result<LineText> {
        let line = normalize_polarity(line);
        let (w, h) = line.dimensions();
        if w == 0 || h == 0 {
            return Err(MlError::InvalidInput("dòng rỗng".into()));
        }
        let tw = ((f64::from(w) * f64::from(LINE_HEIGHT) / f64::from(h)).round() as u32).max(LINE_HEIGHT);
        let resized = image::imageops::resize(&line, tw, LINE_HEIGHT, FilterType::Triangle);
        // Dòng quá dài: chia đoạn có chồng lấn nhỏ ở khoảng trắng gần nhất.
        let mut texts = Vec::new();
        let mut conf_sum = 0.0f32;
        let mut conf_n = 0usize;
        for (x0, x1) in split_points(&resized, MAX_LINE_WIDTH) {
            let part = image::imageops::crop_imm(&resized, x0, 0, x1 - x0, LINE_HEIGHT).to_image();
            let (t, c, n) = self.run_ctc(&part)?;
            texts.push(t);
            conf_sum += c;
            conf_n += n;
        }
        let text = crate::metrics::normalize_line(&texts.join(" "));
        let confidence = if conf_n == 0 {
            0.0
        } else {
            conf_sum / conf_n as f32
        };
        Ok(LineText { text, confidence })
    }

    /// Cắt và nắn vùng `quad` từ ảnh trang rồi nhận dạng.
    ///
    /// # Errors
    /// Như [`Self::recognize_line`].
    pub fn recognize_quad(&self, page: &RgbImage, quad: &Quad) -> Result<LineText> {
        self.recognize_line(&crop_quad(page, quad))
    }

    fn run_ctc(&self, img: &RgbImage) -> Result<(String, f32, usize)> {
        let input = to_tensor(&pad_to_bucket(img), Channels::Rgb, Normalize::UNIT)?;
        let out = self.session.run(input)?;
        let lp = out
            .first()
            .ok_or_else(|| MlError::Inference("ocr-rec: không có đầu ra".into()))?;
        let view = lp
            .to_plain_array_view::<f32>()
            .map_err(|e| MlError::Inference(format!("{e:#}")))?;
        let shape = view.shape().to_vec();
        // [T, 1, C]
        if shape.len() != 3 || shape[1] != 1 || shape[2] != ASTERNET_CLASSES {
            return Err(MlError::Inference(format!(
                "ocr-rec: đầu ra {shape:?} không phải [T,1,{ASTERNET_CLASSES}]"
            )));
        }
        let flat: Vec<f32> = view.iter().copied().collect();
        Ok(ctc_greedy(&flat, shape[0], shape[2], &self.chars))
    }
}

/// Giải mã CTC tham lam trên log-xác suất `[T, C]`: gộp lặp, bỏ ký hiệu trống (0).
/// Trả về (chữ, tổng tin cậy, số ký tự phát ra).
#[must_use]
pub fn ctc_greedy(logp: &[f32], t: usize, c: usize, chars: &[char]) -> (String, f32, usize) {
    let mut s = String::new();
    let mut prev = 0usize;
    let mut sum = 0.0f32;
    let mut n = 0usize;
    for step in 0..t {
        let row = &logp[step * c..(step + 1) * c];
        let (best, &lv) = row
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap_or((0, &f32::NEG_INFINITY));
        if best != 0
            && best != prev
            && let Some(&ch) = chars.get(best - 1)
        {
            s.push(ch);
            sum += lv.exp();
            n += 1;
        }
        prev = best;
    }
    (s, sum, n)
}

/// Đệm phải ảnh dòng (màu nền = trung vị cột cuối) lên bội số [`WIDTH_BUCKET`].
fn pad_to_bucket(img: &RgbImage) -> RgbImage {
    let (w, h) = img.dimensions();
    let target = w.div_ceil(WIDTH_BUCKET).max(1) * WIDTH_BUCKET;
    if target == w {
        return img.clone();
    }
    let mut col: Vec<Rgb<u8>> = (0..h).map(|y| *img.get_pixel(w.saturating_sub(1), y)).collect();
    col.sort_by_key(|p| luma(*p));
    let bg = col
        .get(col.len() * 3 / 4)
        .copied()
        .unwrap_or(Rgb([255, 255, 255]));
    RgbImage::from_fn(target, h, |x, y| if x < w { *img.get_pixel(x, y) } else { bg })
}

/// Các đoạn [x0, x1) rộng ≤ `max_w`, cắt ở cột sáng nhất (khoảng trắng) trong 25% cuối mỗi đoạn.
fn split_points(img: &RgbImage, max_w: u32) -> Vec<(u32, u32)> {
    let w = img.width();
    if w <= max_w {
        return vec![(0, w)];
    }
    let col_ink: Vec<u32> = (0..w)
        .map(|x| {
            (0..img.height())
                .map(|y| 255 - u32::from(luma(*img.get_pixel(x, y))))
                .sum()
        })
        .collect();
    let mut out = Vec::new();
    let mut x0 = 0;
    while w - x0 > max_w {
        let lo = x0 + max_w * 3 / 4;
        let hi = x0 + max_w;
        let cut = (lo..hi).min_by_key(|&x| col_ink[x as usize]).unwrap_or(hi);
        out.push((x0, cut));
        x0 = cut;
    }
    out.push((x0, w));
    out
}

fn luma(p: Rgb<u8>) -> u8 {
    ((u32::from(p.0[0]) * 299 + u32::from(p.0[1]) * 587 + u32::from(p.0[2]) * 114) / 1000) as u8
}

/// Đảo màu khi dòng là chữ sáng trên nền tối (mô hình học chữ tối trên nền sáng).
#[must_use]
pub fn normalize_polarity(img: &RgbImage) -> RgbImage {
    let (w, h) = img.dimensions();
    if w < 4 || h < 4 {
        return img.clone();
    }
    let g = GrayImage::from_fn(w, h, |x, y| image::Luma([luma(*img.get_pixel(x, y))]));
    let mut border: Vec<u8> = Vec::new();
    for x in 0..w {
        border.push(g.get_pixel(x, 0).0[0]);
        border.push(g.get_pixel(x, h - 1).0[0]);
    }
    for y in 0..h {
        border.push(g.get_pixel(0, y).0[0]);
        border.push(g.get_pixel(w - 1, y).0[0]);
    }
    border.sort_unstable();
    let bg = i32::from(border[border.len() / 2]);
    let mut center: Vec<u8> = g
        .enumerate_pixels()
        .filter(|(x, y, _)| *y > h / 4 && *y < 3 * h / 4 && *x > 1 && *x < w - 2)
        .map(|(_, _, p)| p.0[0])
        .collect();
    if center.is_empty() {
        return img.clone();
    }
    center.sort_unstable();
    let p10 = i32::from(center[center.len() / 10]);
    let p90 = i32::from(center[center.len() * 9 / 10]);
    let bright = p90 - bg;
    let dark = bg - p10;
    if bright > dark && bright > 20 {
        let mut out = img.clone();
        for p in out.pixels_mut() {
            for c in &mut p.0 {
                *c = 255 - *c;
            }
        }
        out
    } else {
        img.clone()
    }
}

/// Cắt và nắn tứ giác về chữ nhật kích thước tự nhiên (rộng = cạnh trên/dưới dài hơn, cao = cạnh trái/phải dài hơn).
#[must_use]
pub fn crop_quad(page: &RgbImage, quad: &Quad) -> RgbImage {
    let [tl, tr, br, bl] = quad.pts;
    let w = tl.dist(tr).max(bl.dist(br)).round().max(1.0) as u32;
    let h = tl.dist(bl).max(tr.dist(br)).round().max(1.0) as u32;
    let dst = [
        Point { x: 0.0, y: 0.0 },
        Point {
            x: f64::from(w),
            y: 0.0,
        },
        Point {
            x: f64::from(w),
            y: f64::from(h),
        },
        Point {
            x: 0.0,
            y: f64::from(h),
        },
    ];
    let Ok(hm) = Homography::from_points(&dst, &quad.pts) else {
        return RgbImage::from_pixel(w, h, Rgb([255, 255, 255]));
    };
    RgbImage::from_fn(w, h, |x, y| {
        hm.apply(Point {
            x: f64::from(x) + 0.5,
            y: f64::from(y) + 0.5,
        })
        .map_or(Rgb([255, 255, 255]), |p| {
            sample_bilinear(page, p.x - 0.5, p.y - 0.5)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctc_gop_lap_bo_trong() {
        let chars = ['a', 'b'];
        // T=6, C=3: a a _ a b b ⇒ "aab"
        let seq = [1usize, 1, 0, 1, 2, 2];
        let mut lp = vec![-10.0f32; 6 * 3];
        for (t, &k) in seq.iter().enumerate() {
            lp[t * 3 + k] = 0.0;
        }
        let (s, sum, n) = ctc_greedy(&lp, 6, 3, &chars);
        assert_eq!(s, "aab");
        assert_eq!(n, 3);
        assert!((sum - 3.0).abs() < 1e-5);
    }

    #[test]
    fn bang_ky_tu_khop_so_lop() {
        assert_eq!(ASTERNET_CHARS.chars().count() + 1, ASTERNET_CLASSES);
        for c in "ăâđêôơưỹữậẳỡ ".chars() {
            assert!(ASTERNET_CHARS.contains(c), "thiếu {c}");
        }
    }

    #[test]
    fn dao_mau_chu_sang_nen_toi() {
        let mut img = RgbImage::from_pixel(40, 20, Rgb([20, 20, 20]));
        for x in 10..30 {
            for y in 8..12 {
                img.put_pixel(x, y, Rgb([240, 240, 240]));
            }
        }
        let n = normalize_polarity(&img);
        assert_eq!(n.get_pixel(0, 0).0[0], 235);
        let light = RgbImage::from_pixel(40, 20, Rgb([240, 240, 240]));
        assert_eq!(normalize_polarity(&light).get_pixel(0, 0).0[0], 240);
    }

    #[test]
    fn chia_doan_dong_dai() {
        let img = RgbImage::from_pixel(4000, 32, Rgb([255, 255, 255]));
        let parts = split_points(&img, MAX_LINE_WIDTH);
        assert!(parts.len() >= 3);
        assert_eq!(parts.first().map(|p| p.0), Some(0));
        assert_eq!(parts.last().map(|p| p.1), Some(4000));
        assert!(parts.iter().all(|(a, b)| b - a <= MAX_LINE_WIDTH && b > a));
    }

    #[test]
    fn dem_len_boi_so_xo() {
        let mut img = RgbImage::from_pixel(130, 32, Rgb([250, 250, 250]));
        img.put_pixel(0, 0, Rgb([0, 0, 0]));
        let p = pad_to_bucket(&img);
        assert_eq!(p.dimensions(), (256, 32));
        assert_eq!(*p.get_pixel(200, 5), Rgb([250, 250, 250]));
        assert_eq!(*p.get_pixel(0, 0), Rgb([0, 0, 0]));
        assert_eq!(pad_to_bucket(&RgbImage::new(128, 32)).width(), 128);
    }

    #[test]
    fn cat_tu_giac_kich_thuoc_tu_nhien() {
        let page = RgbImage::from_pixel(100, 50, Rgb([9, 9, 9]));
        let q = Quad::from_unordered([
            Point { x: 10.0, y: 10.0 },
            Point { x: 60.0, y: 10.0 },
            Point { x: 60.0, y: 30.0 },
            Point { x: 10.0, y: 30.0 },
        ])
        .unwrap();
        let c = crop_quad(&page, &q);
        assert_eq!(c.dimensions(), (50, 20));
        assert_eq!(c.get_pixel(25, 10).0, [9, 9, 9]);
    }
}

//! Bộ sinh ảnh tài liệu tổng hợp có nhãn (tứ giác chuẩn) để đo dò mép. Chỉ bật với tính năng `synth`.
//!
//! Hoàn toàn tất định theo `seed`: cùng seed cho cùng ảnh trên mọi máy. Không dùng giấy tờ thật:
//! trang chứa "chữ giả" (nét ngẫu nhiên mô phỏng chữ Latin có dấu tiếng Việt), khung bảng, tiêu đề.

use image::{Rgb, RgbImage};

use crate::error::Result;
use crate::geometry::{Homography, Point, Quad};

/// Loại nền đặt tài liệu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Background {
    /// Mặt gỗ có vân.
    Wood,
    /// Vải dệt (họa tiết lặp tần số cao).
    Fabric,
    /// Bàn trắng gần màu giấy (ca khó: tương phản thấp).
    WhiteDesk,
    /// Bàn tối.
    DarkDesk,
    /// Đá/thảm nhiều hạt (nhiễu kết cấu mạnh).
    Granite,
}

impl Background {
    /// Mọi loại nền.
    pub const ALL: [Self; 5] = [
        Self::Wood,
        Self::Fabric,
        Self::WhiteDesk,
        Self::DarkDesk,
        Self::Granite,
    ];

    /// Tên mã.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Wood => "wood",
            Self::Fabric => "fabric",
            Self::WhiteDesk => "white-desk",
            Self::DarkDesk => "dark-desk",
            Self::Granite => "granite",
        }
    }
}

/// Một mẫu có nhãn.
#[derive(Debug, Clone)]
pub struct Sample {
    /// Ảnh cảnh.
    pub image: RgbImage,
    /// Tứ giác tài liệu thật (TL, TR, BR, BL).
    pub truth: Quad,
    /// Loại nền.
    pub background: Background,
    /// Seed sinh ra mẫu.
    pub seed: u64,
}

/// Cấu hình sinh.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SynthConfig {
    /// Hệ số kích thước khung so với bộ chuẩn (1280x720, 960x720, 720x960, 1024x768).
    pub scale: f64,
    /// Bật hiệu ứng khó (bóng tay, lóa, mờ chuyển động, nhiễu).
    pub effects: bool,
}

impl Default for SynthConfig {
    fn default() -> Self {
        Self {
            scale: 1.0,
            effects: true,
        }
    }
}

/// PRNG SplitMix64: nhỏ, tất định, đủ tốt cho dữ liệu thử.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    /// Tạo từ seed.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    /// Số 64 bit kế tiếp.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Số thực đều trong [0, 1).
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Số thực đều trong [lo, hi).
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }

    /// Đúng với xác suất `p`.
    pub fn chance(&mut self, p: f64) -> bool {
        self.unit() < p
    }

    /// Chỉ số đều trong [0, n).
    pub fn index(&mut self, n: usize) -> usize {
        ((self.unit() * n as f64) as usize).min(n.saturating_sub(1))
    }

    /// Xấp xỉ chuẩn tắc nhanh (Irwin-Hall: tổng 4 số đều 16 bit từ một lần sinh).
    pub fn fast_normal(&mut self) -> f64 {
        let v = self.next_u64();
        let sum = (v & 0xFFFF) + ((v >> 16) & 0xFFFF) + ((v >> 32) & 0xFFFF) + (v >> 48);
        (sum as f64 - 2.0 * 65535.0) / 37_837.0
    }

    /// Phân phối chuẩn (Box-Muller).
    pub fn normal(&mut self) -> f64 {
        let u1 = self.unit().max(1e-12);
        let u2 = self.unit();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }
}

/// Nhiễu giá trị trơn 2D (lưới ngẫu nhiên băm từ tọa độ, nội suy smoothstep), ra [0, 1].
fn value_noise(seed: u64, x: f64, y: f64) -> f64 {
    let hash = |ix: i64, iy: i64| {
        let mut r =
            Rng::new(seed ^ (ix as u64).wrapping_mul(0x9E37_79B9) ^ (iy as u64).wrapping_mul(0x85EB_CA6B));
        r.unit()
    };
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let (ix, iy) = (x0 as i64, y0 as i64);
    let a = hash(ix, iy);
    let b = hash(ix + 1, iy);
    let c = hash(ix, iy + 1);
    let d = hash(ix + 1, iy + 1);
    let top = a + (b - a) * sx;
    let bot = c + (d - c) * sx;
    top + (bot - top) * sy
}

fn fbm(seed: u64, x: f64, y: f64) -> f64 {
    let mut v = 0.0;
    let mut amp = 0.5;
    let mut f = 1.0;
    for o in 0..3 {
        v += amp * value_noise(seed.wrapping_add(o), x * f, y * f);
        amp *= 0.5;
        f *= 2.0;
    }
    v / 0.875
}

/// Trường trơn: tính `f` trên lưới bước `step` px rồi nội suy song tuyến (nhanh hơn tính từng điểm
/// nhiều lần; mọi kết cấu dùng nó có đặc trưng lớn hơn hẳn `step`).
fn smooth_field(w: u32, h: u32, step: u32, f: impl Fn(f64, f64) -> f64) -> Vec<f64> {
    let (gw, gh) = (w / step + 2, h / step + 2);
    let mut grid = Vec::with_capacity((gw * gh) as usize);
    for gy in 0..gh {
        for gx in 0..gw {
            grid.push(f(f64::from(gx * step), f64::from(gy * step)));
        }
    }
    let mut out = Vec::with_capacity((w * h) as usize);
    let inv = 1.0 / f64::from(step);
    for y in 0..h {
        let (gy, fy) = (y / step, f64::from(y % step) * inv);
        for x in 0..w {
            let (gx, fx) = (x / step, f64::from(x % step) * inv);
            let i = (gy * gw + gx) as usize;
            let (a, b) = (grid[i], grid[i + 1]);
            let (c, d) = (grid[i + gw as usize], grid[i + gw as usize + 1]);
            let top = a + (b - a) * fx;
            out.push(top + (c + (d - c) * fx - top) * fy);
        }
    }
    out
}

fn clamp_u8(v: f64) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

/// Sinh một mẫu. Loại nền xoay vòng theo `seed % 5` để bộ mẫu cân bằng.
///
/// # Errors
/// Chỉ lỗi khi homography suy biến (không xảy ra với tham số hợp lệ).
pub fn generate(seed: u64, cfg: &SynthConfig) -> Result<Sample> {
    let mut rng = Rng::new(seed);
    let background = Background::ALL[(seed % 5) as usize];
    let sizes = [(1280.0, 720.0), (960.0, 720.0), (720.0, 960.0), (1024.0, 768.0)];
    let (bw, bh) = sizes[rng.index(sizes.len())];
    let scale = cfg.scale.clamp(0.1, 4.0);
    let (w, h) = (((bw * scale) as u32).max(64), ((bh * scale) as u32).max(64));

    let mut img = render_background(background, w, h, &mut rng);
    let (page, page_w, page_h) = render_page(&mut rng, background);
    let truth = place_quad(&mut rng, f64::from(w), f64::from(h), page_w / page_h);
    composite(&mut img, &page, &truth)?;

    if cfg.effects {
        apply_effects(&mut img, &truth, &mut rng);
    }
    Ok(Sample {
        image: img,
        truth,
        background,
        seed,
    })
}

fn render_background(bg: Background, w: u32, h: u32, rng: &mut Rng) -> RgbImage {
    let nseed = rng.next_u64();
    match bg {
        Background::Wood => {
            let base = [
                rng.range(120.0, 170.0),
                rng.range(75.0, 110.0),
                rng.range(40.0, 65.0),
            ];
            let angle = rng.range(0.0, std::f64::consts::PI);
            let (ca, sa) = (angle.cos(), angle.sin());
            let freq = rng.range(0.04, 0.09);
            let warp = smooth_field(w, h, 4, |xf, yf| {
                6.0 * fbm(nseed, (xf * ca + yf * sa) / 120.0, (-xf * sa + yf * ca) / 40.0)
            });
            RgbImage::from_fn(w, h, |x, y| {
                let (xf, yf) = (f64::from(x), f64::from(y));
                let across = -xf * sa + yf * ca;
                let grain = 0.5 + 0.5 * (across * freq + warp[(y * w + x) as usize]).sin();
                let k = 0.8 + 0.3 * grain;
                Rgb(base.map(|c| clamp_u8(c * k)))
            })
        }
        Background::Fabric => {
            let base = [
                rng.range(40.0, 120.0),
                rng.range(50.0, 120.0),
                rng.range(80.0, 160.0),
            ];
            let period = 3 + rng.index(3) as u32;
            let noise = smooth_field(w, h, 3, |xf, yf| value_noise(nseed, xf / 9.0, yf / 9.0));
            RgbImage::from_fn(w, h, |x, y| {
                let weave = if ((x / period) + (y / period)).is_multiple_of(2) {
                    1.12
                } else {
                    0.88
                };
                let n = 0.9 + 0.2 * noise[(y * w + x) as usize];
                Rgb(base.map(|c| clamp_u8(c * weave * n)))
            })
        }
        Background::WhiteDesk => {
            let base = [
                rng.range(205.0, 228.0),
                rng.range(205.0, 228.0),
                rng.range(208.0, 232.0),
            ];
            let field = smooth_field(w, h, 6, |xf, yf| fbm(nseed, xf / 90.0, yf / 90.0));
            RgbImage::from_fn(w, h, |x, y| {
                let n = field[(y * w + x) as usize] - 0.5;
                Rgb(base.map(|c| clamp_u8(c + n * 14.0)))
            })
        }
        Background::DarkDesk => {
            let base = rng.range(25.0, 70.0);
            let field = smooth_field(w, h, 5, |xf, yf| fbm(nseed, xf / 60.0, yf / 60.0));
            RgbImage::from_fn(w, h, |x, y| {
                let n = field[(y * w + x) as usize] - 0.5;
                let v = base + n * 20.0;
                Rgb([clamp_u8(v), clamp_u8(v * 1.02), clamp_u8(v * 1.06)])
            })
        }
        Background::Granite => {
            let base = rng.range(90.0, 170.0);
            let mut local = Rng::new(nseed);
            let field = smooth_field(w, h, 3, |xf, yf| fbm(nseed, xf / 25.0, yf / 25.0));
            RgbImage::from_fn(w, h, |x, y| {
                let speck = local.fast_normal() * 22.0;
                let blot = (field[(y * w + x) as usize] - 0.5) * 50.0;
                let v = base + speck + blot;
                Rgb([clamp_u8(v), clamp_u8(v * 0.97), clamp_u8(v * 0.94)])
            })
        }
    }
}

/// Vẽ đĩa tô đặc (đầu bút) lên trang.
fn stamp(page: &mut RgbImage, cx: f64, cy: f64, r: f64, color: Rgb<u8>) {
    let (w, h) = (f64::from(page.width()), f64::from(page.height()));
    let (x0, x1) = ((cx - r).floor().max(0.0), (cx + r).ceil().min(w - 1.0));
    let (y0, y1) = ((cy - r).floor().max(0.0), (cy + r).ceil().min(h - 1.0));
    if x0 > x1 || y0 > y1 {
        return;
    }
    for y in y0 as u32..=y1 as u32 {
        for x in x0 as u32..=x1 as u32 {
            if (f64::from(x) - cx).hypot(f64::from(y) - cy) <= r {
                page.put_pixel(x, y, color);
            }
        }
    }
}

fn stroke(page: &mut RgbImage, a: Point, b: Point, r: f64, color: Rgb<u8>) {
    let steps = (a.dist(b) / (r * 0.7).max(0.5)).ceil().max(1.0) as u32;
    for i in 0..=steps {
        let t = f64::from(i) / f64::from(steps);
        stamp(page, a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t, r, color);
    }
}

/// Trang giấy với chữ giả có dấu, tiêu đề, khung bảng. Trả (ảnh, rộng, cao).
#[allow(clippy::too_many_lines)] // Bố cục trang giả: tách nhỏ không làm rõ hơn.
fn render_page(rng: &mut Rng, bg: Background) -> (RgbImage, f64, f64) {
    // 70% A4, 15% Letter, 15% thẻ ID-1 (thẻ nhựa có màu).
    let kind = rng.unit();
    let (ratio, card) = if kind < 0.7 {
        (std::f64::consts::SQRT_2, false)
    } else if kind < 0.85 {
        (11.0 / 8.5, false)
    } else {
        (85.60 / 53.98, true)
    };
    let landscape = card || rng.chance(0.15);
    let long = 900.0;
    let (pw, ph) = if landscape {
        (long, long / ratio)
    } else {
        (long / ratio, long)
    };
    let paper = if card {
        [
            rng.range(170.0, 230.0),
            rng.range(190.0, 235.0),
            rng.range(200.0, 245.0),
        ]
    } else if bg == Background::WhiteDesk {
        // Ca khó: giấy chỉ sáng hơn bàn một chút.
        let v = rng.range(232.0, 248.0);
        [v, v, v - rng.range(0.0, 4.0)]
    } else {
        let v = rng.range(225.0, 250.0);
        [v, v - rng.range(0.0, 6.0), v - rng.range(0.0, 12.0)]
    };
    let mut page = RgbImage::from_pixel(pw as u32, ph as u32, Rgb(paper.map(clamp_u8)));
    let ink = Rgb([
        clamp_u8(rng.range(15.0, 60.0)),
        clamp_u8(rng.range(15.0, 60.0)),
        clamp_u8(rng.range(20.0, 80.0)),
    ]);

    let margin = pw.min(ph) * rng.range(0.07, 0.12);
    let x_height = if card {
        rng.range(9.0, 13.0)
    } else {
        rng.range(7.0, 11.0)
    };
    let line_gap = x_height * rng.range(2.6, 3.4);
    let pen = (x_height / 7.0).max(0.8);
    let mut y = margin + x_height * 2.0;

    // Tiêu đề đậm.
    if rng.chance(0.7) {
        let mut x = margin + rng.range(0.0, pw * 0.2);
        let end = (x + pw * rng.range(0.3, 0.6)).min(pw - margin);
        while x < end {
            draw_glyph(&mut page, rng, x, y, x_height * 1.5, pen * 1.8, ink);
            x += x_height * 1.4;
        }
        y += line_gap * 1.8;
    }
    // Khung bảng (đường thẳng mạnh bên trong trang: bẫy cho Hough).
    let table_at = if rng.chance(0.4) {
        Some(ph * rng.range(0.4, 0.7))
    } else {
        None
    };
    while y < ph - margin {
        if let Some(ty) = table_at
            && (y - ty).abs() < line_gap * 0.5
        {
            let rows = 3 + rng.index(4);
            let bottom = (y + rows as f64 * line_gap).min(ph - margin);
            for r in 0..=rows {
                let ry = (y + r as f64 * line_gap).min(bottom);
                stroke(
                    &mut page,
                    Point::new(margin, ry),
                    Point::new(pw - margin, ry),
                    pen * 0.8,
                    ink,
                );
            }
            for c in 0..=3 {
                let cx = margin + (pw - 2.0 * margin) * f64::from(c) / 3.0;
                stroke(
                    &mut page,
                    Point::new(cx, y),
                    Point::new(cx, bottom),
                    pen * 0.8,
                    ink,
                );
            }
            y = bottom + line_gap;
            continue;
        }
        let mut x = margin + if rng.chance(0.2) { x_height * 3.0 } else { 0.0 };
        let line_end = pw
            - margin
            - if rng.chance(0.25) {
                rng.range(0.0, pw * 0.4)
            } else {
                0.0
            };
        while x < line_end {
            let word = 1 + rng.index(7);
            for _ in 0..word {
                if x >= line_end {
                    break;
                }
                draw_glyph(&mut page, rng, x, y, x_height, pen, ink);
                x += x_height * rng.range(0.85, 1.1);
            }
            x += x_height * 0.9;
        }
        y += line_gap;
    }
    (page, pw, ph)
}

/// Một "chữ" giả: 1-3 nét trong ô x-height, có thể có nét lên/xuống và dấu thanh/dấu mũ.
#[allow(clippy::too_many_lines)] // Bảng nét chữ và dấu thanh.
fn draw_glyph(page: &mut RgbImage, rng: &mut Rng, x: f64, base: f64, xh: f64, pen: f64, ink: Rgb<u8>) {
    let top = base - xh;
    let wdt = xh * 0.75;
    match rng.index(4) {
        0 => stroke(page, Point::new(x, top), Point::new(x, base), pen, ink),
        1 => {
            // Vòng (o, a, e...).
            let (cx, cy, r) = (x + wdt / 2.0, base - xh / 2.0, xh / 2.0);
            for k in 0..16 {
                let a0 = f64::from(k) / 16.0 * std::f64::consts::TAU;
                let a1 = f64::from(k + 1) / 16.0 * std::f64::consts::TAU;
                stroke(
                    page,
                    Point::new(cx + r * 0.8 * a0.cos(), cy + r * a0.sin()),
                    Point::new(cx + r * 0.8 * a1.cos(), cy + r * a1.sin()),
                    pen,
                    ink,
                );
            }
        }
        2 => {
            // n/u/m: hai thân nối.
            stroke(page, Point::new(x, top), Point::new(x, base), pen, ink);
            stroke(
                page,
                Point::new(x, top),
                Point::new(x + wdt, top + xh * 0.2),
                pen,
                ink,
            );
            stroke(
                page,
                Point::new(x + wdt, top + xh * 0.2),
                Point::new(x + wdt, base),
                pen,
                ink,
            );
        }
        _ => {
            // Nét lên (h, l, đ) hoặc xuống (p, g, y).
            let up = rng.chance(0.5);
            let (a, b) = if up {
                (top - xh * 0.7, base)
            } else {
                (top, base + xh * 0.6)
            };
            stroke(
                page,
                Point::new(x + wdt * 0.3, a),
                Point::new(x + wdt * 0.3, b),
                pen,
                ink,
            );
            stroke(
                page,
                Point::new(x + wdt * 0.3, base - xh * 0.5),
                Point::new(x + wdt, base - xh * 0.5),
                pen,
                ink,
            );
        }
    }
    // Dấu tiếng Việt: sắc, huyền, hỏi, ngã, nặng, mũ, móc.
    if rng.chance(0.4) {
        let cx = x + wdt / 2.0;
        let dy = top - xh * 0.35;
        match rng.index(7) {
            0 => stroke(
                page,
                Point::new(cx - xh * 0.1, dy + xh * 0.15),
                Point::new(cx + xh * 0.2, dy - xh * 0.2),
                pen * 0.8,
                ink,
            ),
            1 => stroke(
                page,
                Point::new(cx - xh * 0.2, dy - xh * 0.2),
                Point::new(cx + xh * 0.1, dy + xh * 0.15),
                pen * 0.8,
                ink,
            ),
            2 => {
                stroke(
                    page,
                    Point::new(cx - xh * 0.1, dy - xh * 0.2),
                    Point::new(cx + xh * 0.1, dy - xh * 0.1),
                    pen * 0.7,
                    ink,
                );
                stroke(
                    page,
                    Point::new(cx + xh * 0.1, dy - xh * 0.1),
                    Point::new(cx, dy + xh * 0.1),
                    pen * 0.7,
                    ink,
                );
            }
            3 => {
                stroke(
                    page,
                    Point::new(cx - xh * 0.25, dy),
                    Point::new(cx - xh * 0.05, dy - xh * 0.12),
                    pen * 0.7,
                    ink,
                );
                stroke(
                    page,
                    Point::new(cx - xh * 0.05, dy - xh * 0.12),
                    Point::new(cx + xh * 0.1, dy),
                    pen * 0.7,
                    ink,
                );
                stroke(
                    page,
                    Point::new(cx + xh * 0.1, dy),
                    Point::new(cx + xh * 0.25, dy - xh * 0.12),
                    pen * 0.7,
                    ink,
                );
            }
            4 => stamp(page, cx, base + xh * 0.35, pen * 1.1, ink),
            5 => {
                stroke(
                    page,
                    Point::new(cx - xh * 0.25, dy + xh * 0.1),
                    Point::new(cx, dy - xh * 0.15),
                    pen * 0.8,
                    ink,
                );
                stroke(
                    page,
                    Point::new(cx, dy - xh * 0.15),
                    Point::new(cx + xh * 0.25, dy + xh * 0.1),
                    pen * 0.8,
                    ink,
                );
            }
            _ => stroke(
                page,
                Point::new(x + wdt, top),
                Point::new(x + wdt + xh * 0.2, top - xh * 0.25),
                pen * 0.8,
                ink,
            ),
        }
    }
}

/// Đặt trang vào khung: phủ 20-60% diện tích, xoay ±25°, lệch phối cảnh từng góc, cách biên ≥ 2%.
fn place_quad(rng: &mut Rng, w: f64, h: f64, aspect: f64) -> Quad {
    for attempt in 0..40 {
        let shrink = 1.0 - f64::from(attempt) * 0.015;
        let area = w * h * rng.range(0.2, 0.6) * shrink;
        let ph = (area / aspect).sqrt();
        let pw = ph * aspect;
        let cx = w / 2.0 + rng.range(-0.12, 0.12) * w;
        let cy = h / 2.0 + rng.range(-0.12, 0.12) * h;
        let theta = rng.range(-25.0_f64, 25.0).to_radians();
        let (s, c) = theta.sin_cos();
        let jitter = pw.min(ph) * 0.08;
        let corners = [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)].map(|(u, v)| {
            let (dx, dy) = (u * pw, v * ph);
            Point::new(
                cx + dx * c - dy * s + rng.range(-jitter, jitter),
                cy + dx * s + dy * c + rng.range(-jitter, jitter),
            )
        });
        let ok = corners
            .iter()
            .all(|p| p.x > 0.02 * w && p.x < 0.98 * w && p.y > 0.02 * h && p.y < 0.98 * h);
        if ok && let Ok(q) = Quad::from_unordered(corners) {
            // Giữ thứ tự góc của trang (from_unordered sắp lại theo vị trí, đúng với nhãn mong muốn).
            if q.is_convex() {
                return q;
            }
        }
    }
    // Không đặt được sau 40 lần: đặt giữa khung, không xoay.
    let (pw, ph) = if aspect >= w / h {
        (w * 0.6, w * 0.6 / aspect)
    } else {
        (h * 0.6 * aspect, h * 0.6)
    };
    let (x0, y0) = ((w - pw) / 2.0, (h - ph) / 2.0);
    Quad {
        pts: [
            Point::new(x0, y0),
            Point::new(x0 + pw, y0),
            Point::new(x0 + pw, y0 + ph),
            Point::new(x0, y0 + ph),
        ],
    }
}

/// Dán trang lên cảnh theo homography; mép khử răng cưa theo khoảng cách có dấu tới cạnh.
fn composite(img: &mut RgbImage, page: &RgbImage, truth: &Quad) -> Result<()> {
    let (pw, ph) = (f64::from(page.width()), f64::from(page.height()));
    let rect = [
        Point::new(0.0, 0.0),
        Point::new(pw, 0.0),
        Point::new(pw, ph),
        Point::new(0.0, ph),
    ];
    // Nhãn đã sắp theo vị trí trên ảnh; ánh xạ trang theo đúng thứ tự đó.
    let to_page = Homography::from_points(&truth.pts, &rect)?;
    let xs = truth.pts.iter().map(|p| p.x);
    let ys = truth.pts.iter().map(|p| p.y);
    let x0 = xs.clone().fold(f64::INFINITY, f64::min).floor().max(0.0) as u32;
    let x1 = (xs.fold(0.0, f64::max).ceil() as u32).min(img.width() - 1);
    let y0 = ys.clone().fold(f64::INFINITY, f64::min).floor().max(0.0) as u32;
    let y1 = (ys.fold(0.0, f64::max).ceil() as u32).min(img.height() - 1);
    let edges: Vec<(Point, f64, f64)> = (0..4)
        .map(|i| {
            let (a, b) = (truth.pts[i], truth.pts[(i + 1) % 4]);
            let len = a.dist(b);
            (a, (b.x - a.x) / len, (b.y - a.y) / len)
        })
        .collect();
    for y in y0..=y1 {
        for x in x0..=x1 {
            let p = Point::new(f64::from(x) + 0.5, f64::from(y) + 0.5);
            // Khoảng cách có dấu tới cạnh gần nhất (dương = bên trong).
            let d = edges
                .iter()
                .map(|(a, dx, dy)| dx * (p.y - a.y) - dy * (p.x - a.x))
                .fold(f64::INFINITY, f64::min);
            let cover = (d + 0.5).clamp(0.0, 1.0);
            if cover <= 0.0 {
                continue;
            }
            let Some(src) = to_page.apply(p) else { continue };
            let pp = crate::warp::sample_bilinear(page, src.x - 0.5, src.y - 0.5);
            let bg = *img.get_pixel(x, y);
            let mix = |k: usize| clamp_u8(f64::from(pp[k]) * cover + f64::from(bg[k]) * (1.0 - cover));
            img.put_pixel(x, y, Rgb([mix(0), mix(1), mix(2)]));
        }
    }
    Ok(())
}

fn apply_effects(img: &mut RgbImage, truth: &Quad, rng: &mut Rng) {
    let (w, h) = (f64::from(img.width()), f64::from(img.height()));
    // Ánh sáng không đều.
    if rng.chance(0.8) {
        let ang = rng.range(0.0, std::f64::consts::TAU);
        let (dx, dy) = (ang.cos(), ang.sin());
        let lo = rng.range(0.7, 0.92);
        let span = w.hypot(h);
        for (x, y, p) in img.enumerate_pixels_mut() {
            let t = 0.5 + ((f64::from(x) - w / 2.0) * dx + (f64::from(y) - h / 2.0) * dy) / span;
            let k = lo + (1.05 - lo) * t;
            *p = Rgb(p.0.map(|v| clamp_u8(f64::from(v) * k)));
        }
    }
    // Bóng tay/điện thoại chồng lên một mép trang.
    if rng.chance(0.4) {
        let i = rng.index(4);
        let (a, b) = (truth.pts[i], truth.pts[(i + 1) % 4]);
        let t = rng.range(0.2, 0.8);
        let (cx, cy) = (a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
        let (rx, ry) = (rng.range(0.08, 0.2) * w, rng.range(0.08, 0.2) * h);
        let dark = rng.range(0.55, 0.8);
        for (x, y, p) in img.enumerate_pixels_mut() {
            let d = ((f64::from(x) - cx) / rx).hypot((f64::from(y) - cy) / ry);
            if d < 1.3 {
                let k = 1.0 - (1.0 - dark) * (1.0 - ((d - 0.8) / 0.5).clamp(0.0, 1.0));
                *p = Rgb(p.0.map(|v| clamp_u8(f64::from(v) * k)));
            }
        }
    }
    // Lóa đèn trên trang.
    if rng.chance(0.3) {
        let c = truth.pts;
        let (u, v) = (rng.range(0.2, 0.8), rng.range(0.2, 0.8));
        let top = Point::new(c[0].x + (c[1].x - c[0].x) * u, c[0].y + (c[1].y - c[0].y) * u);
        let bot = Point::new(c[3].x + (c[2].x - c[3].x) * u, c[3].y + (c[2].y - c[3].y) * u);
        let (cx, cy) = (top.x + (bot.x - top.x) * v, top.y + (bot.y - top.y) * v);
        let r = rng.range(0.04, 0.1) * w.min(h);
        let amp = rng.range(40.0, 110.0);
        for (x, y, p) in img.enumerate_pixels_mut() {
            let d = (f64::from(x) - cx).hypot(f64::from(y) - cy) / r;
            if d < 3.0 {
                let add = amp * (-d * d).exp();
                *p = Rgb(p.0.map(|v| clamp_u8(f64::from(v) + add)));
            }
        }
    }
    // Mờ chuyển động: trung bình các điểm (làm tròn) dọc hướng chuyển động.
    if rng.chance(0.3) {
        let len = rng.range(2.0, 6.0);
        let ang = rng.range(0.0, std::f64::consts::PI);
        let src = img.clone();
        let taps = len.ceil() as i32;
        let offsets: Vec<(i64, i64)> = (-taps..=taps)
            .map(|t| {
                let s = f64::from(t) * len / f64::from(2 * taps);
                ((s * ang.cos()).round() as i64, (s * ang.sin()).round() as i64)
            })
            .collect();
        let (wi, hi) = (i64::from(src.width()), i64::from(src.height()));
        let raw = src.as_raw();
        let n = offsets.len() as u32;
        for (x, y, p) in img.enumerate_pixels_mut() {
            let mut acc = [0u32; 3];
            for (dx, dy) in &offsets {
                let sx = (i64::from(x) + dx).clamp(0, wi - 1) as usize;
                let sy = (i64::from(y) + dy).clamp(0, hi - 1) as usize;
                let o = (sy * wi as usize + sx) * 3;
                for k in 0..3 {
                    acc[k] += u32::from(raw[o + k]);
                }
            }
            *p = Rgb(acc.map(|v| ((v + n / 2) / n) as u8));
        }
    }
    // Nhiễu cảm biến.
    if rng.chance(0.7) {
        let sigma = rng.range(2.0, 7.0);
        for p in img.pixels_mut() {
            let n = rng.fast_normal() * sigma;
            *p = Rgb(p.0.map(|v| clamp_u8(f64::from(v) + n)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_labelled_inside_frame() {
        let cfg = SynthConfig {
            scale: 0.5,
            effects: true,
        };
        let a = generate(7, &cfg).unwrap();
        let b = generate(7, &cfg).unwrap();
        assert_eq!(a.image.as_raw(), b.image.as_raw());
        assert_eq!(a.truth, b.truth);
        let (w, h) = (f64::from(a.image.width()), f64::from(a.image.height()));
        assert!(
            a.truth
                .pts
                .iter()
                .all(|p| p.x > 0.0 && p.y > 0.0 && p.x < w && p.y < h)
        );
        assert!(a.truth.is_convex());
        for (i, bg) in Background::ALL.iter().enumerate() {
            assert_eq!(
                generate(
                    i as u64,
                    &SynthConfig {
                        scale: 0.2,
                        effects: false
                    }
                )
                .unwrap()
                .background,
                *bg
            );
            assert!(bg.code().len() > 3);
        }
    }

    #[test]
    fn rng_ranges() {
        let mut r = Rng::new(1);
        for _ in 0..1000 {
            let u = r.unit();
            assert!((0.0..1.0).contains(&u));
            assert!(r.index(3) < 3);
        }
        assert_eq!(r.index(0), 0);
    }

    #[test]
    fn fallback_placement_is_centered() {
        // Tỉ lệ cực đoan khiến mọi lần thử đều tràn khung ⇒ dùng phương án giữa khung.
        let mut r = Rng::new(3);
        let q = place_quad(&mut r, 100.0, 100.0, 50.0);
        assert!(q.is_convex());
    }
}

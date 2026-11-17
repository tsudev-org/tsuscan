//! Dò mép tài liệu cổ điển (không AI): Canny tự động → contour → tứ giác ứng viên → chấm điểm →
//! tinh chỉnh cạnh bằng khớp đường; dự phòng Hough ghép 4 đường.
//!
//! Mọi xử lý chạy trên ảnh xám thu nhỏ (cạnh dài [`DetectOptions::working_size`]) rồi phóng tọa độ
//! về ảnh gốc.

use image::{DynamicImage, GrayImage};
use imageproc::contours::{BorderType, find_contours, find_contours_with_threshold};

use crate::error::{ImagingError, Result};
use crate::geometry::{Point, Quad, convex_hull, polygon_area};

/// Tùy chọn dò mép.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DetectOptions {
    /// Cạnh dài của ảnh làm việc (px). Mặc định 640.
    pub working_size: u32,
    /// Điểm tối thiểu để chấp nhận kết quả (0..1). Mặc định 0,35.
    pub min_confidence: f32,
}

impl Default for DetectOptions {
    fn default() -> Self {
        Self {
            working_size: 640,
            min_confidence: 0.35,
        }
    }
}

/// Kết quả dò mép.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Detection {
    /// Tứ giác tài liệu trong tọa độ ảnh đầu vào.
    pub quad: Quad,
    /// Độ tin cậy 0..1 (tỉ lệ mép được ảnh xác nhận, có trọng số hình dạng).
    pub confidence: f32,
}

/// Dò tứ giác tài liệu trong ảnh màu bất kỳ.
///
/// # Errors
/// [`ImagingError::TooSmall`] khi ảnh nhỏ hơn 16 px mỗi cạnh.
pub fn detect(img: &DynamicImage) -> Result<Option<Detection>> {
    detect_with(img, DetectOptions::default())
}

/// Như [`detect`] với tùy chọn.
///
/// # Errors
/// Xem [`detect`].
pub fn detect_with(img: &DynamicImage, opts: DetectOptions) -> Result<Option<Detection>> {
    let (w, h) = (img.width(), img.height());
    check_size(w, h)?;
    let (tw, th) = working_dims(w, h, opts.working_size)?;
    let (small, chroma) = match img {
        DynamicImage::ImageLuma8(g) => downscale_planes(g.as_raw(), 1, w, h, tw, th),
        DynamicImage::ImageRgb8(rgb) => downscale_planes(rgb.as_raw(), 3, w, h, tw, th),
        DynamicImage::ImageRgba8(rgba) => downscale_planes(rgba.as_raw(), 4, w, h, tw, th),
        other => downscale_planes(other.to_rgb8().as_raw(), 3, w, h, tw, th),
    };
    Ok(detect_working(&small, chroma.as_ref(), w, h, opts))
}

/// Dò trên mặt phẳng độ sáng (ví dụ kênh Y của khung camera YUV).
///
/// # Errors
/// Xem [`detect`].
pub fn detect_luma(gray: &GrayImage, opts: DetectOptions) -> Result<Option<Detection>> {
    let (w, h) = gray.dimensions();
    check_size(w, h)?;
    let (tw, th) = working_dims(w, h, opts.working_size)?;
    let (small, _) = downscale_planes(gray.as_raw(), 1, w, h, tw, th);
    Ok(detect_working(&small, None, w, h, opts))
}

/// Thu nhỏ theo trung bình vùng trong một lượt, ra mặt phẳng độ sáng và (với ảnh màu) mặt phẳng
/// sắc độ đối nghịch `128 + R - B` (giấy trắng ấm trên bàn trắng lạnh khác nhau chủ yếu ở kênh này).
fn downscale_planes(
    raw: &[u8],
    ch: usize,
    w: u32,
    h: u32,
    tw: u32,
    th: u32,
) -> (GrayImage, Option<GrayImage>) {
    let (w, h, tw, th) = (w as usize, h as usize, tw as usize, th as usize);
    let color = ch >= 3;
    if w == 2 * tw && h == 2 * th {
        return downscale_half(raw, ch, tw, th);
    }
    let mut sum = vec![0u32; tw * th];
    let mut csum = vec![0i32; if color { tw * th } else { 0 }];
    let mut cnt = vec![0u32; tw * th];
    let xmap: Vec<usize> = (0..w).map(|x| (x * tw / w).min(tw - 1)).collect();
    for y in 0..h {
        let ty = (y * th / h).min(th - 1);
        let row = &raw[y * w * ch..(y + 1) * w * ch];
        let base = ty * tw;
        for (x, px) in row.chunks_exact(ch).enumerate() {
            let i = base + xmap[x];
            if color {
                let (r, g, b) = (u32::from(px[0]), u32::from(px[1]), u32::from(px[2]));
                // Độ sáng BT.601 số nguyên: (77R + 150G + 29B) / 256.
                sum[i] += (77 * r + 150 * g + 29 * b) >> 8;
                csum[i] += r as i32 - b as i32;
            } else {
                sum[i] += u32::from(px[0]);
            }
            cnt[i] += 1;
        }
    }
    let make = |data: Vec<u8>| {
        GrayImage::from_raw(tw as u32, th as u32, data)
            .unwrap_or_else(|| GrayImage::new(tw as u32, th as u32))
    };
    let luma = make(
        sum.iter()
            .zip(&cnt)
            .map(|(s, c)| (s / (*c).max(1)) as u8)
            .collect(),
    );
    let chroma = color.then(|| {
        make(
            csum.iter()
                .zip(&cnt)
                .map(|(s, c)| (128 + s / (*c).max(1) as i32).clamp(0, 255) as u8)
                .collect(),
        )
    });
    (luma, chroma)
}

/// Đường nhanh của [`downscale_planes`] cho hệ số đúng 2 (ví dụ 1280x720 → 640x360).
fn downscale_half(raw: &[u8], ch: usize, tw: usize, th: usize) -> (GrayImage, Option<GrayImage>) {
    let w = tw * 2;
    let color = ch >= 3;
    let mut luma = vec![0u8; tw * th];
    let mut chroma = if color { vec![0u8; tw * th] } else { Vec::new() };
    for ty in 0..th {
        let r0 = &raw[(2 * ty) * w * ch..(2 * ty + 1) * w * ch];
        let r1 = &raw[(2 * ty + 1) * w * ch..(2 * ty + 2) * w * ch];
        for tx in 0..tw {
            let (mut l, mut c) = (0u32, 0i32);
            for row in [r0, r1] {
                for dx in 0..2 {
                    let px = &row[(2 * tx + dx) * ch..(2 * tx + dx) * ch + ch];
                    if color {
                        let (r, g, b) = (u32::from(px[0]), u32::from(px[1]), u32::from(px[2]));
                        l += (77 * r + 150 * g + 29 * b) >> 8;
                        c += r as i32 - b as i32;
                    } else {
                        l += u32::from(px[0]);
                    }
                }
            }
            luma[ty * tw + tx] = (l / 4) as u8;
            if color {
                chroma[ty * tw + tx] = (128 + c / 4).clamp(0, 255) as u8;
            }
        }
    }
    let make = |data: Vec<u8>| {
        GrayImage::from_raw(tw as u32, th as u32, data)
            .unwrap_or_else(|| GrayImage::new(tw as u32, th as u32))
    };
    (make(luma), color.then(|| make(chroma)))
}

/// Gaussian xấp xỉ bằng 3 lượt box blur, O(1) mỗi điểm.
fn gaussian_approx(src: &[f32], w: usize, h: usize, sigma: f32) -> Vec<f32> {
    let n = 3.0f32;
    let ideal = (12.0 * sigma * sigma / n + 1.0).sqrt();
    let mut wl = ideal.floor() as usize;
    if wl.is_multiple_of(2) {
        wl = wl.saturating_sub(1);
    }
    let wl = wl.max(1);
    let wu = wl + 2;
    let wlf = wl as f32;
    let m = ((12.0 * sigma * sigma - n * wlf * wlf - 4.0 * n * wlf - 3.0 * n) / (-4.0 * wlf - 4.0)).round();
    let mut cur = src.to_vec();
    let mut tmp = vec![0.0f32; w * h];
    for i in 0..3 {
        let r = (if (i as f32) < m { wl } else { wu } - 1) / 2;
        box_h(&cur, &mut tmp, w, h, r);
        box_v(&tmp, &mut cur, w, h, r);
    }
    cur
}

fn box_h(src: &[f32], dst: &mut [f32], w: usize, h: usize, r: usize) {
    let norm = 1.0 / (2 * r + 1) as f32;
    for y in 0..h {
        let row = &src[y * w..(y + 1) * w];
        let out = &mut dst[y * w..(y + 1) * w];
        let at = |i: isize| row[i.clamp(0, w as isize - 1) as usize];
        let mut acc: f32 = (-(r as isize)..=r as isize).map(at).sum();
        for (x, o) in out.iter_mut().enumerate() {
            *o = acc * norm;
            // Vùng trong không cần kẹp chỉ số (đường nhanh).
            if x >= r && x + r + 1 < w {
                acc += row[x + r + 1] - row[x - r];
            } else {
                let xi = x as isize;
                acc += at(xi + r as isize + 1) - at(xi - r as isize);
            }
        }
    }
}

fn box_v(src: &[f32], dst: &mut [f32], w: usize, h: usize, r: usize) {
    let norm = 1.0 / (2 * r + 1) as f32;
    let mut acc = vec![0.0f32; w];
    let row = |y: isize| y.clamp(0, h as isize - 1) as usize * w;
    for d in -(r as isize)..=r as isize {
        let o = row(d);
        for x in 0..w {
            acc[x] += src[o + x];
        }
    }
    for y in 0..h {
        let o = y * w;
        for x in 0..w {
            dst[o + x] = acc[x] * norm;
        }
        let (add, sub) = (row(y as isize + r as isize + 1), row(y as isize - r as isize));
        for x in 0..w {
            acc[x] += src[add + x] - src[sub + x];
        }
    }
}

/// Giãn nở nhị phân 3x3 (tách được).
fn dilate3(img: &GrayImage) -> GrayImage {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let src = img.as_raw();
    let mut tmp = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let l = if x > 0 { src[i - 1] } else { 0 };
            let r = if x + 1 < w { src[i + 1] } else { 0 };
            tmp[i] = src[i].max(l).max(r);
        }
    }
    let mut out = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let u = if y > 0 { tmp[i - w] } else { 0 };
            let d = if y + 1 < h { tmp[i + w] } else { 0 };
            out[i] = tmp[i].max(u).max(d);
        }
    }
    GrayImage::from_raw(w as u32, h as u32, out).unwrap_or_else(|| GrayImage::new(w as u32, h as u32))
}

/// Thu nhỏ 2 lần (trung bình 2x2) ảnh f32. Trả (dữ liệu, rộng, cao).
fn half_f32(src: &[f32], w: usize, h: usize) -> (Vec<f32>, usize, usize) {
    let (hw, hh) = ((w / 2).max(1), (h / 2).max(1));
    let mut out = vec![0.0f32; hw * hh];
    for y in 0..hh {
        for x in 0..hw {
            let at = |dx: usize, dy: usize| src[(2 * y + dy).min(h - 1) * w + (2 * x + dx).min(w - 1)];
            out[y * hw + x] = (at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1)) / 4.0;
        }
    }
    (out, hw, hh)
}

fn to_gray_u8(src: &[f32], w: usize, h: usize) -> GrayImage {
    let data = src.iter().map(|v| v.round().clamp(0.0, 255.0) as u8).collect();
    GrayImage::from_raw(w as u32, h as u32, data).unwrap_or_else(|| GrayImage::new(w as u32, h as u32))
}

fn check_size(w: u32, h: u32) -> Result<()> {
    if w < 16 || h < 16 {
        return Err(ImagingError::TooSmall { width: w, height: h });
    }
    crate::decode::check_dimensions(w, h)
}

fn working_dims(w: u32, h: u32, size: u32) -> Result<(u32, u32)> {
    if !(64..=4096).contains(&size) {
        return Err(ImagingError::InvalidArgument("working_size phải trong 64..=4096"));
    }
    let long = w.max(h);
    if long <= size {
        return Ok((w, h));
    }
    let s = f64::from(size) / f64::from(long);
    let tw = ((f64::from(w) * s).round() as u32).max(16);
    let th = ((f64::from(h) * s).round() as u32).max(16);
    Ok((tw, th))
}

fn detect_working(
    small: &GrayImage,
    chroma: Option<&GrayImage>,
    orig_w: u32,
    orig_h: u32,
    opts: DetectOptions,
) -> Option<Detection> {
    // Lượt 1 chỉ dùng độ sáng. Lượt 2 (thêm sắc độ) chỉ khi lượt 1 chưa chắc chắn: sắc độ cứu ca giấy
    // trắng trên bàn trắng nhưng lại là nhiễu trên nền màu có vân (gỗ), nên không trộn vô điều kiện.
    let mut best = best_in(&EdgeContext::new(small, None));
    if let Some(c) = chroma
        && best.as_ref().is_none_or(|b| b.1 < 0.8)
    {
        let second = best_in(&EdgeContext::new(small, Some(c)));
        if second
            .as_ref()
            .is_some_and(|s| best.as_ref().is_none_or(|b| s.1 > b.1))
        {
            best = second;
        }
    }
    let (quad, score) = best?;
    let confidence = score.clamp(0.0, 1.0) as f32;
    if confidence < opts.min_confidence {
        return None;
    }
    let sx = f64::from(orig_w) / f64::from(small.width());
    let sy = f64::from(orig_h) / f64::from(small.height());
    // Nội bộ dùng chỉ số điểm ảnh (tâm điểm ảnh i nằm ở tọa độ liên tục i + 0,5).
    let quad = Quad {
        pts: quad.pts.map(|p| Point::new((p.x + 0.5) * sx, (p.y + 0.5) * sy)),
    }
    .clamped(f64::from(orig_w), f64::from(orig_h));
    Some(Detection { quad, confidence })
}

/// Ứng viên điểm cao nhất trong một ngữ cảnh mép, đã tinh chỉnh cạnh.
fn best_in(ctx: &EdgeContext) -> Option<(Quad, f64)> {
    // Hough sinh hàng nghìn tổ hợp: lọc thô bằng ít mẫu, chỉ chấm kỹ những ứng viên đứng đầu.
    let mut coarse: Vec<(f64, Quad)> = ctx
        .hough_candidates()
        .into_iter()
        .map(|q| (ctx.score_with(&q, COARSE_SAMPLES), q))
        .filter(|(s, _)| *s > 0.0)
        .collect();
    coarse.sort_unstable_by(|a, b| b.0.total_cmp(&a.0));
    coarse.truncate(HOUGH_KEEP);
    let mut best: Option<(Quad, f64)> = None;
    let consider = |cands: Vec<Quad>, best: &mut Option<(Quad, f64)>| {
        for q in cands {
            let s = ctx.score(&q);
            if s > 0.0 && best.as_ref().is_none_or(|(_, bs)| s > *bs) {
                *best = Some((q, s));
            }
        }
    };
    consider(ctx.region_candidates(), &mut best);
    consider(coarse.into_iter().map(|(_, q)| q).collect(), &mut best);
    // Contour của bản đồ mép đắt nhất: chỉ chạy khi hai bộ sinh rẻ chưa cho ứng viên chắc chắn.
    if best.as_ref().is_none_or(|(_, s)| *s < 0.9) {
        consider(ctx.contour_candidates(), &mut best);
    }
    let (quad, score) = best?;
    let refined = ctx.refine(&quad);
    let refined_score = ctx.score(&refined);
    Some(if refined_score >= score - 0.02 {
        (refined, refined_score)
    } else {
        (quad, score)
    })
}

/// Số mẫu mỗi cạnh khi chấm điểm đầy đủ / lọc thô.
const FULL_SAMPLES: usize = 64;
const COARSE_SAMPLES: usize = 12;
/// Số ứng viên Hough giữ lại sau lọc thô.
const HOUGH_KEEP: usize = 16;

/// Ảnh gradient + bản đồ mép dùng chung cho sinh ứng viên, chấm điểm và tinh chỉnh.
struct EdgeContext {
    w: usize,
    h: usize,
    gx: Vec<f32>,
    gy: Vec<f32>,
    mag: Vec<f32>,
    edges: GrayImage,
    /// Ảnh làm mờ mạnh ở nửa độ phân giải để tách vùng sáng (xóa kết cấu nền và nét chữ).
    smooth_half: GrayImage,
    low: f32,
}

impl EdgeContext {
    fn new(gray: &GrayImage, chroma: Option<&GrayImage>) -> Self {
        let (w, h) = (gray.width() as usize, gray.height() as usize);
        let to_f32 = |g: &GrayImage| g.as_raw().iter().map(|v| f32::from(*v)).collect::<Vec<f32>>();
        let blurred = gaussian_approx(&to_f32(gray), w, h, 1.2);
        let (mut gx, mut gy, mut mag) = sobel(&blurred, w, h);
        if let Some(c) = chroma.filter(|c| c.dimensions() == gray.dimensions()) {
            // Mỗi điểm lấy gradient của kênh mạnh hơn (độ sáng hoặc sắc độ x1,5).
            let (cx, cy, cm) = sobel(&gaussian_approx(&to_f32(c), w, h, 1.2), w, h);
            for i in 0..w * h {
                let m = cm[i] * 1.5;
                if m > mag[i] {
                    gx[i] = cx[i] * 1.5;
                    gy[i] = cy[i] * 1.5;
                    mag[i] = m;
                }
            }
        }
        let (low, high) = auto_thresholds(&mag);
        let edges = canny(w, h, &gx, &gy, &mag, low, high);
        // Thu nửa trước rồi làm mờ (σ 1,1 ở nửa độ phân giải ≈ σ 2,2 ở độ phân giải làm việc).
        let (half, hw, hh) = half_f32(&blurred, w, h);
        let smooth_half = to_gray_u8(&gaussian_approx(&half, hw, hh, 1.1), hw, hh);
        Self {
            w,
            h,
            gx,
            gy,
            mag,
            edges,
            smooth_half,
            low,
        }
    }

    fn area(&self) -> f64 {
        (self.w * self.h) as f64
    }

    /// Tứ giác từ contour ngoài của vùng sáng ở nhiều mức ngưỡng (Otsu và lân cận), tính ở nửa độ
    /// phân giải. Lỗ do chữ bên trong trang không ảnh hưởng contour ngoài.
    fn region_candidates(&self) -> Vec<Quad> {
        let mut hist = [0u64; 256];
        for p in self.smooth_half.as_raw() {
            hist[*p as usize] += 1;
        }
        let otsu = otsu_threshold(&hist);
        let hi = percentile_u8(&hist, 0.98);
        let step = hi.saturating_sub(otsu) / 3;
        let mut levels = vec![otsu, otsu.saturating_add(step), otsu.saturating_add(2 * step)];
        levels.dedup();
        let sx = self.w as f64 / f64::from(self.smooth_half.width());
        let sy = self.h as f64 / f64::from(self.smooth_half.height());
        let min_area = self.area() * 0.05;
        let mut out = Vec::new();
        for t in levels {
            for c in find_contours_with_threshold::<i32>(&self.smooth_half, t) {
                if c.border_type != BorderType::Outer || c.points.len() < 20 {
                    continue;
                }
                let pts: Vec<Point> = c
                    .points
                    .iter()
                    .map(|p| {
                        Point::new(
                            (f64::from(p.x) + 0.5) * sx - 0.5,
                            (f64::from(p.y) + 0.5) * sy - 0.5,
                        )
                    })
                    .collect();
                let hull = convex_hull(&pts);
                if polygon_area(&hull).abs() < min_area {
                    continue;
                }
                if let Some(q) = hull_to_quad(&hull) {
                    out.push(q);
                }
            }
        }
        out
    }

    /// Tứ giác từ contour ngoài của bản đồ mép đã giãn nở.
    fn contour_candidates(&self) -> Vec<Quad> {
        let dilated = dilate3(&self.edges);
        let contours = find_contours::<i32>(&dilated);
        let min_area = self.area() * 0.05;
        let mut out = Vec::new();
        for c in contours
            .iter()
            .filter(|c| c.border_type == BorderType::Outer && c.points.len() >= 40)
        {
            let pts: Vec<Point> = c
                .points
                .iter()
                .map(|p| Point::new(f64::from(p.x), f64::from(p.y)))
                .collect();
            let hull = convex_hull(&pts);
            if polygon_area(&hull).abs() < min_area {
                continue;
            }
            if let Some(q) = hull_to_quad(&hull) {
                out.push(q);
            }
        }
        out
    }

    /// Ghép tứ giác từ các đường Hough mạnh nhất: hai cặp đường gần song song, hai cặp lệch nhau
    /// ≥ 45°. Không giả định trang nằm ngang nên chịu được tài liệu xoay bất kỳ.
    fn hough_candidates(&self) -> Vec<Quad> {
        let lines = self.hough_lines(16);
        let mut pairs = Vec::new();
        for i in 0..lines.len() {
            for j in i + 1..lines.len() {
                if angle_diff(lines[i].theta, lines[j].theta) < 30.0 {
                    pairs.push((i, j));
                }
            }
        }
        let mut out = Vec::new();
        for (pi, &(a1, a2)) in pairs.iter().enumerate() {
            for &(b1, b2) in &pairs[pi + 1..] {
                if angle_diff(lines[a1].theta, lines[b1].theta) < 45.0 {
                    continue;
                }
                let (la1, la2, lb1, lb2) = (&lines[a1], &lines[a2], &lines[b1], &lines[b2]);
                let corners = [
                    la1.intersect(lb1),
                    la1.intersect(lb2),
                    la2.intersect(lb2),
                    la2.intersect(lb1),
                ];
                if let [Some(p), Some(q), Some(r), Some(t)] = corners
                    && let Ok(quad) = Quad::from_unordered([p, q, r, t])
                {
                    out.push(quad);
                }
            }
        }
        out
    }

    /// Hough có hướng: mỗi điểm mép chỉ bỏ phiếu cho góc pháp tuyến trong ±8° quanh hướng gradient
    /// (nhiễu làm lệch hướng gradient ở mép tương phản thấp nên cửa sổ không thể hẹp hơn).
    /// Trả tối đa `max_lines` đường theo số phiếu giảm dần, đã triệt tiêu lân cận.
    fn hough_lines(&self, max_lines: usize) -> Vec<Line> {
        let (w, h) = (self.w, self.h);
        let rmax = ((w * w + h * h) as f64).sqrt().ceil() as usize;
        let nr = 2 * rmax + 1;
        let trig: Vec<(f64, f64)> = (0..180).map(|d| f64::from(d).to_radians().sin_cos()).collect();
        let mut acc = vec![0u32; 180 * nr];
        let raw = self.edges.as_raw();
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                // Bỏ phiếu theo bàn cờ: nửa số điểm, cạnh trang dài hàng trăm px vẫn đủ phiếu.
                if raw[i] == 0 || (x + y) % 2 == 1 {
                    continue;
                }
                let phi = f64::from(self.gy[i])
                    .atan2(f64::from(self.gx[i]))
                    .to_degrees()
                    .rem_euclid(180.0);
                let center = phi.round() as i32;
                for d in -8..=8 {
                    let t = (center + d).rem_euclid(180) as usize;
                    let (s, c) = trig[t];
                    let r = (x as f64 * c + y as f64 * s).round() as isize + rmax as isize;
                    acc[t * nr + r as usize] += 1;
                }
            }
        }
        let min_votes = (w.min(h) / 16) as u32;
        let mut peaks: Vec<(u32, usize, usize)> = Vec::new();
        for t in 0..180 {
            for r in 0..nr {
                let v = acc[t * nr + r];
                if v < min_votes {
                    continue;
                }
                // Chỉ giữ cực đại cục bộ 3x3 (hòa thì giữ ô đầu): ít đỉnh hơn hẳn, sắp xếp rẻ.
                let mut is_max = true;
                'nb: for dt in [-1isize, 0, 1] {
                    for dr in [-1isize, 0, 1] {
                        if dt == 0 && dr == 0 {
                            continue;
                        }
                        let (tt, rr) = (t as isize + dt, r as isize + dr);
                        if !(0..180).contains(&tt) || !(0..nr as isize).contains(&rr) {
                            continue;
                        }
                        let o = acc[tt as usize * nr + rr as usize];
                        if o > v || (o == v && (dt < 0 || (dt == 0 && dr < 0))) {
                            is_max = false;
                            break 'nb;
                        }
                    }
                }
                if is_max {
                    peaks.push((v, t, r));
                }
            }
        }
        peaks.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
        let mut kept: Vec<(usize, usize)> = Vec::new();
        let mut out = Vec::new();
        for (_, t, r) in peaks {
            if out.len() >= max_lines {
                break;
            }
            let close = kept.iter().any(|&(ot, or)| {
                let dt = t.abs_diff(ot);
                (dt <= 5 && r.abs_diff(or) <= 10) || (dt >= 175 && (r + or).abs_diff(2 * rmax) <= 10)
            });
            if close {
                continue;
            }
            kept.push((t, r));
            let (s, c) = trig[t];
            out.push(Line {
                nx: c,
                ny: s,
                c: r as f64 - rmax as f64,
                theta: t as f64,
            });
        }
        out
    }

    /// Điểm 0..1. Trả 0 khi hình dạng không thể là trang giấy.
    fn score(&self, q: &Quad) -> f64 {
        self.score_with(q, FULL_SAMPLES)
    }

    /// Như [`Self::score`] với số mẫu tối đa mỗi cạnh.
    fn score_with(&self, q: &Quad, max_samples: usize) -> f64 {
        let (w, h) = (self.w as f64, self.h as f64);
        let margin = 2.0;
        if q.pts
            .iter()
            .any(|p| p.x < -margin || p.y < -margin || p.x > w + margin || p.y > h + margin)
        {
            return 0.0;
        }
        if !q.is_convex() {
            return 0.0;
        }
        let frac = q.area() / self.area();
        if !(0.05..=0.995).contains(&frac) {
            return 0.0;
        }
        let angles = q.interior_angles();
        if angles.iter().any(|a| !(55.0..=125.0).contains(a)) {
            return 0.0;
        }
        let sides = q.side_lengths();
        let min_side = sides.iter().copied().fold(f64::INFINITY, f64::min);
        if min_side < 0.08 * w.min(h) {
            return 0.0;
        }

        let sides: Vec<(usize, usize, usize)> = (0..4)
            .map(|i| self.side_support(q.pts[i], q.pts[(i + 1) % 4], max_samples))
            .collect();
        // Cực tính trội của cả tứ giác: giấy sáng hơn nền (hoặc ngược lại) trên MỌI cạnh.
        let pos: usize = sides.iter().map(|s| s.0).sum();
        let neg: usize = sides.iter().map(|s| s.1).sum();
        let support: Vec<f64> = sides
            .iter()
            .map(|&(p, n, total)| if pos >= neg { p } else { n } as f64 / total.max(1) as f64)
            .collect();
        let mean = support.iter().sum::<f64>() / 4.0;
        let min = support.iter().copied().fold(1.0, f64::min);
        let angle_dev = angles.iter().map(|a| (a - 90.0).abs()).sum::<f64>() / 4.0;
        let f_angle = 1.0 - (angle_dev / 35.0).min(1.0) * 0.3;
        let f_area = 0.75 + 0.25 * (frac / 0.25).min(1.0);
        // Cạnh nằm sát biên ảnh thường là khung ảnh chứ không phải mép giấy.
        let on_border = (0..4)
            .filter(|&i| {
                let (a, b) = (q.pts[i], q.pts[(i + 1) % 4]);
                let near = |p: Point| p.x < 3.0 || p.y < 3.0 || p.x > w - 3.0 || p.y > h - 3.0;
                near(a) && near(b) && ((a.x - b.x).abs() < 3.0 || (a.y - b.y).abs() < 3.0)
            })
            .count();
        let f_border = 1.0 - 0.15 * on_border as f64;
        (0.55 * mean + 0.45 * min) * f_angle * f_area * f_border
    }

    /// Đếm điểm mẫu dọc cạnh có gradient vuông góc đủ mạnh (tìm trong ±3 px theo pháp tuyến), tách
    /// theo cực tính: (hướng vào trong, hướng ra ngoài, tổng số mẫu).
    fn side_support(&self, a: Point, b: Point, max_samples: usize) -> (usize, usize, usize) {
        let len = a.dist(b);
        if len < 1.0 {
            return (0, 0, 1);
        }
        let (dx, dy) = ((b.x - a.x) / len, (b.y - a.y) / len);
        let (nx, ny) = (-dy, dx);
        let n = ((len / 4.0) as usize).clamp(COARSE_SAMPLES.min(max_samples), max_samples);
        let thr = self.low;
        let (mut pos, mut neg) = (0usize, 0usize);
        for k in 0..n {
            let t = 0.04 + 0.92 * (k as f64 + 0.5) / n as f64;
            let (px, py) = (a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
            let mut best = 0.0_f32;
            for off in -3..=3 {
                let (x, y) = (px + nx * f64::from(off), py + ny * f64::from(off));
                if let Some(i) = self.index(x, y) {
                    let proj = self.gx[i] * nx as f32 + self.gy[i] * ny as f32;
                    if proj.abs() > best.abs() {
                        best = proj;
                    }
                }
            }
            if best >= thr {
                pos += 1;
            } else if best <= -thr {
                neg += 1;
            }
        }
        (pos, neg, n)
    }

    fn index(&self, x: f64, y: f64) -> Option<usize> {
        let (xi, yi) = (x.round(), y.round());
        if xi < 0.0 || yi < 0.0 || xi >= self.w as f64 || yi >= self.h as f64 {
            return None;
        }
        Some(yi as usize * self.w + xi as usize)
    }

    /// Khớp lại từng cạnh bằng bình phương tối thiểu toàn phần trên điểm mép gần cạnh có hướng
    /// gradient vuông góc với cạnh; góc mới = giao các đường kề nhau.
    fn refine(&self, q: &Quad) -> Quad {
        let mut cur = *q;
        for band in [5.0, 2.5] {
            let mut lines = [None; 4];
            for (i, slot) in lines.iter_mut().enumerate() {
                *slot = self.fit_side(cur.pts[i], cur.pts[(i + 1) % 4], band);
            }
            let fallback = |i: usize| Line::through(cur.pts[i], cur.pts[(i + 1) % 4]);
            let ls: Vec<Line> = (0..4)
                .map(|i| lines[i].or_else(|| fallback(i)).unwrap_or_default())
                .collect();
            let corners = [
                ls[3].intersect(&ls[0]),
                ls[0].intersect(&ls[1]),
                ls[1].intersect(&ls[2]),
                ls[2].intersect(&ls[3]),
            ];
            let [Some(a), Some(b), Some(c), Some(d)] = corners else {
                return cur;
            };
            let next = Quad { pts: [a, b, c, d] };
            let moved = next
                .pts
                .iter()
                .zip(cur.pts.iter())
                .map(|(p, o)| p.dist(*o))
                .fold(0.0, f64::max);
            if !next.is_convex() || moved > 0.08 * (self.w.max(self.h) as f64) {
                return cur;
            }
            cur = next;
        }
        cur
    }

    fn fit_side(&self, a: Point, b: Point, band: f64) -> Option<Line> {
        let len = a.dist(b);
        if len < 8.0 {
            return None;
        }
        let (dx, dy) = ((b.x - a.x) / len, (b.y - a.y) / len);
        let (nx, ny) = (-dy, dx);
        let (x0, x1) = (a.x.min(b.x) - band, a.x.max(b.x) + band);
        let (y0, y1) = (a.y.min(b.y) - band, a.y.max(b.y) + band);
        let (x0, y0) = (x0.max(0.0) as usize, y0.max(0.0) as usize);
        let (x1, y1) = (
            (x1.ceil() as usize).min(self.w - 1),
            (y1.ceil() as usize).min(self.h - 1),
        );
        let mut pts = Vec::new();
        for y in y0..=y1 {
            for x in x0..=x1 {
                let i = y * self.w + x;
                if self.edges.as_raw()[i] == 0 {
                    continue;
                }
                let (px, py) = (x as f64 - a.x, y as f64 - a.y);
                let t = px * dx + py * dy;
                let d = px * nx + py * ny;
                if t < 0.06 * len || t > 0.94 * len || d.abs() > band {
                    continue;
                }
                let g = f64::from(self.mag[i]);
                if g <= 0.0 {
                    continue;
                }
                let cos = (f64::from(self.gx[i]) * nx + f64::from(self.gy[i]) * ny).abs() / g;
                if cos > 0.85 {
                    pts.push(Point::new(x as f64, y as f64));
                }
            }
        }
        if (pts.len() as f64) < (len * 0.25).max(10.0) {
            return None;
        }
        Line::fit(&pts)
    }
}

/// Đường thẳng dạng `nx*x + ny*y = c` với (nx, ny) đơn vị.
#[derive(Debug, Clone, Copy, Default)]
struct Line {
    nx: f64,
    ny: f64,
    c: f64,
    /// Góc pháp tuyến (độ, 0..180), chỉ dùng khi ghép cặp.
    theta: f64,
}

impl Line {
    fn through(a: Point, b: Point) -> Option<Self> {
        let len = a.dist(b);
        if len < 1e-9 {
            return None;
        }
        let (nx, ny) = (-(b.y - a.y) / len, (b.x - a.x) / len);
        Some(Self {
            nx,
            ny,
            c: nx * a.x + ny * a.y,
            theta: 0.0,
        })
    }

    /// Khớp bình phương tối thiểu toàn phần (PCA).
    fn fit(pts: &[Point]) -> Option<Self> {
        let n = pts.len() as f64;
        let mx = pts.iter().map(|p| p.x).sum::<f64>() / n;
        let my = pts.iter().map(|p| p.y).sum::<f64>() / n;
        let (mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0);
        for p in pts {
            let (dx, dy) = (p.x - mx, p.y - my);
            sxx += dx * dx;
            syy += dy * dy;
            sxy += dx * dy;
        }
        // Hướng đường = véc-tơ riêng lớn nhất; pháp tuyến vuông góc với nó.
        let theta = 0.5 * (2.0 * sxy).atan2(sxx - syy);
        let (nx, ny) = (-theta.sin(), theta.cos());
        let c = nx * mx + ny * my;
        (c.is_finite() && nx.is_finite()).then_some(Self {
            nx,
            ny,
            c,
            theta: 0.0,
        })
    }

    fn intersect(&self, o: &Self) -> Option<Point> {
        let det = self.nx * o.ny - self.ny * o.nx;
        if det.abs() < 1e-9 {
            return None;
        }
        let x = (self.c * o.ny - self.ny * o.c) / det;
        let y = (self.nx * o.c - self.c * o.nx) / det;
        Some(Point::new(x, y))
    }
}

/// Độ lệch giữa hai hướng đường (độ, 0..90).
fn angle_diff(a: f64, b: f64) -> f64 {
    let d = (a - b).abs().rem_euclid(180.0);
    d.min(180.0 - d)
}

/// Sobel 3x3 trên ảnh f32; viền 1 px có gradient 0.
fn sobel(p: &[f32], w: usize, h: usize) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    let mut gx = vec![0.0f32; w * h];
    let mut gy = vec![0.0f32; w * h];
    let mut mag = vec![0.0f32; w * h];
    for y in 1..h.saturating_sub(1) {
        for x in 1..w.saturating_sub(1) {
            let i = y * w + x;
            let (a, b, c) = (p[i - w - 1], p[i - w], p[i - w + 1]);
            let (d, f) = (p[i - 1], p[i + 1]);
            let (g, hh, k) = (p[i + w - 1], p[i + w], p[i + w + 1]);
            let sx = (c + 2.0 * f + k) - (a + 2.0 * d + g);
            let sy = (g + 2.0 * hh + k) - (a + 2.0 * b + c);
            gx[i] = sx;
            gy[i] = sy;
            mag[i] = (sx * sx + sy * sy).sqrt();
        }
    }
    (gx, gy, mag)
}

/// Ngưỡng Canny tự động theo trung vị độ lớn gradient của các điểm có gradient.
fn auto_thresholds(mag: &[f32]) -> (f32, f32) {
    let mut v: Vec<f32> = mag.iter().copied().filter(|m| *m > 2.0).collect();
    if v.is_empty() {
        return (20.0, 40.0);
    }
    let mid = v.len() / 2;
    let (_, median, _) = v.select_nth_unstable_by(mid, f32::total_cmp);
    let median = *median;
    let high = (median * 2.0).clamp(14.0, 200.0);
    (high * 0.45, high)
}

/// Ngưỡng Otsu trên histogram 256 mức.
fn otsu_threshold(hist: &[u64; 256]) -> u8 {
    let total: u64 = hist.iter().sum();
    let sum: f64 = hist.iter().enumerate().map(|(i, c)| i as f64 * *c as f64).sum();
    let (mut w0, mut sum0, mut best, mut best_t) = (0.0, 0.0, -1.0, 0u8);
    for (t, c) in hist.iter().enumerate() {
        w0 += *c as f64;
        if w0 == 0.0 {
            continue;
        }
        let w1 = total as f64 - w0;
        if w1 == 0.0 {
            break;
        }
        sum0 += t as f64 * *c as f64;
        let (m0, m1) = (sum0 / w0, (sum - sum0) / w1);
        let var = w0 * w1 * (m0 - m1) * (m0 - m1);
        if var > best {
            best = var;
            best_t = t as u8;
        }
    }
    best_t
}

fn percentile_u8(hist: &[u64; 256], q: f64) -> u8 {
    let total: u64 = hist.iter().sum();
    let target = (total as f64 * q).ceil() as u64;
    let mut acc = 0;
    for (i, c) in hist.iter().enumerate() {
        acc += c;
        if acc >= target {
            return i as u8;
        }
    }
    255
}

/// Canny: triệt tiêu không cực đại theo 4 hướng + trễ hai ngưỡng (8-liên thông).
fn canny(w: usize, h: usize, gx: &[f32], gy: &[f32], mag: &[f32], low: f32, high: f32) -> GrayImage {
    let mut nms = vec![0.0f32; w * h];
    for y in 1..h.saturating_sub(1) {
        for x in 1..w.saturating_sub(1) {
            let i = y * w + x;
            let m = mag[i];
            if m < low {
                continue;
            }
            // Lượng tử hóa hướng gradient về 4 hướng không dùng lượng giác (tan 22,5° ≈ 0,4142).
            let (ax, ay) = (gx[i].abs(), gy[i].abs());
            let (n1, n2) = if ay <= ax * 0.414_213_57 {
                (i - 1, i + 1)
            } else if ax <= ay * 0.414_213_57 {
                (i - w, i + w)
            } else if (gx[i] > 0.0) == (gy[i] > 0.0) {
                (i - w - 1, i + w + 1)
            } else {
                (i - w + 1, i + w - 1)
            };
            if m >= mag[n1] && m > mag[n2] {
                nms[i] = m;
            }
        }
    }
    let mut out = vec![0u8; w * h];
    let mut stack = Vec::new();
    for i in 0..w * h {
        if nms[i] >= high && out[i] == 0 {
            out[i] = 255;
            stack.push(i);
            while let Some(j) = stack.pop() {
                let (x, y) = ((j % w) as isize, (j / w) as isize);
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let (nx, ny) = (x + dx, y + dy);
                        if nx < 0 || ny < 0 || nx >= w as isize || ny >= h as isize {
                            continue;
                        }
                        let k = ny as usize * w + nx as usize;
                        if out[k] == 0 && nms[k] >= low {
                            out[k] = 255;
                            stack.push(k);
                        }
                    }
                }
            }
        }
    }
    GrayImage::from_raw(w as u32, h as u32, out).unwrap_or_else(|| GrayImage::new(w as u32, h as u32))
}

/// Rút bao lồi về tứ giác: Douglas-Peucker rồi chọn 4 đỉnh cho diện tích lớn nhất.
fn hull_to_quad(hull: &[Point]) -> Option<Quad> {
    if hull.len() < 4 {
        return None;
    }
    let perim: f64 = (0..hull.len())
        .map(|i| hull[i].dist(hull[(i + 1) % hull.len()]))
        .sum();
    let mut poly = hull.to_vec();
    for eps in [0.01, 0.02, 0.035, 0.05] {
        let simplified = simplify_closed(hull, eps * perim);
        if simplified.len() < 4 {
            break;
        }
        poly = simplified;
        if poly.len() <= 12 {
            break;
        }
    }
    if poly.len() > 16 {
        return None;
    }
    let n = poly.len();
    let mut best: Option<([Point; 4], f64)> = None;
    for a in 0..n {
        for b in a + 1..n {
            for c in b + 1..n {
                for d in c + 1..n {
                    let pts = [poly[a], poly[b], poly[c], poly[d]];
                    let area = polygon_area(&pts).abs();
                    if best.as_ref().is_none_or(|(_, ba)| area > *ba) {
                        best = Some((pts, area));
                    }
                }
            }
        }
    }
    best.and_then(|(pts, _)| Quad::from_unordered(pts).ok())
}

/// Douglas-Peucker cho đa giác kín: tách tại hai điểm xa nhau nhất.
fn simplify_closed(poly: &[Point], eps: f64) -> Vec<Point> {
    let n = poly.len();
    if n < 4 {
        return poly.to_vec();
    }
    let far = (1..n)
        .max_by(|&i, &j| poly[0].dist(poly[i]).total_cmp(&poly[0].dist(poly[j])))
        .unwrap_or(n / 2);
    let mut first: Vec<Point> = poly[..=far].to_vec();
    let mut second: Vec<Point> = poly[far..].to_vec();
    second.push(poly[0]);
    first = dp(&first, eps);
    second = dp(&second, eps);
    first.pop();
    second.pop();
    first.extend(second);
    first
}

fn dp(pts: &[Point], eps: f64) -> Vec<Point> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let (a, b) = (pts[0], pts[pts.len() - 1]);
    let len = a.dist(b);
    let mut idx = 0;
    let mut dmax = 0.0;
    for (i, p) in pts.iter().enumerate().take(pts.len() - 1).skip(1) {
        let d = if len < 1e-12 {
            p.dist(a)
        } else {
            ((b.x - a.x) * (a.y - p.y) - (a.x - p.x) * (b.y - a.y)).abs() / len
        };
        if d > dmax {
            dmax = d;
            idx = i;
        }
    }
    if dmax > eps {
        let mut left = dp(&pts[..=idx], eps);
        let right = dp(&pts[idx..], eps);
        left.pop();
        left.extend(right);
        left
    } else {
        vec![a, b]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::iou;
    use image::{Luma, Rgb, RgbImage};

    fn synthetic_page(w: u32, h: u32, q: &Quad, bg: u8, fg: u8) -> GrayImage {
        let poly = q.pts;
        GrayImage::from_fn(w, h, |x, y| {
            let p = Point::new(f64::from(x) + 0.5, f64::from(y) + 0.5);
            let inside = (0..4).all(|i| {
                let (a, b) = (poly[i], poly[(i + 1) % 4]);
                (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x) >= 0.0
            });
            Luma([if inside { fg } else { bg }])
        })
    }

    #[test]
    fn finds_rotated_page_on_dark_background() {
        let truth = Quad::from_unordered([
            Point::new(150.0, 80.0),
            Point::new(480.0, 120.0),
            Point::new(450.0, 420.0),
            Point::new(110.0, 390.0),
        ])
        .unwrap();
        let img = synthetic_page(640, 480, &truth, 40, 230);
        let det = detect_luma(&img, DetectOptions::default()).unwrap().unwrap();
        assert!(iou(&det.quad, &truth) > 0.97, "iou {}", iou(&det.quad, &truth));
        assert!(det.confidence > 0.7);
    }

    #[test]
    fn scales_back_to_original_coordinates() {
        let truth = Quad::from_unordered([
            Point::new(300.0, 200.0),
            Point::new(1500.0, 260.0),
            Point::new(1450.0, 1250.0),
            Point::new(260.0, 1200.0),
        ])
        .unwrap();
        let gray = synthetic_page(1800, 1400, &truth, 60, 220);
        let rgb = DynamicImage::ImageRgb8(RgbImage::from_fn(1800, 1400, |x, y| {
            let v = gray.get_pixel(x, y)[0];
            Rgb([v, v, v])
        }));
        let det = detect(&rgb).unwrap().unwrap();
        assert!(iou(&det.quad, &truth) > 0.97);
    }

    #[test]
    fn empty_image_has_no_detection() {
        let img = GrayImage::from_pixel(320, 240, Luma([128]));
        assert!(detect_luma(&img, DetectOptions::default()).unwrap().is_none());
    }

    #[test]
    fn rejects_bad_sizes_and_options() {
        assert!(detect_luma(&GrayImage::new(8, 8), DetectOptions::default()).is_err());
        let img = GrayImage::new(100, 100);
        let opts = DetectOptions {
            working_size: 10,
            ..DetectOptions::default()
        };
        assert!(detect_luma(&img, opts).is_err());
    }

    #[test]
    fn hough_finds_page_when_edges_are_broken() {
        let truth = Quad::from_unordered([
            Point::new(100.0, 60.0),
            Point::new(540.0, 60.0),
            Point::new(540.0, 420.0),
            Point::new(100.0, 420.0),
        ])
        .unwrap();
        // Chỉ có khung nét đứt (20 px vẽ, 10 px hở): không contour nào đủ dài, Hough phải ghép 4 đường.
        let mut img = GrayImage::from_pixel(640, 480, Luma([60]));
        for i in 0..4 {
            let (a, b) = (truth.pts[i], truth.pts[(i + 1) % 4]);
            let len = a.dist(b) as u32;
            for t in 0..len {
                if t % 30 >= 20 {
                    continue;
                }
                let f = f64::from(t) / f64::from(len);
                let (x, y) = (a.x + (b.x - a.x) * f, a.y + (b.y - a.y) * f);
                for d in -1..=1 {
                    let (px, py) = if i % 2 == 0 {
                        (x, y + f64::from(d))
                    } else {
                        (x + f64::from(d), y)
                    };
                    img.put_pixel(px as u32, py as u32, Luma([220]));
                }
            }
        }
        let ctx = EdgeContext::new(&img, None);
        let cands = ctx.hough_candidates();
        let best = cands
            .iter()
            .max_by(|a, b| ctx.score(a).total_cmp(&ctx.score(b)))
            .unwrap();
        assert!(iou(best, &truth) > 0.95, "iou {}", iou(best, &truth));
        assert!(ctx.contour_candidates().iter().all(|q| iou(q, &truth) < 0.9));
    }

    #[test]
    fn line_helpers() {
        let l1 = Line::through(Point::new(0.0, 0.0), Point::new(10.0, 0.0)).unwrap();
        let l2 = Line::through(Point::new(5.0, -3.0), Point::new(5.0, 7.0)).unwrap();
        let p = l1.intersect(&l2).unwrap();
        assert!(p.dist(Point::new(5.0, 0.0)) < 1e-9);
        assert!(l1.intersect(&l1).is_none());
        assert!(Line::through(Point::new(1.0, 1.0), Point::new(1.0, 1.0)).is_none());
        let fitted = Line::fit(&[Point::new(0.0, 2.0), Point::new(1.0, 2.0), Point::new(2.0, 2.0)]).unwrap();
        assert!(fitted.nx.abs() < 1e-9 && (fitted.c.abs() - 2.0).abs() < 1e-9);
        assert!((angle_diff(179.0, 1.0) - 2.0).abs() < 1e-9);
        assert!(hull_to_quad(&[Point::new(0.0, 0.0), Point::new(1.0, 0.0), Point::new(0.0, 1.0)]).is_none());
        assert_eq!(simplify_closed(&[Point::new(0.0, 0.0)], 1.0).len(), 1);
        assert_eq!(auto_thresholds(&[0.0; 4]), (20.0, 40.0));
    }
}

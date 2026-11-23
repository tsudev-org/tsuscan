//! Xử lý trang sách bằng thuật toán cổ điển (luôn có, không cần mô hình):
//!
//! - [`find_gutter`] / [`split_spread`]: tìm gáy sách và tách ảnh hai trang thành hai ảnh.
//! - [`remove_fingers`]: xóa ngón tay giữ sách ở mép trang (màu da theo YCrCb, chỉ vùng chạm mép).

use image::{GrayImage, Luma, Rgb, RgbImage, imageops};
use imageproc::region_labelling::{Connectivity, connected_components};

/// Kết quả dò gáy sách.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gutter {
    /// Hoành độ gáy trên ảnh gốc (px).
    pub x: u32,
    /// Độ tin cậy 0..1 (độ sâu thung lũng mật độ chữ so với hai bên).
    pub confidence: f32,
}

fn luma(p: Rgb<u8>) -> u8 {
    ((u32::from(p.0[0]) * 299 + u32::from(p.0[1]) * 587 + u32::from(p.0[2]) * 114) / 1000) as u8
}

/// Tìm gáy sách trong ảnh hai trang (ngang). `None` khi không giống ảnh hai trang.
#[must_use]
pub fn find_gutter(img: &RgbImage) -> Option<Gutter> {
    let (w, h) = img.dimensions();
    if w < 64 || h < 64 || w < h {
        return None;
    }
    let sw = 600u32;
    let sh = ((f64::from(h) * f64::from(sw) / f64::from(w)).round() as u32).max(16);
    let small = imageops::resize(img, sw, sh, imageops::FilterType::Triangle);
    let g = GrayImage::from_fn(sw, sh, |x, y| Luma([luma(*small.get_pixel(x, y))]));
    // Mật độ "mực": điểm tối hơn trung bình cục bộ theo hàng một ngưỡng (chữ), bỏ 5% trên/dưới.
    let (y0, y1) = (sh / 20, sh - sh / 20);
    let mut ink = vec![0f64; sw as usize];
    let mut dark = vec![0f64; sw as usize];
    for y in y0..y1 {
        let row: Vec<i32> = (0..sw).map(|x| i32::from(g.get_pixel(x, y).0[0])).collect();
        for x in 2..sw as usize - 2 {
            let local = i32::midpoint(row[x - 2], row[x + 2]);
            let v = row[x];
            if local - v > 25 || (row[x - 1] - v).abs() > 40 {
                ink[x] += 1.0;
            }
            dark[x] += f64::from(255 - v);
        }
    }
    let k = 6usize;
    let smooth = |v: &[f64]| -> Vec<f64> {
        (0..v.len())
            .map(|i| {
                let (a, b) = (i.saturating_sub(k), (i + k + 1).min(v.len()));
                v[a..b].iter().sum::<f64>() / (b - a) as f64
            })
            .collect()
    };
    let ink = smooth(&ink);
    let dark = smooth(&dark);
    let (lo, hi) = ((f64::from(sw) * 0.35) as usize, (f64::from(sw) * 0.65) as usize);
    let max_dark = dark[lo..hi].iter().copied().fold(1.0, f64::max);
    let mean_ink = ink[sw as usize / 10..sw as usize * 9 / 10].iter().sum::<f64>() / (f64::from(sw) * 0.8);
    if mean_ink < 1.0 {
        return None;
    }
    // Điểm: ít chữ + tối (bóng gáy) + gần giữa.
    let score = |x: usize| {
        let center = (x as f64 / f64::from(sw) - 0.5).abs();
        ink[x] / mean_ink - 0.6 * dark[x] / max_dark + 0.8 * center
    };
    let best = (lo..hi).min_by(|&a, &b| score(a).total_cmp(&score(b)))?;
    // Hai bên phải có chữ.
    let side = |a: usize, b: usize| ink[a..b].iter().sum::<f64>() / (b - a).max(1) as f64;
    let left = side(
        sw as usize / 10,
        best.saturating_sub(10).max(sw as usize / 10 + 1),
    );
    let right = side((best + 10).min(sw as usize * 9 / 10 - 1), sw as usize * 9 / 10);
    let depth = 1.0 - ink[best] / left.min(right).max(1e-9);
    if left < 0.3 * mean_ink || right < 0.3 * mean_ink || depth < 0.5 {
        return None;
    }
    let x = ((best as f64 + 0.5) * f64::from(w) / f64::from(sw)).round() as u32;
    Some(Gutter {
        x,
        confidence: depth.clamp(0.0, 1.0) as f32,
    })
}

/// Tách ảnh hai trang tại gáy. `None` khi không tìm thấy gáy tin cậy.
#[must_use]
pub fn split_spread(img: &RgbImage) -> Option<(RgbImage, RgbImage)> {
    let g = find_gutter(img)?;
    let h = img.height();
    let left = imageops::crop_imm(img, 0, 0, g.x, h).to_image();
    let right = imageops::crop_imm(img, g.x, 0, img.width() - g.x, h).to_image();
    Some((left, right))
}

/// Điểm ảnh có màu da (YCrCb, ngưỡng rộng theo Chai & Ngan 1999 cho nhiều tông da).
#[must_use]
pub fn is_skin(p: Rgb<u8>) -> bool {
    let [r, g, b] = p.0.map(f32::from);
    let y = 0.299 * r + 0.587 * g + 0.114 * b;
    let cr = (r - y) * 0.713 + 128.0;
    let cb = (b - y) * 0.564 + 128.0;
    y > 40.0 && (135.0..=180.0).contains(&cr) && (80.0..=130.0).contains(&cb) && r > b + 15.0
}

/// Xóa ngón tay chạm mép trang: vùng màu da liên thông chạm mép, diện tích ≥ 0,1% ảnh, được tô bằng màu giấy
/// (trung vị điểm không phải da quanh vùng). Trả về ảnh mới và tỉ lệ diện tích đã xóa.
#[must_use]
pub fn remove_fingers(img: &RgbImage) -> (RgbImage, f32) {
    let (w, h) = img.dimensions();
    if w < 8 || h < 8 {
        return (img.clone(), 0.0);
    }
    let mask = GrayImage::from_fn(w, h, |x, y| Luma([u8::from(is_skin(*img.get_pixel(x, y))) * 255]));
    let labels = connected_components(&mask, Connectivity::Eight, Luma([0]));
    let n = labels.pixels().map(|p| p.0[0]).max().unwrap_or(0) as usize;
    let mut area = vec![0usize; n + 1];
    let mut touches = vec![false; n + 1];
    for (x, y, l) in labels.enumerate_pixels() {
        let id = l.0[0] as usize;
        if id == 0 {
            continue;
        }
        area[id] += 1;
        if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
            touches[id] = true;
        }
    }
    let min_area = (f64::from(w) * f64::from(h) * 0.001) as usize;
    let keep: Vec<bool> = (0..=n)
        .map(|i| i > 0 && touches[i] && area[i] >= min_area)
        .collect();
    if !keep.iter().any(|&k| k) {
        return (img.clone(), 0.0);
    }
    let mut finger = GrayImage::from_fn(w, h, |x, y| {
        Luma([u8::from(keep[labels.get_pixel(x, y).0[0] as usize]) * 255])
    });
    // Nở 3 px để không còn viền da.
    finger = imageproc::morphology::dilate(&finger, imageproc::distance_transform::Norm::LInf, 3);
    let ring = imageproc::morphology::dilate(&finger, imageproc::distance_transform::Norm::LInf, 12);
    let mut samples: [Vec<u8>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for (x, y, r) in ring.enumerate_pixels() {
        if r.0[0] > 0 && finger.get_pixel(x, y).0[0] == 0 {
            let p = img.get_pixel(x, y);
            for (s, v) in samples.iter_mut().zip(p.0) {
                s.push(v);
            }
        }
    }
    let fill = Rgb(samples.map(|mut s| {
        if s.is_empty() {
            255
        } else {
            // Bách phân vị 75: giấy sáng hơn chữ, tránh lấy màu chữ.
            s.sort_unstable();
            s[s.len() * 3 / 4]
        }
    }));
    let mut out = img.clone();
    let mut removed = 0usize;
    for (x, y, m) in finger.enumerate_pixels() {
        if m.0[0] > 0 {
            out.put_pixel(x, y, fill);
            removed += 1;
        }
    }
    (out, removed as f32 / (w as f32 * h as f32))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trang giả có các vạch "chữ" đen trên nền giấy.
    fn page(w: u32, h: u32, x0: u32, x1: u32) -> RgbImage {
        let mut img = RgbImage::from_pixel(w, h, Rgb([240, 238, 230]));
        for y in (40..h - 40).step_by(24) {
            for x in (x0..x1).step_by(9) {
                for dy in 0..10 {
                    for dx in 0..5 {
                        img.put_pixel(x + dx, y + dy, Rgb([30, 30, 30]));
                    }
                }
            }
        }
        img
    }

    #[test]
    fn tim_gay_sach_va_tach() {
        let (w, h) = (1200, 800);
        let mut img = page(w, h, 60, 1140);
        // Khe giữa hai trang + bóng gáy lệch tâm một chút.
        let gx = 640u32;
        for x in gx - 50..gx + 50 {
            let shade = 200 - (40 - (x as i32 - gx as i32).abs().min(40)) as u8 * 3;
            for y in 0..h {
                img.put_pixel(x, y, Rgb([shade, shade, shade]));
            }
        }
        let g = find_gutter(&img).unwrap();
        assert!((g.x as i32 - gx as i32).abs() < 20, "gáy {} (cần ≈ {gx})", g.x);
        let (l, r) = split_spread(&img).unwrap();
        assert_eq!(l.width() + r.width(), w);
    }

    #[test]
    fn trang_don_khong_co_gay() {
        assert!(find_gutter(&page(1200, 800, 60, 1140)).is_none()); // chữ trải đều, không có thung lũng
        assert!(find_gutter(&page(600, 800, 60, 540)).is_none()); // ảnh dọc
        assert!(find_gutter(&RgbImage::from_pixel(1200, 800, Rgb([250, 250, 250]))).is_none());
    }

    #[test]
    fn xoa_ngon_tay_cham_mep_giu_chu() {
        let mut img = page(600, 800, 100, 500);
        let skin = Rgb([224, 172, 140]);
        // Ngón tay hình elip chạm mép trái.
        for y in 300..420u32 {
            for x in 0..90u32 {
                let (dx, dy) = (f64::from(x) / 90.0, (f64::from(y) - 360.0) / 60.0);
                if dx * dx + dy * dy <= 1.0 {
                    img.put_pixel(x, y, skin);
                }
            }
        }
        // Vùng màu da KHÔNG chạm mép (ảnh minh họa) phải giữ nguyên.
        for y in 600..640 {
            for x in 250..300 {
                img.put_pixel(x, y, skin);
            }
        }
        let (out, frac) = remove_fingers(&img);
        assert!(frac > 0.01, "{frac}");
        assert!(!is_skin(*out.get_pixel(20, 360)));
        let p = out.get_pixel(20, 360).0;
        assert!(p[0] > 220 && p[2] > 210, "{p:?}");
        assert_eq!(*out.get_pixel(260, 610), skin);
        assert_eq!(*out.get_pixel(100, 40), Rgb([30, 30, 30]));
        let clean = page(200, 200, 20, 180);
        assert!(remove_fingers(&clean).1.abs() < f32::EPSILON);
    }
}

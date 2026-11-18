//! Hình học phẳng: điểm, tứ giác, homography 3x3, IoU đa giác, bao lồi.
//!
//! Hệ tọa độ ảnh: gốc trên-trái, `x` sang phải, `y` xuống dưới. "Chiều kim đồng hồ" là theo
//! cách mắt nhìn ảnh trên màn hình.

use nalgebra::{Matrix3, SMatrix, SVector, Vector3};

use crate::error::{ImagingError, Result};

/// Điểm thực trong hệ tọa độ ảnh.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    /// Hoành độ (px).
    pub x: f64,
    /// Tung độ (px, hướng xuống).
    pub y: f64,
}

impl Point {
    /// Tạo điểm.
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Khoảng cách Euclid.
    #[must_use]
    pub fn dist(self, o: Self) -> f64 {
        (self.x - o.x).hypot(self.y - o.y)
    }

    fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y)
    }

    fn cross(self, o: Self) -> f64 {
        self.x * o.y - self.y * o.x
    }

    fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

/// Tứ giác theo thứ tự: trên-trái, trên-phải, dưới-phải, dưới-trái (chiều kim đồng hồ).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quad {
    /// Bốn góc theo thứ tự TL, TR, BR, BL.
    pub pts: [Point; 4],
}

impl Quad {
    /// Tạo tứ giác từ 4 điểm bất kỳ thứ tự; sắp lại theo chiều kim đồng hồ bắt đầu từ trên-trái.
    ///
    /// # Errors
    /// [`ImagingError::InvalidQuad`] khi có điểm không hữu hạn hoặc diện tích gần 0.
    pub fn from_unordered(points: [Point; 4]) -> Result<Self> {
        if points.iter().any(|p| !p.is_finite()) {
            return Err(ImagingError::InvalidQuad("tọa độ không hữu hạn"));
        }
        let pts = order_corners(points);
        let q = Self { pts };
        if q.area() < 1e-6 {
            return Err(ImagingError::InvalidQuad("diện tích bằng 0"));
        }
        Ok(q)
    }

    /// Tứ giác bao trọn khung `width` x `height`.
    #[must_use]
    pub fn full_frame(width: u32, height: u32) -> Self {
        let (w, h) = (f64::from(width), f64::from(height));
        Self {
            pts: [
                Point::new(0.0, 0.0),
                Point::new(w, 0.0),
                Point::new(w, h),
                Point::new(0.0, h),
            ],
        }
    }

    /// Diện tích (công thức dây giày, luôn dương).
    #[must_use]
    pub fn area(&self) -> f64 {
        polygon_area(&self.pts).abs()
    }

    /// Chu vi.
    #[must_use]
    pub fn perimeter(&self) -> f64 {
        self.side_lengths().iter().sum()
    }

    /// Độ dài 4 cạnh: trên, phải, dưới, trái.
    #[must_use]
    pub fn side_lengths(&self) -> [f64; 4] {
        let p = &self.pts;
        [p[0].dist(p[1]), p[1].dist(p[2]), p[2].dist(p[3]), p[3].dist(p[0])]
    }

    /// Bốn góc trong (độ) tại TL, TR, BR, BL.
    #[must_use]
    pub fn interior_angles(&self) -> [f64; 4] {
        let mut out = [0.0; 4];
        for (i, a) in out.iter_mut().enumerate() {
            let prev = self.pts[(i + 3) % 4];
            let cur = self.pts[i];
            let next = self.pts[(i + 1) % 4];
            let u = prev.sub(cur);
            let v = next.sub(cur);
            let denom = u.x.hypot(u.y) * v.x.hypot(v.y);
            *a = if denom > 0.0 {
                ((u.x * v.x + u.y * v.y) / denom)
                    .clamp(-1.0, 1.0)
                    .acos()
                    .to_degrees()
            } else {
                0.0
            };
        }
        out
    }

    /// Tứ giác lồi (mọi tích có hướng cùng dấu, không suy biến).
    #[must_use]
    pub fn is_convex(&self) -> bool {
        let mut positive: Option<bool> = None;
        for i in 0..4 {
            let a = self.pts[i];
            let b = self.pts[(i + 1) % 4];
            let c = self.pts[(i + 2) % 4];
            let z = b.sub(a).cross(c.sub(b));
            if z.abs() < 1e-9 {
                return false;
            }
            match positive {
                None => positive = Some(z > 0.0),
                Some(p) if p != (z > 0.0) => return false,
                Some(_) => {}
            }
        }
        true
    }

    /// Nhân mọi tọa độ với `s` (đổi giữa ảnh thu nhỏ và ảnh gốc).
    #[must_use]
    pub fn scaled(&self, sx: f64, sy: f64) -> Self {
        Self {
            pts: self.pts.map(|p| Point::new(p.x * sx, p.y * sy)),
        }
    }

    /// Kẹp các góc vào khung `[0, w] x [0, h]`.
    #[must_use]
    pub fn clamped(&self, w: f64, h: f64) -> Self {
        Self {
            pts: self.pts.map(|p| Point::new(p.x.clamp(0.0, w), p.y.clamp(0.0, h))),
        }
    }
}

/// Sắp 4 điểm theo chiều kim đồng hồ quanh trọng tâm, bắt đầu từ điểm có `x + y` nhỏ nhất.
#[must_use]
pub fn order_corners(points: [Point; 4]) -> [Point; 4] {
    let cx = points.iter().map(|p| p.x).sum::<f64>() / 4.0;
    let cy = points.iter().map(|p| p.y).sum::<f64>() / 4.0;
    let mut pts = points;
    // y hướng xuống ⇒ atan2 tăng dần = chiều kim đồng hồ trên màn hình.
    pts.sort_by(|a, b| (a.y - cy).atan2(a.x - cx).total_cmp(&(b.y - cy).atan2(b.x - cx)));
    let start = (0..4)
        .min_by(|&i, &j| (pts[i].x + pts[i].y).total_cmp(&(pts[j].x + pts[j].y)))
        .unwrap_or(0);
    pts.rotate_left(start);
    pts
}

/// Diện tích có hướng của đa giác (dương khi theo chiều kim đồng hồ trong hệ y-xuống).
#[must_use]
pub fn polygon_area(poly: &[Point]) -> f64 {
    let n = poly.len();
    if n < 3 {
        return 0.0;
    }
    let mut s = 0.0;
    for i in 0..n {
        s += poly[i].cross(poly[(i + 1) % n]);
    }
    s / 2.0
}

/// Bao lồi (Andrew monotone chain), trả theo chiều kim đồng hồ trong hệ y-xuống.
#[must_use]
pub fn convex_hull(points: &[Point]) -> Vec<Point> {
    let mut pts: Vec<Point> = points.iter().copied().filter(|p| p.is_finite()).collect();
    pts.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    pts.dedup();
    if pts.len() < 3 {
        return pts;
    }
    let mut lower: Vec<Point> = Vec::with_capacity(pts.len());
    for &p in &pts {
        while lower.len() >= 2 && turn(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0.0 {
            lower.pop();
        }
        lower.push(p);
    }
    let mut upper: Vec<Point> = Vec::with_capacity(pts.len());
    for &p in pts.iter().rev() {
        while upper.len() >= 2 && turn(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0.0 {
            upper.pop();
        }
        upper.push(p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn turn(o: Point, a: Point, b: Point) -> f64 {
    a.sub(o).cross(b.sub(o))
}

/// Cắt đa giác `subject` bằng đa giác lồi `clip` (Sutherland-Hodgman).
///
/// Cả hai đa giác cần cùng chiều; hàm tự chuẩn hóa chiều của `clip`.
#[must_use]
pub fn clip_polygon(subject: &[Point], clip: &[Point]) -> Vec<Point> {
    let mut clip = clip.to_vec();
    if polygon_area(&clip) < 0.0 {
        clip.reverse();
    }
    let mut output = subject.to_vec();
    let n = clip.len();
    for i in 0..n {
        if output.is_empty() {
            break;
        }
        let a = clip[i];
        let b = clip[(i + 1) % n];
        // Phía "trong" của cạnh a→b khi đa giác theo chiều dương: tích có hướng >= 0.
        let inside = |p: Point| b.sub(a).cross(p.sub(a)) >= 0.0;
        let input = std::mem::take(&mut output);
        let m = input.len();
        for j in 0..m {
            let cur = input[j];
            let prev = input[(j + m - 1) % m];
            let (ci, pi) = (inside(cur), inside(prev));
            if ci {
                if !pi && let Some(x) = line_intersection(prev, cur, a, b) {
                    output.push(x);
                }
                output.push(cur);
            } else if pi && let Some(x) = line_intersection(prev, cur, a, b) {
                output.push(x);
            }
        }
    }
    output
}

fn line_intersection(p1: Point, p2: Point, p3: Point, p4: Point) -> Option<Point> {
    let d1 = p2.sub(p1);
    let d2 = p4.sub(p3);
    let denom = d1.cross(d2);
    if denom.abs() < 1e-12 {
        return None;
    }
    let t = p3.sub(p1).cross(d2) / denom;
    Some(Point::new(p1.x + t * d1.x, p1.y + t * d1.y))
}

/// IoU (giao trên hợp) của hai tứ giác. Tứ giác không lồi được thay bằng bao lồi của nó.
#[must_use]
pub fn iou(a: &Quad, b: &Quad) -> f64 {
    let pa = convex_hull(&a.pts);
    let pb = convex_hull(&b.pts);
    let area_a = polygon_area(&pa).abs();
    let area_b = polygon_area(&pb).abs();
    if area_a <= 0.0 || area_b <= 0.0 {
        return 0.0;
    }
    let inter = polygon_area(&clip_polygon(&pa, &pb)).abs();
    let union = area_a + area_b - inter;
    if union <= 0.0 {
        0.0
    } else {
        (inter / union).clamp(0.0, 1.0)
    }
}

/// Phép biến đổi xạ ảnh 3x3 (homography) ánh xạ điểm nguồn sang điểm đích.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Homography {
    m: Matrix3<f64>,
}

impl Homography {
    /// Ma trận đơn vị.
    #[must_use]
    pub fn identity() -> Self {
        Self {
            m: Matrix3::identity(),
        }
    }

    /// Ma trận 3x3 bên trong (hàng trước).
    #[must_use]
    pub fn matrix(&self) -> [[f64; 3]; 3] {
        let m = &self.m;
        [
            [m[(0, 0)], m[(0, 1)], m[(0, 2)]],
            [m[(1, 0)], m[(1, 1)], m[(1, 2)]],
            [m[(2, 0)], m[(2, 1)], m[(2, 2)]],
        ]
    }

    /// Ước lượng homography từ 4 cặp điểm bằng DLT (chuẩn hóa Hartley, giải hệ 8x8).
    ///
    /// # Errors
    /// [`ImagingError::InvalidQuad`] khi cấu hình điểm suy biến (3 điểm thẳng hàng, trùng nhau).
    pub fn from_points(src: &[Point; 4], dst: &[Point; 4]) -> Result<Self> {
        if src.iter().chain(dst.iter()).any(|p| !p.is_finite()) {
            return Err(ImagingError::InvalidQuad("tọa độ không hữu hạn"));
        }
        let ts = normalizer(src).ok_or(ImagingError::InvalidQuad("điểm nguồn trùng nhau"))?;
        let td = normalizer(dst).ok_or(ImagingError::InvalidQuad("điểm đích trùng nhau"))?;
        let s = src.map(|p| apply_m(&ts, p));
        let d = dst.map(|p| apply_m(&td, p));

        let mut a = SMatrix::<f64, 8, 8>::zeros();
        let mut b = SVector::<f64, 8>::zeros();
        for i in 0..4 {
            let (x, y) = (s[i].x, s[i].y);
            let (u, v) = (d[i].x, d[i].y);
            let r = 2 * i;
            a.set_row(
                r,
                &SMatrix::<f64, 1, 8>::from_row_slice(&[x, y, 1.0, 0.0, 0.0, 0.0, -u * x, -u * y]),
            );
            a.set_row(
                r + 1,
                &SMatrix::<f64, 1, 8>::from_row_slice(&[0.0, 0.0, 0.0, x, y, 1.0, -v * x, -v * y]),
            );
            b[r] = u;
            b[r + 1] = v;
        }
        let h = a
            .lu()
            .solve(&b)
            .ok_or(ImagingError::InvalidQuad("cấu hình điểm suy biến"))?;
        let hn = Matrix3::new(h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7], 1.0);
        let td_inv = td
            .try_inverse()
            .ok_or(ImagingError::InvalidQuad("chuẩn hóa suy biến"))?;
        let mut m = td_inv * hn * ts;
        let scale = m[(2, 2)];
        if scale.abs() < 1e-12 || !m.iter().all(|v| v.is_finite()) {
            return Err(ImagingError::InvalidQuad("cấu hình điểm suy biến"));
        }
        m /= scale;
        let out = Self { m };
        // Ca suy biến (3 điểm thẳng hàng) cho nghiệm không tái tạo được điểm: kiểm lại.
        for i in 0..4 {
            match out.apply(src[i]) {
                Some(p) if p.dist(dst[i]) <= 1e-6 * (1.0 + dst[i].x.abs() + dst[i].y.abs()) => {}
                _ => return Err(ImagingError::InvalidQuad("cấu hình điểm suy biến")),
            }
        }
        Ok(out)
    }

    /// Biến đổi một điểm; `None` khi điểm rơi về vô cực.
    #[must_use]
    pub fn apply(&self, p: Point) -> Option<Point> {
        let r = self.m * Vector3::new(p.x, p.y, 1.0);
        if r.z.abs() < 1e-12 {
            return None;
        }
        let q = Point::new(r.x / r.z, r.y / r.z);
        q.is_finite().then_some(q)
    }

    /// Ma trận nghịch đảo.
    #[must_use]
    pub fn inverse(&self) -> Option<Self> {
        self.m.try_inverse().map(|m| Self { m })
    }
}

fn apply_m(m: &Matrix3<f64>, p: Point) -> Point {
    let r = m * Vector3::new(p.x, p.y, 1.0);
    Point::new(r.x / r.z, r.y / r.z)
}

/// Ma trận chuẩn hóa Hartley: dời trọng tâm về gốc, khoảng cách trung bình = √2.
fn normalizer(pts: &[Point; 4]) -> Option<Matrix3<f64>> {
    let cx = pts.iter().map(|p| p.x).sum::<f64>() / 4.0;
    let cy = pts.iter().map(|p| p.y).sum::<f64>() / 4.0;
    let mean = pts.iter().map(|p| (p.x - cx).hypot(p.y - cy)).sum::<f64>() / 4.0;
    if mean < 1e-12 {
        return None;
    }
    let s = std::f64::consts::SQRT_2 / mean;
    Some(Matrix3::new(s, 0.0, -s * cx, 0.0, s, -s * cy, 0.0, 0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn orders_corners_clockwise_from_top_left() {
        let q = Quad::from_unordered([p(10.0, 90.0), p(100.0, 0.0), p(0.0, 5.0), p(95.0, 100.0)]).unwrap();
        assert_eq!(q.pts, [p(0.0, 5.0), p(100.0, 0.0), p(95.0, 100.0), p(10.0, 90.0)]);
        assert!(q.is_convex());
        assert!(polygon_area(&q.pts) > 0.0);
    }

    #[test]
    fn rejects_degenerate_quads() {
        assert!(Quad::from_unordered([p(0.0, 0.0); 4]).is_err());
        assert!(Quad::from_unordered([p(f64::NAN, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)]).is_err());
    }

    #[test]
    fn angles_sides_and_scaling() {
        let q = Quad::full_frame(4, 2);
        assert_eq!(q.side_lengths(), [4.0, 2.0, 4.0, 2.0]);
        assert!((q.perimeter() - 12.0).abs() < 1e-12);
        for a in q.interior_angles() {
            assert!((a - 90.0).abs() < 1e-9);
        }
        let s = q.scaled(2.0, 3.0);
        assert!((s.area() - 48.0).abs() < 1e-9);
        let c = Quad {
            pts: [p(-5.0, -5.0), p(10.0, 0.0), p(10.0, 10.0), p(0.0, 10.0)],
        }
        .clamped(4.0, 4.0);
        assert_eq!(c.pts[0], p(0.0, 0.0));
        let concave = Quad {
            pts: [p(0.0, 0.0), p(10.0, 0.0), p(2.0, 2.0), p(0.0, 10.0)],
        };
        assert!(!concave.is_convex());
    }

    #[test]
    fn iou_known_values() {
        let a = Quad::full_frame(10, 10);
        assert!((iou(&a, &a) - 1.0).abs() < 1e-12);
        let b = Quad {
            pts: a.pts.map(|q| p(q.x + 5.0, q.y)),
        };
        // Giao 50, hợp 150.
        assert!((iou(&a, &b) - 1.0 / 3.0).abs() < 1e-9);
        let far = Quad {
            pts: a.pts.map(|q| p(q.x + 50.0, q.y)),
        };
        assert!(iou(&a, &far).abs() < 1e-12);
        // Hình thoi nội tiếp: diện tích 50 nằm trọn trong a.
        let diamond = Quad {
            pts: [p(5.0, 0.0), p(10.0, 5.0), p(5.0, 10.0), p(0.0, 5.0)],
        };
        assert!((iou(&a, &diamond) - 0.5).abs() < 1e-9);
        // Thứ tự ngược chiều vẫn đúng.
        let rev = Quad {
            pts: [a.pts[3], a.pts[2], a.pts[1], a.pts[0]],
        };
        assert!((iou(&rev, &a) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn hull_basic() {
        let h = convex_hull(&[
            p(0.0, 0.0),
            p(1.0, 1.0),
            p(2.0, 0.0),
            p(2.0, 2.0),
            p(0.0, 2.0),
            p(1.0, 1.0),
        ]);
        assert_eq!(h.len(), 4);
        assert!(convex_hull(&[p(0.0, 0.0), p(0.0, 0.0)]).len() <= 2);
    }

    #[test]
    fn homography_roundtrip() {
        let src = [p(0.0, 0.0), p(800.0, 0.0), p(800.0, 1131.0), p(0.0, 1131.0)];
        let dst = [p(103.2, 57.9), p(611.4, 88.0), p(655.0, 701.3), p(66.1, 640.2)];
        let h = Homography::from_points(&src, &dst).unwrap();
        for i in 0..4 {
            assert!(h.apply(src[i]).unwrap().dist(dst[i]) < 1e-6);
        }
        let inv = h.inverse().unwrap();
        let mid = p(400.0, 500.0);
        assert!(inv.apply(h.apply(mid).unwrap()).unwrap().dist(mid) < 1e-6);
        assert!((Homography::identity().matrix()[2][2] - 1.0).abs() < 1e-12);
        assert!((h.matrix()[2][2] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn homography_rejects_degenerate() {
        let src = [p(0.0, 0.0), p(1.0, 0.0), p(2.0, 0.0), p(0.0, 1.0)];
        let dst = [p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)];
        assert!(Homography::from_points(&src, &dst).is_err());
        assert!(Homography::from_points(&[p(1.0, 1.0); 4], &dst).is_err());
        let nan = [p(f64::NAN, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)];
        assert!(Homography::from_points(&nan, &dst).is_err());
    }
}

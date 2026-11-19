//! Quét bằng camera (P6): dò mép trên khung xem trước, ảnh chụp giải mã MỘT lần rồi giữ trong Rust (opaque) để
//! dò mép, nắn và lọc nhiều lần khi người dùng chỉnh góc/đổi bộ lọc, cuối cùng ghi thẳng vào kho.
//!
//! Tọa độ góc qua cầu luôn chuẩn hóa 0..1 theo ảnh ĐÃ xoay đúng chiều hiển thị, thứ tự TL, TR, BR, BL - Dart không
//! cần biết kích thước điểm ảnh của cảm biến hay ảnh chụp. Mọi dữ liệu ảnh ở lại trong máy (ADR-0002).

use flutter_rust_bridge::frb;
use image::{DynamicImage, GrayImage, imageops::FilterType};
use tsuscan_imaging::{
    DetectOptions, Filter, Interpolation, Point, Quad, WarpOptions, detect_luma, detect_with,
};
use zeroize::Zeroize;

use super::error::{ApiError, ApiErrorKind, ApiResult};

/// Cạnh dài ảnh làm việc khi dò trên khung xem trước: nhỏ hơn mặc định 640 để giữ < 20 ms/khung trên máy tầm trung.
pub const PREVIEW_WORKING_SIZE: u32 = 480;
/// Cạnh lớn nhất của khung xem trước chấp nhận (camera xem trước thường ≤ 1920).
pub const MAX_PREVIEW_SIDE: u32 = 4096;
/// Cạnh dài tối đa của trang lưu vào kho (≈ 500 dpi trên A4 - đủ cho OCR, chặn ảnh 50 MP phình kho).
pub const MAX_SCAN_SIDE: u32 = 4096;
/// Cạnh dài tối đa của ảnh xem trước kết quả trả về Dart.
pub const MAX_RENDER_SIDE: u32 = 2048;
/// Chất lượng JPEG trang quét lưu vào kho.
pub const SCAN_JPEG_QUALITY: u8 = 90;
/// Lề cho phép ngoài khung khi người dùng kéo góc (sau đó bị kẹp về 0..1).
const CORNER_SLACK: f64 = 0.05;

/// Tứ giác tài liệu: 8 số chuẩn hóa `[x_tl, y_tl, x_tr, y_tr, x_br, y_br, x_bl, y_bl]` + độ tin cậy 0..1.
#[derive(Debug, Clone, PartialEq)]
pub struct ScanQuad {
    /// Bốn góc chuẩn hóa 0..1 (TL, TR, BR, BL), theo ảnh đã xoay đúng chiều hiển thị.
    pub corners: Vec<f64>,
    /// Độ tin cậy của bộ dò.
    pub confidence: f32,
}

/// Mã các bộ lọc theo thứ tự hiển thị (`original`, `auto`, `color`, `gray`, `bw`, `shadow`, `whiteboard`).
#[frb(sync)]
#[must_use]
pub fn scan_filters() -> Vec<String> {
    Filter::ALL.iter().map(|f| f.code().to_owned()).collect()
}

/// Dò tài liệu trên mặt phẳng độ sáng (kênh Y của YUV420/NV21) của một khung xem trước.
///
/// `row_stride` là số byte mỗi hàng (≥ `width`, có đệm). `rotation` là số độ (0/90/180/270) phải xoay ảnh cảm biến
/// theo chiều kim đồng hồ để đúng chiều hiển thị; tọa độ trả về đã xoay theo đó.
///
/// # Errors
/// `InvalidInput` khi kích thước/stride/góc xoay không hợp lệ hoặc bộ đệm thiếu byte.
pub fn detect_preview(
    luma: Vec<u8>,
    width: u32,
    height: u32,
    row_stride: u32,
    rotation: u32,
) -> ApiResult<Option<ScanQuad>> {
    let rot = Rotation::from_degrees(rotation)?;
    let gray = luma_plane(luma, width, height, row_stride)?;
    let opts = DetectOptions {
        working_size: PREVIEW_WORKING_SIZE,
        ..DetectOptions::default()
    };
    let Some(d) = detect_luma(&gray, opts)? else {
        return Ok(None);
    };
    Ok(Some(normalize(&d.quad, width, height, rot, d.confidence)))
}

/// Chép mặt phẳng Y (có thể có đệm cuối hàng) thành ảnh xám liền.
fn luma_plane(mut luma: Vec<u8>, width: u32, height: u32, row_stride: u32) -> ApiResult<GrayImage> {
    let bad = |m: &'static str| ApiError::new(ApiErrorKind::InvalidInput, m);
    if !(16..=MAX_PREVIEW_SIDE).contains(&width) || !(16..=MAX_PREVIEW_SIDE).contains(&height) {
        return Err(bad("kích thước khung xem trước"));
    }
    if row_stride < width || row_stride > 4 * MAX_PREVIEW_SIDE {
        return Err(bad("stride khung xem trước"));
    }
    let (w, h, s) = (width as usize, height as usize, row_stride as usize);
    // Hàng cuối có thể không có phần đệm (Android cắt bớt) ⇒ chỉ đòi stride*(h-1) + w.
    if luma.len() < s * (h - 1) + w {
        return Err(bad("thiếu byte khung xem trước"));
    }
    let data = if s == w {
        luma.truncate(w * h);
        luma
    } else {
        let mut out = Vec::with_capacity(w * h);
        for row in 0..h {
            out.extend_from_slice(&luma[row * s..row * s + w]);
        }
        out
    };
    GrayImage::from_raw(width, height, data).ok_or_else(|| bad("bộ đệm khung xem trước"))
}

/// Góc xoay cảm biến → hiển thị (chiều kim đồng hồ).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rotation {
    R0,
    R90,
    R180,
    R270,
}

impl Rotation {
    fn from_degrees(d: u32) -> ApiResult<Self> {
        match d {
            0 => Ok(Self::R0),
            90 => Ok(Self::R90),
            180 => Ok(Self::R180),
            270 => Ok(Self::R270),
            _ => Err(ApiError::new(
                ApiErrorKind::InvalidInput,
                "góc xoay phải là 0/90/180/270",
            )),
        }
    }

    /// Xoay một điểm chuẩn hóa: ảnh W x H xoay 90° thuận chiều kim đồng hồ thì (x, y) → (1 - y, x).
    fn apply(self, x: f64, y: f64) -> (f64, f64) {
        match self {
            Self::R0 => (x, y),
            Self::R90 => (1.0 - y, x),
            Self::R180 => (1.0 - x, 1.0 - y),
            Self::R270 => (y, 1.0 - x),
        }
    }
}

/// Chuẩn hóa tứ giác pixel về 0..1, xoay, rồi sắp lại TL, TR, BR, BL theo chiều hiển thị.
fn normalize(quad: &Quad, width: u32, height: u32, rot: Rotation, confidence: f32) -> ScanQuad {
    let (fw, fh) = (f64::from(width), f64::from(height));
    let pts = quad.pts.map(|p| {
        let (nx, ny) = rot.apply((p.x / fw).clamp(0.0, 1.0), (p.y / fh).clamp(0.0, 1.0));
        Point::new(nx, ny)
    });
    // Xoay giữ diện tích ⇒ không thể suy biến khi tứ giác gốc hợp lệ; dự phòng giữ thứ tự cũ.
    let pts = Quad::from_unordered(pts).map_or(pts, |q| q.pts);
    ScanQuad {
        corners: pts.iter().flat_map(|p| [p.x, p.y]).collect(),
        confidence,
    }
}

/// Ảnh vừa chụp (hoặc chọn từ thư viện), đã giải mã có giới hạn và xoay theo EXIF; siêu dữ liệu gốc bị bỏ. Dart giữ
/// đối tượng này (opaque) trong lúc duyệt trang rồi `dispose()`; bộ đệm điểm ảnh bị xóa trắng khi giải phóng.
#[frb(opaque)]
pub struct CapturedImage {
    /// Luôn là `ImageRgb8` (dựng trong [`CapturedImage::decode`]); giữ dạng `DynamicImage` để bộ dò mượn trực tiếp.
    img: DynamicImage,
}

impl Drop for CapturedImage {
    fn drop(&mut self) {
        // Ảnh giấy tờ tùy thân (D3): không để lại trong vùng nhớ đã trả cho bộ cấp phát.
        if let DynamicImage::ImageRgb8(rgb) = &mut self.img {
            let buf: &mut [u8] = rgb;
            buf.zeroize();
        }
    }
}

impl CapturedImage {
    /// Giải mã ảnh chụp (JPEG/PNG/WebP).
    ///
    /// # Errors
    /// `UnsupportedImage`, `ImageTooLarge`.
    pub fn decode(bytes: Vec<u8>) -> ApiResult<CapturedImage> {
        let mut bytes = bytes;
        let img = tsuscan_imaging::decode(&bytes);
        bytes.zeroize();
        let img = img?;
        let img = match img {
            DynamicImage::ImageRgb8(_) => img,
            other => DynamicImage::ImageRgb8(other.to_rgb8()),
        };
        Ok(Self { img })
    }

    /// Chiều rộng (px, sau khi xoay EXIF).
    #[frb(sync, getter)]
    #[must_use]
    pub fn width(&self) -> u32 {
        self.img.width()
    }

    /// Chiều cao (px, sau khi xoay EXIF).
    #[frb(sync, getter)]
    #[must_use]
    pub fn height(&self) -> u32 {
        self.img.height()
    }

    /// Dò tài liệu trên ảnh đầy đủ (chính xác hơn khung xem trước: độ phân giải cao, có kênh màu).
    ///
    /// # Errors
    /// `UnsupportedImage` khi ảnh quá nhỏ.
    pub fn detect(&self) -> ApiResult<Option<ScanQuad>> {
        let found = detect_with(&self.img, DetectOptions::default());
        let (w, h) = (self.img.width(), self.img.height());
        Ok(found?.map(|d| normalize(&d.quad, w, h, Rotation::R0, d.confidence)))
    }

    /// Nắn theo `corners` (8 số chuẩn hóa; rỗng = giữ cả khung) rồi áp bộ lọc; trả JPEG cạnh dài ≤ `max_side`
    /// (kẹp 256..=[`MAX_RENDER_SIDE`]) để xem trước.
    ///
    /// # Errors
    /// `InvalidInput` (góc/bộ lọc sai), `UnsupportedImage`.
    pub fn render(&self, corners: Vec<f64>, filter: String, max_side: u32) -> ApiResult<Vec<u8>> {
        let out = self.process(&corners, &filter, max_side.clamp(256, MAX_RENDER_SIDE))?;
        super::library::encode_jpeg(&out, 85)
    }

    /// Ảnh trang cuối cùng (độ phân giải đầy đủ, ≤ [`MAX_SCAN_SIDE`]) - dùng khi lưu vào kho.
    pub(crate) fn process(&self, corners: &[f64], filter: &str, max_side: u32) -> ApiResult<DynamicImage> {
        let filter =
            Filter::from_code(filter).map_err(|_| ApiError::new(ApiErrorKind::InvalidInput, "bộ lọc"))?;
        let (w, h) = (self.img.width(), self.img.height());
        let page = if corners.is_empty() {
            let long = w.max(h);
            if long > max_side {
                self.img
                    .resize(max_side, max_side, FilterType::CatmullRom)
                    .to_rgb8()
            } else {
                self.img.to_rgb8()
            }
        } else {
            let quad = quad_from(corners, w, h)?;
            let opts = WarpOptions {
                interpolation: Interpolation::Bicubic,
                snap_paper: true,
                max_side,
            };
            let warped = match &self.img {
                DynamicImage::ImageRgb8(rgb) => tsuscan_imaging::warp::warp_rgb(rgb, &quad, opts),
                other => tsuscan_imaging::warp(other, &quad, opts),
            };
            warped
                .map_err(|e| match e {
                    tsuscan_imaging::ImagingError::InvalidQuad(m) => {
                        ApiError::new(ApiErrorKind::InvalidInput, m)
                    }
                    other => other.into(),
                })?
                .image
        };
        Ok(tsuscan_imaging::filters::apply(&page, filter, None))
    }
}

/// Dựng tứ giác pixel từ 8 số chuẩn hóa do người dùng kéo (cho phép lệch nhẹ ngoài khung rồi kẹp lại).
fn quad_from(c: &[f64], w: u32, h: u32) -> ApiResult<Quad> {
    let bad = |m: &'static str| ApiError::new(ApiErrorKind::InvalidInput, m);
    if c.len() != 8 {
        return Err(bad("cần đúng 8 tọa độ góc"));
    }
    let lo = -CORNER_SLACK;
    let hi = 1.0 + CORNER_SLACK;
    if c.iter().any(|v| !v.is_finite() || *v < lo || *v > hi) {
        return Err(bad("tọa độ góc ngoài khung"));
    }
    let (fw, fh) = (f64::from(w), f64::from(h));
    let pts =
        [0, 1, 2, 3].map(|i| Point::new(c[2 * i].clamp(0.0, 1.0) * fw, c[2 * i + 1].clamp(0.0, 1.0) * fh));
    let q = Quad::from_unordered(pts).map_err(|_| bad("tứ giác suy biến"))?;
    if !q.is_convex() || q.area() < 0.01 * fw * fh {
        return Err(bad("tứ giác không lồi hoặc quá nhỏ"));
    }
    Ok(q)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xoay_diem_chuan_hoa() {
        assert_eq!(Rotation::R90.apply(0.0, 0.0), (1.0, 0.0));
        assert_eq!(Rotation::R90.apply(1.0, 0.0), (1.0, 1.0));
        assert_eq!(Rotation::R180.apply(0.25, 0.75), (0.75, 0.25));
        assert_eq!(Rotation::R270.apply(0.0, 0.0), (0.0, 1.0));
        assert!(Rotation::from_degrees(45).is_err());
    }

    #[test]
    fn mat_phang_y_co_stride_bo_dem_cuoi_hang() -> ApiResult<()> {
        // 20x16, stride 24, hàng cuối thiếu đệm.
        let (w, h, s) = (20u32, 16u32, 24usize);
        let mut buf = vec![0u8; s * (h as usize - 1) + w as usize];
        for y in 0..h as usize {
            for x in 0..w as usize {
                buf[y * s + x] = (x + y) as u8;
            }
        }
        let g = luma_plane(buf, w, h, s as u32)?;
        assert_eq!(g.get_pixel(19, 15).0[0], 34);
        assert!(luma_plane(vec![0; 10], w, h, s as u32).is_err());
        assert!(luma_plane(vec![0; 400], w, h, 10).is_err());
        Ok(())
    }

    #[test]
    fn goc_sai_bi_tu_choi() {
        assert!(quad_from(&[0.0; 6], 100, 100).is_err());
        assert!(quad_from(&[0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, f64::NAN], 100, 100).is_err());
        assert!(quad_from(&[0.0, 0.0, 2.0, 0.0, 1.0, 1.0, 0.0, 1.0], 100, 100).is_err());
        // Thắt nơ (tự cắt) ⇒ sắp lại thành lồi, hợp lệ; ba điểm thẳng hàng ⇒ suy biến.
        assert!(quad_from(&[0.0, 0.0, 0.5, 0.0, 1.0, 0.0, 0.0, 0.0], 100, 100).is_err());
        assert!(quad_from(&[0.1, 0.1, 0.9, 0.1, 0.9, 0.9, 0.1, 0.9], 100, 100).is_ok());
    }
}

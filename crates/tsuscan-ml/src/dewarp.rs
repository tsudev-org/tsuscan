//! Làm phẳng trang cong (sách, giấy gấp) bằng UVDoc (PaddlePaddle, Apache-2.0).
//!
//! Mạng UVDoc nhận ảnh 712×488 và dự đoán lưới tọa độ thô 2×45×31 (chuẩn hóa [-1, 1], `align_corners`). Bản ONNX tự
//! phóng lưới và `GridSample`, nhưng `Resize`/`GridSample` của tract cho kết quả sai với mô hình này (đối chiếu
//! onnxruntime 18/11/2026) ⇒ tract chỉ chạy tới lưới thô; phóng lưới + lấy mẫu song tuyến trên ẢNH GỐC làm bằng Rust
//! (giữ độ nét, bộ nhớ ít hơn). Dùng ở chế độ Sách (P6), sau bước nắn phối cảnh cổ điển (P1).

use image::{Rgb, RgbImage, imageops};
use tsuscan_imaging::warp::sample_bilinear;

use crate::error::{MlError, Result};
use crate::prep::{Channels, Normalize, check_image, to_tensor};
use crate::runtime::{Dim, Session};

/// Mã mô hình trong manifest.
pub const MODEL: &str = "dewarp";
/// Kích thước vào của mạng (rộng × cao).
pub const NET_SIZE: (u32, u32) = (488, 712);
/// Tensor lưới thô trong đồ thị UVDoc (trước `Resize` cuối).
const GRID_TENSOR: &str = "p2o.pd_op.add.41.0";

/// Bộ làm phẳng trang.
#[derive(Debug)]
pub struct Dewarper {
    session: Session,
}

/// Lưới tọa độ thô: `x[k]`, `y[k]` chuẩn hóa [-1, 1], kích thước `gw × gh`.
#[derive(Debug, Clone)]
pub struct Grid {
    /// Số cột.
    pub gw: usize,
    /// Số hàng.
    pub gh: usize,
    /// Hoành độ chuẩn hóa theo hàng.
    pub x: Vec<f32>,
    /// Tung độ chuẩn hóa theo hàng.
    pub y: Vec<f32>,
}

impl Grid {
    /// Nội suy song tuyến (align_corners) tại tọa độ chuẩn hóa đầu ra (u, v) ∈ [0, 1].
    #[must_use]
    pub fn at(&self, u: f64, v: f64) -> (f64, f64) {
        let fx = (u * (self.gw - 1) as f64).clamp(0.0, (self.gw - 1) as f64);
        let fy = (v * (self.gh - 1) as f64).clamp(0.0, (self.gh - 1) as f64);
        let (x0, y0) = (fx.floor() as usize, fy.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(self.gw - 1), (y0 + 1).min(self.gh - 1));
        let (tx, ty) = (fx - x0 as f64, fy - y0 as f64);
        let lerp = |g: &[f32]| {
            let p = |x: usize, y: usize| f64::from(g[y * self.gw + x]);
            let top = p(x0, y0) * (1.0 - tx) + p(x1, y0) * tx;
            let bot = p(x0, y1) * (1.0 - tx) + p(x1, y1) * tx;
            top * (1.0 - ty) + bot * ty
        };
        (lerp(&self.x), lerp(&self.y))
    }
}

impl Dewarper {
    /// Dựng từ byte mô hình đã kiểm.
    ///
    /// # Errors
    /// Mô hình không dịch được.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let dims = [
            Dim::Fixed(1),
            Dim::Fixed(3),
            Dim::Fixed(NET_SIZE.1 as usize),
            Dim::Fixed(NET_SIZE.0 as usize),
        ];
        let session = Session::from_bytes_with_output(MODEL, bytes, &dims, Some(GRID_TENSOR))?;
        Ok(Self { session })
    }

    /// Dự đoán lưới thô cho ảnh.
    ///
    /// # Errors
    /// Ảnh rỗng/quá lớn hoặc lỗi suy luận.
    pub fn grid(&self, img: &RgbImage) -> Result<Grid> {
        check_image(img)?;
        let small = imageops::resize(img, NET_SIZE.0, NET_SIZE.1, imageops::FilterType::Triangle);
        let out = self
            .session
            .run(to_tensor(&small, Channels::Rgb, Normalize::UNIT)?)?;
        let t = out
            .first()
            .ok_or_else(|| MlError::Inference("dewarp: không có đầu ra".into()))?;
        let v = t
            .to_plain_array_view::<f32>()
            .map_err(|e| MlError::Inference(format!("{e:#}")))?;
        let shape = v.shape().to_vec();
        if shape.len() != 4 || shape[0] != 1 || shape[1] != 2 || shape[2] < 2 || shape[3] < 2 {
            return Err(MlError::Inference(format!(
                "dewarp: lưới {shape:?} không phải [1,2,h,w]"
            )));
        }
        let data: Vec<f32> = v.iter().copied().collect();
        if data.iter().any(|f| !f.is_finite()) {
            return Err(MlError::Inference("dewarp: lưới có giá trị không hữu hạn".into()));
        }
        let plane = shape[2] * shape[3];
        Ok(Grid {
            gw: shape[3],
            gh: shape[2],
            x: data[..plane].to_vec(),
            y: data[plane..].to_vec(),
        })
    }

    /// Làm phẳng ảnh trang; ảnh ra cùng kích thước ảnh vào.
    ///
    /// # Errors
    /// Ảnh rỗng/quá nhỏ/quá lớn hoặc lỗi suy luận.
    pub fn dewarp(&self, img: &RgbImage) -> Result<RgbImage> {
        let (w, h) = img.dimensions();
        if w < 32 || h < 32 {
            return Err(MlError::InvalidInput(format!("ảnh {w}x{h} quá nhỏ để làm phẳng")));
        }
        let grid = self.grid(img)?;
        Ok(apply_grid(img, &grid))
    }
}

/// Lấy mẫu ảnh gốc theo lưới (GridSample song tuyến, align_corners, ngoài biên lấy màu biên).
#[must_use]
pub fn apply_grid(img: &RgbImage, grid: &Grid) -> RgbImage {
    let (w, h) = img.dimensions();
    let (wm, hm) = (f64::from(w.max(2) - 1), f64::from(h.max(2) - 1));
    RgbImage::from_fn(w, h, |x, y| {
        let (gx, gy) = grid.at(f64::from(x) / wm, f64::from(y) / hm);
        let (sx, sy) = (f64::midpoint(gx, 1.0) * wm, f64::midpoint(gy, 1.0) * hm);
        if sx.is_finite() && sy.is_finite() {
            sample_bilinear(img, sx, sy)
        } else {
            Rgb([255, 255, 255])
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luoi_dong_nhat_giu_nguyen_anh() {
        // Lưới đồng nhất 3×2: x = -1..1, y = -1..1.
        let grid = Grid {
            gw: 3,
            gh: 2,
            x: vec![-1.0, 0.0, 1.0, -1.0, 0.0, 1.0],
            y: vec![-1.0, -1.0, -1.0, 1.0, 1.0, 1.0],
        };
        let img = RgbImage::from_fn(40, 30, |x, y| Rgb([(x * 6) as u8, (y * 8) as u8, 7]));
        let out = apply_grid(&img, &grid);
        for (x, y) in [(0, 0), (39, 29), (17, 11)] {
            assert_eq!(out.get_pixel(x, y), img.get_pixel(x, y), "({x},{y})");
        }
        let (gx, gy) = grid.at(0.25, 0.5);
        assert!((gx + 0.5).abs() < 1e-9 && gy.abs() < 1e-9);
    }
}

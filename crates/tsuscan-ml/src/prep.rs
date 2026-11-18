//! Tiền xử lý ảnh → tensor NCHW f32.

use image::{RgbImage, imageops::FilterType};
use tract_onnx::prelude::Tensor;

use crate::error::{MlError, Result};

/// Thứ tự kênh mô hình mong đợi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channels {
    /// R, G, B.
    Rgb,
    /// B, G, R (mô hình PaddlePaddle).
    Bgr,
}

/// Chuẩn hóa `(x/255 - mean) / std` theo kênh (sau khi đổi thứ tự kênh).
#[derive(Debug, Clone, Copy)]
pub struct Normalize {
    /// Trung bình theo kênh.
    pub mean: [f32; 3],
    /// Độ lệch chuẩn theo kênh.
    pub std: [f32; 3],
}

impl Normalize {
    /// Chỉ chia 255.
    pub const UNIT: Self = Self {
        mean: [0.0; 3],
        std: [1.0; 3],
    };
    /// Chuẩn ImageNet.
    pub const IMAGENET: Self = Self {
        mean: [0.485, 0.456, 0.406],
        std: [0.229, 0.224, 0.225],
    };
}

/// Chặn ảnh rỗng hoặc vượt giới hạn lõi ảnh (≤ 100 MP, cạnh ≤ 30000) trước mọi bước suy luận.
///
/// # Errors
/// [`MlError::InvalidInput`] hoặc lỗi kích thước của lõi ảnh.
pub fn check_image(img: &RgbImage) -> Result<()> {
    if img.width() == 0 || img.height() == 0 {
        return Err(MlError::InvalidInput("ảnh rỗng".into()));
    }
    tsuscan_imaging::decode::check_dimensions(img.width(), img.height())?;
    Ok(())
}

/// Ảnh → tensor `[1, 3, h, w]`.
///
/// # Errors
/// Ảnh rỗng.
pub fn to_tensor(img: &RgbImage, ch: Channels, norm: Normalize) -> Result<Tensor> {
    let (w, h) = (img.width() as usize, img.height() as usize);
    if w == 0 || h == 0 {
        return Err(MlError::InvalidInput("ảnh rỗng".into()));
    }
    let plane = w * h;
    let mut data = vec![0f32; 3 * plane];
    let order: [usize; 3] = match ch {
        Channels::Rgb => [0, 1, 2],
        Channels::Bgr => [2, 1, 0],
    };
    for (i, p) in img.pixels().enumerate() {
        for c in 0..3 {
            let v = f32::from(p.0[order[c]]) / 255.0;
            data[c * plane + i] = (v - norm.mean[c]) / norm.std[c];
        }
    }
    Tensor::from_shape(&[1, 3, h, w], &data).map_err(|e| MlError::Inference(format!("{e:#}")))
}

/// Thu phóng giữ tỉ lệ để cạnh dài ≤ `limit`, rồi làm tròn mỗi cạnh về bội số `multiple` (≥ `multiple`).
/// Trả về ảnh mới và hệ số (sx, sy) từ ảnh mới về ảnh gốc.
#[must_use]
pub fn resize_limit(img: &RgbImage, limit: u32, multiple: u32) -> (RgbImage, f64, f64) {
    let (w, h) = img.dimensions();
    let long = w.max(h).max(1);
    let s = if long > limit {
        f64::from(limit) / f64::from(long)
    } else {
        1.0
    };
    let round = |v: u32| -> u32 {
        let v = (f64::from(v) * s).round() as u32;
        (v.div_ceil(multiple) * multiple).max(multiple)
    };
    let (nw, nh) = (round(w), round(h));
    let out = image::imageops::resize(img, nw, nh, FilterType::Triangle);
    (out, f64::from(w) / f64::from(nw), f64::from(h) / f64::from(nh))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tensor_dung_thu_tu_kenh() {
        let img = RgbImage::from_pixel(2, 1, image::Rgb([255, 0, 51]));
        let t = to_tensor(&img, Channels::Bgr, Normalize::UNIT).unwrap();
        assert_eq!(t.shape(), &[1, 3, 1, 2]);
        let v = t
            .to_plain_array_view::<f32>()
            .unwrap()
            .iter()
            .copied()
            .collect::<Vec<f32>>();
        assert!((v[0] - 0.2).abs() < 1e-6); // B
        assert!((v[4] - 1.0).abs() < 1e-6); // R
        assert!(to_tensor(&RgbImage::new(0, 0), Channels::Rgb, Normalize::UNIT).is_err());
    }

    #[test]
    fn resize_boi_so_32() {
        let img = RgbImage::new(1000, 500);
        let (r, sx, sy) = resize_limit(&img, 640, 32);
        assert_eq!(r.dimensions(), (640, 320));
        assert!((sx - 1000.0 / 640.0).abs() < 1e-9 && (sy - 500.0 / 320.0).abs() < 1e-9);
        let (r, _, _) = resize_limit(&RgbImage::new(10, 10), 640, 32);
        assert_eq!(r.dimensions(), (32, 32));
    }
}

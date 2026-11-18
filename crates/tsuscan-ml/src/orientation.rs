//! Hướng trang 0/90/180/270 độ bằng PP-LCNet x1.0 doc_ori, và xoay trang về đúng chiều đọc.

use image::{RgbImage, imageops};

use crate::error::{MlError, Result};
use crate::prep::{Channels, Normalize, to_tensor};
use crate::runtime::{Dim, Session};

/// Mã mô hình trong manifest.
pub const MODEL: &str = "doc-orientation";

/// Góc xoay (theo chiều kim đồng hồ) cần áp lên ảnh để trang về đúng chiều đọc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    /// Đã đúng chiều.
    None,
    /// Xoay 90° theo chiều kim đồng hồ.
    Cw90,
    /// Xoay 180°.
    Cw180,
    /// Xoay 270° theo chiều kim đồng hồ (= 90° ngược chiều).
    Cw270,
}

impl Rotation {
    /// Áp phép xoay.
    #[must_use]
    pub fn apply(self, img: &RgbImage) -> RgbImage {
        match self {
            Self::None => img.clone(),
            Self::Cw90 => imageops::rotate90(img),
            Self::Cw180 => imageops::rotate180(img),
            Self::Cw270 => imageops::rotate270(img),
        }
    }

    /// Số độ theo chiều kim đồng hồ.
    #[must_use]
    pub fn degrees(self) -> u16 {
        match self {
            Self::None => 0,
            Self::Cw90 => 90,
            Self::Cw180 => 180,
            Self::Cw270 => 270,
        }
    }
}

/// Bộ phân loại hướng trang.
#[derive(Debug)]
pub struct OrientationClassifier {
    session: Session,
}

/// Kết quả phân loại.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrientationResult {
    /// Phép xoay sửa hướng.
    pub fix: Rotation,
    /// Xác suất của lớp thắng (0..1).
    pub confidence: f32,
}

impl OrientationClassifier {
    /// Dựng từ byte mô hình đã kiểm.
    ///
    /// # Errors
    /// Mô hình không dịch được.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let session = Session::from_bytes(
            MODEL,
            bytes,
            &[Dim::Fixed(1), Dim::Fixed(3), Dim::Fixed(224), Dim::Fixed(224)],
        )?;
        Ok(Self { session })
    }

    /// Phân loại hướng của ảnh trang.
    ///
    /// # Errors
    /// Ảnh rỗng hoặc lỗi suy luận.
    pub fn classify(&self, img: &RgbImage) -> Result<OrientationResult> {
        let input = to_tensor(&preprocess(img)?, Channels::Rgb, Normalize::IMAGENET)?;
        let out = self.session.run(input)?;
        let probs = out
            .first()
            .ok_or_else(|| MlError::Inference("doc-orientation: không có đầu ra".into()))?;
        let v: Vec<f32> = probs
            .to_plain_array_view::<f32>()
            .map_err(|e| MlError::Inference(format!("{e:#}")))?
            .iter()
            .copied()
            .collect();
        if v.len() != 4 {
            return Err(MlError::Inference(format!(
                "doc-orientation: {} lớp, cần 4",
                v.len()
            )));
        }
        let (best, &p) = v
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap_or((0, &0.0));
        // Nhãn mô hình = góc trang đang bị xoay THEO chiều kim đồng hồ (đã kiểm bằng `tests/models.rs`)
        // ⇒ sửa bằng xoay ngược lại.
        let fix = [Rotation::None, Rotation::Cw270, Rotation::Cw180, Rotation::Cw90][best];
        Ok(OrientationResult { fix, confidence: p })
    }

    /// Xoay trang về đúng chiều nếu tin cậy ≥ `min_confidence`; trả về ảnh và phép xoay đã áp.
    ///
    /// # Errors
    /// Như [`Self::classify`].
    pub fn upright(&self, img: &RgbImage, min_confidence: f32) -> Result<(RgbImage, Rotation)> {
        let r = self.classify(img)?;
        let fix = if r.confidence >= min_confidence {
            r.fix
        } else {
            Rotation::None
        };
        Ok((fix.apply(img), fix))
    }
}

/// Thu cạnh ngắn về 256 rồi cắt giữa 224×224 (như huấn luyện PaddleClas).
fn preprocess(img: &RgbImage) -> Result<RgbImage> {
    crate::prep::check_image(img)?;
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return Err(MlError::InvalidInput("ảnh rỗng".into()));
    }
    let s = 256.0 / f64::from(w.min(h));
    let (nw, nh) = (
        ((f64::from(w) * s).round() as u32).max(224),
        ((f64::from(h) * s).round() as u32).max(224),
    );
    let r = imageops::resize(img, nw, nh, imageops::FilterType::Triangle);
    Ok(imageops::crop_imm(&r, (nw - 224) / 2, (nh - 224) / 2, 224, 224).to_image())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xoay_va_doi_chieu() {
        let img = RgbImage::new(3, 2);
        assert_eq!(Rotation::Cw90.apply(&img).dimensions(), (2, 3));
        assert_eq!(Rotation::Cw180.apply(&img).dimensions(), (3, 2));
        assert_eq!(Rotation::Cw270.degrees(), 270);
        assert_eq!(
            preprocess(&RgbImage::new(500, 1000)).unwrap().dimensions(),
            (224, 224)
        );
        assert!(preprocess(&RgbImage::new(0, 5)).is_err());
    }
}

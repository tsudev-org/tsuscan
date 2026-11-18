//! Nén ảnh trang thông minh: tự nhận trang màu / xám / đen trắng rồi chọn bộ nén hợp.
//!
//! - Màu: JPEG baseline xen kẽ YCbCr 4:2:0.
//! - Xám: JPEG một kênh (nhỏ hơn màu ≈ 1/3, không lệch màu giả do nén chroma).
//! - Đen trắng: CCITT Group 4 1 bit/điểm (KHÔNG mất dữ liệu, thường nhỏ hơn JPEG 5-15 lần với trang chữ).
//!
//! JBIG2 không dùng: chưa có bộ mã hóa thuần Rust, và chế độ ký hiệu có mất dữ liệu từng gây thay chữ số trên
//! bản scan (sự cố máy Xerox 2013) - không chấp nhận được với hợp đồng/giấy tờ. Lý do đầy đủ: ADR-0008.

use image::{DynamicImage, GrayImage, RgbImage, imageops::FilterType};

use crate::error::{DocError, Result};

/// Chế độ màu khi xuất.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorMode {
    /// Tự nhận theo nội dung trang.
    #[default]
    Auto,
    /// Giữ màu.
    Color,
    /// Ép thang xám.
    Gray,
    /// Ép đen trắng 1 bit.
    Bilevel,
}

/// Mức chất lượng / dung lượng.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Quality {
    /// Lưu trữ: JPEG q90, tối đa 300 DPI.
    High,
    /// Cân bằng (mặc định): JPEG q80, tối đa 200 DPI.
    #[default]
    Balanced,
    /// Gửi qua tin nhắn/email: JPEG q65, tối đa 150 DPI.
    Small,
}

impl Quality {
    /// Chất lượng JPEG 1..=100.
    #[must_use]
    pub fn jpeg_quality(self) -> u8 {
        match self {
            Self::High => 90,
            Self::Balanced => 80,
            Self::Small => 65,
        }
    }

    /// DPI tối đa cho ảnh màu/xám (lớn hơn thì thu nhỏ).
    #[must_use]
    pub fn max_dpi(self) -> f32 {
        match self {
            Self::High => 300.0,
            Self::Balanced => 200.0,
            Self::Small => 150.0,
        }
    }

    /// DPI tối đa cho ảnh đen trắng (G4 không mất dữ liệu; chữ nhỏ cần độ phân giải cao hơn ảnh xám).
    #[must_use]
    pub fn max_dpi_bilevel(self) -> f32 {
        match self {
            Self::High => 600.0,
            Self::Balanced => 300.0,
            Self::Small => 200.0,
        }
    }

    /// Mã ngắn cho CLI.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Balanced => "balanced",
            Self::Small => "small",
        }
    }

    /// Đọc từ mã ngắn.
    ///
    /// # Errors
    /// [`DocError::InvalidArgument`] khi mã lạ.
    pub fn from_code(code: &str) -> Result<Self> {
        match code {
            "high" => Ok(Self::High),
            "balanced" => Ok(Self::Balanced),
            "small" => Ok(Self::Small),
            _ => Err(DocError::InvalidArgument("chất lượng: high | balanced | small")),
        }
    }
}

/// Loại nội dung trang.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    /// Có màu.
    Color,
    /// Thang xám.
    Gray,
    /// Gần như chỉ đen và trắng.
    Bilevel,
}

/// Cách mã hóa luồng ảnh trong PDF.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// JPEG (DCTDecode); `gray` = 1 kênh.
    Jpeg {
        /// Ảnh xám một kênh.
        gray: bool,
    },
    /// CCITT Group 4 (K = -1), 1 bit, 0 = đen.
    CcittG4,
}

/// Ảnh đã nén sẵn sàng ghi vào PDF.
#[derive(Debug, Clone)]
pub struct EncodedImage {
    /// Rộng (điểm ảnh của luồng đã nén, có thể nhỏ hơn ảnh gốc khi thu DPI).
    pub width: u32,
    /// Cao.
    pub height: u32,
    /// Dữ liệu nén.
    pub data: Vec<u8>,
    /// Bộ nén đã dùng.
    pub encoding: Encoding,
}

/// Ngưỡng tỉ lệ điểm ảnh "có màu" để coi trang là màu (0,2%).
const COLOR_FRACTION: f64 = 0.002;
/// Độ lệch kênh tối thiểu để coi một điểm là có màu.
const CHROMA_MIN: u8 = 24;
/// Tỉ lệ điểm trung gian tối đa để coi trang là đen trắng (1%).
const MIDTONE_FRACTION: f64 = 0.01;

/// Nhận loại nội dung (lấy mẫu lưới 1/4 mỗi chiều để nhanh trên ảnh nhiều MP).
#[must_use]
pub fn classify(img: &DynamicImage) -> ImageKind {
    let step = if img.width() * img.height() > 1_000_000 {
        4
    } else {
        1
    };
    let mut total = 0u64;
    let mut colored = 0u64;
    let mut mid = 0u64;
    let rgb_owned;
    let rgb: &RgbImage = if let DynamicImage::ImageRgb8(r) = img {
        r
    } else if img.color().has_color() {
        rgb_owned = img.to_rgb8();
        &rgb_owned
    } else {
        // Ảnh xám sẵn: chỉ cần xét điểm trung gian.
        let g = img.to_luma8();
        for y in (0..g.height()).step_by(step) {
            for x in (0..g.width()).step_by(step) {
                total += 1;
                let v = g.get_pixel(x, y).0[0];
                mid += u64::from((48..=207).contains(&v));
            }
        }
        return if (mid as f64) <= MIDTONE_FRACTION * total.max(1) as f64 {
            ImageKind::Bilevel
        } else {
            ImageKind::Gray
        };
    };
    for y in (0..rgb.height()).step_by(step) {
        for x in (0..rgb.width()).step_by(step) {
            let [r, g, b] = rgb.get_pixel(x, y).0;
            total += 1;
            let hi = r.max(g).max(b);
            let lo = r.min(g).min(b);
            colored += u64::from(hi - lo >= CHROMA_MIN);
            let l = luma(r, g, b);
            mid += u64::from((48..=207).contains(&l));
        }
    }
    let total = total.max(1) as f64;
    if colored as f64 > COLOR_FRACTION * total {
        ImageKind::Color
    } else if mid as f64 <= MIDTONE_FRACTION * total {
        ImageKind::Bilevel
    } else {
        ImageKind::Gray
    }
}

fn luma(r: u8, g: u8, b: u8) -> u8 {
    ((u32::from(r) * 299 + u32::from(g) * 587 + u32::from(b) * 114 + 500) / 1000) as u8
}

/// Thu nhỏ ảnh nếu DPI vượt trần; trả ảnh mới (hoặc bản sao rẻ khi không cần) và DPI thực tế.
fn limit_dpi(img: &DynamicImage, dpi: f32, max_dpi: f32) -> (DynamicImage, f32) {
    if dpi <= max_dpi * 1.02 {
        return (img.clone(), dpi);
    }
    let s = max_dpi / dpi;
    let w = ((img.width() as f32 * s).round() as u32).max(1);
    let h = ((img.height() as f32 * s).round() as u32).max(1);
    (img.resize_exact(w, h, FilterType::CatmullRom), max_dpi)
}

/// Nén ảnh trang theo chế độ màu và chất lượng. Trả ảnh đã nén và DPI thực tế (không đổi khổ giấy).
///
/// # Errors
/// [`DocError::InvalidArgument`] khi ảnh rỗng hoặc cạnh > 65.535; [`DocError::Encode`] khi bộ nén lỗi.
pub fn encode_page_image(
    img: &DynamicImage,
    dpi: f32,
    mode: ColorMode,
    quality: Quality,
) -> Result<EncodedImage> {
    if img.width() == 0 || img.height() == 0 {
        return Err(DocError::InvalidArgument("ảnh trang rỗng"));
    }
    let kind = match mode {
        ColorMode::Auto => classify(img),
        ColorMode::Color => ImageKind::Color,
        ColorMode::Gray => ImageKind::Gray,
        ColorMode::Bilevel => ImageKind::Bilevel,
    };
    match kind {
        ImageKind::Bilevel => {
            let (scaled, _) = limit_dpi(img, dpi, quality.max_dpi_bilevel());
            let gray = scaled.to_luma8();
            Ok(EncodedImage {
                width: gray.width(),
                height: gray.height(),
                data: encode_g4(&gray)?,
                encoding: Encoding::CcittG4,
            })
        }
        ImageKind::Gray => {
            let (scaled, _) = limit_dpi(img, dpi, quality.max_dpi());
            let gray = scaled.to_luma8();
            Ok(EncodedImage {
                width: gray.width(),
                height: gray.height(),
                data: encode_jpeg(
                    gray.as_raw(),
                    gray.width(),
                    gray.height(),
                    true,
                    quality.jpeg_quality(),
                )?,
                encoding: Encoding::Jpeg { gray: true },
            })
        }
        ImageKind::Color => {
            let (scaled, _) = limit_dpi(img, dpi, quality.max_dpi());
            let rgb = scaled.to_rgb8();
            Ok(EncodedImage {
                width: rgb.width(),
                height: rgb.height(),
                data: encode_jpeg(
                    rgb.as_raw(),
                    rgb.width(),
                    rgb.height(),
                    false,
                    quality.jpeg_quality(),
                )?,
                encoding: Encoding::Jpeg { gray: false },
            })
        }
    }
}

fn dim16(v: u32) -> Result<u16> {
    u16::try_from(v).map_err(|_| DocError::InvalidArgument("cạnh ảnh vượt 65.535 điểm"))
}

/// Nén JPEG baseline xen kẽ, bảng Huffman chuẩn - dạng tương thích rộng nhất.
///
/// KHÔNG bật bảng Huffman tối ưu: khi đó `jpeg-encoder` ghi mỗi thành phần một lượt quét (không xen kẽ) - hợp lệ
/// nhưng bộ giải mã của `hayro` (zune-jpeg) chỉ giải lượt đầu ⇒ trang đỏ hiện xanh lá (đã đo 19/11/2026). Đổi lại
/// tệp lớn hơn vài phần trăm.
///
/// # Errors
/// [`DocError::Encode`] khi bộ nén lỗi hoặc kích thước sai.
pub fn encode_jpeg(data: &[u8], width: u32, height: u32, gray: bool, quality: u8) -> Result<Vec<u8>> {
    let (w, h) = (dim16(width)?, dim16(height)?);
    let mut out = Vec::new();
    let mut enc = jpeg_encoder::Encoder::new(&mut out, quality.clamp(1, 100));
    if !gray {
        enc.set_sampling_factor(jpeg_encoder::SamplingFactor::F_2_2);
    }
    let ct = if gray {
        jpeg_encoder::ColorType::Luma
    } else {
        jpeg_encoder::ColorType::Rgb
    };
    enc.encode(data, w, h, ct)
        .map_err(|e| DocError::Encode(e.to_string()))?;
    renumber_components(&mut out);
    Ok(out)
}

/// `jpeg-encoder` đánh mã thành phần 0, 1, 2; JFIF quy ước 1, 2, 3 và một số bộ giải mã (đã thấy ở `hayro`) dựa vào
/// mã để đoán không gian màu ⇒ trang đỏ hiện thành xanh lá. Đổi mã trong SOF và mọi SOS sang 1..=n (mã chỉ là nhãn,
/// bảng lượng tử/Huffman tham chiếu theo chỉ số bảng nên không đổi gì khác).
fn renumber_components(jpeg: &mut [u8]) {
    let mut i = 2; // sau SOI
    while i + 4 <= jpeg.len() {
        if jpeg[i] != 0xFF {
            return;
        }
        let marker = jpeg[i + 1];
        if marker == 0xD9 {
            return;
        }
        let len = usize::from(u16::from_be_bytes([jpeg[i + 2], jpeg[i + 3]]));
        let seg = i + 4;
        if seg + len - 2 > jpeg.len() || len < 2 {
            return;
        }
        match marker {
            // SOF0..SOF2: P, Y, X, Nf, rồi Nf × (C, H/V, Tq).
            0xC0..=0xC2 if len >= 8 => {
                let nf = usize::from(jpeg[seg + 5]);
                for k in 0..nf {
                    let at = seg + 6 + 3 * k;
                    if at < jpeg.len() {
                        jpeg[at] = jpeg[at].wrapping_add(1);
                    }
                }
            }
            // SOS: Ns, rồi Ns × (Cs, Td/Ta); sau đó là dữ liệu entropy tới marker kế tiếp.
            0xDA => {
                let ns = usize::from(jpeg[seg]);
                for k in 0..ns {
                    let at = seg + 1 + 2 * k;
                    if at < jpeg.len() {
                        jpeg[at] = jpeg[at].wrapping_add(1);
                    }
                }
                let mut j = seg + len - 2;
                while j + 1 < jpeg.len()
                    && !(jpeg[j] == 0xFF && jpeg[j + 1] != 0 && !(0xD0..=0xD7).contains(&jpeg[j + 1]))
                {
                    j += 1;
                }
                i = j;
                continue;
            }
            _ => {}
        }
        i = seg + len - 2;
    }
}

/// Nén CCITT Group 4 (T.6): điểm có độ sáng < 128 là đen. Kết thúc bằng EOFB.
///
/// # Errors
/// [`DocError::InvalidArgument`] khi ảnh rỗng.
pub fn encode_g4(gray: &GrayImage) -> Result<Vec<u8>> {
    if gray.width() == 0 || gray.height() == 0 {
        return Err(DocError::InvalidArgument("ảnh trang rỗng"));
    }
    let width = gray.width();
    let mut enc = fax::encoder::Encoder::new(fax::VecWriter::with_capacity(
        (width as usize * gray.height() as usize) / 16,
    ));
    for row in gray.rows() {
        let pels = row.map(|p| {
            if p.0[0] < 128 {
                fax::Color::Black
            } else {
                fax::Color::White
            }
        });
        // VecWriter không bao giờ lỗi (Error = Infallible).
        let Ok(()) = enc.encode_line(pels, width);
    }
    let Ok(writer) = enc.finish();
    Ok(writer.finish())
}

/// Ngưỡng hóa về ảnh 0/255 (dùng khi xuất ảnh đen trắng ra PNG).
#[must_use]
pub fn to_bilevel(gray: &GrayImage) -> GrayImage {
    GrayImage::from_fn(gray.width(), gray.height(), |x, y| {
        image::Luma([if gray.get_pixel(x, y).0[0] < 128 { 0 } else { 255 }])
    })
}

/// Nén zlib (FlateDecode), mức 9.
#[must_use]
pub fn deflate(data: &[u8]) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec_zlib(data, 9)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Luma, Rgb};

    fn page(kind: ImageKind) -> DynamicImage {
        let img = RgbImage::from_fn(400, 300, |x, y| {
            let ink = (x / 7 + y / 9) % 5 == 0;
            match kind {
                ImageKind::Bilevel => {
                    if ink {
                        Rgb([0, 0, 0])
                    } else {
                        Rgb([255, 255, 255])
                    }
                }
                ImageKind::Gray => {
                    let v = (x * 255 / 400) as u8;
                    Rgb([v, v, v])
                }
                ImageKind::Color => {
                    if ink {
                        Rgb([200, 30, 30])
                    } else {
                        Rgb([250, 250, 250])
                    }
                }
            }
        });
        DynamicImage::ImageRgb8(img)
    }

    #[test]
    fn classify_three_kinds() {
        for k in [ImageKind::Bilevel, ImageKind::Gray, ImageKind::Color] {
            assert_eq!(classify(&page(k)), k);
        }
        let g = DynamicImage::ImageLuma8(GrayImage::from_fn(50, 50, |x, _| {
            Luma([if x < 25 { 0 } else { 255 }])
        }));
        assert_eq!(classify(&g), ImageKind::Bilevel);
        let g2 = DynamicImage::ImageLuma8(GrayImage::from_fn(50, 50, |x, _| Luma([(x * 5) as u8])));
        assert_eq!(classify(&g2), ImageKind::Gray);
    }

    #[test]
    fn encode_picks_codec_and_limits_dpi() {
        let bw = encode_page_image(
            &page(ImageKind::Bilevel),
            300.0,
            ColorMode::Auto,
            Quality::Balanced,
        )
        .unwrap();
        assert_eq!(bw.encoding, Encoding::CcittG4);
        assert_eq!((bw.width, bw.height), (400, 300));
        let col =
            encode_page_image(&page(ImageKind::Color), 400.0, ColorMode::Auto, Quality::Balanced).unwrap();
        assert_eq!(col.encoding, Encoding::Jpeg { gray: false });
        assert_eq!((col.width, col.height), (200, 150));
        assert_eq!(&col.data[..2], &[0xFF, 0xD8]);
        // Mã thành phần đã chuẩn hóa 1, 2, 3 (SOF0 + SOS) và ảnh vẫn giải mã đúng màu.
        let sof = col
            .data
            .windows(4)
            .position(|w| w == [0xFF, 0xC0, 0x00, 0x11])
            .unwrap();
        assert_eq!(
            [col.data[sof + 10], col.data[sof + 13], col.data[sof + 16]],
            [1, 2, 3]
        );
        let sos = col.data.windows(2).position(|w| w == [0xFF, 0xDA]).unwrap();
        assert_eq!(
            [col.data[sos + 5], col.data[sos + 7], col.data[sos + 9]],
            [1, 2, 3]
        );
        let back = image::load_from_memory(&col.data).unwrap().to_rgb8();
        let p = back.get_pixel(1, 1).0;
        assert!(p[0] > 150 && p[1] < 80, "{p:?}");
        let gray =
            encode_page_image(&page(ImageKind::Color), 300.0, ColorMode::Gray, Quality::Small).unwrap();
        assert_eq!(gray.encoding, Encoding::Jpeg { gray: true });
        assert!(
            encode_page_image(
                &DynamicImage::new_rgb8(0, 0),
                300.0,
                ColorMode::Auto,
                Quality::High
            )
            .is_err()
        );
        assert!(Quality::from_code("x").is_err());
        for q in [Quality::High, Quality::Balanced, Quality::Small] {
            assert_eq!(Quality::from_code(q.code()).ok(), Some(q));
        }
    }

    #[test]
    fn g4_is_much_smaller_than_jpeg_for_text() {
        let p = page(ImageKind::Bilevel).to_luma8();
        let g4 = encode_g4(&p).unwrap();
        let jpg = encode_jpeg(p.as_raw(), p.width(), p.height(), true, 80).unwrap();
        assert!(g4.len() * 3 < jpg.len(), "g4 {} jpg {}", g4.len(), jpg.len());
        assert_eq!(to_bilevel(&p).get_pixel(0, 0).0[0], 0);
        assert_eq!(
            miniz_oxide::inflate::decompress_to_vec_zlib(&deflate(b"abc")).unwrap(),
            b"abc"
        );
    }
}

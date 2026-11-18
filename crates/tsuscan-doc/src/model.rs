//! Mô hình tài liệu dùng chung cho mọi bộ xuất: trang = ảnh đã xử lý + DPI + kết quả OCR (tùy chọn).

use image::DynamicImage;
use tsuscan_ml::OcrPageResult;

use crate::error::{DocError, Result};

/// DPI mặc định khi không biết (ảnh camera đã nắn về khổ giấy thường ≈ 200-300 DPI).
pub const DEFAULT_DPI: f32 = 300.0;

/// Một trang tài liệu.
#[derive(Debug, Clone)]
pub struct Page {
    /// Ảnh trang đã xử lý (nắn, lọc).
    pub image: DynamicImage,
    /// Độ phân giải để quy ra khổ giấy thật (điểm ảnh / inch).
    pub dpi: f32,
    /// Kết quả OCR với tọa độ theo điểm ảnh của `image` (None = chưa OCR).
    pub ocr: Option<OcrPageResult>,
}

impl Page {
    /// Trang từ ảnh, DPI mặc định, chưa OCR.
    #[must_use]
    pub fn new(image: DynamicImage) -> Self {
        Self {
            image,
            dpi: DEFAULT_DPI,
            ocr: None,
        }
    }

    /// Gắn kết quả OCR.
    #[must_use]
    pub fn with_ocr(mut self, ocr: OcrPageResult) -> Self {
        self.ocr = Some(ocr);
        self
    }

    /// Đặt DPI.
    #[must_use]
    pub fn with_dpi(mut self, dpi: f32) -> Self {
        self.dpi = dpi;
        self
    }

    /// DPI hợp lệ (hữu hạn, 36..=2400), nếu không thì [`DEFAULT_DPI`].
    #[must_use]
    pub fn effective_dpi(&self) -> f32 {
        if self.dpi.is_finite() && (36.0..=2400.0).contains(&self.dpi) {
            self.dpi
        } else {
            DEFAULT_DPI
        }
    }

    /// Kích thước trang theo point PDF (1/72 inch).
    #[must_use]
    pub fn size_pt(&self) -> (f32, f32) {
        let s = 72.0 / self.effective_dpi();
        (self.image.width() as f32 * s, self.image.height() as f32 * s)
    }
}

/// Thời điểm có múi giờ (không phụ thuộc thư viện ngày giờ).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timestamp {
    /// Giây kể từ 1970-01-01T00:00:00Z.
    pub unix: i64,
    /// Lệch múi giờ so với UTC, phút (Việt Nam: +420).
    pub offset_minutes: i16,
}

/// Ngày giờ dương lịch đã quy về múi giờ địa phương.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CivilTime {
    /// Năm.
    pub year: i64,
    /// Tháng 1..=12.
    pub month: u32,
    /// Ngày 1..=31.
    pub day: u32,
    /// Giờ.
    pub hour: u32,
    /// Phút.
    pub minute: u32,
    /// Giây.
    pub second: u32,
}

impl Timestamp {
    /// Bây giờ theo đồng hồ hệ thống, với lệch múi giờ cho trước.
    #[must_use]
    pub fn now(offset_minutes: i16) -> Self {
        let unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0));
        Self { unix, offset_minutes }
    }

    /// Quy ra ngày giờ địa phương (thuật toán days-from-civil của H. Hinnant).
    #[must_use]
    pub fn civil(self) -> CivilTime {
        let local = self.unix + i64::from(self.offset_minutes) * 60;
        let days = local.div_euclid(86_400);
        let secs = local.rem_euclid(86_400);
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let year = yoe + era * 400 + i64::from(month <= 2);
        CivilTime {
            year,
            month,
            day,
            hour: (secs / 3600) as u32,
            minute: (secs % 3600 / 60) as u32,
            second: (secs % 60) as u32,
        }
    }

    fn offset_parts(self) -> (char, u32, u32) {
        let sign = if self.offset_minutes < 0 { '-' } else { '+' };
        let m = u32::from(self.offset_minutes.unsigned_abs());
        (sign, m / 60, m % 60)
    }

    /// Dạng ngày PDF: `D:YYYYMMDDHHmmSS+HH'mm'`.
    #[must_use]
    pub fn pdf_date(self) -> String {
        let c = self.civil();
        let (sign, oh, om) = self.offset_parts();
        format!(
            "D:{:04}{:02}{:02}{:02}{:02}{:02}{sign}{oh:02}'{om:02}'",
            c.year, c.month, c.day, c.hour, c.minute, c.second
        )
    }

    /// Dạng ISO 8601 cho XMP/OOXML: `YYYY-MM-DDTHH:mm:SS+HH:mm`.
    #[must_use]
    pub fn iso8601(self) -> String {
        let c = self.civil();
        let (sign, oh, om) = self.offset_parts();
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{sign}{oh:02}:{om:02}",
            c.year, c.month, c.day, c.hour, c.minute, c.second
        )
    }

    /// Dạng UTC `YYYY-MM-DDTHH:mm:SSZ` (OOXML `dcterms:W3CDTF`).
    #[must_use]
    pub fn iso8601_utc(self) -> String {
        let c = Self {
            unix: self.unix,
            offset_minutes: 0,
        }
        .civil();
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            c.year, c.month, c.day, c.hour, c.minute, c.second
        )
    }
}

/// Thông tin mô tả tài liệu (ghi vào Info + XMP của PDF, docProps của DOCX/XLSX).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Metadata {
    /// Tiêu đề.
    pub title: String,
    /// Tác giả (để trống nếu không muốn lộ tên người quét).
    pub author: String,
    /// Chủ đề.
    pub subject: String,
    /// Từ khóa.
    pub keywords: Vec<String>,
    /// Thời điểm tạo. `None` = không ghi ngày (PDF/A vẫn cần ngày: dùng [`Timestamp::now`] UTC).
    pub created: Option<Timestamp>,
}

/// Tên phần mềm ghi vào Creator/Producer.
pub const PRODUCER: &str = concat!("Tsu Scan ", env!("CARGO_PKG_VERSION"));

/// Tài liệu nhiều trang.
#[derive(Debug, Clone, Default)]
pub struct Document {
    /// Các trang theo thứ tự.
    pub pages: Vec<Page>,
    /// Thông tin mô tả.
    pub meta: Metadata,
}

impl Document {
    /// Tài liệu từ danh sách trang.
    #[must_use]
    pub fn new(pages: Vec<Page>) -> Self {
        Self {
            pages,
            meta: Metadata::default(),
        }
    }

    /// Kiểm tra có ít nhất một trang.
    ///
    /// # Errors
    /// [`DocError::Empty`] khi không có trang.
    pub fn ensure_pages(&self) -> Result<()> {
        if self.pages.is_empty() {
            Err(DocError::Empty)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_known_dates() {
        let c = Timestamp {
            unix: 0,
            offset_minutes: 0,
        }
        .civil();
        assert_eq!((c.year, c.month, c.day, c.hour), (1970, 1, 1, 0));
        let leap = Timestamp {
            unix: 951_782_400, // 2000-02-29T00:00:00Z
            offset_minutes: 0,
        };
        assert_eq!(leap.iso8601_utc(), "2000-02-29T00:00:00Z");
        let neg = Timestamp {
            unix: -1,
            offset_minutes: 0,
        };
        assert_eq!(neg.iso8601(), "1969-12-31T23:59:59+00:00");
        let vn = Timestamp {
            unix: 1_763_487_000, // 2025-11-18T17:30:00Z
            offset_minutes: 420,
        };
        assert_eq!(vn.iso8601(), "2025-11-19T00:30:00+07:00");
        assert_eq!(vn.pdf_date(), "D:20251119003000+07'00'");
        let west = Timestamp {
            unix: 1_763_487_000,
            offset_minutes: -150,
        };
        assert_eq!(west.iso8601(), "2025-11-18T15:00:00-02:30");
    }

    #[test]
    fn page_size_and_dpi_guard() {
        let p = Page::new(DynamicImage::new_rgb8(2480, 3508));
        let (w, h) = p.size_pt();
        assert!((w - 595.2).abs() < 0.1 && (h - 841.92).abs() < 0.1);
        let bad = Page::new(DynamicImage::new_rgb8(10, 10)).with_dpi(f32::NAN);
        assert!((bad.effective_dpi() - DEFAULT_DPI).abs() < f32::EPSILON);
        assert!(Document::default().ensure_pages().is_err());
    }
}

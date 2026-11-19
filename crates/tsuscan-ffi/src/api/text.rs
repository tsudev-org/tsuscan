//! Chuẩn hóa và tô sáng tiếng Việt cho giao diện - dùng ĐÚNG hàm của kho (`tsuscan_vault::normalize`) để kết quả
//! tô sáng khớp với cách FTS5 so khớp (`SEARCH_AND_FILTER.md`).

use tsuscan_vault::normalize::{vi_normalize, vi_normalize_with_map};

/// Một đoạn tô sáng, vị trí tính theo đơn vị UTF-16 (như `String` của Dart).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighlightRange {
    /// Vị trí bắt đầu (UTF-16, bao gồm).
    pub start: u32,
    /// Vị trí kết thúc (UTF-16, không bao gồm).
    pub end: u32,
}

/// Chuẩn hóa tìm kiếm (bỏ dấu, chữ thường, gộp khoảng trắng; "đ" → "d").
#[flutter_rust_bridge::frb(sync)]
#[must_use]
pub fn vi_normalize_text(input: String) -> String {
    vi_normalize(&input)
}

/// Các đoạn trong `text` khớp từng từ của `query` (không dấu, không phân biệt hoa thường), đã gộp chồng lấn.
#[flutter_rust_bridge::frb(sync)]
#[must_use]
pub fn vi_highlight(text: String, query: String) -> Vec<HighlightRange> {
    let (norm, map) = vi_normalize_with_map(&text);
    let norm_chars: Vec<char> = norm.chars().collect();
    let utf16_at = |byte: usize| text.get(..byte).map_or(0, |s| s.encode_utf16().count()) as u32;
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for word in vi_normalize(&query).split(' ').filter(|w| !w.is_empty()) {
        let w: Vec<char> = word.chars().collect();
        let mut i = 0;
        while i + w.len() <= norm_chars.len() {
            if norm_chars[i..i + w.len()] == w[..] {
                ranges.push((i, i + w.len()));
                i += w.len();
            } else {
                i += 1;
            }
        }
    }
    ranges.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (s, e) in ranges {
        match merged.last_mut() {
            Some(last) if s <= last.1 => last.1 = last.1.max(e),
            _ => merged.push((s, e)),
        }
    }
    merged
        .into_iter()
        .map(|(s, e)| {
            let start_byte = map.get(s).copied().unwrap_or(text.len());
            // Kết thúc = đầu ký tự gốc kế tiếp (gồm cả dấu tổ hợp của ký tự cuối), hoặc cuối chuỗi.
            let end_byte = map.get(e).copied().unwrap_or(text.len());
            let end_byte = trim_trailing_space(&text, start_byte, end_byte);
            HighlightRange {
                start: utf16_at(start_byte),
                end: utf16_at(end_byte),
            }
        })
        .collect()
}

fn trim_trailing_space(text: &str, start: usize, end: usize) -> usize {
    text.get(start..end).map_or(end, |s| start + s.trim_end().len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(text: &str, q: &str) -> Vec<String> {
        let units: Vec<u16> = text.encode_utf16().collect();
        vi_highlight(text.into(), q.into())
            .into_iter()
            .map(|r| String::from_utf16_lossy(&units[r.start as usize..r.end as usize]))
            .collect()
    }

    #[test]
    fn go_khong_dau_to_sang_chu_co_dau() {
        assert_eq!(spans("Hợp đồng thuê nhà", "hop dong"), ["Hợp", "đồng"]);
        assert_eq!(spans("Căn cước công dân", "CUOC"), ["cước"]);
    }

    #[test]
    fn dau_to_hop_nfd_van_to_dung_ca_dau() {
        let nfd = "Ho\u{031b}\u{0323}p";
        assert_eq!(spans(nfd, "hop"), [nfd]);
    }

    #[test]
    fn nhieu_lan_xuat_hien_va_chong_lan_duoc_gop() {
        assert_eq!(spans("an an toàn", "an"), ["an", "an", "àn"]);
        assert_eq!(spans("hóa đơn", "hoa hoa don"), ["hóa", "đơn"]);
        assert_eq!(spans("bảng tính", "ang an"), ["ảng"]);
    }

    #[test]
    fn khong_khop_hoac_truy_van_rong_thi_khong_to() {
        assert_eq!(spans("Biên bản", "hợp"), Vec::<String>::new());
        assert_eq!(spans("Biên bản", "   "), Vec::<String>::new());
    }

    #[test]
    fn vi_tri_utf16_dung_voi_ky_tu_ngoai_bmp() {
        assert_eq!(spans("📄 Hóa đơn", "hoa"), ["Hóa"]);
    }

    #[test]
    fn chuan_hoa_khop_ham_cua_kho() {
        assert_eq!(vi_normalize_text("  Đường   Lê Lợi ".into()), "duong le loi");
    }
}

//! Chuẩn hóa tiếng Việt cho tìm kiếm (`.standards/docs/SEARCH_AND_FILTER.md` mục 3). MỌI chỗ cần chuẩn hóa gọi
//! module này, không viết lại.

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

/// Bỏ dấu tiếng Việt, xử lý riêng `đ`/`Đ` (NFD không tách được).
#[must_use]
pub fn vi_remove_diacritics(input: &str) -> String {
    input
        .nfd()
        .filter(|c| !is_combining_mark(*c))
        .map(|c| match c {
            'đ' => 'd',
            'Đ' => 'D',
            _ => c,
        })
        .collect()
}

/// Chuỗi có dấu đã chuẩn hóa để khớp đúng dấu: NFC → chữ thường → gộp khoảng trắng.
#[must_use]
pub fn vi_fold_case(input: &str) -> String {
    let lower: String = input.nfc().collect::<String>().to_lowercase();
    lower.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Chuẩn hóa để so khớp không dấu: NFC → chữ thường → bỏ dấu → gộp khoảng trắng.
#[must_use]
pub fn vi_normalize(input: &str) -> String {
    vi_normalize_with_map(input).0
}

/// Như [`vi_normalize`], kèm vị trí byte trong chuỗi GỐC của từng ký tự kết quả (để tô sáng trên chuỗi gốc).
///
/// `map[i]` là byte bắt đầu (trong `input`) của ký tự gốc sinh ra ký tự thứ `i` (tính theo `char`) của kết quả.
#[must_use]
pub fn vi_normalize_with_map(input: &str) -> (String, Vec<usize>) {
    let mut out = String::with_capacity(input.len());
    let mut map = Vec::with_capacity(input.len());
    let mut pending_space: Option<usize> = None;
    // Duyệt theo cụm "ký tự gốc + dấu tổ hợp theo sau" để dấu rời (NFD) vẫn ánh xạ về đúng chữ.
    let mut chars = input.char_indices().peekable();
    while let Some((start, c)) = chars.next() {
        let mut cluster = String::from(c);
        while let Some(&(_, m)) = chars.peek() {
            if is_combining_mark(m) {
                cluster.push(m);
                chars.next();
            } else {
                break;
            }
        }
        if c.is_whitespace() {
            if !out.is_empty() {
                pending_space = Some(start);
            }
            continue;
        }
        let folded = vi_remove_diacritics(&cluster.nfc().collect::<String>().to_lowercase());
        if folded.is_empty() {
            continue;
        }
        if let Some(sp) = pending_space.take() {
            out.push(' ');
            map.push(sp);
        }
        for f in folded.chars() {
            out.push(f);
            map.push(start);
        }
    }
    (out, map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_diacritics_including_d_stroke() {
        assert_eq!(vi_normalize("  GIÁO   dục\nĐường  "), "giao duc duong");
        assert_eq!(vi_normalize("Thư viện"), "thu vien");
        assert_eq!(vi_remove_diacritics("Đắk Lắk"), "Dak Lak");
    }

    #[test]
    fn nfc_and_nfd_inputs_agree() {
        let nfc = "Hợp đồng thuê nhà";
        let nfd: String = nfc.nfd().collect();
        assert_ne!(nfc, nfd);
        assert_eq!(vi_normalize(nfc), vi_normalize(&nfd));
        assert_eq!(vi_fold_case(&nfd), "hợp đồng thuê nhà");
    }

    #[test]
    fn map_points_back_to_original_bytes() {
        for input in ["Căn cước  CÔNG dân", "Hợp đồng"] {
            let (norm, map) = vi_normalize_with_map(input);
            assert_eq!(norm.chars().count(), map.len());
            // "cong" trong chuỗi chuẩn hóa ⇒ tô sáng "CÔNG" trong chuỗi gốc.
            if let Some(byte_pos) = norm.find("cong") {
                let ci = norm[..byte_pos].chars().count();
                let (s, e) = (map[ci], map[ci + 3]);
                let end = e + input[e..].chars().next().map_or(0, char::len_utf8);
                assert_eq!(&input[s..end], "CÔNG");
            }
        }
        let nfd: String = "Hợp đồng".nfd().collect();
        let (norm, map) = vi_normalize_with_map(&nfd);
        assert_eq!(norm, "hop dong");
        assert_eq!(map.len(), 8);
        assert!(nfd.is_char_boundary(map[5]));
    }
}

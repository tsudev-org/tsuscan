//! Độ đo chất lượng OCR: CER/WER theo khoảng cách Levenshtein trên chuỗi đã chuẩn hóa NFC.

use unicode_normalization::UnicodeNormalization;

/// Chuẩn hóa một dòng: NFC, gộp khoảng trắng liên tiếp thành một, bỏ khoảng trắng đầu/cuối.
#[must_use]
pub fn normalize_line(s: &str) -> String {
    s.nfc()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Chuẩn hóa cả văn bản: từng dòng qua [`normalize_line`], bỏ dòng rỗng, nối bằng `\n`.
#[must_use]
pub fn normalize_text(s: &str) -> String {
    s.lines()
        .map(normalize_line)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Khoảng cách Levenshtein giữa hai dãy.
#[must_use]
pub fn levenshtein<T: PartialEq>(a: &[T], b: &[T]) -> usize {
    if a.is_empty() {
        return b.len();
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for (i, x) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, y) in b.iter().enumerate() {
            let sub = prev[j] + usize::from(x != y);
            cur[j + 1] = sub.min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// Số lỗi ký tự và độ dài chuẩn (để cộng dồn nhiều trang trước khi chia).
#[must_use]
pub fn char_errors(truth: &str, hyp: &str) -> (usize, usize) {
    let t: Vec<char> = normalize_text(truth).chars().collect();
    let h: Vec<char> = normalize_text(hyp).chars().collect();
    (levenshtein(&t, &h), t.len())
}

/// Tỉ lệ lỗi ký tự (CER) = Levenshtein / độ dài chuẩn.
#[must_use]
pub fn cer(truth: &str, hyp: &str) -> f64 {
    let (e, n) = char_errors(truth, hyp);
    if n == 0 {
        if e == 0 { 0.0 } else { 1.0 }
    } else {
        e as f64 / n as f64
    }
}

/// Tỉ lệ lỗi từ (WER).
#[must_use]
pub fn wer(truth: &str, hyp: &str) -> f64 {
    let t = normalize_text(truth);
    let h = normalize_text(hyp);
    let t: Vec<&str> = t.split_whitespace().collect();
    let h: Vec<&str> = h.split_whitespace().collect();
    if t.is_empty() {
        return if h.is_empty() { 0.0 } else { 1.0 };
    }
    levenshtein(&t, &h) as f64 / t.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levenshtein_co_ban() {
        let a: Vec<char> = "kitten".chars().collect();
        let b: Vec<char> = "sitting".chars().collect();
        assert_eq!(levenshtein(&a, &b), 3);
        assert_eq!(levenshtein::<char>(&[], &b), 7);
    }

    #[test]
    fn cer_nfc_va_khoang_trang() {
        // "ệ" dựng sẵn và tổ hợp (e + ̣ + ̂) phải coi là một.
        let nfd = "Vie\u{0323}\u{0302}t  Nam";
        assert!(cer("Việt Nam", nfd).abs() < 1e-9);
        assert!((cer("ab", "ac") - 0.5).abs() < 1e-9);
        assert!((cer("", "x") - 1.0).abs() < 1e-9);
        assert!(cer("", "").abs() < 1e-9);
    }

    #[test]
    fn wer_co_ban() {
        assert!((wer("một hai ba", "một hai bốn") - 1.0 / 3.0).abs() < 1e-9);
        assert!(wer("", "").abs() < 1e-9);
    }
}

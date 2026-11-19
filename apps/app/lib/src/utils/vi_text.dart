// Chuẩn hóa tiếng Việt phía Dart - CHỈ dùng cho kho bộ nhớ (web xem trước, test). Bản thật dùng hàm Rust
// `tsuscan_vault::normalize` để tìm kiếm và tô sáng khớp FTS5 (SEARCH_AND_FILTER.md).

const _groups = <String, String>{
  'a': 'àáảãạăằắẳẵặâầấẩẫậ',
  'e': 'èéẻẽẹêềếểễệ',
  'i': 'ìíỉĩị',
  'o': 'òóỏõọôồốổỗộơờớởỡợ',
  'u': 'ùúủũụưừứửữự',
  'y': 'ỳýỷỹỵ',
  'd': 'đ',
};

final Map<int, String> _fold = () {
  final m = <int, String>{};
  _groups.forEach((base, chars) {
    for (final r in chars.runes) {
      m[r] = base;
    }
  });
  return m;
}();

bool _isCombining(int r) => r >= 0x0300 && r <= 0x036F;

/// Bỏ dấu + chữ thường + gộp khoảng trắng, kèm vị trí (UTF-16) trong chuỗi gốc của từng ký tự kết quả.
(String, List<int>) viNormalizeWithMap(String input) {
  final out = StringBuffer();
  final map = <int>[];
  int? pendingSpace;
  var i = 0;
  final units = input.runes.toList();
  var offset = 0;
  while (i < units.length) {
    final r = units[i];
    final start = offset;
    offset += r > 0xFFFF ? 2 : 1;
    i++;
    while (i < units.length && _isCombining(units[i])) {
      offset += 1;
      i++;
    }
    final ch = String.fromCharCode(r);
    if (ch.trim().isEmpty) {
      if (out.isNotEmpty) pendingSpace = start;
      continue;
    }
    if (pendingSpace != null) {
      out.write(' ');
      map.add(pendingSpace);
      pendingSpace = null;
    }
    final lower = ch.toLowerCase();
    for (final lr in lower.runes) {
      final piece = _fold[lr] ?? String.fromCharCode(lr);
      out.write(piece);
      // Bản đồ theo từng đơn vị UTF-16 của kết quả (String.indexOf của Dart đếm theo UTF-16).
      for (var k = 0; k < piece.length; k++) {
        map.add(start);
      }
    }
  }
  return (out.toString(), map);
}

/// Bỏ dấu + chữ thường + gộp khoảng trắng.
String viNormalize(String input) => viNormalizeWithMap(input).$1;

/// Đoạn tô sáng (UTF-16) của từng từ trong [query] xuất hiện trong [text], đã gộp chồng lấn.
List<(int, int)> viHighlightRanges(String text, String query) {
  final (norm, map) = viNormalizeWithMap(text);
  final found = <(int, int)>[];
  for (final word in viNormalize(query).split(' ').where((w) => w.isNotEmpty)) {
    var from = 0;
    while (true) {
      final at = norm.indexOf(word, from);
      if (at < 0) break;
      found.add((at, at + word.length));
      from = at + word.length;
    }
  }
  found.sort((a, b) => a.$1.compareTo(b.$1));
  final merged = <(int, int)>[];
  for (final r in found) {
    if (merged.isNotEmpty && r.$1 <= merged.last.$2) {
      final last = merged.removeLast();
      merged.add((last.$1, r.$2 > last.$2 ? r.$2 : last.$2));
    } else {
      merged.add(r);
    }
  }
  return [for (final (s, e) in merged) (map[s], _trimEnd(text, map[s], e < map.length ? map[e] : text.length))];
}

int _trimEnd(String text, int start, int end) {
  var e = end;
  while (e > start && text[e - 1].trim().isEmpty) {
    e--;
  }
  return e;
}

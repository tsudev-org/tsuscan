/// Định dạng ngày giờ toàn hệ thống: `HH:mm DD/MM/YYYY` (DESIGN_SYSTEM.md mục 4).
String formatDateTimeVN(DateTime t) {
  String two(int v) => v.toString().padLeft(2, '0');
  return '${two(t.hour)}:${two(t.minute)} ${two(t.day)}/${two(t.month)}/${t.year}';
}

/// Ngày `DD/MM/YYYY`.
String formatDateVN(DateTime t) {
  String two(int v) => v.toString().padLeft(2, '0');
  return '${two(t.day)}/${two(t.month)}/${t.year}';
}

/// Từ ms Unix sang giờ địa phương.
DateTime fromMillis(int ms) => DateTime.fromMillisecondsSinceEpoch(ms);

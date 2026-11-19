// TỆP SINH TỰ ĐỘNG bởi tool/gen_tokens.dart - KHÔNG SỬA TAY.
// Nguồn: .standards/tokens/design-tokens.json v2.0.0

import 'dart:ui' show Color;

/// Bảng màu một chế độ (Sáng/Tối). Chế độ Ấm đã ngừng dùng (DESIGN_SYSTEM.md mục 1).
final class TsPalette {
  const TsPalette({
    required this.bgBase,
    required this.bgSurface,
    required this.bgSubtle,
    required this.bgHover,
    required this.border,
    required this.borderStrong,
    required this.borderControl,
    required this.textPrimary,
    required this.textSecondary,
    required this.textMuted,
    required this.textLink,
    required this.primary,
    required this.primaryHover,
    required this.primaryActive,
    required this.onPrimary,
    required this.focusRing,
    required this.success,
    required this.warning,
    required this.danger,
    required this.info,
    required this.onStatus,
    required this.overlay,
  });
  final Color bgBase;
  final Color bgSurface;
  final Color bgSubtle;
  final Color bgHover;
  final Color border;
  final Color borderStrong;
  final Color borderControl;
  final Color textPrimary;
  final Color textSecondary;
  final Color textMuted;
  final Color textLink;
  final Color primary;
  final Color primaryHover;
  final Color primaryActive;
  final Color onPrimary;
  final Color focusRing;
  final Color success;
  final Color warning;
  final Color danger;
  final Color info;
  final Color onStatus;
  final Color overlay;
  static const light = TsPalette(
    bgBase: Color(0xFFEEF4FB),
    bgSurface: Color(0xFFFFFFFF),
    bgSubtle: Color(0xFFE3EDF8),
    bgHover: Color(0xFFDCE8F6),
    border: Color(0xFFC9D9EC),
    borderStrong: Color(0xFF9FB8D4),
    borderControl: Color(0xFF74899F),
    textPrimary: Color(0xFF1C2B3A),
    textSecondary: Color(0xFF44607C),
    textMuted: Color(0xFF52627A),
    textLink: Color(0xFF1D4ED8),
    primary: Color(0xFF2563EB),
    primaryHover: Color(0xFF1D4ED8),
    primaryActive: Color(0xFF1E40AF),
    onPrimary: Color(0xFFFFFFFF),
    focusRing: Color(0xFF2563EB),
    success: Color(0xFF15803D),
    warning: Color(0xFFB45309),
    danger: Color(0xFFDC2626),
    info: Color(0xFF0369A1),
    onStatus: Color(0xFFFFFFFF),
    overlay: Color(0x730F1B2D),
  );
  static const dark = TsPalette(
    bgBase: Color(0xFF0F1B2D),
    bgSurface: Color(0xFF16263C),
    bgSubtle: Color(0xFF1C2F4A),
    bgHover: Color(0xFF223A5A),
    border: Color(0xFF2C4363),
    borderStrong: Color(0xFF3F5B80),
    borderControl: Color(0xFF6E88AE),
    textPrimary: Color(0xFFE6EDF5),
    textSecondary: Color(0xFFA9BBD0),
    textMuted: Color(0xFF9BB0C9),
    textLink: Color(0xFF7FB2F7),
    primary: Color(0xFF66A3F2),
    primaryHover: Color(0xFF7FB2F7),
    primaryActive: Color(0xFF4E8FE6),
    onPrimary: Color(0xFF0C1930),
    focusRing: Color(0xFF66A3F2),
    success: Color(0xFF4ADE80),
    warning: Color(0xFFFBBF24),
    danger: Color(0xFFF87171),
    info: Color(0xFF38BDF8),
    onStatus: Color(0xFF0C1930),
    overlay: Color(0x99000000),
  );
}

/// Bo góc (px).
abstract final class TsRadius {
  static const double none = 0.0;
  static const double sm = 4.0;
  static const double md = 6.0;
  static const double lg = 8.0;
}

/// Khoảng cách bội số 4px.
abstract final class TsSpace {
  static const double s1 = 4.0;
  static const double s2 = 8.0;
  static const double s3 = 12.0;
  static const double s4 = 16.0;
  static const double s5 = 20.0;
  static const double s6 = 24.0;
  static const double s8 = 32.0;
  static const double s10 = 40.0;
  static const double s12 = 48.0;
}

/// Cỡ chữ (px).
abstract final class TsFontSize {
  static const double xs = 12.0;
  static const double sm = 13.0;
  static const double bodyDesktop = 14.0;
  static const double bodyDesktopLg = 15.0;
  static const double bodyWeb = 15.0;
  static const double bodyWebLg = 16.0;
  static const double h4 = 18.0;
  static const double h3 = 20.0;
  static const double h2 = 24.0;
  static const double h1 = 30.0;
}

/// Thời lượng chuyển động (ms).
abstract final class TsMotion {
  static const double fast = 120.0;
  static const double base = 180.0;
  static const double slow = 260.0;
}

/// Chiều cao dòng (hệ số).
abstract final class TsLineHeight {
  static const double heading = 1.3;
  static const double body = 1.55;
  static const double long = 1.6;
}

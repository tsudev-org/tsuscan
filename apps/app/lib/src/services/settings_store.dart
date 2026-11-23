import 'package:flutter/material.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../ui/theme/app_theme.dart';

/// Cài đặt không bí mật (giao diện, ngôn ngữ, bật khóa app). Lưu bằng SharedPreferences.
class SettingsStore extends ChangeNotifier {
  SettingsStore(this._prefs)
    : _theme = ThemeChoice.parse(_prefs.getString(_kTheme)),
      _localeCode = _prefs.getString(_kLocale),
      _lockEnabled = _prefs.getBool(_kLock) ?? false,
      _biometricEnabled = _prefs.getBool(_kBiometric) ?? false;

  static const _kTheme = 'theme';
  static const _kLocale = 'locale';
  static const _kLock = 'app_lock';
  static const _kBiometric = 'app_lock_biometric';
  static const _kScanMode = 'scan_mode';
  static const supportedLocales = [Locale('vi'), Locale('en')];

  final SharedPreferences _prefs;
  ThemeChoice _theme;
  String? _localeCode;
  bool _lockEnabled;
  bool _biometricEnabled;

  ThemeChoice get theme => _theme;

  /// `null` = theo hệ thống.
  Locale? get locale => _localeCode == null ? null : Locale(_localeCode!);
  bool get lockEnabled => _lockEnabled;
  bool get biometricEnabled => _biometricEnabled;

  /// Mã chế độ quét dùng lần trước (`ScanMode.name`; tầng quét tự kiểm hợp lệ).
  String? get scanMode => _prefs.getString(_kScanMode);

  Future<void> setScanMode(String v) => _prefs.setString(_kScanMode, v);

  /// Mốc bộ chọn số bản ghi (`DATA_TABLE.md` mục 2).
  static const pageSizes = [10, 20, 50, 100, 200];

  /// Số bản ghi mỗi trang đã nhớ cho đúng bảng [tableId] (`tsudev.pagesize.<mã-bảng>`); sai mốc ⇒ 10.
  int pageSize(String tableId) {
    final v = _prefs.getInt('tsudev.pagesize.$tableId');
    return pageSizes.contains(v) ? v! : 10;
  }

  Future<void> setPageSize(String tableId, int v) async {
    if (!pageSizes.contains(v)) return;
    await _prefs.setInt('tsudev.pagesize.$tableId', v);
  }

  Future<void> setTheme(ThemeChoice v) async {
    _theme = v;
    notifyListeners();
    await _prefs.setString(_kTheme, v.name);
  }

  Future<void> setLocale(Locale? v) async {
    _localeCode = v?.languageCode;
    notifyListeners();
    if (v == null) {
      await _prefs.remove(_kLocale);
    } else {
      await _prefs.setString(_kLocale, v.languageCode);
    }
  }

  Future<void> setLockEnabled(bool v) async {
    _lockEnabled = v;
    if (!v) _biometricEnabled = false;
    notifyListeners();
    await _prefs.setBool(_kLock, v);
    await _prefs.setBool(_kBiometric, _biometricEnabled);
  }

  Future<void> setBiometricEnabled(bool v) async {
    _biometricEnabled = v;
    notifyListeners();
    await _prefs.setBool(_kBiometric, v);
  }
}

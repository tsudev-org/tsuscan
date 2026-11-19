import 'package:flutter/foundation.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';

/// Kho bí mật của thiết bị (Android Keystore, Keychain, DPAPI, libsecret). Web KHÔNG có kho an toàn.
abstract interface class KeyStorage {
  Future<String?> read(String key);
  Future<void> write(String key, String value);
  Future<void> delete(String key);
}

final class SecureKeyStorage implements KeyStorage {
  SecureKeyStorage()
    : _s = const FlutterSecureStorage(
        aOptions: AndroidOptions(),
        // Không đồng bộ iCloud, chỉ đọc được sau lần mở khóa đầu tiên của máy.
        iOptions: IOSOptions(accessibility: KeychainAccessibility.first_unlock_this_device),
        // macOS: keychain đăng nhập (không cần entitlement Keychain Sharing / provisioning profile).
        mOptions: MacOsOptions(usesDataProtectionKeychain: false),
      );
  final FlutterSecureStorage _s;

  @override
  Future<String?> read(String key) => _s.read(key: key);

  @override
  Future<void> write(String key, String value) => _s.write(key: key, value: value);

  @override
  Future<void> delete(String key) => _s.delete(key: key);
}

/// Dùng cho test và web xem trước (không lưu bền).
final class MemoryKeyStorage implements KeyStorage {
  final Map<String, String> values = {};

  @override
  Future<String?> read(String key) async => values[key];

  @override
  Future<void> write(String key, String value) async => values[key] = value;

  @override
  Future<void> delete(String key) async => values.remove(key);
}

/// Tên khóa trong kho bí mật.
abstract final class SecretKeys {
  /// Khóa CSDL 32 byte (base64) - ADR-0010.
  static const dbKey = 'tsuscan.db_key.v1';

  /// Bộ kiểm PIN khóa app (`pin1$...`).
  static const pinVerifier = 'tsuscan.pin.v1';

  /// Số lần nhập PIN sai liên tiếp + thời điểm được thử lại (ms) - để trong kho bí mật để không xóa được bằng cách
  /// xóa tệp cài đặt thường.
  static const pinFailures = 'tsuscan.pin_failures.v1';
  static const pinLockedUntil = 'tsuscan.pin_locked_until.v1';
}

/// Web không có kho bí mật đáng tin.
bool get secureStorageSupported => !kIsWeb;

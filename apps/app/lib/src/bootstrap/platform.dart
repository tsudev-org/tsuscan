import '../data/library_repository.dart';
import '../data/scan_engine.dart';
import '../services/app_lock.dart';
import '../services/key_storage.dart';
import 'platform_native.dart' if (dart.library.js_interop) 'platform_web.dart' as impl;

/// Kết quả mở kho lúc khởi động.
sealed class LibraryOpenResult {
  const LibraryOpenResult();
}

final class LibraryReady extends LibraryOpenResult {
  const LibraryReady(this.repository);
  final LibraryRepository repository;
}

/// Có kho trên đĩa nhưng kho bí mật của máy không còn khóa (khôi phục máy, gỡ cài đặt dở).
final class LibraryKeyMissing extends LibraryOpenResult {
  const LibraryKeyMissing();
}

final class LibraryFailed extends LibraryOpenResult {
  const LibraryFailed(this.error);
  final Object error;
}

/// Mọi thứ phụ thuộc nền tảng mà giao diện cần.
abstract interface class PlatformBindings {
  KeyStorage get storage;
  PinCrypto get pin;
  BiometricAuth get biometric;
  String get coreVersion;

  /// Dò mép, nắn, lọc ảnh quét.
  ScanEngine get scan;

  /// Mở kho có sẵn hoặc tạo kho mới ở lần chạy đầu.
  Future<LibraryOpenResult> openLibrary();

  /// Xóa hẳn kho cũ (không đọc được nữa) và tạo kho mới.
  Future<LibraryOpenResult> resetLibrary();
}

/// Nạp lõi Rust (native) hoặc bản xem trước bộ nhớ (web).
Future<PlatformBindings> loadPlatform() => impl.loadPlatform();

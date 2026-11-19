import '../data/memory_library.dart';
import '../data/scan_engine.dart';
import '../services/app_lock.dart';
import '../services/key_storage.dart';
import 'platform.dart';

/// Web xem trước: chưa có lõi WASM (P11) ⇒ kho bộ nhớ, không khóa app, không sinh trắc.
Future<PlatformBindings> loadPlatform() async => _WebBindings();

final class _WebBindings implements PlatformBindings {
  @override
  final KeyStorage storage = MemoryKeyStorage();
  @override
  final PinCrypto pin = const _NoPin();
  @override
  final BiometricAuth biometric = const _NoBiometric();
  @override
  String get coreVersion => 'web-preview';
  @override
  final ScanEngine scan = const MemoryScanEngine();

  @override
  Future<LibraryOpenResult> openLibrary() async => LibraryReady(MemoryLibraryRepository());

  @override
  Future<LibraryOpenResult> resetLibrary() => openLibrary();
}

final class _NoPin implements PinCrypto {
  const _NoPin();
  @override
  Future<String> createVerifier(String pin) async => throw UnsupportedError('web: không có khóa app');
  @override
  Future<bool> verify(String pin, String verifier) async => false;
  @override
  int lockoutSeconds(int failures) => 0;
}

final class _NoBiometric implements BiometricAuth {
  const _NoBiometric();
  @override
  Future<bool> available() async => false;
  @override
  Future<bool> authenticate(String reason) async => false;
}

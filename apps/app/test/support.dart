import 'package:shared_preferences/shared_preferences.dart';
import 'package:tsuscan/src/bootstrap/platform.dart';
import 'package:tsuscan/src/data/memory_library.dart';
import 'package:tsuscan/src/data/scan_engine.dart';
import 'package:tsuscan/src/services/app_lock.dart';
import 'package:tsuscan/src/services/key_storage.dart';
import 'package:tsuscan/src/services/settings_store.dart';

/// PIN giả: bộ kiểm = `fake$<pin>` (logic mật mã thật được test trong crate Rust).
final class FakePin implements PinCrypto {
  @override
  Future<String> createVerifier(String pin) async => 'fake\$$pin';
  @override
  Future<bool> verify(String pin, String verifier) async => verifier == 'fake\$$pin';
  @override
  int lockoutSeconds(int failures) => failures < 5 ? 0 : 30;
}

final class FakeBiometric implements BiometricAuth {
  FakeBiometric({this.ok = false});
  bool ok;
  int calls = 0;
  @override
  Future<bool> available() async => ok;
  @override
  Future<bool> authenticate(String reason) async {
    calls++;
    return ok;
  }
}

/// Nền tảng giả: kho bộ nhớ nhưng báo "persistent" để màn cài đặt hiện phần khóa app.
final class FakePlatform implements PlatformBindings {
  FakePlatform({MemoryLibraryRepository? repo}) : repo = repo ?? MemoryLibraryRepository(persistent: true);
  final MemoryLibraryRepository repo;
  @override
  final KeyStorage storage = MemoryKeyStorage();
  @override
  final PinCrypto pin = FakePin();
  @override
  final FakeBiometric biometric = FakeBiometric();
  @override
  String get coreVersion => '0.1.0';
  @override
  ScanEngine scan = const MemoryScanEngine();
  LibraryOpenResult result = const LibraryKeyMissing();
  bool useResult = false;
  int resets = 0;

  @override
  Future<LibraryOpenResult> openLibrary() async => useResult ? result : LibraryReady(repo);

  @override
  Future<LibraryOpenResult> resetLibrary() async {
    resets++;
    useResult = false;
    return LibraryReady(repo);
  }
}

Future<SettingsStore> freshSettings([Map<String, Object> values = const {}]) async {
  SharedPreferences.setMockInitialValues(values);
  return SettingsStore(await SharedPreferences.getInstance());
}

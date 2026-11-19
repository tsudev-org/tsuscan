import 'dart:convert';
import 'dart:io';

import 'package:flutter/services.dart';
import 'package:local_auth/local_auth.dart';
import 'package:path_provider/path_provider.dart';

import '../bridge/generated/api.dart' as api;
import '../bridge/generated/api/library.dart' show LibraryHandle, generateDbKey, libraryExists;
import '../bridge/generated/api/pin.dart' as pin_api;
import '../bridge/generated/frb_generated.dart';
import '../data/library_repository.dart';
import '../services/app_lock.dart';
import '../services/key_storage.dart';
import 'platform.dart';

Future<PlatformBindings> loadPlatform() async {
  await RustLib.init();
  final support = await getApplicationSupportDirectory();
  return _NativeBindings(SecureKeyStorage(), '${support.path}${Platform.pathSeparator}library');
}

final class _NativeBindings implements PlatformBindings {
  _NativeBindings(this.storage, this._root);

  @override
  final KeyStorage storage;
  final String _root;
  @override
  final PinCrypto pin = const _RustPin();
  @override
  final BiometricAuth biometric = _LocalBiometric();
  @override
  String get coreVersion => api.coreVersion();

  @override
  Future<LibraryOpenResult> openLibrary() async {
    try {
      final stored = await storage.read(SecretKeys.dbKey);
      if (libraryExists(root: _root)) {
        if (stored == null) return const LibraryKeyMissing();
        final h = await LibraryHandle.open(root: _root, dbKey: base64Decode(stored));
        return LibraryReady(RustLibraryRepository(h));
      }
      return await _create();
    } on ApiError catch (e) {
      return e.kind == ApiErrorKind.wrongKey ? const LibraryKeyMissing() : LibraryFailed(e);
    } on Object catch (e) {
      return LibraryFailed(e);
    }
  }

  Future<LibraryOpenResult> _create() async {
    // Ghi khóa vào kho bí mật TRƯỚC khi tạo kho: nếu ghi lỗi thì chưa có dữ liệu nào bị mã hóa bằng khóa mất.
    final key = await generateDbKey();
    await storage.write(SecretKeys.dbKey, base64Encode(key));
    key.fillRange(0, key.length, 0);
    final stored = await storage.read(SecretKeys.dbKey);
    if (stored == null) throw StateError('kho bí mật không lưu được khóa');
    final h = await LibraryHandle.create(root: _root, dbKey: base64Decode(stored));
    return LibraryReady(RustLibraryRepository(h));
  }

  @override
  Future<LibraryOpenResult> resetLibrary() async {
    try {
      final dir = Directory(_root);
      if (dir.existsSync()) await dir.delete(recursive: true);
      await storage.delete(SecretKeys.dbKey);
      return await _create();
    } on Object catch (e) {
      return LibraryFailed(e);
    }
  }
}

final class _RustPin implements PinCrypto {
  const _RustPin();
  @override
  Future<String> createVerifier(String pin) => pin_api.pinCreateVerifier(pin: pin);
  @override
  Future<bool> verify(String pin, String verifier) async {
    try {
      return await pin_api.pinVerify(pin: pin, verifier: verifier);
    } on ApiError {
      return false;
    }
  }

  @override
  int lockoutSeconds(int failures) => pin_api.pinLockoutSeconds(failures: failures);
}

final class _LocalBiometric implements BiometricAuth {
  final _auth = LocalAuthentication();

  // local_auth chưa có bản Linux.
  bool get _platformHasPlugin => !Platform.isLinux;

  @override
  Future<bool> available() async {
    if (!_platformHasPlugin) return false;
    try {
      return await _auth.isDeviceSupported() && (await _auth.getAvailableBiometrics()).isNotEmpty;
    } on PlatformException {
      return false;
    }
  }

  @override
  Future<bool> authenticate(String reason) async {
    if (!_platformHasPlugin) return false;
    try {
      return await _auth.authenticate(localizedReason: reason, biometricOnly: true);
    } on PlatformException {
      return false;
    } on LocalAuthException {
      return false;
    }
  }
}

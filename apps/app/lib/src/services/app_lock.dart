import 'package:flutter/widgets.dart';

import 'key_storage.dart';
import 'settings_store.dart';

/// Hàm mật mã PIN (bản thật gọi lõi Rust; test truyền bản giả).
abstract interface class PinCrypto {
  Future<String> createVerifier(String pin);
  Future<bool> verify(String pin, String verifier);
  int lockoutSeconds(int failures);
}

/// Sinh trắc học của hệ điều hành.
abstract interface class BiometricAuth {
  Future<bool> available();
  Future<bool> authenticate(String reason);
}

/// Kết quả một lần nhập PIN.
sealed class PinAttempt {
  const PinAttempt();
}

final class PinAccepted extends PinAttempt {
  const PinAccepted();
}

final class PinRejected extends PinAttempt {
  const PinRejected(this.failures, this.waitSeconds);
  final int failures;
  final int waitSeconds;
}

final class PinLockedOut extends PinAttempt {
  const PinLockedOut(this.waitSeconds);
  final int waitSeconds;
}

/// Khóa app bằng PIN/sinh trắc. Khóa khi mở app và khi quay lại sau [backgroundGrace] ở nền.
class AppLockController extends ChangeNotifier with WidgetsBindingObserver {
  AppLockController({
    required this.settings,
    required this.storage,
    required this.crypto,
    required this.biometric,
    DateTime Function()? clock,
    this.backgroundGrace = const Duration(minutes: 1),
  }) : _clock = clock ?? DateTime.now {
    _locked = settings.lockEnabled;
  }

  final SettingsStore settings;
  final KeyStorage storage;
  final PinCrypto crypto;
  final BiometricAuth biometric;
  final Duration backgroundGrace;
  final DateTime Function() _clock;
  bool _locked = false;
  DateTime? _pausedAt;

  bool get locked => _locked && settings.lockEnabled;

  void attach() => WidgetsBinding.instance.addObserver(this);

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) => onLifecycle(state);

  @visibleForTesting
  void onLifecycle(AppLifecycleState state) {
    if (!settings.lockEnabled) return;
    if (state == AppLifecycleState.paused || state == AppLifecycleState.hidden) {
      _pausedAt ??= _clock();
    } else if (state == AppLifecycleState.resumed) {
      final at = _pausedAt;
      _pausedAt = null;
      if (at != null && _clock().difference(at) >= backgroundGrace) lock();
    }
  }

  void lock() {
    if (!settings.lockEnabled || _locked) return;
    _locked = true;
    notifyListeners();
  }

  Future<int> _failures() async => int.tryParse(await storage.read(SecretKeys.pinFailures) ?? '') ?? 0;

  /// Số giây còn phải chờ trước khi được thử PIN tiếp.
  Future<int> remainingLockout() async {
    final until = int.tryParse(await storage.read(SecretKeys.pinLockedUntil) ?? '') ?? 0;
    final left = until - _clock().millisecondsSinceEpoch;
    return left <= 0 ? 0 : (left / 1000).ceil();
  }

  Future<PinAttempt> unlockWithPin(String pin) async {
    final wait = await remainingLockout();
    if (wait > 0) return PinLockedOut(wait);
    final verifier = await storage.read(SecretKeys.pinVerifier);
    if (verifier != null && await crypto.verify(pin, verifier)) {
      await _resetFailures();
      _unlock();
      return const PinAccepted();
    }
    final failures = await _failures() + 1;
    final waitNext = crypto.lockoutSeconds(failures);
    await storage.write(SecretKeys.pinFailures, '$failures');
    if (waitNext > 0) {
      final until = _clock().millisecondsSinceEpoch + waitNext * 1000;
      await storage.write(SecretKeys.pinLockedUntil, '$until');
    }
    return PinRejected(failures, waitNext);
  }

  Future<bool> unlockWithBiometric(String reason) async {
    if (!settings.biometricEnabled) return false;
    if (!await biometric.authenticate(reason)) return false;
    await _resetFailures();
    _unlock();
    return true;
  }

  /// Bật khóa với PIN mới (hoặc đổi PIN).
  Future<void> setPin(String pin) async {
    await storage.write(SecretKeys.pinVerifier, await crypto.createVerifier(pin));
    await _resetFailures();
    await settings.setLockEnabled(true);
  }

  Future<void> disable() async {
    await storage.delete(SecretKeys.pinVerifier);
    await _resetFailures();
    await settings.setLockEnabled(false);
    _unlock();
  }

  Future<void> _resetFailures() async {
    await storage.delete(SecretKeys.pinFailures);
    await storage.delete(SecretKeys.pinLockedUntil);
  }

  void _unlock() {
    if (!_locked) return;
    _locked = false;
    notifyListeners();
  }
}

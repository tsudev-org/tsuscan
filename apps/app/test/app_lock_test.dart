import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:tsuscan/src/services/app_lock.dart';
import 'package:tsuscan/src/services/key_storage.dart';

import 'support.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  late DateTime now;
  late MemoryKeyStorage storage;
  late FakeBiometric bio;

  Future<AppLockController> make({bool enabled = true}) async {
    final settings = await freshSettings({'app_lock': enabled});
    return AppLockController(settings: settings, storage: storage, crypto: FakePin(), biometric: bio, clock: () => now);
  }

  setUp(() {
    now = DateTime(2026, 11, 19, 8);
    storage = MemoryKeyStorage();
    bio = FakeBiometric();
  });

  test('bật khóa thì mở app ở trạng thái khóa; PIN đúng mở khóa', () async {
    final c = await make();
    await c.setPin('2468');
    expect(c.locked, isTrue);
    expect(await c.unlockWithPin('2468'), isA<PinAccepted>());
    expect(c.locked, isFalse);
  });

  test('sai 5 lần thì phải chờ, hết hạn chờ mới được thử tiếp', () async {
    final c = await make();
    await c.setPin('2468');
    for (var i = 1; i <= 4; i++) {
      expect(await c.unlockWithPin('0000'), isA<PinRejected>().having((r) => r.waitSeconds, 'wait', 0));
    }
    expect(await c.unlockWithPin('0000'), isA<PinRejected>().having((r) => r.waitSeconds, 'wait', 30));
    expect(await c.unlockWithPin('2468'), isA<PinLockedOut>());
    expect(c.locked, isTrue);
    now = now.add(const Duration(seconds: 31));
    expect(await c.unlockWithPin('2468'), isA<PinAccepted>());
    expect(await storage.read(SecretKeys.pinFailures), isNull);
  });

  test('quay lại sau hơn 1 phút ở nền thì khóa, dưới 1 phút thì không', () async {
    final c = await make();
    await c.setPin('2468');
    await c.unlockWithPin('2468');
    c.onLifecycle(AppLifecycleState.paused);
    now = now.add(const Duration(seconds: 30));
    c.onLifecycle(AppLifecycleState.resumed);
    expect(c.locked, isFalse);
    c.onLifecycle(AppLifecycleState.paused);
    now = now.add(const Duration(seconds: 61));
    c.onLifecycle(AppLifecycleState.resumed);
    expect(c.locked, isTrue);
  });

  test('sinh trắc chỉ dùng được khi đã bật trong cài đặt', () async {
    final c = await make();
    await c.setPin('2468');
    bio.ok = true;
    expect(await c.unlockWithBiometric('x'), isFalse);
    expect(bio.calls, 0);
    await c.settings.setBiometricEnabled(true);
    expect(await c.unlockWithBiometric('x'), isTrue);
    expect(c.locked, isFalse);
  });

  test('tắt khóa xóa bộ kiểm PIN và mở khóa', () async {
    final c = await make();
    await c.setPin('2468');
    await c.disable();
    expect(c.locked, isFalse);
    expect(await storage.read(SecretKeys.pinVerifier), isNull);
    expect(c.settings.lockEnabled, isFalse);
  });
}

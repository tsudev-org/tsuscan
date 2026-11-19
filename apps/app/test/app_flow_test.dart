import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:tsuscan/src/app.dart';
import 'package:tsuscan/src/services/app_lock.dart';
import 'package:tsuscan/src/services/key_storage.dart';

import 'support.dart';

Future<FakePlatform> pumpApp(WidgetTester tester, {Map<String, Object> prefs = const {}, Size? size}) async {
  if (size != null) {
    tester.view.physicalSize = size;
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
  }
  final platform = FakePlatform();
  final settings = await freshSettings({'locale': 'vi', ...prefs});
  await tester.pumpWidget(TsuScanApp(platform: platform, settings: settings));
  await tester.pumpAndSettle();
  return platform;
}

void main() {
  testWidgets('lần đầu: thư viện rỗng, tạo tài liệu mới rồi thấy trong danh sách', (tester) async {
    await pumpApp(tester);
    expect(find.text('Chưa có tài liệu nào'), findsOneWidget);

    await tester.tap(find.widgetWithText(FloatingActionButton, 'Tài liệu mới'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), 'Hợp đồng thuê nhà');
    await tester.tap(find.widgetWithText(FilledButton, 'Lưu'));
    await tester.pumpAndSettle();

    // Mở thẳng màn tài liệu vừa tạo.
    expect(find.text('Tài liệu chưa có trang. Thêm ảnh để bắt đầu.'), findsOneWidget);
    await tester.tap(find.byType(BackButton));
    await tester.pumpAndSettle();
    expect(find.text('Hợp đồng thuê nhà'), findsOneWidget);
    expect(find.text('Chưa có tài liệu nào'), findsNothing);
  });

  testWidgets('tìm không dấu ra tài liệu có dấu', (tester) async {
    final platform = await pumpApp(tester);
    await platform.repo.createDocument('Căn cước công dân');
    await tester.tap(find.text('Tìm kiếm').last);
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), 'can cuoc');
    await tester.pump(const Duration(milliseconds: 300));
    await tester.pumpAndSettle();
    expect(find.textContaining('Căn cước', findRichText: true), findsOneWidget);
  });

  testWidgets('đổi ngôn ngữ sang English cập nhật thanh điều hướng ngay', (tester) async {
    await pumpApp(tester);
    await tester.tap(find.text('Cài đặt').last);
    await tester.pumpAndSettle();
    await tester.tap(find.text('English'));
    await tester.pumpAndSettle();
    expect(find.text('Settings'), findsWidgets);
    expect(find.text('Documents'), findsOneWidget);
  });

  testWidgets('bật khóa PIN: khóa lại thì phải nhập đúng PIN mới vào được', (tester) async {
    final platform = FakePlatform();
    final settings = await freshSettings({'locale': 'vi'});
    final lock = AppLockController(
      settings: settings,
      storage: platform.storage,
      crypto: platform.pin,
      biometric: platform.biometric,
    );
    await tester.pumpWidget(TsuScanApp(platform: platform, settings: settings, lock: lock));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Cài đặt').last);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Khóa ứng dụng bằng mã PIN'));
    await tester.pumpAndSettle();
    for (var i = 0; i < 2; i++) {
      await tester.enterText(find.byKey(const ValueKey('pin-setup-field')), '2468');
      await tester.tap(find.widgetWithText(FilledButton, 'Lưu'));
      await tester.pumpAndSettle();
    }
    expect(await platform.storage.read(SecretKeys.pinVerifier), 'fake\$2468');

    lock.lock();
    await tester.pumpAndSettle();
    expect(find.text('Tsu Scan đang khóa'), findsOneWidget);
    await tester.enterText(find.byKey(const ValueKey('lock-pin-field')), '1111');
    await tester.tap(find.widgetWithText(FilledButton, 'Mở khóa'));
    await tester.pumpAndSettle();
    expect(find.text('Sai mã PIN (1 lần)'), findsOneWidget);
    await tester.enterText(find.byKey(const ValueKey('lock-pin-field')), '2468');
    await tester.tap(find.widgetWithText(FilledButton, 'Mở khóa'));
    await tester.pumpAndSettle();
    expect(find.text('Tsu Scan đang khóa'), findsNothing);
  });

  testWidgets('kho bí mật mất khóa: báo lỗi và cho tạo kho mới sau khi xác nhận', (tester) async {
    final platform = FakePlatform()..useResult = true;
    final settings = await freshSettings({'locale': 'vi'});
    await tester.pumpWidget(TsuScanApp(platform: platform, settings: settings));
    await tester.pumpAndSettle();
    expect(find.text('Không mở được kho tài liệu'), findsOneWidget);
    await tester.tap(find.widgetWithText(FilledButton, 'Tạo kho mới (xóa dữ liệu cũ)'));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, 'Tạo kho mới (xóa dữ liệu cũ)').last);
    await tester.pumpAndSettle();
    expect(platform.resets, 1);
    expect(find.text('Chưa có tài liệu nào'), findsOneWidget);
  });

  testWidgets('chân vùng bản ghi: 25 tài liệu, mặc định 10/trang, sang trang 3 còn 5', (tester) async {
    final platform = FakePlatform();
    for (var i = 0; i < 25; i++) {
      await platform.repo.createDocument('Tài liệu ${i.toString().padLeft(2, '0')}');
    }
    final settings = await freshSettings({'locale': 'vi'});
    tester.view.physicalSize = const Size(1280, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(TsuScanApp(platform: platform, settings: settings));
    await tester.pumpAndSettle();
    expect(find.text('1-10 / 25'), findsOneWidget);
    await tester.tap(find.text('3'));
    await tester.pumpAndSettle();
    expect(find.text('21-25 / 25'), findsOneWidget);
  });
}

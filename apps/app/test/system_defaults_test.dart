// PRODUCT_POLICY.md mục 1.4 (phiếu T3): lần chạy đầu theo ngôn ngữ + giao diện của hệ thống.

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:tsuscan/src/app.dart';
import 'package:tsuscan/src/ui/theme/app_theme.dart';

import 'support.dart';

void main() {
  for (final (system, expected) in [('vi', 'Tài liệu'), ('en', 'Documents'), ('ja', 'Documents')]) {
    testWidgets('ngôn ngữ hệ thống $system ⇒ giao diện ${expected == 'Tài liệu' ? 'vi' : 'en'}', (tester) async {
      tester.platformDispatcher.localesTestValue = [Locale(system)];
      addTearDown(tester.platformDispatcher.clearLocalesTestValue);
      final settings = await freshSettings();
      await tester.pumpWidget(TsuScanApp(platform: FakePlatform(), settings: settings));
      await tester.pumpAndSettle();
      expect(find.text(expected), findsWidgets);
    });
  }

  for (final (brightness, palette) in [(Brightness.light, TsPalette.light), (Brightness.dark, TsPalette.dark)]) {
    testWidgets('giao diện hệ thống ${brightness.name} ⇒ đúng bảng màu', (tester) async {
      tester.platformDispatcher.platformBrightnessTestValue = brightness;
      addTearDown(tester.platformDispatcher.clearPlatformBrightnessTestValue);
      final settings = await freshSettings();
      await tester.pumpWidget(TsuScanApp(platform: FakePlatform(), settings: settings));
      await tester.pumpAndSettle();
      final ctx = tester.element(find.byType(Scaffold).first);
      expect(Theme.of(ctx).scaffoldBackgroundColor, palette.bgBase);
    });
  }

  test('lựa chọn đã lưu thắng mặc định hệ thống', () async {
    final settings = await freshSettings({'theme': 'dark', 'locale': 'vi'});
    expect(settings.theme, ThemeChoice.dark);
    expect(settings.locale, const Locale('vi'));
    expect(resolveAppLocale(const Locale('fr')), const Locale('en'));
  });
}

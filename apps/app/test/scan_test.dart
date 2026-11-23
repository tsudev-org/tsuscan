import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:tsuscan/l10n/app_localizations.dart';
import 'package:tsuscan/src/app.dart';
import 'package:tsuscan/src/app_scope.dart';
import 'package:tsuscan/src/data/memory_library.dart';
import 'package:tsuscan/src/data/scan_engine.dart';
import 'package:tsuscan/src/features/scan/scan_review_page.dart';
import 'package:tsuscan/src/features/scan/scan_session.dart';
import 'package:tsuscan/src/services/app_lock.dart';
import 'package:tsuscan/src/ui/theme/app_theme.dart';

import 'support.dart';

/// PNG 1x1 hợp lệ (Image.memory giải mã được trong test).
final _png = base64Decode(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==',
);

ScanQuad _rect(double x0, double y0, double x1, double y1, [double conf = 0.9]) =>
    ScanQuad([x0, y0, x1, y0, x1, y1, x0, y1], conf);

void main() {
  test('packLumaRows lấy hàng 0, k, 2k..., bỏ đệm, hàng cuối không đệm', () {
    // Khung 4x7, stride 6 (2 byte đệm), hàng cuối chỉ có 4 byte; giá trị = 10 * hàng + cột.
    const w = 4, h = 7, stride = 6;
    final luma = Uint8List(stride * (h - 1) + w);
    for (var y = 0; y < h; y++) {
      for (var x = 0; x < w; x++) {
        luma[y * stride + x] = 10 * y + x;
      }
    }
    expect(packLumaRows(luma, w, h, stride, 3), [0, 1, 2, 3, 30, 31, 32, 33, 60, 61, 62, 63]);
    expect(packLumaRows(luma, w, h, stride, 1).length, w * h);
    // Bộ đệm hỏng (thiếu byte): dừng ở hàng cuối còn đủ, không đọc tràn.
    expect(packLumaRows(Uint8List.sublistView(luma, 0, 20), w, h, stride, 3), [0, 1, 2, 3]);
  });

  test('mã bộ lọc Dart trùng tsuscan_imaging::Filter::code (cùng thứ tự)', () {
    final src = File('../../crates/tsuscan-imaging/src/filters.rs').readAsStringSync();
    final body = RegExp(r'pub fn code\(self\)[^{]*\{(.*?)\n    \}', dotAll: true).firstMatch(src)!.group(1)!;
    final rust = RegExp(r'=> "([a-z]+)"').allMatches(body).map((m) => m.group(1)).toList();
    expect(ScanFilter.values.map((f) => f.code).toList(), rust);
  });

  group('ScanQuad', () {
    test('diện tích, lồi, kẹp góc vào khung', () {
      final q = _rect(0.1, 0.1, 0.9, 0.9);
      expect(q.area, closeTo(0.64, 1e-9));
      expect(q.isConvex, isTrue);
      // Kéo góc TR sang trái quá góc TL ⇒ tự cắt ⇒ không lồi.
      expect(q.withPoint(1, 0.0, 0.1).isConvex, isFalse);
      expect(q.withPoint(2, 1.5, -1).point(2), (x: 1.0, y: 0.0));
      expect(() => ScanQuad(const [0, 0], 1), throwsArgumentError);
    });
  });

  group('AutoCaptureTracker', () {
    test('đứng yên đủ số khung thì chụp, rồi chờ trang đổi mới chụp tiếp', () {
      final t = AutoCaptureTracker(stableFrames: 4);
      final q = _rect(0.1, 0.1, 0.9, 0.9);
      expect([for (var i = 0; i < 4; i++) t.feed(q)], [false, false, false, true]);
      expect(t.waitingForChange, isTrue);
      // Vẫn trang cũ ⇒ không chụp lặp.
      expect([for (var i = 0; i < 10; i++) t.feed(q)].contains(true), isFalse);
      // Trang mới (dời hẳn chỗ) ⇒ đếm lại.
      final next = _rect(0.2, 0.15, 0.95, 0.95);
      expect([for (var i = 0; i < 4; i++) t.feed(next)], [false, false, false, true]);
    });

    test('rung tay, tin cậy thấp, tài liệu quá nhỏ: không chụp', () {
      final t = AutoCaptureTracker(stableFrames: 3);
      for (var i = 0; i < 20; i++) {
        final d = i.isEven ? 0 : 0.03;
        expect(t.feed(_rect(0.1 + d, 0.1, 0.9, 0.9)), isFalse);
      }
      for (var i = 0; i < 10; i++) {
        expect(t.feed(_rect(0.1, 0.1, 0.9, 0.9, 0.2)), isFalse);
        expect(t.feed(_rect(0.45, 0.45, 0.55, 0.55)), isFalse);
      }
    });

    test('mất tài liệu khỏi khung thì trang tiếp (dù cùng chỗ) được chụp', () {
      final t = AutoCaptureTracker(stableFrames: 2);
      final q = _rect(0.1, 0.1, 0.9, 0.9);
      t.feed(q);
      expect(t.feed(q), isTrue);
      expect(t.feed(null), isFalse);
      expect(t.waitingForChange, isFalse);
      t.feed(q);
      expect(t.feed(q), isTrue);
    });
  });

  group('ScanSession', () {
    test('thêm, đổi thứ tự, xóa, lưu vào kho rồi xóa trắng ảnh gốc', () async {
      final s = ScanSession(const MemoryScanEngine(), ScanMode.document);
      final a = Uint8List.fromList(_png);
      final b = Uint8List.fromList(_png);
      final c = Uint8List.fromList(_png);
      await s.addCapture(a);
      await s.addCapture(b);
      await s.addCapture(c);
      expect(s.length, 3);
      s.move(2, 0);
      expect(identical(s.pages.first.original, c), isTrue);
      s.remove(1);
      expect(a.every((v) => v == 0), isTrue, reason: 'trang xóa phải bị xóa trắng');
      expect(s.pages.first.filter, ScanFilter.auto);

      final repo = MemoryLibraryRepository();
      final doc = await repo.createDocument('Quét');
      var progress = <int>[];
      await s.saveTo(repo, doc, onProgress: progress.add);
      expect(progress, [1, 2]);
      expect((await repo.listPages(doc)).length, 2);
      expect(s.isEmpty, isTrue);
      expect(b.every((v) => v == 0) && c.every((v) => v == 0), isTrue);
      s.dispose();
      progress = [];
    });

    test('chế độ quyết định bộ lọc mặc định và có cắt hay không', () {
      expect(ScanMode.photo.crop, isFalse);
      expect(ScanMode.whiteboard.filter, ScanFilter.whiteboard);
      expect(ScanMode.idCard.sides, 2);
      expect(ScanMode.parse('khong-co'), ScanMode.document);
      expect(ScanMode.parse('book').splitBook, isTrue);
      expect(ScanMode.codes.readCodes && !ScanMode.codes.crop && !ScanMode.codes.autoCapture, isTrue);
    });

    test('Sách: gáy tìm thấy ⇒ 2 trang, mỗi trang giữ bản sao ảnh gốc riêng', () async {
      final left = ScanQuad(const [0, 0, 0.5, 0, 0.5, 1, 0, 1], 0.8);
      final right = ScanQuad(const [0.5, 0, 1, 0, 1, 1, 0.5, 1], 0.8);
      final s = ScanSession(_BookEngine([left, right]), ScanMode.book);
      final bytes = Uint8List.fromList(_png);
      final added = await s.addCapture(bytes);
      expect(added.length, 2);
      expect(s.pages.map((p) => p.quad), [left, right]);
      expect(identical(s.pages[0].original, s.pages[1].original), isFalse);
      s.remove(0);
      expect(s.pages.single.original, _png, reason: 'xóa trang trái không được xóa trắng ảnh của trang phải');
      s.dispose();

      // Không tìm thấy gáy ⇒ một trang như chế độ Tài liệu.
      final one = ScanSession(_BookEngine(null), ScanMode.book);
      expect((await one.addCapture(Uint8List.fromList(_png))).length, 1);
      one.dispose();
    });
  });

  testWidgets('mục Quét: chọn chế độ, có nút camera và nhập ảnh', (tester) async {
    final platform = FakePlatform();
    final settings = await freshSettings({'locale': 'vi'});
    await tester.pumpWidget(TsuScanApp(platform: platform, settings: settings));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Quét').last);
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(ChoiceChip, 'CCCD 2 mặt'));
    await tester.pumpAndSettle();
    final chip = tester.widget<ChoiceChip>(find.widgetWithText(ChoiceChip, 'CCCD 2 mặt'));
    expect(chip.selected, isTrue);
    expect(settings.scanMode, 'idCard', reason: 'nhớ chế độ cho lần quét sau');
    expect(find.widgetWithText(ChoiceChip, 'Sách'), findsOneWidget);
    expect(find.widgetWithText(ChoiceChip, 'QR/mã vạch'), findsOneWidget);
    final scroll = find.byType(Scrollable).last;
    await tester.scrollUntilVisible(find.text('Nhập ảnh có sẵn'), 100, scrollable: scroll);
    expect(find.text('Chụp bằng camera'), findsOneWidget);
    expect(find.text('Nhập ảnh có sẵn'), findsOneWidget);
  });

  testWidgets('duyệt: xóa một trang rồi lưu thành tài liệu có đúng số trang', (tester) async {
    final platform = FakePlatform();
    final settings = await freshSettings({'locale': 'vi'});
    final repo = platform.repo;
    final changes = LibraryChanges();
    final lock = AppLockController(
      settings: settings,
      storage: platform.storage,
      crypto: platform.pin,
      biometric: platform.biometric,
    );
    addTearDown(lock.dispose);
    final session = ScanSession(const MemoryScanEngine(), ScanMode.document);
    for (var i = 0; i < 3; i++) {
      await tester.runAsync(() => session.addCapture(Uint8List.fromList(_png)));
    }
    ReviewOutcome? outcome;
    await tester.pumpWidget(
      // Như app thật: AppScope nằm TRÊN Navigator (MaterialApp.builder) nên route đẩy lên vẫn thấy nó.
      AppScope(
        platform: platform,
        settings: settings,
        lock: lock,
        repository: repo,
        changes: changes,
        child: MaterialApp(
          locale: const Locale('vi'),
          theme: buildTheme(Brightness.light),
          supportedLocales: const [Locale('vi'), Locale('en')],
          localizationsDelegates: const [
            AppLocalizations.delegate,
            GlobalMaterialLocalizations.delegate,
            GlobalWidgetsLocalizations.delegate,
            GlobalCupertinoLocalizations.delegate,
          ],
          home: Builder(
            builder: (context) => Scaffold(
              body: TextButton(
                onPressed: () async {
                  outcome = await Navigator.of(context)
                      .push(MaterialPageRoute<ReviewOutcome>(builder: (_) => ScanReviewPage(session: session)));
                },
                child: const Text('mở'),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('mở'));
    await tester.pumpAndSettle();
    expect(find.text('Duyệt trang · 3 trang'), findsOneWidget);

    await tester.tap(find.byType(PopupMenuButton<String>).at(1));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Xóa trang'));
    await tester.pumpAndSettle();
    expect(find.text('Duyệt trang · 2 trang'), findsOneWidget);

    await tester.tap(find.text('Lưu tài liệu'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), 'Hóa đơn tháng 11');
    await tester.tap(find.widgetWithText(FilledButton, 'Lưu'));
    await tester.pumpAndSettle();

    expect(outcome, isA<ReviewSaved>());
    final id = (outcome! as ReviewSaved).documentId;
    expect((await repo.document(id)).title, 'Hóa đơn tháng 11');
    expect((await repo.listPages(id)).length, 2);
    expect(changes.revision, greaterThan(0));
    session.dispose();
    changes.dispose();
  });
}

/// Lõi giả cho chế độ Sách: dò được cả khung, tách theo [halves].
final class _BookEngine implements ScanEngine {
  const _BookEngine(this.halves);
  final List<ScanQuad>? halves;

  @override
  bool get supportsDetection => true;

  @override
  Future<ScanCapture> open(Uint8List bytes) async => _BookCapture(halves);

  @override
  Future<ScanQuad?> detectPreview(Uint8List luma, int width, int height, int rowStride, int rotation) async => null;

  @override
  Future<List<ScannedCode>> readCodesPreview(Uint8List luma, int width, int height, int rowStride) async => const [];
}

final class _BookCapture implements ScanCapture {
  _BookCapture(this.halves);
  final List<ScanQuad>? halves;

  @override
  int get width => 100;
  @override
  int get height => 50;
  @override
  Future<ScanQuad?> detect() async => ScanQuad.full;
  @override
  Future<List<ScanQuad>?> splitBook(ScanQuad? quad) async => halves;
  @override
  Future<List<ScannedCode>> readCodes() async => const [];
  @override
  Future<Uint8List> render(ScanQuad? quad, ScanFilter filter, int maxSide) async => Uint8List.fromList(_png);
  @override
  void dispose() {}
}

import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:tsuscan/src/data/library_repository.dart';
import 'package:tsuscan/src/data/memory_library.dart';

import 'app_flow_test.dart' show pumpApp;

final _jpeg = Uint8List.fromList([0xFF, 0xD8, 0xFF, 0xE0, 0, 0]);
const _day = 24 * 60 * 60 * 1000;

void main() {
  group('kho bộ nhớ - P7a', () {
    late MemoryLibraryRepository repo;
    var now = 0;
    setUp(() {
      now = 1000;
      repo = MemoryLibraryRepository(clock: () => DateTime.fromMillisecondsSinceEpoch(now));
    });

    test('thùng rác: ẩn khỏi danh sách/tìm kiếm, khôi phục, tự xóa sau 30 ngày', () async {
      final a = await repo.createDocument('Hợp đồng thuê nhà');
      await repo.createDocument('Hóa đơn');
      await repo.trashDocument(a);
      expect((await repo.listDocuments(const AllFolders(), limit: 10, offset: 0)).total, 1);
      expect(await repo.search('hop dong'), isEmpty);
      await expectLater(repo.document(a), throwsA(isA<ApiError>()));
      final t = (await repo.listTrash()).single;
      expect(t.purgeAtMs - t.trashedMs, trashRetentionDays * _day);

      await repo.restoreDocument(a);
      expect((await repo.search('hop dong')).single.documentId, a);

      await repo.trashDocument(a);
      now += trashRetentionDays * _day - 1;
      expect(await repo.purgeTrash(expiredOnly: true), 0);
      now += 1;
      expect(await repo.purgeTrash(expiredOnly: true), 1);
      expect(await repo.listTrash(), isEmpty);
    });

    test('xóa vĩnh viễn chỉ áp dụng cho tài liệu trong thùng rác', () async {
      final a = await repo.createDocument('A');
      await expectLater(repo.purgeDocument(a), throwsA(isA<ApiError>()));
      await repo.trashDocument(a);
      await repo.purgeDocument(a);
      expect(await repo.listTrash(), isEmpty);
    });

    test('sắp xếp, chuyển trang (tách/ghép), xoay đổi kích thước', () async {
      final a = await repo.createDocument('A');
      final b = await repo.createDocument('B');
      final a1 = await repo.importImagePage(a, _jpeg);
      final a2 = await repo.importImagePage(a, _jpeg);
      final a3 = await repo.importImagePage(a, _jpeg);
      final b1 = await repo.importImagePage(b, _jpeg);
      Future<List<String>> ids(String d) async => [for (final p in await repo.listPages(d)) p.id];

      await repo.reorderPages(a, [a3, a1, a2]);
      expect(await ids(a), [a3, a1, a2]);
      await expectLater(repo.reorderPages(a, [a1, a2]), throwsA(isA<ApiError>()));

      await repo.movePages([a1, a2], b, at: 0);
      expect(await ids(b), [a1, a2, b1]);
      expect(await ids(a), [a3]);
      expect((await repo.document(a)).pageCount, 1);
      expect((await repo.listPages(b)).map((p) => p.position), [0, 1, 2]);
      await expectLater(repo.movePages([a1], b), throwsA(isA<ApiError>()));
      await expectLater(repo.movePages(const [], b), throwsA(isA<ApiError>()));
    });
  });

  testWidgets('xóa tài liệu ⇒ vào thùng rác; khôi phục từ màn Thùng rác', (tester) async {
    final platform = await pumpApp(tester);
    await tester.tap(find.widgetWithText(FloatingActionButton, 'Tài liệu mới'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), 'Sổ đỏ');
    await tester.tap(find.widgetWithText(FilledButton, 'Lưu'));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.more_vert));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Chuyển vào thùng rác'));
    await tester.pumpAndSettle();
    expect(find.textContaining('30 ngày'), findsOneWidget);
    await tester.tap(find.widgetWithText(FilledButton, 'Chuyển vào thùng rác'));
    await tester.pumpAndSettle();

    expect(find.text('Đã chuyển "Sổ đỏ" vào thùng rác'), findsOneWidget);
    expect(find.text('Chưa có tài liệu nào'), findsOneWidget);
    expect((await platform.repo.listTrash()).single.document.title, 'Sổ đỏ');

    await tester.tap(find.byTooltip('Thùng rác'));
    await tester.pumpAndSettle();
    expect(find.text('Sổ đỏ'), findsOneWidget);
    await tester.tap(find.byTooltip('Khôi phục'));
    await tester.pumpAndSettle();
    expect(find.text('Thùng rác trống'), findsOneWidget);
    await tester.tap(find.byType(BackButton));
    await tester.pumpAndSettle();
    expect(find.text('Sổ đỏ'), findsOneWidget);
  });
}

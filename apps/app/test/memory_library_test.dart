import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:tsuscan/src/data/library_repository.dart';
import 'package:tsuscan/src/data/memory_library.dart';
import 'package:tsuscan/src/features/library/import_images.dart';

final jpeg = Uint8List.fromList([0xFF, 0xD8, 0xFF, 0xE0, 0, 0]);

void main() {
  late MemoryLibraryRepository repo;
  var now = 1000;
  setUp(() {
    now = 1000;
    repo = MemoryLibraryRepository(clock: () => DateTime.fromMillisecondsSinceEpoch(now += 10));
  });

  test('phân trang trả đúng lát cắt và tổng, mới sửa trước', () async {
    for (var i = 0; i < 25; i++) {
      await repo.createDocument('Tài liệu $i');
    }
    final p = await repo.listDocuments(const AllFolders(), limit: 10, offset: 20);
    expect(p.total, 25);
    expect(p.items.map((d) => d.title), ['Tài liệu 4', 'Tài liệu 3', 'Tài liệu 2', 'Tài liệu 1', 'Tài liệu 0']);
  });

  test('lọc theo thư mục, gốc và nhãn', () async {
    final f = await repo.createFolder('Hồ sơ');
    final a = await repo.createDocument('A', folderId: f);
    await repo.createDocument('B');
    final t = await repo.createTag('Quan trọng');
    await repo.setTag(a, t, on: true);
    expect((await repo.listDocuments(InFolder(f), limit: 10, offset: 0)).items.single.id, a);
    expect((await repo.listDocuments(const RootFolder(), limit: 10, offset: 0)).items.single.title, 'B');
    expect((await repo.listDocuments(const AllFolders(), tagId: t, limit: 10, offset: 0)).total, 1);
  });

  test('xóa thư mục giữ tài liệu, chuyển lên thư mục cha', () async {
    final f = await repo.createFolder('Hồ sơ');
    final a = await repo.createDocument('A', folderId: f);
    await repo.deleteFolder(f);
    expect((await repo.document(a)).folderId, isNull);
  });

  test('tên rỗng bị từ chối với lỗi đầu vào', () async {
    expect(
      () => repo.createDocument('   '),
      throwsA(isA<ApiError>().having((e) => e.kind, 'kind', ApiErrorKind.invalidInput)),
    );
  });

  test('nhập ảnh: bỏ qua tệp không phải ảnh và đếm lại', () async {
    final d = await repo.createDocument('Ảnh');
    final r = await importImages(repo, d, [
      jpeg,
      Uint8List.fromList([1, 2, 3]),
      jpeg,
    ]);
    expect(r, (added: 2, skipped: 1));
    expect((await repo.document(d)).pageCount, 2);
    final pages = await repo.listPages(d);
    await repo.deletePage(pages.first.id);
    expect((await repo.listPages(d)).single.position, 0);
  });

  test('tìm không dấu và tô sáng', () async {
    final d = await repo.createDocument('Hợp đồng thuê nhà');
    await repo.createDocument('Hóa đơn');
    final hits = await repo.search('hop dong');
    expect(hits.single.documentId, d);
    expect(await repo.search('h'), isEmpty);
    final r = repo.highlight('Hợp đồng', 'dong');
    expect('Hợp đồng'.substring(r.single.start, r.single.end), 'đồng');
  });
}

import 'dart:typed_data';

import '../utils/vi_text.dart';
import 'library_repository.dart';
import 'scan_engine.dart';

/// Kho trong bộ nhớ: web xem trước (chưa có lõi WASM - P11) và test giao diện. Không lưu gì xuống đĩa.
final class MemoryLibraryRepository implements LibraryRepository {
  /// [persistent] chỉ để test giả lập kho thật (giao diện hiện phần khóa app).
  MemoryLibraryRepository({DateTime Function()? clock, this.persistent = false}) : _clock = clock ?? DateTime.now;

  final DateTime Function() _clock;
  final _folders = <String, FolderDto>{};
  final _tags = <String, TagDto>{};
  final _docs = <String, Document>{};
  final _docTags = <String, Set<String>>{};
  final _pages = <String, List<(PageInfo, Uint8List)>>{};
  var _seq = 0;

  String _id() => (++_seq).toRadixString(16).padLeft(32, '0');
  int get _now => _clock().millisecondsSinceEpoch;

  static String _clean(String name) {
    final t = name.trim();
    if (t.isEmpty || t.runes.length > 200 || t.runes.any((r) => r < 0x20 || r == 0x7F)) {
      throw const ApiError(kind: ApiErrorKind.invalidInput, detail: 'tên không hợp lệ');
    }
    return t;
  }

  T _need<T>(T? v) => v ?? (throw const ApiError(kind: ApiErrorKind.notFound, detail: 'không tìm thấy'));

  @override
  final bool persistent;

  @override
  Future<List<FolderDto>> listFolders() async => _folders.values.toList()..sort((a, b) => a.name.compareTo(b.name));

  @override
  Future<String> createFolder(String name, {String? parentId}) async {
    if (parentId != null) _need(_folders[parentId]);
    final id = _id();
    _folders[id] = FolderDto(id: id, parentId: parentId, name: _clean(name));
    return id;
  }

  @override
  Future<void> renameFolder(String id, String name) async {
    final f = _need(_folders[id]);
    _folders[id] = FolderDto(id: id, parentId: f.parentId, name: _clean(name));
  }

  @override
  Future<void> deleteFolder(String id) async {
    final f = _need(_folders.remove(id));
    for (final d in _docs.values.where((d) => d.folderId == id).toList()) {
      _docs[d.id] = d.copyWith(folderId: () => f.parentId, updatedMs: _now);
    }
    for (final c in _folders.values.where((c) => c.parentId == id).toList()) {
      _folders[c.id] = FolderDto(id: c.id, parentId: f.parentId, name: c.name);
    }
  }

  @override
  Future<List<TagDto>> listTags() async => _tags.values.toList()..sort((a, b) => a.name.compareTo(b.name));

  @override
  Future<String> createTag(String name) async {
    final n = _clean(name);
    for (final t in _tags.values) {
      if (t.name.toLowerCase() == n.toLowerCase()) return t.id;
    }
    final id = _id();
    _tags[id] = TagDto(id: id, name: n);
    return id;
  }

  @override
  Future<void> deleteTag(String id) async {
    _need(_tags.remove(id));
    for (final s in _docTags.values) {
      s.remove(id);
    }
  }

  @override
  Future<void> setTag(String documentId, String tagId, {required bool on}) async {
    _need(_docs[documentId]);
    _need(_tags[tagId]);
    final s = _docTags.putIfAbsent(documentId, () => {});
    on ? s.add(tagId) : s.remove(tagId);
  }

  @override
  Future<List<TagDto>> documentTags(String documentId) async =>
      [for (final id in _docTags[documentId] ?? <String>{}) ?_tags[id]]..sort((a, b) => a.name.compareTo(b.name));

  @override
  Future<DocumentPage> listDocuments(
    FolderScope scope, {
    String? tagId,
    required int limit,
    required int offset,
  }) async {
    final all = _docs.values.where((d) {
      final inScope = switch (scope) {
        AllFolders() => true,
        RootFolder() => d.folderId == null,
        InFolder(:final id) => d.folderId == id,
      };
      return inScope && (tagId == null || (_docTags[d.id]?.contains(tagId) ?? false));
    }).toList()..sort((a, b) => b.updatedMs.compareTo(a.updatedMs));
    return DocumentPage(items: all.skip(offset).take(limit).toList(), total: all.length);
  }

  @override
  Future<Document> document(String id) async => _need(_docs[id]);

  @override
  Future<String> createDocument(String title, {String? folderId}) async {
    if (folderId != null) _need(_folders[folderId]);
    final id = _id();
    final now = _now;
    _docs[id] = Document(
      id: id,
      title: _clean(title),
      folderId: folderId,
      createdMs: now,
      updatedMs: now,
      pageCount: 0,
    );
    return id;
  }

  @override
  Future<void> renameDocument(String id, String title) async =>
      _docs[id] = _need(_docs[id]).copyWith(title: _clean(title), updatedMs: _now);

  @override
  Future<void> moveDocument(String id, String? folderId) async {
    if (folderId != null) _need(_folders[folderId]);
    _docs[id] = _need(_docs[id]).copyWith(folderId: () => folderId, updatedMs: _now);
  }

  @override
  Future<void> deleteDocument(String id) async {
    _need(_docs.remove(id));
    _pages.remove(id);
    _docTags.remove(id);
  }

  @override
  Future<List<PageInfo>> listPages(String documentId) async => [
    for (final (p, _) in _pages[documentId] ?? const <(PageInfo, Uint8List)>[]) p,
  ];

  @override
  Future<String> importImagePage(String documentId, Uint8List image) async {
    final d = _need(_docs[documentId]);
    if (!_looksLikeImage(image)) {
      throw const ApiError(kind: ApiErrorKind.unsupportedImage, detail: 'không phải ảnh');
    }
    final list = _pages.putIfAbsent(documentId, () => []);
    final id = _id();
    list.add((PageInfo(id: id, position: list.length, width: 0, height: 0), image));
    _docs[documentId] = d.copyWith(pageCount: list.length, updatedMs: _now);
    return id;
  }

  @override
  Future<String> addScannedPage(String documentId, ScanCapture capture, ScanQuad? quad, ScanFilter filter) async =>
      importImagePage(documentId, Uint8List.fromList(await capture.render(quad, filter, 2048)));

  static bool _looksLikeImage(Uint8List b) =>
      (b.length > 3 && b[0] == 0xFF && b[1] == 0xD8) ||
      (b.length > 8 && b[0] == 0x89 && b[1] == 0x50 && b[2] == 0x4E && b[3] == 0x47) ||
      (b.length > 12 && b[8] == 0x57 && b[9] == 0x45 && b[10] == 0x42 && b[11] == 0x50);

  @override
  Future<Uint8List> pageThumbnail(String pageId, int maxSide) async {
    for (final list in _pages.values) {
      for (final (p, bytes) in list) {
        if (p.id == pageId) return bytes;
      }
    }
    throw const ApiError(kind: ApiErrorKind.notFound, detail: 'trang');
  }

  @override
  Future<void> deletePage(String pageId) async {
    for (final MapEntry(key: docId, value: list) in _pages.entries) {
      final i = list.indexWhere((e) => e.$1.id == pageId);
      if (i >= 0) {
        list.removeAt(i);
        for (var k = 0; k < list.length; k++) {
          final p = list[k].$1;
          list[k] = (PageInfo(id: p.id, position: k, width: p.width, height: p.height, ocrText: p.ocrText), list[k].$2);
        }
        _docs[docId] = _docs[docId]!.copyWith(pageCount: list.length, updatedMs: _now);
        return;
      }
    }
    throw const ApiError(kind: ApiErrorKind.notFound, detail: 'trang');
  }

  @override
  Future<List<SearchHitDto>> search(String query, {int limit = 200}) async {
    final q = viNormalize(query);
    if (q.runes.length < 2) return [];
    final words = q.split(' ');
    final hits = <SearchHitDto>[];
    for (final d in _docs.values) {
      final text = viNormalize(
        [d.title, for (final (p, _) in _pages[d.id] ?? const <(PageInfo, Uint8List)>[]) p.ocrText].join(' '),
      );
      if (words.every(text.contains)) {
        final exact = d.title.toLowerCase().contains(query.trim().toLowerCase());
        hits.add(SearchHitDto(documentId: d.id, title: d.title, score: exact ? 1 : 0.8, accentExact: exact));
      }
    }
    hits.sort((a, b) => b.score.compareTo(a.score));
    return hits.take(limit).toList();
  }

  @override
  List<HighlightRange> highlight(String text, String query) => [
    for (final (s, e) in viHighlightRanges(text, query)) HighlightRange(start: s, end: e),
  ];
}

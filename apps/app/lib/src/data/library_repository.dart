import 'dart:typed_data';

import '../bridge/generated/api/library.dart' as ffi;
import '../bridge/generated/api/library.dart' show FolderDto, SearchHitDto, TagDto;
import '../bridge/generated/api/text.dart';
import 'scan_engine.dart';

export '../bridge/generated/api/error.dart' show ApiError, ApiErrorKind;
export '../bridge/generated/api/library.dart' show FolderDto, SearchHitDto, TagDto;
export '../bridge/generated/api/text.dart' show HighlightRange;

/// Tài liệu. Mốc thời gian là ms Unix (DTO cầu nối dùng i64 ⇒ `BigInt` trên web, nên giao diện dùng kiểu này).
final class Document {
  const Document({
    required this.id,
    required this.title,
    required this.folderId,
    required this.createdMs,
    required this.updatedMs,
    required this.pageCount,
  });
  final String id;
  final String title;
  final String? folderId;
  final int createdMs;
  final int updatedMs;
  final int pageCount;

  Document copyWith({String? title, String? Function()? folderId, int? updatedMs, int? pageCount}) => Document(
    id: id,
    title: title ?? this.title,
    folderId: folderId == null ? this.folderId : folderId(),
    createdMs: createdMs,
    updatedMs: updatedMs ?? this.updatedMs,
    pageCount: pageCount ?? this.pageCount,
  );
}

/// Một trang danh sách tài liệu + tổng số khớp bộ lọc.
final class DocumentPage {
  const DocumentPage({required this.items, required this.total});
  final List<Document> items;
  final int total;
}

/// Trang của tài liệu (ảnh lấy riêng qua [LibraryRepository.pageThumbnail]).
final class PageInfo {
  const PageInfo({
    required this.id,
    required this.position,
    required this.width,
    required this.height,
    this.ocrText = '',
  });
  final String id;
  final int position;
  final int width;
  final int height;
  final String ocrText;
}

/// Tài liệu trong thùng rác: tự xóa hẳn lúc [purgeAtMs] (30 ngày sau khi bỏ vào).
final class TrashedDocument {
  const TrashedDocument({required this.document, required this.trashedMs, required this.purgeAtMs});
  final Document document;
  final int trashedMs;
  final int purgeAtMs;
}

/// Số ngày tài liệu nằm trong thùng rác trước khi bị xóa hẳn (khớp `TRASH_RETENTION_DAYS` của kho).
const trashRetentionDays = 30;

/// Phạm vi thư mục khi liệt kê tài liệu.
sealed class FolderScope {
  const FolderScope();
}

/// Mọi thư mục.
final class AllFolders extends FolderScope {
  const AllFolders();
  @override
  bool operator ==(Object other) => other is AllFolders;
  @override
  int get hashCode => 0;
}

/// Chỉ tài liệu chưa xếp thư mục.
final class RootFolder extends FolderScope {
  const RootFolder();
  @override
  bool operator ==(Object other) => other is RootFolder;
  @override
  int get hashCode => 1;
}

/// Một thư mục.
final class InFolder extends FolderScope {
  const InFolder(this.id);
  final String id;
  @override
  bool operator ==(Object other) => other is InFolder && other.id == id;
  @override
  int get hashCode => id.hashCode;
}

/// Kho tài liệu mà giao diện dùng. Bản thật: [RustLibraryRepository] (SQLite3MC qua lõi Rust). Bản bộ nhớ:
/// `MemoryLibraryRepository` cho web xem trước và test.
abstract interface class LibraryRepository {
  /// Dữ liệu có lưu bền và mã hóa trên máy không (web xem trước: không).
  bool get persistent;

  Future<List<FolderDto>> listFolders();
  Future<String> createFolder(String name, {String? parentId});
  Future<void> renameFolder(String id, String name);
  Future<void> deleteFolder(String id);

  Future<List<TagDto>> listTags();
  Future<String> createTag(String name);
  Future<void> deleteTag(String id);
  Future<void> setTag(String documentId, String tagId, {required bool on});
  Future<List<TagDto>> documentTags(String documentId);

  Future<DocumentPage> listDocuments(FolderScope scope, {String? tagId, required int limit, required int offset});
  Future<Document> document(String id);
  Future<String> createDocument(String title, {String? folderId});
  Future<void> renameDocument(String id, String title);
  Future<void> moveDocument(String id, String? folderId);
  Future<void> deleteDocument(String id);

  /// Bỏ vào thùng rác (thao tác "Xóa" của người dùng).
  Future<void> trashDocument(String id);
  Future<void> restoreDocument(String id);
  Future<List<TrashedDocument>> listTrash();

  /// Xóa vĩnh viễn một tài liệu trong thùng rác.
  Future<void> purgeDocument(String id);

  /// Dọn thùng rác: tất cả, hoặc chỉ tài liệu đã quá [trashRetentionDays] ngày. Trả số tài liệu đã xóa hẳn.
  Future<int> purgeTrash({required bool expiredOnly});

  Future<List<PageInfo>> listPages(String documentId);
  Future<String> importImagePage(String documentId, Uint8List image);

  /// Lưu trang vừa quét: nắn theo [quad] (`null` = cả khung), lọc, mã hóa JPEG một lần rồi ghi vào kho.
  Future<String> addScannedPage(String documentId, ScanCapture capture, ScanQuad? quad, ScanFilter filter);
  Future<Uint8List> pageThumbnail(String pageId, int maxSide);
  Future<void> deletePage(String pageId);

  /// Xoay trang theo chiều kim đồng hồ [quarterTurns] × 90°.
  Future<void> rotatePage(String pageId, int quarterTurns);

  /// Sắp lại trang: [order] là hoán vị đủ mọi trang của tài liệu.
  Future<void> reorderPages(String documentId, List<String> order);

  /// Chuyển trang sang tài liệu khác, chèn tại [at] (`null` = cuối). Dùng cho tách/ghép tài liệu.
  Future<void> movePages(List<String> pageIds, String targetDocumentId, {int? at});

  Future<List<SearchHitDto>> search(String query, {int limit = 200});

  /// Các đoạn cần tô sáng trong [text] theo [query] (cùng quy tắc chuẩn hóa với tìm kiếm).
  List<HighlightRange> highlight(String text, String query);
}

/// Bản thật: mọi lời gọi đi qua lõi Rust.
final class RustLibraryRepository implements LibraryRepository {
  RustLibraryRepository(this._h);
  final ffi.LibraryHandle _h;

  @override
  bool get persistent => true;

  @override
  Future<List<FolderDto>> listFolders() => _h.listFolders();
  @override
  Future<String> createFolder(String name, {String? parentId}) => _h.createFolder(name: name, parentId: parentId);
  @override
  Future<void> renameFolder(String id, String name) => _h.renameFolder(id: id, name: name);
  @override
  Future<void> deleteFolder(String id) => _h.deleteFolder(id: id);

  @override
  Future<List<TagDto>> listTags() => _h.listTags();
  @override
  Future<String> createTag(String name) => _h.createTag(name: name);
  @override
  Future<void> deleteTag(String id) => _h.deleteTag(id: id);
  @override
  Future<void> setTag(String documentId, String tagId, {required bool on}) =>
      _h.setTag(documentId: documentId, tagId: tagId, on_: on);
  @override
  Future<List<TagDto>> documentTags(String documentId) => _h.documentTags(documentId: documentId);

  @override
  Future<DocumentPage> listDocuments(
    FolderScope scope, {
    String? tagId,
    required int limit,
    required int offset,
  }) async {
    final page = await _h.listDocuments(
      folderId: scope is InFolder ? scope.id : null,
      rootOnly: scope is RootFolder,
      tagId: tagId,
      limit: limit,
      offset: BigInt.from(offset),
    );
    return DocumentPage(items: page.items.map(_doc).toList(), total: page.total.toInt());
  }

  @override
  Future<Document> document(String id) async => _doc(await _h.document(id: id));

  static Document _doc(ffi.DocumentDto d) => Document(
    id: d.id,
    title: d.title,
    folderId: d.folderId,
    createdMs: d.createdMs.toInt(),
    updatedMs: d.updatedMs.toInt(),
    pageCount: d.pageCount.toInt(),
  );
  @override
  Future<String> createDocument(String title, {String? folderId}) =>
      _h.createDocument(title: title, folderId: folderId);
  @override
  Future<void> renameDocument(String id, String title) => _h.renameDocument(id: id, title: title);
  @override
  Future<void> moveDocument(String id, String? folderId) => _h.moveDocument(id: id, folderId: folderId);
  @override
  Future<void> deleteDocument(String id) => _h.deleteDocument(id: id);

  @override
  Future<void> trashDocument(String id) => _h.trashDocument(id: id);
  @override
  Future<void> restoreDocument(String id) => _h.restoreDocument(id: id);
  @override
  Future<List<TrashedDocument>> listTrash() async => [
    for (final t in await _h.listTrash())
      TrashedDocument(document: _doc(t.document), trashedMs: t.trashedMs.toInt(), purgeAtMs: t.purgeAtMs.toInt()),
  ];
  @override
  Future<void> purgeDocument(String id) => _h.purgeDocument(id: id);
  @override
  Future<int> purgeTrash({required bool expiredOnly}) => _h.purgeTrash(expiredOnly: expiredOnly);

  @override
  Future<List<PageInfo>> listPages(String documentId) async => [
    for (final p in await _h.listPages(documentId: documentId))
      PageInfo(id: p.id, position: p.position.toInt(), width: p.width, height: p.height, ocrText: p.ocrText),
  ];
  @override
  Future<String> importImagePage(String documentId, Uint8List image) =>
      _h.importImagePage(documentId: documentId, image: image);
  @override
  Future<String> addScannedPage(String documentId, ScanCapture capture, ScanQuad? quad, ScanFilter filter) {
    if (capture is! RustScanCapture) throw ArgumentError('ảnh chụp không thuộc lõi Rust');
    return _h.addScannedPage(
      documentId: documentId,
      image: capture.inner,
      corners: quad?.corners ?? const [],
      filter: filter.code,
    );
  }

  @override
  Future<Uint8List> pageThumbnail(String pageId, int maxSide) => _h.pageThumbnail(pageId: pageId, maxSide: maxSide);
  @override
  Future<void> deletePage(String pageId) => _h.deletePage(pageId: pageId);
  @override
  Future<void> rotatePage(String pageId, int quarterTurns) => _h.rotatePage(pageId: pageId, quarterTurns: quarterTurns);
  @override
  Future<void> reorderPages(String documentId, List<String> order) =>
      _h.reorderPages(documentId: documentId, order: order);
  @override
  Future<void> movePages(List<String> pageIds, String targetDocumentId, {int? at}) =>
      _h.movePages(pageIds: pageIds, targetDocumentId: targetDocumentId, at: at);

  @override
  Future<List<SearchHitDto>> search(String query, {int limit = 200}) => _h.search(query: query, limit: limit);

  @override
  List<HighlightRange> highlight(String text, String query) => viHighlight(text: text, query: query);
}

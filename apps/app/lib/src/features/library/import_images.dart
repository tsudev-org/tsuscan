import 'dart:typed_data';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';

import '../../../l10n/app_localizations.dart';
import '../../app_scope.dart';
import '../../data/library_repository.dart';
import '../../ui/widgets/feedback.dart';

const _images = XTypeGroup(
  label: 'images',
  extensions: ['jpg', 'jpeg', 'png', 'webp'],
  mimeTypes: ['image/jpeg', 'image/png', 'image/webp'],
  uniformTypeIdentifiers: ['public.jpeg', 'public.png', 'org.webmproject.webp'],
  webWildCards: ['image/*'],
);

/// Kết quả nhập ảnh.
typedef ImportOutcome = ({int added, int skipped});

/// Chọn ảnh từ máy. Trả danh sách rỗng khi người dùng hủy.
Future<List<Uint8List>> pickImages() async {
  final files = await openFiles(acceptedTypeGroups: const [_images]);
  return [for (final f in files) await f.readAsBytes()];
}

/// Thêm từng ảnh làm trang cuối của [documentId]. Ảnh lỗi bị bỏ qua (đếm lại để báo), lỗi lưu trữ thì dừng.
Future<ImportOutcome> importImages(LibraryRepository repo, String documentId, List<Uint8List> images) async {
  var added = 0;
  var skipped = 0;
  for (final img in images) {
    try {
      await repo.importImagePage(documentId, img);
      added++;
    } on ApiError catch (e) {
      if (e.kind == ApiErrorKind.unsupportedImage || e.kind == ApiErrorKind.imageTooLarge) {
        skipped++;
      } else {
        rethrow;
      }
    }
  }
  return (added: added, skipped: skipped);
}

/// Báo kết quả nhập cho người dùng.
void reportImport(BuildContext context, ImportOutcome r) {
  final l = AppLocalizations.of(context);
  if (r.skipped > 0) {
    showWarning(context, '${l.documentImported(r.added)} · ${l.documentImportSkipped(r.skipped)}');
  } else if (r.added > 0) {
    showInfo(context, l.documentImported(r.added));
  }
}

/// Chọn ảnh rồi thêm vào tài liệu có sẵn; báo kết quả. Trả `true` nếu có thay đổi.
Future<bool> pickAndImportInto(BuildContext context, String documentId) async {
  final scope = AppScope.of(context);
  try {
    final images = await pickImages();
    if (images.isEmpty) return false;
    final r = await importImages(scope.repository, documentId, images);
    scope.changes.bump();
    if (context.mounted) reportImport(context, r);
    return r.added > 0;
  } on Object catch (e) {
    if (context.mounted) showError(context, e);
    return false;
  }
}

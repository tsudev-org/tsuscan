import 'package:flutter/material.dart';

import '../../../l10n/app_localizations.dart';
import '../../app_scope.dart';
import '../../ui/theme/app_theme.dart';
import '../../ui/widgets/feedback.dart';
import '../../utils/format.dart';
import '../library/document_page.dart';
import '../library/import_images.dart';

/// Quét: camera đến ở P6; lúc này nhập ảnh có sẵn thành tài liệu mới.
class ScanPage extends StatelessWidget {
  const ScanPage({super.key});

  Future<void> _import(BuildContext context) async {
    final app = AppScope.of(context);
    final l = AppLocalizations.of(context);
    try {
      final images = await pickImages();
      if (images.isEmpty) return;
      final id = await app.repository.createDocument(l.dialogDefaultDocumentTitle(formatDateTimeVN(DateTime.now())));
      final r = await importImages(app.repository, id, images);
      app.changes.bump();
      if (!context.mounted) return;
      reportImport(context, r);
      await Navigator.of(context).push(MaterialPageRoute<void>(builder: (_) => DocumentScreen(documentId: id)));
    } on Object catch (e) {
      if (context.mounted) showError(context, e);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return Scaffold(
      appBar: AppBar(title: Text(l.scanTitle)),
      body: EmptyState(
        icon: Icons.document_scanner_outlined,
        title: l.scanTitle,
        body: l.scanComingBody,
        action: FilledButton.icon(
          onPressed: () => _import(context),
          icon: const Icon(Icons.add_photo_alternate_outlined),
          label: Padding(
            padding: const EdgeInsets.symmetric(horizontal: TsSpace.s1),
            child: Text(l.scanImportToNew),
          ),
        ),
      ),
    );
  }
}

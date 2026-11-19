import 'package:flutter/material.dart';

import '../../../l10n/app_localizations.dart';
import '../../app_scope.dart';
import '../../data/library_repository.dart';
import '../../ui/theme/app_theme.dart';
import '../../ui/widgets/feedback.dart';
import '../library/document_page.dart';
import '../library/import_images.dart';
import 'camera_scan_page.dart';
import 'scan_labels.dart';
import 'scan_review_page.dart';
import 'scan_session.dart';

/// Mục Quét: chọn chế độ rồi chụp bằng camera (Android/iOS) hoặc nhập ảnh có sẵn. Cả hai đường đều dò mép, nắn và
/// lọc trên máy rồi qua màn duyệt trước khi lưu.
class ScanPage extends StatefulWidget {
  const ScanPage({super.key, this.cameraAvailable});

  /// Ghi đè để test; mặc định theo nền tảng.
  final bool? cameraAvailable;

  @override
  State<ScanPage> createState() => _ScanPageState();
}

class _ScanPageState extends State<ScanPage> {
  ScanMode _mode = ScanMode.document;
  bool _busy = false;

  bool get _camera => widget.cameraAvailable ?? cameraScanSupported;

  Future<void> _openDocument(String id) async {
    if (!mounted) return;
    await Navigator.of(context).push(MaterialPageRoute<void>(builder: (_) => DocumentScreen(documentId: id)));
  }

  Future<void> _scanWithCamera() async {
    final app = AppScope.of(context);
    final session = ScanSession(app.platform.scan, _mode);
    try {
      // Navigator gốc: camera phủ toàn màn hình, ẩn thanh điều hướng.
      final id = await Navigator.of(
        context,
        rootNavigator: true,
      ).push(MaterialPageRoute<String>(fullscreenDialog: true, builder: (_) => CameraScanPage(session: session)));
      if (mounted) setState(() => _mode = session.mode);
      if (id != null) await _openDocument(id);
    } finally {
      session.dispose();
    }
  }

  Future<void> _import() async {
    final app = AppScope.of(context);
    final l = AppLocalizations.of(context);
    final session = ScanSession(app.platform.scan, _mode);
    try {
      final images = await pickImages();
      if (images.isEmpty || !mounted) return;
      setState(() => _busy = true);
      var skipped = 0;
      for (final img in images) {
        try {
          await session.addCapture(img);
        } on ApiError catch (e) {
          if (e.kind != ApiErrorKind.unsupportedImage && e.kind != ApiErrorKind.imageTooLarge) rethrow;
          skipped++;
        }
      }
      if (!mounted) return;
      setState(() => _busy = false);
      if (skipped > 0) showWarning(context, l.documentImportSkipped(skipped));
      if (session.isEmpty) return;
      final out = await Navigator.of(
        context,
        rootNavigator: true,
      ).push(MaterialPageRoute<ReviewOutcome>(builder: (_) => ScanReviewPage(session: session, canCaptureMore: false)));
      if (out is ReviewSaved) await _openDocument(out.documentId);
    } on Object catch (e) {
      if (mounted) showError(context, e);
    } finally {
      session.dispose();
      if (mounted && _busy) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final t = Theme.of(context).textTheme;
    return Scaffold(
      appBar: AppBar(title: Text(l.scanTitle)),
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 560),
          child: ListView(
            padding: const EdgeInsets.all(TsSpace.s6),
            shrinkWrap: true,
            children: [
              Icon(Icons.document_scanner_outlined, size: 56, color: context.palette.primary),
              const SizedBox(height: TsSpace.s4),
              Text(l.scanIntro, textAlign: TextAlign.center, style: t.bodyMedium),
              if (!_camera) ...[
                const SizedBox(height: TsSpace.s3),
                Text(
                  l.scanCameraUnsupported,
                  textAlign: TextAlign.center,
                  style: t.bodySmall?.copyWith(color: context.palette.textSecondary),
                ),
              ],
              const SizedBox(height: TsSpace.s6),
              Text(l.scanModeLabel, style: t.titleSmall),
              const SizedBox(height: TsSpace.s2),
              Wrap(
                spacing: TsSpace.s2,
                runSpacing: TsSpace.s2,
                children: [
                  for (final m in ScanMode.values)
                    ChoiceChip(
                      avatar: Icon(m.icon, size: 18),
                      label: Text(m.label(l)),
                      selected: m == _mode,
                      onSelected: _busy ? null : (_) => setState(() => _mode = m),
                    ),
                ],
              ),
              const SizedBox(height: TsSpace.s6),
              if (_camera) ...[
                FilledButton.icon(
                  onPressed: _busy ? null : _scanWithCamera,
                  icon: const Icon(Icons.photo_camera_outlined),
                  label: Text(l.scanStartCamera),
                ),
                const SizedBox(height: TsSpace.s3),
              ],
              OutlinedButton.icon(
                onPressed: _busy ? null : _import,
                icon: _busy
                    ? const SizedBox.square(dimension: 18, child: CircularProgressIndicator(strokeWidth: 2))
                    : const Icon(Icons.add_photo_alternate_outlined),
                label: Text(_busy ? l.scanProcessing : l.scanImport),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

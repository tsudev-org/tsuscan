import 'package:flutter/material.dart';

import '../../../l10n/app_localizations.dart';
import '../../app_scope.dart';
import '../../ui/theme/app_theme.dart';
import '../../ui/widgets/feedback.dart';
import '../../utils/format.dart';
import 'page_editor_page.dart';
import 'scan_labels.dart';
import 'scan_session.dart';

/// Kết quả màn duyệt.
sealed class ReviewOutcome {
  const ReviewOutcome();
}

/// Đã lưu thành tài liệu [documentId].
final class ReviewSaved extends ReviewOutcome {
  const ReviewSaved(this.documentId);
  final String documentId;
}

/// Quay lại camera chụp thêm.
final class ReviewCaptureMore extends ReviewOutcome {
  const ReviewCaptureMore();
}

/// Duyệt các trang của phiên quét: xem, sửa góc/bộ lọc, đổi thứ tự, xóa, rồi lưu thành một tài liệu.
class ScanReviewPage extends StatefulWidget {
  const ScanReviewPage({super.key, required this.session, this.canCaptureMore = true});
  final ScanSession session;
  final bool canCaptureMore;

  @override
  State<ScanReviewPage> createState() => _ScanReviewPageState();
}

class _ScanReviewPageState extends State<ScanReviewPage> {
  bool _saving = false;
  final _done = ValueNotifier<int>(0);

  ScanSession get _s => widget.session;

  @override
  void dispose() {
    _done.dispose();
    super.dispose();
  }

  Future<void> _edit(int i) async {
    final updated = await Navigator.of(context).push(
      MaterialPageRoute<ScanDraftPage>(
        builder: (_) => PageEditorPage(engine: _s.engine, page: _s.pages[i]),
      ),
    );
    if (updated != null && i < _s.length) _s.replace(i, updated);
  }

  Future<void> _save() async {
    final app = AppScope.of(context);
    final l = AppLocalizations.of(context);
    final title = await askName(
      context,
      title: l.scanSaveDocument,
      label: l.dialogDocumentTitle,
      initial: '${_s.mode.label(l)} ${formatDateTimeVN(DateTime.now())}',
    );
    if (title == null || !mounted) return;
    final total = _s.length;
    setState(() => _saving = true);
    _done.value = 0;
    String? id;
    try {
      id = await app.repository.createDocument(title);
      await _s.saveTo(app.repository, id, onProgress: (n) => _done.value = n);
      app.changes.bump();
      if (!mounted) return;
      showInfo(context, l.documentImported(total));
      Navigator.of(context).pop(ReviewSaved(id));
    } on Object catch (e) {
      // Lưu dở: giữ tài liệu (đã có một số trang) để không mất ảnh đã ghi; trang chưa ghi còn trong phiên.
      if (id != null) app.changes.bump();
      if (mounted) {
        setState(() => _saving = false);
        showError(context, e);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return ListenableBuilder(
      listenable: _s,
      builder: (context, _) {
        final pages = _s.pages;
        return Scaffold(
          appBar: AppBar(title: Text('${l.scanReviewTitle} · ${l.scanPageCount(pages.length)}')),
          body: pages.isEmpty
              ? EmptyState(icon: Icons.filter_none, title: l.scanEmptyReview)
              : ReorderableListView.builder(
                  padding: const EdgeInsets.all(TsSpace.s3),
                  buildDefaultDragHandles: false,
                  itemCount: pages.length,
                  onReorderItem: _s.move,
                  itemBuilder: (context, i) => _PageTile(
                    key: ObjectKey(pages[i]),
                    index: i,
                    page: pages[i],
                    count: pages.length,
                    enabled: !_saving,
                    onEdit: () => _edit(i),
                    onDelete: () => _s.remove(i),
                    onMove: (to) => _s.move(i, to),
                  ),
                ),
          bottomNavigationBar: SafeArea(
            child: Padding(
              padding: const EdgeInsets.all(TsSpace.s3),
              child: _saving
                  ? ValueListenableBuilder<int>(
                      valueListenable: _done,
                      builder: (_, n, _) => Column(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          LinearProgressIndicator(value: pages.isEmpty ? null : n / pages.length),
                          const SizedBox(height: TsSpace.s2),
                          Text(l.scanSaving(n, pages.length)),
                        ],
                      ),
                    )
                  : Row(
                      children: [
                        if (widget.canCaptureMore)
                          Expanded(
                            child: OutlinedButton.icon(
                              onPressed: () => Navigator.of(context).pop(const ReviewCaptureMore()),
                              icon: const Icon(Icons.add_a_photo_outlined),
                              label: Text(l.scanAddMore),
                            ),
                          ),
                        if (widget.canCaptureMore) const SizedBox(width: TsSpace.s3),
                        Expanded(
                          child: FilledButton.icon(
                            onPressed: pages.isEmpty ? null : _save,
                            icon: const Icon(Icons.save_outlined),
                            label: Text(l.scanSaveDocument),
                          ),
                        ),
                      ],
                    ),
            ),
          ),
        );
      },
    );
  }
}

class _PageTile extends StatelessWidget {
  const _PageTile({
    super.key,
    required this.index,
    required this.page,
    required this.count,
    required this.enabled,
    required this.onEdit,
    required this.onDelete,
    required this.onMove,
  });

  final int index;
  final ScanDraftPage page;
  final int count;
  final bool enabled;
  final VoidCallback onEdit;
  final VoidCallback onDelete;
  final ValueChanged<int> onMove;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return Card(
      margin: const EdgeInsets.only(bottom: TsSpace.s3),
      child: Padding(
        padding: const EdgeInsets.all(TsSpace.s2),
        child: Row(
          children: [
            ReorderableDragStartListener(
              index: index,
              enabled: enabled,
              child: const Padding(padding: EdgeInsets.all(TsSpace.s2), child: Icon(Icons.drag_indicator)),
            ),
            Semantics(
              button: true,
              label: '${l.scanPageN(index + 1)}, ${l.scanEditPage}',
              child: InkWell(
                onTap: enabled ? onEdit : null,
                child: SizedBox(
                  width: 96,
                  height: 128,
                  child: Image.memory(page.thumb, fit: BoxFit.contain, gaplessPlayback: true),
                ),
              ),
            ),
            const SizedBox(width: TsSpace.s3),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(l.scanPageN(index + 1), style: Theme.of(context).textTheme.titleSmall),
                  const SizedBox(height: TsSpace.s1),
                  Text(page.filter.label(l), style: Theme.of(context).textTheme.bodySmall),
                ],
              ),
            ),
            IconButton(
              tooltip: l.scanEditPage,
              onPressed: enabled ? onEdit : null,
              icon: const Icon(Icons.crop_rotate),
            ),
            PopupMenuButton<String>(
              enabled: enabled,
              onSelected: (v) => switch (v) {
                'up' => onMove(index - 1),
                'down' => onMove(index + 1),
                _ => onDelete(),
              },
              itemBuilder: (_) => [
                if (index > 0) PopupMenuItem(value: 'up', child: Text(l.scanMoveUp)),
                if (index < count - 1) PopupMenuItem(value: 'down', child: Text(l.scanMoveDown)),
                PopupMenuItem(value: 'delete', child: Text(l.scanDeletePage)),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

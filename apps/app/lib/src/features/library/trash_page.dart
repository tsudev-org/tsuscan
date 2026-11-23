import 'package:flutter/material.dart';

import '../../../l10n/app_localizations.dart';
import '../../app_scope.dart';
import '../../data/library_repository.dart';
import '../../ui/theme/app_theme.dart';
import '../../ui/widgets/feedback.dart';
import '../../utils/format.dart';

/// Thùng rác: khôi phục, xóa vĩnh viễn từng tài liệu, dọn sạch. Tài liệu tự xóa hẳn sau [trashRetentionDays] ngày.
class TrashScreen extends StatefulWidget {
  const TrashScreen({super.key});

  @override
  State<TrashScreen> createState() => _TrashScreenState();
}

class _TrashScreenState extends State<TrashScreen> {
  List<TrashedDocument>? _items;
  Object? _error;
  LibraryChanges? _changes;

  AppScope get _app => AppScope.of(context);

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final changes = _app.changes;
    if (!identical(changes, _changes)) {
      _changes?.removeListener(_reload);
      _changes = changes..addListener(_reload);
      _reload();
    }
  }

  @override
  void dispose() {
    _changes?.removeListener(_reload);
    super.dispose();
  }

  Future<void> _reload() async {
    try {
      final items = await _app.repository.listTrash();
      if (mounted) {
        setState(() {
          _items = items;
          _error = null;
        });
      }
    } on Object catch (e) {
      if (mounted) setState(() => _error = e);
    }
  }

  Future<void> _run(Future<void> Function() action, {String? done}) async {
    try {
      await action();
      _app.changes.bump();
      if (done != null && mounted) showInfo(context, done);
    } on Object catch (e) {
      if (mounted) showError(context, e);
    }
  }

  Future<void> _restore(TrashedDocument t) {
    final l = AppLocalizations.of(context);
    return _run(() => _app.repository.restoreDocument(t.document.id), done: l.trashRestored(t.document.title));
  }

  Future<void> _purge(TrashedDocument t) async {
    final l = AppLocalizations.of(context);
    final ok = await confirmDanger(
      context,
      title: l.confirmPurgeTitle,
      body: l.confirmPurgeBody(t.document.title),
      action: l.actionDeleteForever,
    );
    if (ok) await _run(() => _app.repository.purgeDocument(t.document.id));
  }

  Future<void> _emptyAll() async {
    final l = AppLocalizations.of(context);
    final n = _items?.length ?? 0;
    if (n == 0) return;
    final ok = await confirmDanger(
      context,
      title: l.confirmEmptyTrashTitle,
      body: l.confirmEmptyTrashBody(n),
      action: l.trashEmptyAll,
    );
    if (ok) await _run(() => _app.repository.purgeTrash(expiredOnly: false));
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final p = context.palette;
    final items = _items;
    final Widget body;
    if (items == null) {
      body = _error == null
          ? const Center(child: CircularProgressIndicator())
          : EmptyState(
              icon: Icons.error_outline,
              title: describeError(l, _error!),
              action: OutlinedButton(onPressed: _reload, child: Text(l.actionRetry)),
            );
    } else if (items.isEmpty) {
      body = EmptyState(icon: Icons.delete_outline, title: l.trashEmpty);
    } else {
      body = ListView.separated(
        padding: const EdgeInsets.only(bottom: TsSpace.s6),
        itemCount: items.length + 1,
        separatorBuilder: (_, _) => Divider(height: 1, color: p.border),
        itemBuilder: (_, i) {
          if (i == 0) {
            return Padding(
              padding: const EdgeInsets.all(TsSpace.s4),
              child: Text(l.trashHint(trashRetentionDays), style: TextStyle(color: p.textSecondary)),
            );
          }
          final t = items[i - 1];
          return ListTile(
            leading: const Icon(Icons.description_outlined),
            title: Text(t.document.title, maxLines: 1, overflow: TextOverflow.ellipsis),
            subtitle: Text(
              '${l.libraryPages(t.document.pageCount)} · ${l.trashPurgeOn(formatDateVN(fromMillis(t.purgeAtMs)))}',
            ),
            trailing: Wrap(
              children: [
                IconButton(tooltip: l.actionRestore, icon: const Icon(Icons.restore), onPressed: () => _restore(t)),
                IconButton(
                  tooltip: l.actionDeleteForever,
                  icon: Icon(Icons.delete_forever_outlined, color: p.danger),
                  onPressed: () => _purge(t),
                ),
              ],
            ),
          );
        },
      );
    }
    return Scaffold(
      appBar: AppBar(
        title: Text(l.trashTitle),
        actions: [
          if (items != null && items.isNotEmpty)
            TextButton(
              onPressed: _emptyAll,
              child: Text(l.trashEmptyAll, style: TextStyle(color: p.danger)),
            ),
          const SizedBox(width: TsSpace.s2),
        ],
      ),
      body: body,
    );
  }
}

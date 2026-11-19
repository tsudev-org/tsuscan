import 'package:flutter/material.dart';

import '../../../l10n/app_localizations.dart';
import '../../app_scope.dart';
import '../../data/library_repository.dart';
import '../../ui/theme/app_theme.dart';
import '../../ui/widgets/feedback.dart';
import '../../utils/format.dart';
import 'import_images.dart';
import 'page_thumbnail.dart';

/// Chi tiết tài liệu: lưới trang, nhãn, đổi tên/chuyển thư mục/xóa, thêm trang từ ảnh.
class DocumentScreen extends StatefulWidget {
  const DocumentScreen({super.key, required this.documentId});
  final String documentId;

  @override
  State<DocumentScreen> createState() => _DocumentPageState();
}

class _DocumentPageState extends State<DocumentScreen> {
  Document? _doc;
  List<PageInfo> _pages = const [];
  List<TagDto> _tags = const [];
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
    final repo = _app.repository;
    try {
      final doc = await repo.document(widget.documentId);
      final pages = await repo.listPages(widget.documentId);
      final tags = await repo.documentTags(widget.documentId);
      if (!mounted) return;
      setState(() {
        _doc = doc;
        _pages = pages;
        _tags = tags;
        _error = null;
      });
    } on ApiError catch (e) {
      if (!mounted) return;
      // Tài liệu bị xóa (ở màn này hoặc nơi khác) ⇒ đóng màn.
      if (e.kind == ApiErrorKind.notFound) {
        Navigator.of(context).maybePop();
        return;
      }
      setState(() => _error = e);
    } on Object catch (e) {
      if (mounted) setState(() => _error = e);
    }
  }

  Future<void> _run(Future<void> Function() action) async {
    try {
      await action();
      _app.changes.bump();
    } on Object catch (e) {
      if (mounted) showError(context, e);
    }
  }

  Future<void> _rename() async {
    final l = AppLocalizations.of(context);
    final title = await askName(context, title: l.actionRename, label: l.dialogDocumentTitle, initial: _doc!.title);
    if (title != null) await _run(() => _app.repository.renameDocument(widget.documentId, title));
  }

  Future<void> _move() async {
    final l = AppLocalizations.of(context);
    final folders = await _app.repository.listFolders();
    if (!mounted) return;
    final current = _doc!.folderId;
    final choice = await showDialog<(String?,)>(
      context: context,
      builder: (c) => SimpleDialog(
        title: Text(l.actionMove),
        children: [
          _choice(c, l.documentMoveRoot, Icons.inbox_outlined, current == null, (null,)),
          for (final f in folders) _choice(c, f.name, Icons.folder_outlined, current == f.id, (f.id,)),
        ],
      ),
    );
    if (choice != null && choice.$1 != current) {
      await _run(() => _app.repository.moveDocument(widget.documentId, choice.$1));
    }
  }

  Widget _choice(BuildContext c, String label, IconData icon, bool selected, (String?,) value) => ListTile(
    leading: Icon(icon),
    title: Text(label),
    trailing: selected ? Icon(Icons.check, color: c.palette.primary) : null,
    onTap: () => Navigator.of(c).pop(value),
  );

  Future<void> _editTags() async {
    final l = AppLocalizations.of(context);
    final repo = _app.repository;
    final all = await repo.listTags();
    if (!mounted) return;
    final on = {for (final t in _tags) t.id};
    await showDialog<void>(
      context: context,
      builder: (c) => StatefulBuilder(
        builder: (c, setLocal) => AlertDialog(
          title: Text(l.documentTagsTitle),
          content: SizedBox(
            width: 440,
            child: ListView(
              shrinkWrap: true,
              children: [
                for (final t in all)
                  CheckboxListTile(
                    value: on.contains(t.id),
                    title: Text(t.name),
                    onChanged: (v) async {
                      try {
                        await repo.setTag(widget.documentId, t.id, on: v ?? false);
                        setLocal(() => (v ?? false) ? on.add(t.id) : on.remove(t.id));
                        _app.changes.bump();
                      } on Object catch (e) {
                        if (mounted) showError(context, e);
                      }
                    },
                  ),
                ListTile(
                  leading: const Icon(Icons.add),
                  title: Text(l.libraryNewTag),
                  onTap: () async {
                    final name = await askName(c, title: l.libraryNewTag, label: l.dialogTagName);
                    if (name == null) return;
                    try {
                      final id = await repo.createTag(name);
                      await repo.setTag(widget.documentId, id, on: true);
                      final fresh = await repo.listTags();
                      setLocal(() {
                        all
                          ..clear()
                          ..addAll(fresh);
                        on.add(id);
                      });
                      _app.changes.bump();
                    } on Object catch (e) {
                      if (mounted) showError(context, e);
                    }
                  },
                ),
              ],
            ),
          ),
          actions: [TextButton(onPressed: () => Navigator.of(c).pop(), child: Text(l.actionClose))],
        ),
      ),
    );
  }

  Future<void> _delete() async {
    final l = AppLocalizations.of(context);
    final ok = await confirmDanger(
      context,
      title: l.confirmDeleteDocumentTitle,
      body: l.confirmDeleteDocumentBody(_doc!.title),
    );
    if (!ok || !mounted) return;
    try {
      await _app.repository.deleteDocument(widget.documentId);
      if (mounted) Navigator.of(context).pop();
      _app.changes.bump();
    } on Object catch (e) {
      if (mounted) showError(context, e);
    }
  }

  Future<void> _deletePage(PageInfo page) async {
    final l = AppLocalizations.of(context);
    final ok = await confirmDanger(context, title: l.confirmDeletePageTitle, body: l.confirmDeletePageBody);
    if (ok) await _run(() => _app.repository.deletePage(page.id));
  }

  void _view(PageInfo page) {
    final l = AppLocalizations.of(context);
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => Scaffold(
          appBar: AppBar(
            title: Text(l.documentPageLabel(page.position + 1)),
            actions: [
              IconButton(
                tooltip: l.actionDelete,
                icon: const Icon(Icons.delete_outline),
                onPressed: () async {
                  final nav = Navigator.of(context);
                  await _deletePage(page);
                  if (!_pages.any((p) => p.id == page.id)) nav.pop();
                },
              ),
            ],
          ),
          body: Column(
            children: [
              Expanded(
                child: InteractiveViewer(
                  maxScale: 6,
                  child: Center(
                    child: PageThumbnail(pageId: page.id, maxSide: 1024, fit: BoxFit.contain),
                  ),
                ),
              ),
              if (page.ocrText.isEmpty)
                Padding(
                  padding: const EdgeInsets.all(TsSpace.s4),
                  child: Text(l.documentOcrPending, style: TextStyle(color: context.palette.textMuted)),
                ),
            ],
          ),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final p = context.palette;
    final doc = _doc;
    if (doc == null) {
      return Scaffold(
        appBar: AppBar(),
        body: _error == null
            ? const Center(child: CircularProgressIndicator())
            : EmptyState(
                icon: Icons.error_outline,
                title: describeError(l, _error!),
                action: OutlinedButton(onPressed: _reload, child: Text(l.actionRetry)),
              ),
      );
    }
    return Scaffold(
      appBar: AppBar(
        title: Text(doc.title, maxLines: 1, overflow: TextOverflow.ellipsis),
        actions: [
          IconButton(tooltip: l.documentTagsTitle, onPressed: _editTags, icon: const Icon(Icons.sell_outlined)),
          PopupMenuButton<String>(
            onSelected: (v) => switch (v) {
              'rename' => _rename(),
              'move' => _move(),
              _ => _delete(),
            },
            itemBuilder: (_) => [
              PopupMenuItem(value: 'rename', child: Text(l.actionRename)),
              PopupMenuItem(value: 'move', child: Text(l.actionMove)),
              PopupMenuItem(
                value: 'delete',
                child: Text(l.actionDelete, style: TextStyle(color: p.danger)),
              ),
            ],
          ),
        ],
      ),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: () => pickAndImportInto(context, widget.documentId),
        icon: const Icon(Icons.add_photo_alternate_outlined),
        label: Text(l.documentAddPages),
      ),
      body: CustomScrollView(
        slivers: [
          SliverToBoxAdapter(
            child: Padding(
              padding: const EdgeInsets.fromLTRB(TsSpace.s4, TsSpace.s2, TsSpace.s4, TsSpace.s2),
              child: Wrap(
                spacing: TsSpace.s2,
                runSpacing: TsSpace.s2,
                crossAxisAlignment: WrapCrossAlignment.center,
                children: [
                  Text(
                    '${l.libraryPages(doc.pageCount)} · ${l.libraryUpdated(formatDateTimeVN(fromMillis(doc.updatedMs)))}',
                    style: TextStyle(color: p.textSecondary, fontSize: TsFontSize.sm),
                  ),
                  for (final t in _tags)
                    Chip(
                      avatar: const Icon(Icons.sell_outlined, size: 14),
                      label: Text(t.name),
                      visualDensity: VisualDensity.compact,
                    ),
                ],
              ),
            ),
          ),
          if (_pages.isEmpty)
            SliverFillRemaining(
              hasScrollBody: false,
              child: EmptyState(
                icon: Icons.add_photo_alternate_outlined,
                title: l.documentNoPages,
                action: FilledButton.icon(
                  onPressed: () => pickAndImportInto(context, widget.documentId),
                  icon: const Icon(Icons.add_photo_alternate_outlined),
                  label: Text(l.documentAddPages),
                ),
              ),
            )
          else
            SliverPadding(
              padding: const EdgeInsets.fromLTRB(TsSpace.s4, TsSpace.s2, TsSpace.s4, 96),
              sliver: SliverGrid.builder(
                gridDelegate: const SliverGridDelegateWithMaxCrossAxisExtent(
                  maxCrossAxisExtent: 200,
                  mainAxisSpacing: TsSpace.s3,
                  crossAxisSpacing: TsSpace.s3,
                  childAspectRatio: 0.72,
                ),
                itemCount: _pages.length,
                itemBuilder: (_, i) => _PageCard(
                  page: _pages[i],
                  label: l.documentPageLabel(i + 1),
                  onTap: () => _view(_pages[i]),
                  onDelete: () => _deletePage(_pages[i]),
                  deleteLabel: l.actionDelete,
                ),
              ),
            ),
        ],
      ),
    );
  }
}

class _PageCard extends StatelessWidget {
  const _PageCard({
    required this.page,
    required this.label,
    required this.onTap,
    required this.onDelete,
    required this.deleteLabel,
  });
  final PageInfo page;
  final String label;
  final VoidCallback onTap;
  final VoidCallback onDelete;
  final String deleteLabel;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    return Semantics(
      button: true,
      label: label,
      child: Card(
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          onTap: onTap,
          onLongPress: onDelete,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Expanded(child: PageThumbnail(pageId: page.id)),
              Divider(height: 1, color: p.border),
              Row(
                children: [
                  Expanded(
                    child: Padding(
                      padding: const EdgeInsets.only(left: TsSpace.s3),
                      child: Text(label, style: const TextStyle(fontSize: TsFontSize.sm)),
                    ),
                  ),
                  IconButton(
                    tooltip: deleteLabel,
                    icon: Icon(Icons.delete_outline, size: 18, color: p.textSecondary),
                    onPressed: onDelete,
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}

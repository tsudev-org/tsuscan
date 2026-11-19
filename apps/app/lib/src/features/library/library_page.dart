import 'package:flutter/material.dart';

import '../../../l10n/app_localizations.dart';
import '../../app_scope.dart';
import '../../data/library_repository.dart';
import '../../ui/theme/app_theme.dart';
import '../../ui/widgets/feedback.dart';
import '../../ui/widgets/pager_footer.dart';
import '../../utils/format.dart';
import 'document_page.dart';
import 'import_images.dart';
import 'page_thumbnail.dart';

/// Mã bảng để nhớ số bản ghi mỗi trang (`DATA_TABLE.md` mục 5).
const libraryTableId = 'library.documents';

/// Thư viện: lọc theo thư mục/nhãn, danh sách tài liệu có phân trang.
class LibraryPage extends StatefulWidget {
  const LibraryPage({super.key});

  @override
  State<LibraryPage> createState() => _LibraryPageState();
}

class _LibraryPageState extends State<LibraryPage> {
  FolderScope _scope = const AllFolders();
  String? _tagId;
  int _offset = 0;
  int? _pageSize;
  bool _loading = true;
  Object? _error;
  DocumentPage? _page;
  int? _unfilteredTotal;
  List<FolderDto> _folders = const [];
  List<TagDto> _tags = const [];
  LibraryChanges? _changes;

  AppScope get _app => AppScope.of(context);

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _pageSize ??= _app.settings.pageSize(libraryTableId);
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
    setState(() => _loading = true);
    try {
      final folders = await repo.listFolders();
      final tags = await repo.listTags();
      // Thư mục/nhãn đang lọc vừa bị xóa ở màn khác ⇒ quay về "Tất cả".
      if (_scope case InFolder(:final id) when !folders.any((f) => f.id == id)) _scope = const AllFolders();
      if (_tagId != null && !tags.any((t) => t.id == _tagId)) _tagId = null;
      var page = await repo.listDocuments(_scope, tagId: _tagId, limit: _pageSize!, offset: _offset);
      // Trang hiện tại rỗng sau khi xóa ⇒ lùi về trang cuối còn dữ liệu.
      if (page.items.isEmpty && _offset > 0 && page.total > 0) {
        final total = page.total;
        _offset = ((total - 1) ~/ _pageSize!) * _pageSize!;
        page = await repo.listDocuments(_scope, tagId: _tagId, limit: _pageSize!, offset: _offset);
      }
      final filtered = _scope is! AllFolders || _tagId != null;
      final unfiltered = filtered ? (await repo.listDocuments(const AllFolders(), limit: 1, offset: 0)).total : null;
      if (!mounted) return;
      setState(() {
        _folders = folders;
        _tags = tags;
        _page = page;
        _unfilteredTotal = unfiltered;
        _error = null;
        _loading = false;
      });
    } on Object catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e;
        _loading = false;
      });
    }
  }

  void _setFilter({FolderScope? scope, String? tagId, bool clearTag = false}) {
    setState(() {
      _scope = scope ?? _scope;
      _tagId = clearTag ? null : (tagId ?? _tagId);
      _offset = 0; // Bộ lọc đổi ⇒ về trang 1, giữ nguyên mốc.
    });
    _reload();
  }

  Future<void> _run(Future<void> Function() action) async {
    try {
      await action();
      _app.changes.bump();
    } on Object catch (e) {
      if (mounted) showError(context, e);
    }
  }

  Future<void> _newDocument() async {
    final l = AppLocalizations.of(context);
    final title = await askName(
      context,
      title: l.libraryNewDocument,
      label: l.dialogDocumentTitle,
      initial: l.dialogDefaultDocumentTitle(formatDateTimeVN(DateTime.now())),
    );
    if (title == null || !mounted) return;
    final folderId = switch (_scope) {
      InFolder(:final id) => id,
      _ => null,
    };
    try {
      final id = await _app.repository.createDocument(title, folderId: folderId);
      if (_tagId != null) await _app.repository.setTag(id, _tagId!, on: true);
      _app.changes.bump();
      if (mounted) await _open(id);
    } on Object catch (e) {
      if (mounted) showError(context, e);
    }
  }

  Future<void> _importAsNew() async {
    final l = AppLocalizations.of(context);
    final repo = _app.repository;
    try {
      final images = await pickImages();
      if (images.isEmpty || !mounted) return;
      final folderId = switch (_scope) {
        InFolder(:final id) => id,
        _ => null,
      };
      final id = await repo.createDocument(
        l.dialogDefaultDocumentTitle(formatDateTimeVN(DateTime.now())),
        folderId: folderId,
      );
      final r = await importImages(repo, id, images);
      _app.changes.bump();
      if (mounted) reportImport(context, r);
    } on Object catch (e) {
      if (mounted) showError(context, e);
    }
  }

  Future<void> _newFolder() async {
    final l = AppLocalizations.of(context);
    final name = await askName(context, title: l.libraryNewFolder, label: l.dialogFolderName);
    if (name == null) return;
    await _run(() async {
      final id = await _app.repository.createFolder(name);
      _scope = InFolder(id);
      _offset = 0;
    });
  }

  Future<void> _newTag() async {
    final l = AppLocalizations.of(context);
    final name = await askName(context, title: l.libraryNewTag, label: l.dialogTagName);
    if (name == null) return;
    await _run(() => _app.repository.createTag(name));
  }

  Future<void> _folderMenu(FolderDto f, String action) async {
    final l = AppLocalizations.of(context);
    if (action == 'rename') {
      final name = await askName(context, title: l.actionRename, label: l.dialogFolderName, initial: f.name);
      if (name != null) await _run(() => _app.repository.renameFolder(f.id, name));
    } else if (action == 'delete') {
      final ok = await confirmDanger(
        context,
        title: l.confirmDeleteFolderTitle,
        body: l.confirmDeleteFolderBody(f.name),
      );
      if (ok) await _run(() => _app.repository.deleteFolder(f.id));
    }
  }

  Future<void> _deleteTag(TagDto t) async {
    final l = AppLocalizations.of(context);
    final ok = await confirmDanger(context, title: l.confirmDeleteTagTitle, body: l.confirmDeleteTagBody(t.name));
    if (ok) await _run(() => _app.repository.deleteTag(t.id));
  }

  Future<void> _open(String id) =>
      Navigator.of(context).push(MaterialPageRoute<void>(builder: (_) => DocumentScreen(documentId: id)));

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return LayoutBuilder(
      builder: (context, c) {
        final wide = c.maxWidth >= 900;
        final filters = _Filters(
          vertical: wide,
          folders: _folders,
          tags: _tags,
          scope: _scope,
          tagId: _tagId,
          onScope: (s) => _setFilter(scope: s),
          onTag: (id) => id == _tagId ? _setFilter(clearTag: true) : _setFilter(tagId: id),
          onFolderMenu: _folderMenu,
          onDeleteTag: _deleteTag,
          onNewFolder: _newFolder,
          onNewTag: _newTag,
        );
        final list = _DocumentList(
          page: _page,
          loading: _loading,
          error: _error,
          filtered: _scope is! AllFolders || _tagId != null,
          onOpen: _open,
          onRetry: _reload,
          onClearFilter: () => _setFilter(scope: const AllFolders(), clearTag: true),
          onNew: _newDocument,
          onImport: _importAsNew,
        );
        final total = _page?.total ?? 0;
        final footer = total == 0
            ? null
            : PagerFooter(
                total: total,
                offset: _offset,
                pageSize: _pageSize!,
                unfilteredTotal: _unfilteredTotal,
                loading: _loading,
                onPageSize: (size, offset) {
                  _app.settings.setPageSize(libraryTableId, size);
                  setState(() {
                    _pageSize = size;
                    _offset = offset;
                  });
                  _reload();
                },
                onOffset: (o) {
                  setState(() => _offset = o);
                  _reload();
                },
              );
        final body = wide
            ? Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  SizedBox(width: 280, child: filters),
                  VerticalDivider(width: 1, color: context.palette.border),
                  Expanded(
                    child: Column(
                      children: [
                        Expanded(child: list),
                        ?footer,
                      ],
                    ),
                  ),
                ],
              )
            : Column(
                children: [
                  filters,
                  Expanded(child: list),
                ],
              );
        return Scaffold(
          appBar: AppBar(
            title: Text(l.navLibrary),
            actions: [
              IconButton(
                tooltip: l.libraryImportImages,
                onPressed: _importAsNew,
                icon: const Icon(Icons.add_photo_alternate_outlined),
              ),
              IconButton(
                tooltip: l.libraryNewFolder,
                onPressed: _newFolder,
                icon: const Icon(Icons.create_new_folder_outlined),
              ),
              const SizedBox(width: TsSpace.s2),
            ],
          ),
          // Hẹp: chân vùng bản ghi là bottomNavigationBar ⇒ Scaffold tự đặt FAB phía trên. Rộng: đẩy FAB lên 48px
          // để không che phân trang.
          bottomNavigationBar: wide ? null : footer,
          floatingActionButton: Padding(
            padding: EdgeInsets.only(bottom: wide && footer != null ? 48 : 0),
            child: FloatingActionButton.extended(
              onPressed: _newDocument,
              icon: const Icon(Icons.note_add_outlined),
              label: Text(l.libraryNewDocument),
            ),
          ),
          body: SafeArea(top: false, child: body),
        );
      },
    );
  }
}

class _Filters extends StatelessWidget {
  const _Filters({
    required this.vertical,
    required this.folders,
    required this.tags,
    required this.scope,
    required this.tagId,
    required this.onScope,
    required this.onTag,
    required this.onFolderMenu,
    required this.onDeleteTag,
    required this.onNewFolder,
    required this.onNewTag,
  });

  final bool vertical;
  final List<FolderDto> folders;
  final List<TagDto> tags;
  final FolderScope scope;
  final String? tagId;
  final ValueChanged<FolderScope> onScope;
  final ValueChanged<String> onTag;
  final Future<void> Function(FolderDto, String) onFolderMenu;
  final ValueChanged<TagDto> onDeleteTag;
  final VoidCallback onNewFolder;
  final VoidCallback onNewTag;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final p = context.palette;
    Widget folderMenu(FolderDto f) => PopupMenuButton<String>(
      tooltip: f.name,
      icon: Icon(Icons.more_vert, size: 18, color: p.textSecondary),
      onSelected: (a) => onFolderMenu(f, a),
      itemBuilder: (_) => [
        PopupMenuItem(value: 'rename', child: Text(l.actionRename)),
        PopupMenuItem(value: 'delete', child: Text(l.actionDelete)),
      ],
    );

    if (vertical) {
      final t = Theme.of(context).textTheme;
      Widget header(String text, VoidCallback onAdd, String addLabel) => Padding(
        padding: const EdgeInsets.fromLTRB(TsSpace.s4, TsSpace.s4, TsSpace.s2, TsSpace.s1),
        child: Row(
          children: [
            Expanded(
              child: Text(
                text,
                style: t.labelLarge?.copyWith(color: p.textSecondary, fontWeight: FontWeight.w600),
              ),
            ),
            IconButton(tooltip: addLabel, onPressed: onAdd, icon: const Icon(Icons.add, size: 20)),
          ],
        ),
      );
      Widget item(String label, IconData icon, bool selected, VoidCallback onTap, {Widget? trailing}) => ListTile(
        dense: true,
        selected: selected,
        selectedTileColor: p.bgSubtle,
        selectedColor: p.textPrimary,
        leading: Icon(icon, size: 20, color: selected ? p.primary : p.textSecondary),
        title: Text(label, maxLines: 1, overflow: TextOverflow.ellipsis),
        trailing: trailing,
        onTap: onTap,
      );
      return Material(
        color: p.bgSurface,
        child: ListView(
          children: [
            header(l.libraryFolders, onNewFolder, l.libraryNewFolder),
            item(l.libraryAllDocuments, Icons.inventory_2_outlined, scope is AllFolders, () {
              onScope(const AllFolders());
            }),
            item(l.libraryRootFolder, Icons.inbox_outlined, scope is RootFolder, () => onScope(const RootFolder())),
            for (final f in folders)
              item(
                f.name,
                Icons.folder_outlined,
                scope == InFolder(f.id),
                () => onScope(InFolder(f.id)),
                trailing: folderMenu(f),
              ),
            header(l.libraryTags, onNewTag, l.libraryNewTag),
            for (final tag in tags)
              item(
                tag.name,
                Icons.sell_outlined,
                tag.id == tagId,
                () => onTag(tag.id),
                trailing: IconButton(
                  tooltip: l.actionDelete,
                  icon: Icon(Icons.close, size: 18, color: p.textSecondary),
                  onPressed: () => onDeleteTag(tag),
                ),
              ),
          ],
        ),
      );
    }

    Widget chip(String label, bool selected, VoidCallback onTap, {IconData? icon, VoidCallback? onLong}) => Padding(
      padding: const EdgeInsets.only(right: TsSpace.s2),
      child: GestureDetector(
        onLongPress: onLong,
        child: FilterChip(
          avatar: icon == null ? null : Icon(icon, size: 16),
          label: Text(label),
          selected: selected,
          showCheckmark: false,
          onSelected: (_) => onTap(),
        ),
      ),
    );
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          padding: const EdgeInsets.fromLTRB(TsSpace.s4, TsSpace.s2, TsSpace.s4, 0),
          child: Row(
            children: [
              chip(l.libraryAllDocuments, scope is AllFolders, () => onScope(const AllFolders())),
              chip(l.libraryRootFolder, scope is RootFolder, () => onScope(const RootFolder())),
              for (final f in folders)
                chip(
                  f.name,
                  scope == InFolder(f.id),
                  () => onScope(InFolder(f.id)),
                  icon: Icons.folder_outlined,
                  onLong: () => _sheet(context, f),
                ),
            ],
          ),
        ),
        if (tags.isNotEmpty)
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            padding: const EdgeInsets.fromLTRB(TsSpace.s4, TsSpace.s2, TsSpace.s4, 0),
            child: Row(
              children: [
                for (final tag in tags)
                  chip(
                    tag.name,
                    tag.id == tagId,
                    () => onTag(tag.id),
                    icon: Icons.sell_outlined,
                    onLong: () {
                      onDeleteTag(tag);
                    },
                  ),
                ActionChip(avatar: const Icon(Icons.add, size: 16), label: Text(l.libraryNewTag), onPressed: onNewTag),
              ],
            ),
          ),
        const SizedBox(height: TsSpace.s2),
        Divider(height: 1, color: p.border),
      ],
    );
  }

  void _sheet(BuildContext context, FolderDto f) {
    final l = AppLocalizations.of(context);
    showModalBottomSheet<void>(
      context: context,
      builder: (c) => SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            ListTile(title: Text(f.name, style: Theme.of(c).textTheme.titleMedium)),
            ListTile(
              leading: const Icon(Icons.drive_file_rename_outline),
              title: Text(l.actionRename),
              onTap: () {
                Navigator.of(c).pop();
                onFolderMenu(f, 'rename');
              },
            ),
            ListTile(
              leading: const Icon(Icons.delete_outline),
              title: Text(l.actionDelete),
              onTap: () {
                Navigator.of(c).pop();
                onFolderMenu(f, 'delete');
              },
            ),
          ],
        ),
      ),
    );
  }
}

class _DocumentList extends StatelessWidget {
  const _DocumentList({
    required this.page,
    required this.loading,
    required this.error,
    required this.filtered,
    required this.onOpen,
    required this.onRetry,
    required this.onClearFilter,
    required this.onNew,
    required this.onImport,
  });

  final DocumentPage? page;
  final bool loading;
  final Object? error;
  final bool filtered;
  final ValueChanged<String> onOpen;
  final VoidCallback onRetry;
  final VoidCallback onClearFilter;
  final VoidCallback onNew;
  final VoidCallback onImport;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final p = page;
    if (error != null && p == null) {
      return EmptyState(
        icon: Icons.error_outline,
        title: describeError(l, error!),
        action: OutlinedButton(onPressed: onRetry, child: Text(l.actionRetry)),
      );
    }
    if (p == null) return const Center(child: CircularProgressIndicator());
    if (p.items.isEmpty) {
      return filtered
          ? EmptyState(
              icon: Icons.filter_alt_off_outlined,
              title: l.libraryNoMatch,
              action: OutlinedButton(onPressed: onClearFilter, child: Text(l.libraryClearFilter)),
            )
          : EmptyState(
              icon: Icons.document_scanner_outlined,
              title: l.libraryEmptyTitle,
              body: l.libraryEmptyBody,
              action: Wrap(
                spacing: TsSpace.s2,
                runSpacing: TsSpace.s2,
                alignment: WrapAlignment.center,
                children: [
                  FilledButton.icon(
                    onPressed: onNew,
                    icon: const Icon(Icons.note_add_outlined),
                    label: Text(l.libraryNewDocument),
                  ),
                  OutlinedButton.icon(
                    onPressed: onImport,
                    icon: const Icon(Icons.add_photo_alternate_outlined),
                    label: Text(l.libraryImportImages),
                  ),
                ],
              ),
            );
    }
    return AnimatedOpacity(
      opacity: loading ? 0.6 : 1,
      duration: Duration(milliseconds: TsMotion.fast.round()),
      child: ListView.separated(
        padding: const EdgeInsets.only(bottom: 88),
        itemCount: p.items.length,
        separatorBuilder: (_, _) => Divider(height: 1, color: context.palette.border),
        itemBuilder: (_, i) => DocumentTile(doc: p.items[i], onTap: () => onOpen(p.items[i].id)),
      ),
    );
  }
}

/// Một dòng tài liệu: ảnh trang đầu, tên, số trang, giờ sửa.
class DocumentTile extends StatelessWidget {
  const DocumentTile({super.key, required this.doc, required this.onTap});
  final Document doc;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final p = context.palette;
    return ListTile(
      onTap: onTap,
      contentPadding: const EdgeInsets.symmetric(horizontal: TsSpace.s4, vertical: TsSpace.s1),
      leading: ClipRRect(
        borderRadius: BorderRadius.circular(TsRadius.sm),
        child: SizedBox(
          width: 44,
          height: 56,
          child: _FirstPage(key: ValueKey('${doc.id}@${doc.updatedMs}'), documentId: doc.id, pages: doc.pageCount),
        ),
      ),
      title: Text(doc.title, maxLines: 2, overflow: TextOverflow.ellipsis),
      subtitle: Text(
        '${l.libraryPages(doc.pageCount)} · ${l.libraryUpdated(formatDateTimeVN(fromMillis(doc.updatedMs)))}',
        style: TextStyle(color: p.textSecondary, fontSize: TsFontSize.sm),
      ),
      trailing: Icon(Icons.chevron_right, color: p.textMuted),
    );
  }
}

class _FirstPage extends StatefulWidget {
  const _FirstPage({super.key, required this.documentId, required this.pages});
  final String documentId;
  final int pages;

  @override
  State<_FirstPage> createState() => _FirstPageState();
}

class _FirstPageState extends State<_FirstPage> {
  Future<List<PageInfo>>? _pages;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (widget.pages > 0) _pages ??= AppScope.of(context).repository.listPages(widget.documentId);
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final empty = ColoredBox(
      color: p.bgSubtle,
      child: Icon(Icons.description_outlined, color: p.textMuted),
    );
    if (_pages == null) return empty;
    return FutureBuilder<List<PageInfo>>(
      future: _pages,
      builder: (_, snap) {
        final first = snap.data?.firstOrNull;
        return first == null ? empty : PageThumbnail(pageId: first.id, maxSide: 128);
      },
    );
  }
}

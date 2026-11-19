import 'dart:async';

import 'package:flutter/material.dart';

import '../../../l10n/app_localizations.dart';
import '../../app_scope.dart';
import '../../data/library_repository.dart';
import '../../ui/theme/app_theme.dart';
import '../../ui/widgets/feedback.dart';
import '../../ui/widgets/highlighted_text.dart';
import '../../ui/widgets/pager_footer.dart';
import '../library/document_page.dart';

/// Mã bảng kết quả tìm kiếm (nhớ số bản ghi mỗi trang).
const searchTableId = 'search.results';

/// Tìm không dấu trên tên tài liệu + chữ OCR (FTS5 trong lõi Rust).
class SearchPage extends StatefulWidget {
  const SearchPage({super.key});

  @override
  State<SearchPage> createState() => _SearchPageState();
}

class _SearchPageState extends State<SearchPage> {
  final _query = TextEditingController();
  Timer? _debounce;
  List<SearchHitDto> _hits = const [];
  String _shown = '';
  bool _loading = false;
  int _offset = 0;
  int? _pageSize;
  int _seq = 0;
  LibraryChanges? _changes;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final app = AppScope.of(context);
    _pageSize ??= app.settings.pageSize(searchTableId);
    if (!identical(app.changes, _changes)) {
      _changes?.removeListener(_rerun);
      _changes = app.changes..addListener(_rerun);
    }
  }

  @override
  void dispose() {
    _debounce?.cancel();
    _changes?.removeListener(_rerun);
    _query.dispose();
    super.dispose();
  }

  void _rerun() => _search(_query.text, keepOffset: true);

  void _onChanged(String v) {
    _debounce?.cancel();
    _debounce = Timer(const Duration(milliseconds: 250), () => _search(v));
  }

  Future<void> _search(String q, {bool keepOffset = false}) async {
    final seq = ++_seq;
    if (q.trim().runes.length < 2) {
      setState(() {
        _hits = const [];
        _shown = q.trim();
        _offset = 0;
      });
      return;
    }
    setState(() => _loading = true);
    try {
      // Kết quả có trần cứng 200 (mốc lớn nhất) - đủ cho kho trên máy, phân trang trong danh sách đã xếp hạng.
      final hits = await AppScope.of(context).repository.search(q, limit: 200);
      if (!mounted || seq != _seq) return;
      setState(() {
        _hits = hits;
        _shown = q.trim();
        if (!keepOffset || _offset >= hits.length) _offset = 0;
        _loading = false;
      });
    } on Object catch (e) {
      if (!mounted || seq != _seq) return;
      setState(() => _loading = false);
      showError(context, e);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final p = context.palette;
    final repo = AppScope.of(context).repository;
    final visible = _hits.skip(_offset).take(_pageSize!).toList();
    Widget body;
    if (_shown.runes.length < 2) {
      body = EmptyState(icon: Icons.search, title: l.searchMinChars);
    } else if (_hits.isEmpty && !_loading) {
      body = EmptyState(icon: Icons.search_off, title: l.searchNoResults(_shown));
    } else {
      body = ListView.separated(
        itemCount: visible.length,
        separatorBuilder: (_, _) => Divider(height: 1, color: p.border),
        itemBuilder: (_, i) {
          final h = visible[i];
          return ListTile(
            leading: Icon(Icons.description_outlined, color: p.textSecondary),
            title: HighlightedText(
              h.title,
              ranges: repo.highlight(h.title, _shown),
              maxLines: 2,
              highlightStyle: TextStyle(
                fontWeight: FontWeight.w600,
                backgroundColor: p.primary.withValues(alpha: 0.18),
              ),
            ),
            subtitle: h.accentExact
                ? null
                : Text(
                    l.searchAccentLess,
                    style: TextStyle(color: p.textMuted, fontSize: TsFontSize.sm),
                  ),
            trailing: Icon(Icons.chevron_right, color: p.textMuted),
            onTap: () =>
                Navigator.of(context)
                    .push(MaterialPageRoute<void>(builder: (_) => DocumentScreen(documentId: h.documentId))),
          );
        },
      );
    }
    return Scaffold(
      appBar: AppBar(title: Text(l.navSearch)),
      body: SafeArea(
        top: false,
        child: Column(
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(TsSpace.s4, TsSpace.s2, TsSpace.s4, TsSpace.s3),
              child: TextField(
                controller: _query,
                autofocus: false,
                textInputAction: TextInputAction.search,
                onChanged: _onChanged,
                onSubmitted: _search,
                decoration: InputDecoration(
                  prefixIcon: const Icon(Icons.search),
                  hintText: l.searchHint,
                  suffixIcon: _query.text.isEmpty
                      ? null
                      : IconButton(
                          tooltip: l.libraryClearFilter,
                          icon: const Icon(Icons.close),
                          onPressed: () {
                            _query.clear();
                            _search('');
                          },
                        ),
                ),
              ),
            ),
            if (_loading) const LinearProgressIndicator(minHeight: 2),
            Expanded(child: body),
            if (_hits.isNotEmpty)
              PagerFooter(
                total: _hits.length,
                offset: _offset,
                pageSize: _pageSize!,
                loading: _loading,
                onPageSize: (size, offset) {
                  AppScope.of(context).settings.setPageSize(searchTableId, size);
                  setState(() {
                    _pageSize = size;
                    _offset = offset;
                  });
                },
                onOffset: (o) => setState(() => _offset = o),
              ),
          ],
        ),
      ),
    );
  }
}

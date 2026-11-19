import 'package:flutter/material.dart';
import 'package:flutter/semantics.dart';

import '../../../l10n/app_localizations.dart';
import '../../services/settings_store.dart';
import '../theme/app_theme.dart';

/// Chân vùng bản ghi (`DATA_TABLE.md` mục 3-7): trái bộ chọn số bản ghi, giữa tóm tắt, phải phân trang.
class PagerFooter extends StatelessWidget {
  const PagerFooter({
    super.key,
    required this.total,
    required this.offset,
    required this.pageSize,
    required this.onPageSize,
    required this.onOffset,
    this.unfilteredTotal,
    this.loading = false,
  });

  final int total;
  final int offset;
  final int pageSize;

  /// Tổng trước khi lọc - có thì tóm tắt ghi "(lọc từ n)".
  final int? unfilteredTotal;
  final bool loading;

  /// Đổi mốc: nhận mốc mới và offset mới giữ nguyên bản ghi đầu đang thấy.
  final void Function(int pageSize, int offset) onPageSize;
  final ValueChanged<int> onOffset;

  int get _pages => total == 0 ? 1 : (total + pageSize - 1) ~/ pageSize;
  int get _page => offset ~/ pageSize + 1;

  String? _summary(AppLocalizations l) {
    if (total == 0) return null;
    String s;
    if (_pages == 1) {
      s = l.pagerItems(total);
    } else {
      final to = (offset + pageSize).clamp(0, total);
      s = l.pagerSummary(offset + 1, to, total);
    }
    final all = unfilteredTotal;
    if (all != null && all != total) s = l.pagerFiltered(s, all);
    return s;
  }

  void _changeSize(BuildContext context, int size) {
    // Giữ nguyên bản ghi đầu đang thấy: trang_mới = floor(chỉ_số_đầu / mốc_mới) + 1.
    final newOffset = (offset ~/ size) * size;
    onPageSize(size, newOffset);
    final l = AppLocalizations.of(context);
    final to = (newOffset + size).clamp(0, total);
    if (total > 0) {
      SemanticsService.sendAnnouncement(
        View.of(context),
        l.pagerAnnounce(newOffset + 1, to, total),
        Directionality.of(context),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final p = context.palette;
    final muted = TextStyle(fontSize: TsFontSize.sm, color: p.textMuted);
    final summary = _summary(l);
    final selector = Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Semantics(
          label: l.pagerPageSizeLabel,
          child: SizedBox(
            height: 32,
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: p.bgSurface,
                border: Border.all(color: p.borderControl),
                borderRadius: BorderRadius.circular(TsRadius.md),
              ),
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: TsSpace.s2),
                child: DropdownButtonHideUnderline(
                  child: DropdownButton<int>(
                    value: pageSize,
                    isDense: true,
                    focusColor: Colors.transparent,
                    dropdownColor: p.bgSurface,
                    borderRadius: BorderRadius.circular(TsRadius.md),
                    items: [for (final s in SettingsStore.pageSizes) DropdownMenuItem(value: s, child: Text('$s'))],
                    onChanged: loading ? null : (v) => v == null ? null : _changeSize(context, v),
                  ),
                ),
              ),
            ),
          ),
        ),
        const SizedBox(width: TsSpace.s2),
        ExcludeSemantics(child: Text(l.pagerPerPage, style: muted)),
      ],
    );
    return LayoutBuilder(
      builder: (context, c) {
        final narrow = c.maxWidth < 640;
        final box = BoxDecoration(
          color: p.bgSubtle,
          border: Border(top: BorderSide(color: p.border)),
        );
        if (narrow) {
          return DecoratedBox(
            decoration: box,
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: TsSpace.s4, vertical: TsSpace.s2),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Row(
                    children: [
                      selector,
                      const Spacer(),
                      if (summary != null) Text(summary, style: muted),
                    ],
                  ),
                  if (_pages > 1) ...[
                    const SizedBox(height: TsSpace.s2),
                    Row(
                      children: [
                        Expanded(
                          child: OutlinedButton.icon(
                            onPressed: _page > 1 && !loading ? () => onOffset(offset - pageSize) : null,
                            icon: const Icon(Icons.chevron_left),
                            label: Text(l.pagerPrevious),
                          ),
                        ),
                        const SizedBox(width: TsSpace.s2),
                        Expanded(
                          child: OutlinedButton.icon(
                            onPressed: _page < _pages && !loading ? () => onOffset(offset + pageSize) : null,
                            icon: const Icon(Icons.chevron_right),
                            iconAlignment: IconAlignment.end,
                            label: Text(l.pagerNext),
                          ),
                        ),
                      ],
                    ),
                  ],
                ],
              ),
            ),
          );
        }
        return Container(
          height: 48,
          decoration: box,
          padding: const EdgeInsets.symmetric(horizontal: TsSpace.s4),
          child: Row(
            children: [
              selector,
              Expanded(
                child: Center(child: summary == null ? null : Text(summary, style: muted)),
              ),
              _NumberPager(
                page: _page,
                pages: _pages,
                enabled: !loading,
                onPage: (n) => onOffset((n - 1) * pageSize),
                previousLabel: l.pagerPrevious,
                nextLabel: l.pagerNext,
              ),
            ],
          ),
        );
      },
    );
  }
}

/// Phân trang dạng số, tối đa 7 ô: `< 1 … 5 6 7 … 13 >`.
class _NumberPager extends StatelessWidget {
  const _NumberPager({
    required this.page,
    required this.pages,
    required this.enabled,
    required this.onPage,
    required this.previousLabel,
    required this.nextLabel,
  });
  final int page;
  final int pages;
  final bool enabled;
  final ValueChanged<int> onPage;
  final String previousLabel;
  final String nextLabel;

  /// Các ô hiển thị; `null` là dấu "…".
  static List<int?> cells(int page, int pages) {
    if (pages <= 7) return [for (var i = 1; i <= pages; i++) i];
    if (page <= 4) return [1, 2, 3, 4, 5, null, pages];
    if (page >= pages - 3) return [1, null, for (var i = pages - 4; i <= pages; i++) i];
    return [1, null, page - 1, page, page + 1, null, pages];
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    Widget cell(Widget child, {VoidCallback? onTap, bool current = false, String? tooltip}) {
      final w = SizedBox(
        height: 32,
        child: TextButton(
          onPressed: enabled ? onTap : null,
          style: TextButton.styleFrom(
            minimumSize: const Size(32, 32),
            padding: const EdgeInsets.symmetric(horizontal: TsSpace.s2),
            backgroundColor: current ? p.primary : null,
            foregroundColor: current ? p.onPrimary : p.textPrimary,
            disabledForegroundColor: current ? p.onPrimary : p.textMuted.withValues(alpha: 0.5),
            shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(TsRadius.md)),
          ),
          child: child,
        ),
      );
      return tooltip == null ? w : Tooltip(message: tooltip, child: w);
    }

    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        cell(
          const Icon(Icons.chevron_left, size: 18),
          onTap: page > 1 ? () => onPage(page - 1) : null,
          tooltip: previousLabel,
        ),
        for (final c in cells(page, pages))
          c == null
              ? Padding(
                  padding: const EdgeInsets.symmetric(horizontal: TsSpace.s1),
                  child: Text('…', style: TextStyle(color: p.textMuted)),
                )
              : cell(Text('$c'), onTap: c == page ? null : () => onPage(c), current: c == page),
        cell(
          const Icon(Icons.chevron_right, size: 18),
          onTap: page < pages ? () => onPage(page + 1) : null,
          tooltip: nextLabel,
        ),
      ],
    );
  }
}

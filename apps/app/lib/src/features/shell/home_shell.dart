import 'package:flutter/material.dart';

import '../../../l10n/app_localizations.dart';
import '../../ui/theme/app_theme.dart';
import '../library/library_page.dart';
import '../scan/scan_page.dart';
import '../search/search_page.dart';
import '../settings/settings_page.dart';

/// Khung 4 mục: Tài liệu · Quét · Tìm kiếm · Cài đặt. Hẹp: thanh dưới; rộng (≥ 840): thanh bên.
class HomeShell extends StatefulWidget {
  const HomeShell({super.key});

  @override
  State<HomeShell> createState() => _HomeShellState();
}

class _HomeShellState extends State<HomeShell> {
  int _index = 0;
  // Mỗi mục giữ Navigator riêng để mở tài liệu không mất thanh điều hướng.
  final _keys = List.generate(4, (_) => GlobalKey<NavigatorState>());

  static const _pages = <Widget>[LibraryPage(), ScanPage(), SearchPage(), SettingsPage()];

  void _select(int i) {
    if (i == _index) {
      _keys[i].currentState?.popUntil((r) => r.isFirst);
    } else {
      setState(() => _index = i);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final p = context.palette;
    final items = [
      (Icons.folder_copy_outlined, Icons.folder_copy, l.navLibrary),
      (Icons.document_scanner_outlined, Icons.document_scanner, l.navScan),
      (Icons.search, Icons.search, l.navSearch),
      (Icons.settings_outlined, Icons.settings, l.navSettings),
    ];
    final stack = IndexedStack(
      index: _index,
      children: [
        for (var i = 0; i < 4; i++)
          HeroControllerScope.none(
            child: Navigator(
              key: _keys[i],
              onGenerateRoute: (_) => MaterialPageRoute<void>(builder: (_) => _pages[i]),
            ),
          ),
      ],
    );
    final body = NavigatorPopHandler(onPopWithResult: (_) => _keys[_index].currentState?.maybePop(), child: stack);
    return LayoutBuilder(
      builder: (context, c) {
        if (c.maxWidth >= 840) {
          return Scaffold(
            body: Row(
              children: [
                NavigationRail(
                  selectedIndex: _index,
                  onDestinationSelected: _select,
                  labelType: NavigationRailLabelType.all,
                  destinations: [
                    for (final (icon, selected, label) in items)
                      NavigationRailDestination(icon: Icon(icon), selectedIcon: Icon(selected), label: Text(label)),
                  ],
                ),
                VerticalDivider(width: 1, color: p.border),
                Expanded(child: body),
              ],
            ),
          );
        }
        return Scaffold(
          body: body,
          bottomNavigationBar: NavigationBar(
            selectedIndex: _index,
            onDestinationSelected: _select,
            destinations: [
              for (final (icon, selected, label) in items)
                NavigationDestination(icon: Icon(icon), selectedIcon: Icon(selected), label: label),
            ],
          ),
        );
      },
    );
  }
}

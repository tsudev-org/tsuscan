import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';

import '../l10n/app_localizations.dart';
import 'app_scope.dart';
import 'bootstrap/platform.dart';
import 'data/library_repository.dart';
import 'features/lock/lock_screen.dart';
import 'features/shell/home_shell.dart';
import 'services/app_lock.dart';
import 'services/settings_store.dart';
import 'ui/theme/app_theme.dart';
import 'ui/widgets/feedback.dart';

/// Gốc app: giao diện + ngôn ngữ theo cài đặt, mở kho, khóa app.
class TsuScanApp extends StatefulWidget {
  const TsuScanApp({super.key, required this.platform, required this.settings, this.lock});
  final PlatformBindings platform;
  final SettingsStore settings;

  /// Cho test truyền bộ khóa có đồng hồ giả.
  final AppLockController? lock;

  @override
  State<TsuScanApp> createState() => _TsuScanAppState();
}

class _TsuScanAppState extends State<TsuScanApp> {
  late final AppLockController _lock =
      widget.lock ??
            AppLockController(
              settings: widget.settings,
              storage: widget.platform.storage,
              crypto: widget.platform.pin,
              biometric: widget.platform.biometric,
            )
        ..attach();
  final _changes = LibraryChanges();
  late Future<LibraryOpenResult> _open = widget.platform.openLibrary();

  void _reopen(Future<LibraryOpenResult> Function() action) {
    final next = action();
    setState(() {
      _open = next;
    });
  }

  @override
  void dispose() {
    _lock.dispose();
    _changes.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: widget.settings,
      builder: (context, _) => MaterialApp(
        onGenerateTitle: (c) => AppLocalizations.of(c).appTitle,
        debugShowCheckedModeBanner: false,
        theme: buildTheme(Brightness.light),
        darkTheme: buildTheme(Brightness.dark),
        themeMode: widget.settings.theme.mode,
        locale: widget.settings.locale,
        supportedLocales: SettingsStore.supportedLocales,
        localizationsDelegates: const [
          AppLocalizations.delegate,
          GlobalMaterialLocalizations.delegate,
          GlobalWidgetsLocalizations.delegate,
          GlobalCupertinoLocalizations.delegate,
        ],
        // Chưa chọn ngôn ngữ ⇒ theo hệ điều hành; ngôn ngữ chưa hỗ trợ ⇒ en (PRODUCT_POLICY.md mục 1, phiếu T3).
        localeResolutionCallback: (locale, _) => resolveAppLocale(locale),
        home: FutureBuilder<LibraryOpenResult>(
          future: _open,
          builder: (context, snap) => switch (snap.data) {
            null => const _Splash(),
            LibraryReady(:final repository) => _Ready(
              platform: widget.platform,
              settings: widget.settings,
              lock: _lock,
              repository: repository,
              changes: _changes,
            ),
            final LibraryKeyMissing _ => _StartupError(
              missingKey: true,
              onReset: () => _reopen(widget.platform.resetLibrary),
              onRetry: () => _reopen(widget.platform.openLibrary),
            ),
            LibraryFailed(:final error) => _StartupError(
              error: error,
              onRetry: () => _reopen(widget.platform.openLibrary),
            ),
          },
        ),
      ),
    );
  }
}

/// Ngôn ngữ app từ ngôn ngữ hệ điều hành: vi/en giữ nguyên, còn lại ⇒ en.
Locale resolveAppLocale(Locale? system) => system?.languageCode == 'vi' ? const Locale('vi') : const Locale('en');

class _Ready extends StatelessWidget {
  const _Ready({
    required this.platform,
    required this.settings,
    required this.lock,
    required this.repository,
    required this.changes,
  });
  final PlatformBindings platform;
  final SettingsStore settings;
  final AppLockController lock;
  final LibraryRepository repository;
  final LibraryChanges changes;

  @override
  Widget build(BuildContext context) {
    return AppScope(
      platform: platform,
      settings: settings,
      lock: lock,
      repository: repository,
      changes: changes,
      child: ListenableBuilder(
        listenable: lock,
        // Màn khóa phủ lên khung chính (giữ nguyên trạng thái điều hướng khi mở khóa).
        builder: (context, child) => Stack(
          children: [
            ExcludeSemantics(excluding: lock.locked, child: child!),
            if (lock.locked) Positioned.fill(child: LockScreen(controller: lock)),
          ],
        ),
        child: Column(
          children: [
            if (!repository.persistent) const _WebNotice(),
            const Expanded(child: HomeShell()),
          ],
        ),
      ),
    );
  }
}

class _WebNotice extends StatelessWidget {
  const _WebNotice();

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    return Material(
      color: p.bgSubtle,
      child: SafeArea(
        bottom: false,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: TsSpace.s4, vertical: TsSpace.s2),
          child: Row(
            children: [
              Icon(Icons.info_outline, size: 18, color: p.info),
              const SizedBox(width: TsSpace.s2),
              Expanded(
                child: Text(
                  AppLocalizations.of(context).startupWebNotice,
                  style: TextStyle(fontSize: TsFontSize.sm, color: p.textSecondary),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _Splash extends StatelessWidget {
  const _Splash();

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return Scaffold(
      body: Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const CircularProgressIndicator(),
            const SizedBox(height: TsSpace.s4),
            Text(l.startupOpening),
          ],
        ),
      ),
    );
  }
}

class _StartupError extends StatelessWidget {
  const _StartupError({this.error, this.missingKey = false, this.onReset, required this.onRetry});
  final Object? error;
  final bool missingKey;
  final VoidCallback? onReset;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final p = context.palette;
    return Scaffold(
      body: EmptyState(
        icon: missingKey ? Icons.key_off_outlined : Icons.error_outline,
        title: l.startupErrorTitle,
        body: missingKey ? l.startupKeyMissing : describeError(l, error ?? Object()),
        action: Wrap(
          spacing: TsSpace.s2,
          runSpacing: TsSpace.s2,
          alignment: WrapAlignment.center,
          children: [
            OutlinedButton(onPressed: onRetry, child: Text(l.actionRetry)),
            if (onReset != null)
              FilledButton(
                style: FilledButton.styleFrom(backgroundColor: p.danger, foregroundColor: p.onStatus),
                onPressed: () async {
                  final ok = await confirmDanger(
                    context,
                    title: l.startupResetLibrary,
                    body: l.startupResetConfirm,
                    action: l.startupResetLibrary,
                  );
                  if (ok) onReset!();
                },
                child: Text(l.startupResetLibrary),
              ),
          ],
        ),
      ),
    );
  }
}

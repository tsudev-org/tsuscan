import 'package:flutter/material.dart';

import '../../../l10n/app_localizations.dart';
import '../../app_scope.dart';
import '../../ui/theme/app_theme.dart';
import '../../ui/widgets/feedback.dart';
import '../lock/pin_setup.dart';

/// Phiên bản app hiển thị (khớp pubspec `version`).
const appVersion = '26.11.1901';

/// Cài đặt: giao diện, ngôn ngữ, khóa app, giới thiệu.
class SettingsPage extends StatefulWidget {
  const SettingsPage({super.key});

  @override
  State<SettingsPage> createState() => _SettingsPageState();
}

class _SettingsPageState extends State<SettingsPage> {
  bool? _biometricAvailable;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_biometricAvailable == null) {
      AppScope.of(context).platform.biometric.available().then((v) {
        if (mounted) setState(() => _biometricAvailable = v);
      });
    }
  }

  Future<void> _toggleLock(bool on) async {
    final app = AppScope.of(context);
    try {
      if (on) {
        final pin = await askNewPin(context);
        if (pin != null) await app.lock.setPin(pin);
      } else {
        await app.lock.disable();
      }
    } on Object catch (e) {
      if (mounted) showError(context, e);
    }
  }

  Future<void> _changePin() async {
    final app = AppScope.of(context);
    final pin = await askNewPin(context);
    if (pin == null) return;
    try {
      await app.lock.setPin(pin);
    } on Object catch (e) {
      if (mounted) showError(context, e);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final app = AppScope.of(context);
    final s = app.settings;
    final p = context.palette;
    final t = Theme.of(context).textTheme;
    Widget section(String title) => Padding(
      padding: const EdgeInsets.fromLTRB(TsSpace.s4, TsSpace.s6, TsSpace.s4, TsSpace.s2),
      child: Text(
        title,
        style: t.titleSmall?.copyWith(color: p.textSecondary, fontWeight: FontWeight.w600),
      ),
    );
    final lockSupported = app.repository.persistent;
    return ListenableBuilder(
      listenable: s,
      builder: (context, _) => Scaffold(
        appBar: AppBar(title: Text(l.navSettings)),
        body: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 720),
            child: ListView(
              padding: const EdgeInsets.only(bottom: TsSpace.s8),
              children: [
                section(l.settingsAppearance),
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: TsSpace.s4),
                  child: SegmentedButton<ThemeChoice>(
                    segments: [
                      ButtonSegment(
                        value: ThemeChoice.light,
                        icon: const Icon(Icons.light_mode_outlined),
                        label: Text(l.settingsThemeLight),
                      ),
                      ButtonSegment(
                        value: ThemeChoice.dark,
                        icon: const Icon(Icons.dark_mode_outlined),
                        label: Text(l.settingsThemeDark),
                      ),
                      ButtonSegment(
                        value: ThemeChoice.system,
                        icon: const Icon(Icons.brightness_auto_outlined),
                        label: Text(l.settingsThemeSystem),
                      ),
                    ],
                    selected: {s.theme},
                    showSelectedIcon: false,
                    onSelectionChanged: (v) => s.setTheme(v.first),
                  ),
                ),
                section(l.settingsLanguage),
                RadioGroup<String>(
                  groupValue: s.locale?.languageCode ?? '',
                  onChanged: (v) => s.setLocale(v == null || v.isEmpty ? null : Locale(v)),
                  child: Column(
                    children: [
                      RadioListTile(value: '', title: Text(l.settingsLanguageSystem)),
                      const RadioListTile(value: 'vi', title: Text('Tiếng Việt')),
                      const RadioListTile(value: 'en', title: Text('English')),
                    ],
                  ),
                ),
                if (lockSupported) ...[
                  section(l.settingsSecurity),
                  SwitchListTile(
                    value: s.lockEnabled,
                    title: Text(l.settingsAppLock),
                    subtitle: Text(l.settingsAppLockBody),
                    onChanged: _toggleLock,
                  ),
                  if (s.lockEnabled) ...[
                    SwitchListTile(
                      value: s.biometricEnabled,
                      title: Text(l.settingsBiometric),
                      subtitle: _biometricAvailable == false ? Text(l.settingsBiometricUnavailable) : null,
                      onChanged: _biometricAvailable == true ? s.setBiometricEnabled : null,
                    ),
                    ListTile(
                      leading: const Icon(Icons.password_outlined),
                      title: Text(l.settingsChangePin),
                      onTap: _changePin,
                    ),
                  ],
                  Padding(
                    padding: const EdgeInsets.fromLTRB(TsSpace.s4, TsSpace.s2, TsSpace.s4, 0),
                    child: Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Icon(Icons.shield_outlined, size: 18, color: p.success),
                        const SizedBox(width: TsSpace.s2),
                        Expanded(child: Text(l.settingsStorageNote, style: t.bodySmall)),
                      ],
                    ),
                  ),
                ],
                section(l.settingsAbout),
                ListTile(
                  leading: const Icon(Icons.info_outline),
                  title: Text(l.appTitle),
                  subtitle: Text('${l.settingsVersion(appVersion, app.platform.coreVersion)}\n${l.settingsLicense}'),
                  isThreeLine: true,
                ),
                ListTile(
                  leading: const Icon(Icons.gavel_outlined),
                  title: Text(l.settingsLicenses),
                  onTap: () => showLicensePage(
                    context: context,
                    applicationName: l.appTitle,
                    applicationVersion: appVersion,
                    applicationLegalese: '© TsuDEV · AGPL-3.0-only',
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

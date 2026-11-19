import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../l10n/app_localizations.dart';
import '../../services/app_lock.dart';
import '../../ui/theme/app_theme.dart';

/// Màn khóa: nhập PIN, hoặc sinh trắc học nếu đã bật. Chờ tăng dần sau 5 lần sai.
class LockScreen extends StatefulWidget {
  const LockScreen({super.key, required this.controller});
  final AppLockController controller;

  @override
  State<LockScreen> createState() => _LockScreenState();
}

class _LockScreenState extends State<LockScreen> {
  final _c = TextEditingController();
  String? _message;
  int _wait = 0;
  bool _busy = false;
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    unawaited(_initWait());
    if (widget.controller.settings.biometricEnabled) {
      WidgetsBinding.instance.addPostFrameCallback((_) => _biometric());
    }
  }

  @override
  void dispose() {
    _timer?.cancel();
    _c.dispose();
    super.dispose();
  }

  Future<void> _initWait() async => _startCountdown(await widget.controller.remainingLockout());

  void _startCountdown(int seconds) {
    _timer?.cancel();
    if (!mounted) return;
    setState(() => _wait = seconds);
    if (seconds <= 0) return;
    _timer = Timer.periodic(const Duration(seconds: 1), (t) {
      if (!mounted) return t.cancel();
      setState(() => _wait = _wait - 1);
      if (_wait <= 0) t.cancel();
    });
  }

  Future<void> _biometric() async {
    if (!mounted) return;
    await widget.controller.unlockWithBiometric(AppLocalizations.of(context).lockBiometricReason);
  }

  Future<void> _submit() async {
    if (_busy || _wait > 0 || _c.text.isEmpty) return;
    final l = AppLocalizations.of(context);
    setState(() => _busy = true);
    final r = await widget.controller.unlockWithPin(_c.text);
    if (!mounted) return;
    _c.clear();
    setState(() {
      _busy = false;
      _message = switch (r) {
        PinAccepted() => null,
        PinRejected(:final failures) => l.lockWrongPin(failures),
        PinLockedOut() => null,
      };
    });
    switch (r) {
      case PinRejected(:final waitSeconds) when waitSeconds > 0:
        _startCountdown(waitSeconds);
      case PinLockedOut(:final waitSeconds):
        _startCountdown(waitSeconds);
      default:
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final p = context.palette;
    final t = Theme.of(context).textTheme;
    final blocked = _wait > 0;
    return Scaffold(
      body: SafeArea(
        child: Center(
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(TsSpace.s6),
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 360),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Icon(Icons.lock_outline, size: 48, color: p.primary),
                  const SizedBox(height: TsSpace.s4),
                  Text(l.lockTitle, style: t.titleLarge, textAlign: TextAlign.center),
                  const SizedBox(height: TsSpace.s6),
                  TextField(
                    key: const ValueKey('lock-pin-field'),
                    controller: _c,
                    enabled: !blocked && !_busy,
                    autofocus: true,
                    obscureText: true,
                    keyboardType: TextInputType.number,
                    textAlign: TextAlign.center,
                    inputFormatters: [FilteringTextInputFormatter.digitsOnly, LengthLimitingTextInputFormatter(12)],
                    decoration: InputDecoration(
                      labelText: l.lockEnterPin,
                      errorText: blocked ? l.lockWait(_wait) : _message,
                    ),
                    onSubmitted: (_) => _submit(),
                  ),
                  const SizedBox(height: TsSpace.s4),
                  FilledButton(onPressed: blocked || _busy ? null : _submit, child: Text(l.lockUnlock)),
                  if (widget.controller.settings.biometricEnabled) ...[
                    const SizedBox(height: TsSpace.s2),
                    OutlinedButton.icon(
                      onPressed: _biometric,
                      icon: const Icon(Icons.fingerprint),
                      label: Text(l.lockUseBiometric),
                    ),
                  ],
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

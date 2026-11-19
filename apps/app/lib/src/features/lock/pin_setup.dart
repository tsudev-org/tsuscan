import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../l10n/app_localizations.dart';

/// Hỏi PIN mới (nhập 2 lần). Trả `null` khi hủy.
Future<String?> askNewPin(BuildContext context) =>
    showDialog<String>(context: context, builder: (_) => const _PinSetupDialog());

class _PinSetupDialog extends StatefulWidget {
  const _PinSetupDialog();

  @override
  State<_PinSetupDialog> createState() => _PinSetupDialogState();
}

class _PinSetupDialogState extends State<_PinSetupDialog> {
  final _c = TextEditingController();
  String? _first;
  String? _error;

  @override
  void dispose() {
    _c.dispose();
    super.dispose();
  }

  static bool isValidPin(String v) => RegExp(r'^[0-9]{4,12}$').hasMatch(v);

  void _next() {
    final l = AppLocalizations.of(context);
    final v = _c.text;
    if (!isValidPin(v)) {
      setState(() => _error = l.pinInvalid);
      return;
    }
    if (_first == null) {
      setState(() {
        _first = v;
        _error = null;
        _c.clear();
      });
      return;
    }
    if (v != _first) {
      setState(() {
        _first = null;
        _error = l.pinMismatch;
        _c.clear();
      });
      return;
    }
    Navigator.of(context).pop(v);
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return AlertDialog(
      title: Text(_first == null ? l.pinSetTitle : l.pinConfirmTitle),
      content: SizedBox(
        width: 400,
        child: TextField(
          key: const ValueKey('pin-setup-field'),
          controller: _c,
          autofocus: true,
          obscureText: true,
          keyboardType: TextInputType.number,
          inputFormatters: [FilteringTextInputFormatter.digitsOnly, LengthLimitingTextInputFormatter(12)],
          decoration: InputDecoration(labelText: l.pinHint, errorText: _error),
          onSubmitted: (_) => _next(),
        ),
      ),
      actions: [
        TextButton(onPressed: () => Navigator.of(context).pop(), child: Text(l.actionCancel)),
        FilledButton(onPressed: _next, child: Text(l.actionSave)),
      ],
    );
  }
}

import 'package:flutter/material.dart';

import '../../../l10n/app_localizations.dart';
import '../../data/library_repository.dart';
import '../theme/app_theme.dart';

/// Thông báo lỗi đã dịch, không lộ chi tiết nội bộ.
String describeError(AppLocalizations l, Object error) {
  if (error is ApiError) {
    return switch (error.kind) {
      ApiErrorKind.wrongKey => l.errorWrongKey,
      ApiErrorKind.notFound => l.errorNotFound,
      ApiErrorKind.invalidInput => l.errorInvalidInput,
      ApiErrorKind.unsupportedImage => l.errorUnsupportedImage,
      ApiErrorKind.imageTooLarge => l.errorImageTooLarge,
      ApiErrorKind.alreadyExists || ApiErrorKind.storage => l.errorStorage,
      ApiErrorKind.internal => l.errorUnknown,
    };
  }
  return l.errorUnknown;
}

/// Toast lỗi (6 giây, kèm icon - màu trạng thái không đứng một mình).
void showError(BuildContext context, Object error) {
  final l = AppLocalizations.of(context);
  final p = context.palette;
  ScaffoldMessenger.of(context)
    ..hideCurrentSnackBar()
    ..showSnackBar(
      SnackBar(
        duration: const Duration(seconds: 6),
        backgroundColor: p.bgSurface,
        content: Row(
          children: [
            Icon(Icons.error_outline, color: p.danger),
            const SizedBox(width: TsSpace.s2),
            Expanded(
              child: Text(describeError(l, error), style: TextStyle(color: p.textPrimary)),
            ),
          ],
        ),
      ),
    );
}

/// Toast cảnh báo (6 giây).
void showWarning(BuildContext context, String message) =>
    _toast(context, message, Icons.warning_amber_outlined, context.palette.warning, const Duration(seconds: 6));

/// Toast thông tin (4 giây).
void showInfo(BuildContext context, String message) =>
    _toast(context, message, Icons.check_circle_outline, context.palette.success, const Duration(seconds: 4));

void _toast(BuildContext context, String message, IconData icon, Color color, Duration duration) {
  final p = context.palette;
  ScaffoldMessenger.of(context)
    ..hideCurrentSnackBar()
    ..showSnackBar(
      SnackBar(
        duration: duration,
        backgroundColor: p.bgSurface,
        content: Row(
          children: [
            Icon(icon, color: color),
            const SizedBox(width: TsSpace.s2),
            Expanded(
              child: Text(message, style: TextStyle(color: p.textPrimary)),
            ),
          ],
        ),
      ),
    );
}

/// Hộp thoại nhập tên. Trả `null` khi hủy.
Future<String?> askName(BuildContext context, {required String title, required String label, String initial = ''}) {
  return showDialog<String>(
    context: context,
    builder: (_) => _NameDialog(title: title, label: label, initial: initial),
  );
}

class _NameDialog extends StatefulWidget {
  const _NameDialog({required this.title, required this.label, required this.initial});
  final String title;
  final String label;
  final String initial;

  @override
  State<_NameDialog> createState() => _NameDialogState();
}

class _NameDialogState extends State<_NameDialog> {
  late final _c = TextEditingController(text: widget.initial)
    ..selection = TextSelection(baseOffset: 0, extentOffset: widget.initial.length);
  String? _error;

  @override
  void dispose() {
    _c.dispose();
    super.dispose();
  }

  void _submit() {
    final v = _c.text.trim();
    if (v.isEmpty || v.runes.length > 200) {
      setState(() => _error = AppLocalizations.of(context).dialogNameRequired);
      return;
    }
    Navigator.of(context).pop(v);
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return AlertDialog(
      title: Text(widget.title),
      content: SizedBox(
        width: 440,
        child: TextField(
          controller: _c,
          autofocus: true,
          maxLength: 200,
          decoration: InputDecoration(labelText: widget.label, errorText: _error),
          onSubmitted: (_) => _submit(),
        ),
      ),
      actions: [
        TextButton(onPressed: () => Navigator.of(context).pop(), child: Text(l.actionCancel)),
        FilledButton(onPressed: _submit, child: Text(l.actionSave)),
      ],
    );
  }
}

/// Hộp thoại xác nhận thao tác nguy hiểm (nút Xóa màu danger, nằm ngoài cùng bên phải).
Future<bool> confirmDanger(BuildContext context, {required String title, required String body, String? action}) async {
  final l = AppLocalizations.of(context);
  final p = context.palette;
  final ok = await showDialog<bool>(
    context: context,
    builder: (c) => AlertDialog(
      title: Text(title),
      content: SizedBox(width: 440, child: Text(body)),
      actions: [
        TextButton(onPressed: () => Navigator.of(c).pop(false), child: Text(l.actionCancel)),
        FilledButton(
          style: FilledButton.styleFrom(backgroundColor: p.danger, foregroundColor: p.onStatus),
          onPressed: () => Navigator.of(c).pop(true),
          child: Text(action ?? l.actionDelete),
        ),
      ],
    ),
  );
  return ok ?? false;
}

/// Trạng thái rỗng có icon + tiêu đề + mô tả + hành động.
class EmptyState extends StatelessWidget {
  const EmptyState({super.key, required this.icon, required this.title, this.body, this.action});
  final IconData icon;
  final String title;
  final String? body;
  final Widget? action;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final t = Theme.of(context).textTheme;
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 480),
        child: Padding(
          padding: const EdgeInsets.all(TsSpace.s6),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(icon, size: 48, color: p.textMuted),
              const SizedBox(height: TsSpace.s4),
              Text(title, style: t.titleMedium, textAlign: TextAlign.center),
              if (body != null) ...[
                const SizedBox(height: TsSpace.s2),
                Text(
                  body!,
                  style: t.bodyMedium?.copyWith(color: p.textSecondary),
                  textAlign: TextAlign.center,
                ),
              ],
              if (action != null) ...[const SizedBox(height: TsSpace.s6), action!],
            ],
          ),
        ),
      ),
    );
  }
}

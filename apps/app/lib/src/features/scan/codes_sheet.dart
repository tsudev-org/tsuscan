import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../l10n/app_localizations.dart';
import '../../data/scan_engine.dart';
import '../../ui/theme/app_theme.dart';

/// Hiện các QR/mã vạch đọc được. Nội dung là dữ liệu KHÔNG tin cậy (mã có thể do bất kỳ ai in ra): chỉ hiển thị chữ
/// thuần, cho chọn và sao chép - không tự nhận diện/mở liên kết, không gửi đi đâu (ADR-0002).
Future<void> showScannedCodes(BuildContext context, List<ScannedCode> codes) {
  return showModalBottomSheet<void>(
    context: context,
    isScrollControlled: true,
    showDragHandle: true,
    builder: (context) => _CodesSheet(codes: codes),
  );
}

class _CodesSheet extends StatelessWidget {
  const _CodesSheet({required this.codes});
  final List<ScannedCode> codes;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final t = Theme.of(context).textTheme;
    return SafeArea(
      child: ConstrainedBox(
        constraints: BoxConstraints(maxHeight: MediaQuery.sizeOf(context).height * 0.7),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(TsSpace.s4, 0, TsSpace.s4, TsSpace.s2),
              child: Text(l.scanCodesTitle(codes.length), style: t.titleMedium),
            ),
            if (codes.isEmpty)
              Padding(
                padding: const EdgeInsets.all(TsSpace.s4),
                child: Text(l.scanCodesNone, style: t.bodyMedium),
              ),
            Flexible(
              child: ListView.separated(
                shrinkWrap: true,
                padding: const EdgeInsets.fromLTRB(TsSpace.s4, 0, TsSpace.s4, TsSpace.s4),
                itemCount: codes.length,
                separatorBuilder: (_, _) => const Divider(height: TsSpace.s4),
                itemBuilder: (context, i) {
                  final c = codes[i];
                  return Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              c.format.toUpperCase(),
                              style: t.labelSmall?.copyWith(color: context.palette.textSecondary),
                            ),
                            const SizedBox(height: TsSpace.s1),
                            SelectableText(c.text, style: t.bodyMedium),
                          ],
                        ),
                      ),
                      IconButton(
                        tooltip: l.scanCodeCopy,
                        icon: const Icon(Icons.copy_outlined),
                        onPressed: () async {
                          final messenger = ScaffoldMessenger.maybeOf(context);
                          await Clipboard.setData(ClipboardData(text: c.text));
                          messenger?.showSnackBar(SnackBar(content: Text(l.scanCodeCopied)));
                        },
                      ),
                    ],
                  );
                },
              ),
            ),
          ],
        ),
      ),
    );
  }
}

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../l10n/app_localizations.dart';
import '../../data/scan_engine.dart';
import 'scan_labels.dart';

/// Vẽ tứ giác chuẩn hóa lên vùng [size] (khung xem trước camera hoặc ảnh chụp).
class QuadPainter extends CustomPainter {
  const QuadPainter({required this.quad, required this.color, this.fillAlpha = 0.18, this.strokeWidth = 2.5});

  final ScanQuad? quad;
  final Color color;
  final double fillAlpha;
  final double strokeWidth;

  @override
  void paint(Canvas canvas, Size size) {
    final q = quad;
    if (q == null) return;
    final path = Path();
    for (var i = 0; i < 4; i++) {
      final p = q.point(i);
      final o = Offset(p.x * size.width, p.y * size.height);
      i == 0 ? path.moveTo(o.dx, o.dy) : path.lineTo(o.dx, o.dy);
    }
    path.close();
    canvas.drawPath(path, Paint()..color = color.withValues(alpha: fillAlpha));
    canvas.drawPath(
      path,
      Paint()
        ..color = color
        ..style = PaintingStyle.stroke
        ..strokeWidth = strokeWidth
        ..strokeJoin = StrokeJoin.round,
    );
  }

  @override
  bool shouldRepaint(QuadPainter old) => old.quad != quad || old.color != color;
}

/// Ảnh đầy đủ + bốn tay nắm góc kéo được. Mỗi tay nắm nhận tiêu điểm và di chuyển bằng phím mũi tên (bàn phím, trình
/// đọc màn hình) - không chỉ kéo bằng ngón tay (ACCESSIBILITY.md).
class QuadEditor extends StatelessWidget {
  const QuadEditor({
    super.key,
    required this.image,
    required this.aspectRatio,
    required this.quad,
    required this.onChanged,
    required this.valid,
  });

  final Uint8List image;

  /// Rộng / cao của ảnh.
  final double aspectRatio;
  final ScanQuad quad;
  final ValueChanged<ScanQuad> onChanged;
  final bool valid;

  /// Vùng chạm của tay nắm (≥ 44 px - ACCESSIBILITY.md).
  static const _handle = 48.0;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final scheme = Theme.of(context).colorScheme;
    final color = valid ? scheme.primary : scheme.error;
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(_handle / 2),
        child: AspectRatio(
          aspectRatio: aspectRatio,
          child: LayoutBuilder(
            builder: (context, c) {
              final size = Size(c.maxWidth, c.maxHeight);
              return Stack(
                clipBehavior: Clip.none,
                children: [
                  Positioned.fill(child: Image.memory(image, fit: BoxFit.fill, gaplessPlayback: true)),
                  Positioned.fill(
                    child: CustomPaint(
                      painter: QuadPainter(quad: quad, color: color),
                    ),
                  ),
                  for (var i = 0; i < 4; i++)
                    Positioned(
                      left: quad.point(i).x * size.width - _handle / 2,
                      top: quad.point(i).y * size.height - _handle / 2,
                      width: _handle,
                      height: _handle,
                      child: _Handle(
                        label: '${cornerLabel(l, i)}. ${l.scanCornerHint}',
                        color: color,
                        onDrag: (d) {
                          final p = quad.point(i);
                          onChanged(quad.withPoint(i, p.x + d.dx / size.width, p.y + d.dy / size.height));
                        },
                        onNudge: (dx, dy) {
                          final p = quad.point(i);
                          onChanged(quad.withPoint(i, p.x + dx, p.y + dy));
                        },
                      ),
                    ),
                ],
              );
            },
          ),
        ),
      ),
    );
  }
}

class _Handle extends StatefulWidget {
  const _Handle({required this.label, required this.color, required this.onDrag, required this.onNudge});
  final String label;
  final Color color;
  final ValueChanged<Offset> onDrag;
  final void Function(double dx, double dy) onNudge;

  @override
  State<_Handle> createState() => _HandleState();
}

class _HandleState extends State<_Handle> {
  bool _focused = false;
  static const _step = 0.005;

  KeyEventResult _key(FocusNode _, KeyEvent e) {
    if (e is! KeyDownEvent && e is! KeyRepeatEvent) return KeyEventResult.ignored;
    final big = HardwareKeyboard.instance.isShiftPressed ? 5 : 1;
    final (dx, dy) = switch (e.logicalKey) {
      LogicalKeyboardKey.arrowLeft => (-_step, 0.0),
      LogicalKeyboardKey.arrowRight => (_step, 0.0),
      LogicalKeyboardKey.arrowUp => (0.0, -_step),
      LogicalKeyboardKey.arrowDown => (0.0, _step),
      _ => (0.0, 0.0),
    };
    if (dx == 0 && dy == 0) return KeyEventResult.ignored;
    widget.onNudge(dx * big, dy * big);
    return KeyEventResult.handled;
  }

  @override
  Widget build(BuildContext context) {
    return Semantics(
      label: widget.label,
      button: true,
      child: Focus(
        onKeyEvent: _key,
        onFocusChange: (f) => setState(() => _focused = f),
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onPanUpdate: (d) => widget.onDrag(d.delta),
          child: Center(
            child: Container(
              width: 22,
              height: 22,
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                color: widget.color.withValues(alpha: 0.25),
                border: Border.all(color: widget.color, width: _focused ? 4 : 2.5),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

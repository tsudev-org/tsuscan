import 'package:flutter/material.dart';

import '../../data/library_repository.dart';

/// Chữ có tô sáng các đoạn [ranges] (vị trí UTF-16).
class HighlightedText extends StatelessWidget {
  const HighlightedText(this.text, {super.key, required this.ranges, this.style, this.highlightStyle, this.maxLines});
  final String text;
  final List<HighlightRange> ranges;
  final TextStyle? style;
  final TextStyle? highlightStyle;
  final int? maxLines;

  @override
  Widget build(BuildContext context) {
    final spans = <TextSpan>[];
    var at = 0;
    for (final r in ranges) {
      final s = r.start.clamp(at, text.length);
      final e = r.end.clamp(s, text.length);
      if (s > at) spans.add(TextSpan(text: text.substring(at, s)));
      if (e > s) spans.add(TextSpan(text: text.substring(s, e), style: highlightStyle));
      at = e;
    }
    if (at < text.length) spans.add(TextSpan(text: text.substring(at)));
    return Text.rich(
      TextSpan(style: style, children: spans),
      maxLines: maxLines,
      overflow: maxLines == null ? null : TextOverflow.ellipsis,
    );
  }
}

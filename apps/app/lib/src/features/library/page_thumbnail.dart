import 'dart:typed_data';

import 'package:flutter/material.dart';

import '../../app_scope.dart';
import '../../ui/theme/app_theme.dart';

/// Ảnh thu nhỏ của một trang, giải mã trong lõi Rust (Dart chỉ nhận JPEG nhỏ).
class PageThumbnail extends StatefulWidget {
  const PageThumbnail({super.key, required this.pageId, this.maxSide = 320, this.fit = BoxFit.cover});
  final String pageId;
  final int maxSide;
  final BoxFit fit;

  @override
  State<PageThumbnail> createState() => _PageThumbnailState();
}

class _PageThumbnailState extends State<PageThumbnail> {
  Future<Uint8List>? _bytes;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _bytes ??= AppScope.of(context).repository.pageThumbnail(widget.pageId, widget.maxSide);
  }

  @override
  void didUpdateWidget(PageThumbnail old) {
    super.didUpdateWidget(old);
    if (old.pageId != widget.pageId || old.maxSide != widget.maxSide) {
      _bytes = AppScope.of(context).repository.pageThumbnail(widget.pageId, widget.maxSide);
    }
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    return FutureBuilder<Uint8List>(
      future: _bytes,
      builder: (context, snap) {
        final data = snap.data;
        if (data != null) {
          return Image.memory(
            data,
            fit: widget.fit,
            gaplessPlayback: true,
            errorBuilder: (_, _, _) => _placeholder(p, Icons.broken_image_outlined),
          );
        }
        return _placeholder(p, snap.hasError ? Icons.broken_image_outlined : Icons.description_outlined);
      },
    );
  }

  Widget _placeholder(TsPalette p, IconData icon) => ColoredBox(
    color: p.bgSubtle,
    child: Center(child: Icon(icon, color: p.textMuted)),
  );
}

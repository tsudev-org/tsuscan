import 'dart:async';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';

import '../../../l10n/app_localizations.dart';
import '../../data/scan_engine.dart';
import '../../ui/theme/app_theme.dart';
import '../../ui/widgets/feedback.dart';
import 'quad_view.dart';
import 'scan_labels.dart';
import 'scan_session.dart';

/// Chỉnh một trang đã chụp: kéo 4 góc, dò lại, giữ cả khung, đổi bộ lọc, xem kết quả. Trả trang đã cập nhật (hoặc
/// `null` khi hủy). Ảnh giải mã chỉ sống trong màn này rồi bị xóa trắng.
class PageEditorPage extends StatefulWidget {
  const PageEditorPage({super.key, required this.engine, required this.page});
  final ScanEngine engine;
  final ScanDraftPage page;

  @override
  State<PageEditorPage> createState() => _PageEditorPageState();
}

class _PageEditorPageState extends State<PageEditorPage> {
  ScanCapture? _cap;
  Uint8List? _full;
  double _aspect = 1;
  late ScanQuad _quad = widget.page.quad ?? ScanQuad.full;
  late bool _crop = widget.page.quad != null;
  late ScanFilter _filter = widget.page.filter;
  bool _showResult = false;
  Uint8List? _result;
  int _renderSeq = 0;
  bool _busy = false;
  Object? _error;

  @override
  void initState() {
    super.initState();
    unawaited(_load());
  }

  Future<void> _load() async {
    try {
      final cap = await widget.engine.open(widget.page.original);
      if (!mounted) return cap.dispose();
      _cap = cap;
      final full = await cap.render(null, ScanFilter.original, 1600);
      final img = await _size(full);
      if (!mounted) return;
      setState(() {
        _full = full;
        _aspect = img.width / img.height;
      });
    } on Object catch (e) {
      if (mounted) setState(() => _error = e);
    }
  }

  static Future<ui.Size> _size(Uint8List bytes) async {
    final codec = await ui.instantiateImageCodec(bytes);
    final frame = await codec.getNextFrame();
    final s = ui.Size(frame.image.width.toDouble(), frame.image.height.toDouble());
    frame.image.dispose();
    codec.dispose();
    return s;
  }

  ScanQuad? get _effective => _crop ? _quad : null;
  bool get _valid => !_crop || _quad.isConvex;

  Future<void> _renderResult() async {
    final cap = _cap;
    if (cap == null || !_valid) return;
    final seq = ++_renderSeq;
    try {
      final r = await cap.render(_effective, _filter, 1600);
      if (mounted && seq == _renderSeq) setState(() => _result = r);
    } on Object catch (e) {
      if (mounted) showError(context, e);
    }
  }

  Future<void> _redetect() async {
    final cap = _cap;
    if (cap == null) return;
    final q = await cap.detect();
    if (!mounted) return;
    if (q == null) {
      showWarning(context, AppLocalizations.of(context).scanNotFound);
      return;
    }
    setState(() {
      _quad = q;
      _crop = true;
    });
    if (_showResult) unawaited(_renderResult());
  }

  Future<void> _done() async {
    final cap = _cap;
    final l = AppLocalizations.of(context);
    if (cap == null) return;
    if (!_valid) {
      showWarning(context, l.scanInvalidQuad);
      return;
    }
    setState(() => _busy = true);
    try {
      final thumb = await cap.render(_effective, _filter, kThumbSide);
      if (!mounted) return;
      final q = _effective;
      Navigator.of(context).pop(widget.page.copyWith(quad: () => q, filter: _filter, thumb: thumb));
    } on Object catch (e) {
      if (mounted) {
        setState(() => _busy = false);
        showError(context, e);
      }
    }
  }

  @override
  void dispose() {
    _cap?.dispose();
    for (final b in [_full, _result]) {
      b?.fillRange(0, b.length, 0);
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final full = _full;
    Widget body;
    if (_error != null) {
      body = Center(child: Text(describeError(l, _error!)));
    } else if (full == null) {
      body = const Center(child: CircularProgressIndicator());
    } else if (_showResult) {
      final r = _result;
      body = r == null
          ? const Center(child: CircularProgressIndicator())
          : InteractiveViewer(maxScale: 6, child: Center(child: Image.memory(r, gaplessPlayback: true)));
    } else if (_crop) {
      body = QuadEditor(
        image: full,
        aspectRatio: _aspect,
        quad: _quad,
        valid: _valid,
        onChanged: (q) => setState(() => _quad = q),
      );
    } else {
      body = Center(
        child: Padding(padding: const EdgeInsets.all(TsSpace.s6), child: Image.memory(full)),
      );
    }

    return Scaffold(
      appBar: AppBar(
        title: Text(l.scanEditTitle),
        actions: [
          Padding(
            padding: const EdgeInsets.only(right: TsSpace.s2),
            child: FilledButton(onPressed: _busy || full == null ? null : _done, child: Text(l.scanDone)),
          ),
        ],
      ),
      body: SafeArea(
        child: Column(
          children: [
            Expanded(child: body),
            if (!_valid)
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: TsSpace.s4),
                child: Text(l.scanInvalidQuad, style: TextStyle(color: Theme.of(context).colorScheme.error)),
              ),
            Padding(
              padding: const EdgeInsets.fromLTRB(TsSpace.s2, TsSpace.s2, TsSpace.s2, 0),
              child: Wrap(
                spacing: TsSpace.s2,
                runSpacing: TsSpace.s1,
                alignment: WrapAlignment.center,
                children: [
                  SegmentedButton<bool>(
                    segments: [
                      ButtonSegment(value: false, icon: const Icon(Icons.crop_free), label: Text(l.scanShowCorners)),
                      ButtonSegment(
                        value: true,
                        icon: const Icon(Icons.visibility_outlined),
                        label: Text(l.scanShowResult),
                      ),
                    ],
                    selected: {_showResult},
                    onSelectionChanged: full == null
                        ? null
                        : (s) {
                            setState(() => _showResult = s.first);
                            if (_showResult) unawaited(_renderResult());
                          },
                  ),
                  if (widget.engine.supportsDetection)
                    OutlinedButton.icon(
                      onPressed: full == null ? null : _redetect,
                      icon: const Icon(Icons.auto_fix_high_outlined),
                      label: Text(l.scanRedetect),
                    ),
                  if (widget.engine.supportsDetection)
                    FilterChip(
                      label: Text(l.scanFullFrame),
                      selected: !_crop,
                      onSelected: full == null
                          ? null
                          : (on) {
                              setState(() {
                                _crop = !on;
                                if (_crop && !_quad.isConvex) _quad = ScanQuad.full;
                              });
                              if (_showResult) unawaited(_renderResult());
                            },
                    ),
                ],
              ),
            ),
            SizedBox(
              height: 56,
              child: ListView(
                scrollDirection: Axis.horizontal,
                padding: const EdgeInsets.symmetric(horizontal: TsSpace.s2, vertical: TsSpace.s2),
                children: [
                  for (final f in ScanFilter.values)
                    Padding(
                      padding: const EdgeInsets.only(right: TsSpace.s2),
                      child: ChoiceChip(
                        label: Text(f.label(l)),
                        selected: f == _filter,
                        onSelected: (_) {
                          setState(() => _filter = f);
                          if (_showResult) unawaited(_renderResult());
                        },
                      ),
                    ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

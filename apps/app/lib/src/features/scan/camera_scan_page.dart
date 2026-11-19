import 'dart:async';

import 'package:camera/camera.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../l10n/app_localizations.dart';
import '../../app_scope.dart';
import '../../data/scan_engine.dart';
import '../../services/app_lock.dart';
import '../../ui/theme/app_theme.dart';
import '../../ui/widgets/feedback.dart';
import '../../utils/temp_file.dart';
import 'quad_view.dart';
import 'scan_labels.dart';
import 'scan_review_page.dart';
import 'scan_session.dart';

/// Quét bằng camera trong app: hiện có trên Android và iOS (máy tính/web nhập ảnh có sẵn).
bool get cameraScanSupported =>
    !kIsWeb && (defaultTargetPlatform == TargetPlatform.android || defaultTargetPlatform == TargetPlatform.iOS);

/// Khoảng tối thiểu giữa hai lần dò trên khung xem trước (≈ 8 khung/giây là đủ mượt, tiết kiệm pin).
const _detectInterval = Duration(milliseconds: 120);

/// Màn chụp toàn màn hình: xem trước + tứ giác dò được, tự chụp khi ổn định, chụp loạt, đèn, chế độ. Trả id tài liệu
/// đã lưu (hoặc `null`). Camera chỉ chạy khi màn này ở tiền cảnh: app xuống nền thì giải phóng ngay.
class CameraScanPage extends StatefulWidget {
  const CameraScanPage({super.key, required this.session});
  final ScanSession session;

  @override
  State<CameraScanPage> createState() => _CameraScanPageState();
}

enum _CamError { denied, none, failed }

class _CameraScanPageState extends State<CameraScanPage> with WidgetsBindingObserver {
  CameraController? _ctrl;
  CameraDescription? _desc;
  _CamError? _error;
  bool _starting = false;

  ScanQuad? _quad;
  bool _detecting = false;
  DateTime _lastDetect = DateTime.fromMillisecondsSinceEpoch(0);
  final _tracker = AutoCaptureTracker();
  late bool _auto = widget.session.mode.autoCapture;
  FlashMode _flash = FlashMode.off;
  bool _capturing = false;
  bool _flashAnim = false;
  bool _inReview = false;
  AppLockController? _lock;

  ScanSession get _s => widget.session;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    unawaited(SystemChrome.setPreferredOrientations([DeviceOrientation.portraitUp]));
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_lock != null) return;
    // Khóa app ⇒ tắt camera ngay (không quay sau màn khóa); mở khóa ⇒ bật lại.
    _lock = AppScope.of(context).lock..addListener(_onLock);
    unawaited(_start());
  }

  void _onLock() {
    if (_lock?.locked ?? false) {
      unawaited(_stop());
    } else if (!_inReview && mounted) {
      unawaited(_start());
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _lock?.removeListener(_onLock);
    unawaited(SystemChrome.setPreferredOrientations(const []));
    unawaited(_stop());
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) {
      if (!_inReview) unawaited(_start());
    } else if (state == AppLifecycleState.inactive || state == AppLifecycleState.paused) {
      unawaited(_stop());
    }
  }

  Future<void> _start() async {
    if (_ctrl != null || _starting || (_lock?.locked ?? true)) return;
    _starting = true;
    try {
      final desc = _desc ?? await _pickCamera();
      if (desc == null) {
        if (mounted) setState(() => _error = _CamError.none);
        return;
      }
      _desc = desc;
      final ctrl = CameraController(
        desc,
        // Ảnh chụp cần độ phân giải cao cho OCR; khung xem trước được lõi thu nhỏ khi dò.
        ResolutionPreset.max,
        enableAudio: false,
        imageFormatGroup: ImageFormatGroup.yuv420,
      );
      await ctrl.initialize();
      if (!mounted) {
        await ctrl.dispose();
        return;
      }
      await ctrl.lockCaptureOrientation(DeviceOrientation.portraitUp);
      await _applyFlash(ctrl);
      _ctrl = ctrl;
      _tracker.reset();
      await _startStream();
      setState(() => _error = null);
    } on CameraException catch (e) {
      final denied = e.code.startsWith('CameraAccessDenied') || e.code == 'CameraAccessRestricted';
      if (mounted) setState(() => _error = denied ? _CamError.denied : _CamError.failed);
    } finally {
      _starting = false;
    }
  }

  static Future<CameraDescription?> _pickCamera() async {
    final cams = await availableCameras();
    if (cams.isEmpty) return null;
    return cams.firstWhere((c) => c.lensDirection == CameraLensDirection.back, orElse: () => cams.first);
  }

  Future<void> _stop() async {
    final c = _ctrl;
    _ctrl = null;
    _quad = null;
    if (c == null) return;
    try {
      if (c.value.isStreamingImages) await c.stopImageStream();
    } on CameraException {
      // Đang đóng: bỏ qua.
    }
    await c.dispose();
    if (mounted) setState(() {});
  }

  Future<void> _startStream() async {
    final c = _ctrl;
    if (c == null || c.value.isStreamingImages) return;
    await c.startImageStream(_onFrame);
  }

  void _onFrame(CameraImage img) {
    if (_detecting || _capturing || !mounted || !_s.engine.supportsDetection) return;
    final now = DateTime.now();
    if (now.difference(_lastDetect) < _detectInterval) return;
    _lastDetect = now;
    final y = img.planes.first;
    _detecting = true;
    unawaited(
      _s.engine
          .detectPreview(y.bytes, img.width, img.height, y.bytesPerRow, _desc?.sensorOrientation ?? 90)
          .then(_onQuad, onError: (Object _) => _onQuad(null))
          .whenComplete(() => _detecting = false),
    );
  }

  void _onQuad(ScanQuad? q) {
    if (!mounted || _capturing) return;
    final show = _s.mode.crop ? q : null;
    setState(() => _quad = show);
    if (_auto && _s.mode.autoCapture && _tracker.feed(q)) unawaited(_capture(auto: true));
  }

  Future<void> _capture({bool auto = false}) async {
    final c = _ctrl;
    if (c == null || _capturing || !c.value.isInitialized) return;
    final hint = _quad;
    setState(() {
      _capturing = true;
      _flashAnim = true;
    });
    unawaited(HapticFeedback.mediumImpact());
    try {
      if (c.value.isStreamingImages) await c.stopImageStream();
      final file = await c.takePicture();
      final bytes = await file.readAsBytes();
      // Ảnh JPEG gốc vừa được plugin ghi vào cache: xóa ngay, chỉ giữ bản trong bộ nhớ.
      await deleteTempFile(file.path);
      if (mounted) setState(() => _flashAnim = false);
      await _s.addCapture(bytes, hint: hint);
      if (!auto) _tracker.markCaptured(hint);
    } on Object catch (e) {
      if (mounted) showError(context, e);
    } finally {
      if (mounted) {
        setState(() {
          _capturing = false;
          _flashAnim = false;
        });
        if (_ctrl != null) unawaited(_startStream());
      }
    }
  }

  Future<void> _applyFlash(CameraController c) async {
    try {
      await c.setFlashMode(_flash);
    } on CameraException {
      // Máy không có đèn.
    }
  }

  Future<void> _cycleFlash() async {
    final next = switch (_flash) {
      FlashMode.off => FlashMode.auto,
      FlashMode.auto => FlashMode.torch,
      _ => FlashMode.off,
    };
    setState(() => _flash = next);
    final c = _ctrl;
    if (c != null) await _applyFlash(c);
  }

  Future<void> _review() async {
    _inReview = true;
    await _stop();
    if (!mounted) return;
    final out = await Navigator.of(context)
        .push(MaterialPageRoute<ReviewOutcome>(builder: (_) => ScanReviewPage(session: _s)));
    _inReview = false;
    if (!mounted) return;
    if (out is ReviewSaved) {
      Navigator.of(context).pop(out.documentId);
      return;
    }
    _tracker.reset();
    unawaited(_start());
  }

  Future<bool> _confirmLeave() async {
    if (_s.isEmpty) return true;
    final l = AppLocalizations.of(context);
    return confirmDanger(context, title: l.scanDiscardTitle, body: l.scanDiscardBody(_s.length), action: l.scanDiscard);
  }

  void _setMode(ScanMode m) {
    setState(() {
      _s.mode = m;
      _auto = m.autoCapture;
      _quad = null;
    });
    _tracker.reset();
  }

  String _hint(AppLocalizations l) {
    if (_s.mode == ScanMode.idCard) {
      final side = _s.length.isEven ? l.scanSideFront : l.scanSideBack;
      return '$side · ${_hintBody(l)}';
    }
    return _hintBody(l);
  }

  String _hintBody(AppLocalizations l) {
    if (!_auto || !_s.mode.autoCapture) return l.scanHintManual;
    if (_tracker.waitingForChange) return l.scanHintNext;
    if (_quad == null) return l.scanHintFind;
    return l.scanHintHold;
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return PopScope(
      canPop: false,
      onPopInvokedWithResult: (didPop, _) async {
        if (didPop) return;
        final nav = Navigator.of(context);
        if (await _confirmLeave() && mounted) nav.pop();
      },
      child: Scaffold(
        backgroundColor: Colors.black,
        body: SafeArea(
          child: Column(
            children: [
              _topBar(l),
              Expanded(child: _preview(l)),
              _modes(l),
              _bottomBar(l),
            ],
          ),
        ),
      ),
    );
  }

  Widget _topBar(AppLocalizations l) {
    final (icon, label) = switch (_flash) {
      FlashMode.auto => (Icons.flash_auto, l.scanFlashAuto),
      FlashMode.torch => (Icons.flashlight_on, l.scanFlashTorch),
      _ => (Icons.flash_off, l.scanFlashOff),
    };
    return Row(
      children: [
        IconButton(
          tooltip: MaterialLocalizations.of(context).closeButtonTooltip,
          color: Colors.white,
          onPressed: () => Navigator.of(context).maybePop(),
          icon: const Icon(Icons.close),
        ),
        const Spacer(),
        IconButton(tooltip: label, color: Colors.white, onPressed: _cycleFlash, icon: Icon(icon)),
        if (_s.mode.autoCapture)
          Padding(
            padding: const EdgeInsets.only(right: TsSpace.s2),
            child: Row(
              children: [
                Text(l.scanAutoCapture, style: const TextStyle(color: Colors.white)),
                Switch(
                  value: _auto,
                  onChanged: (v) {
                    setState(() => _auto = v);
                    _tracker.reset();
                  },
                ),
              ],
            ),
          ),
      ],
    );
  }

  Widget _preview(AppLocalizations l) {
    final err = _error;
    if (err != null) {
      final msg = switch (err) {
        _CamError.denied => l.scanPermissionDenied,
        _CamError.none => l.scanNoCamera,
        _CamError.failed => l.scanCameraError,
      };
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(TsSpace.s6),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Icon(Icons.no_photography_outlined, color: Colors.white, size: 48),
              const SizedBox(height: TsSpace.s4),
              Text(
                msg,
                textAlign: TextAlign.center,
                style: const TextStyle(color: Colors.white),
              ),
              const SizedBox(height: TsSpace.s4),
              FilledButton(
                onPressed: () {
                  setState(() => _error = null);
                  unawaited(_start());
                },
                child: Text(l.actionRetry),
              ),
            ],
          ),
        ),
      );
    }
    final c = _ctrl;
    if (c == null || !c.value.isInitialized) {
      return const Center(child: CircularProgressIndicator());
    }
    final color = _tracker.waitingForChange ? context.palette.success : Theme.of(context).colorScheme.primary;
    return Stack(
      alignment: Alignment.center,
      children: [
        Center(
          child: AspectRatio(
            aspectRatio: 1 / c.value.aspectRatio,
            child: Stack(
              fit: StackFit.expand,
              children: [
                CameraPreview(c),
                CustomPaint(
                  painter: QuadPainter(quad: _quad, color: color),
                ),
                AnimatedOpacity(
                  opacity: _flashAnim ? 0.7 : 0,
                  duration: const Duration(milliseconds: 90),
                  child: const ColoredBox(color: Colors.white),
                ),
              ],
            ),
          ),
        ),
        Positioned(
          top: TsSpace.s3,
          child: Semantics(
            liveRegion: true,
            child: DecoratedBox(
              decoration: BoxDecoration(color: Colors.black54, borderRadius: BorderRadius.circular(TsRadius.lg)),
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: TsSpace.s3, vertical: TsSpace.s2),
                child: Text(_hint(l), style: const TextStyle(color: Colors.white)),
              ),
            ),
          ),
        ),
      ],
    );
  }

  Widget _modes(AppLocalizations l) => SizedBox(
    height: 52,
    child: Semantics(
      label: l.scanModeLabel,
      child: ListView(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.symmetric(horizontal: TsSpace.s3, vertical: TsSpace.s2),
        children: [
          for (final m in ScanMode.values)
            Padding(
              padding: const EdgeInsets.only(right: TsSpace.s2),
              child: ChoiceChip(
                avatar: Icon(m.icon, size: 18),
                label: Text(m.label(l)),
                selected: m == _s.mode,
                onSelected: (_) => _setMode(m),
              ),
            ),
        ],
      ),
    ),
  );

  Widget _bottomBar(AppLocalizations l) => ListenableBuilder(
    listenable: _s,
    builder: (context, _) {
      final pages = _s.pages;
      return Padding(
        padding: const EdgeInsets.fromLTRB(TsSpace.s4, TsSpace.s2, TsSpace.s4, TsSpace.s4),
        child: Row(
          children: [
            SizedBox(
              width: 64,
              height: 64,
              child: pages.isEmpty
                  ? null
                  : Semantics(
                      button: true,
                      label: '${l.scanReviewTitle}, ${l.scanPageCount(pages.length)}',
                      child: InkWell(
                        onTap: _review,
                        child: Stack(
                          children: [
                            Positioned.fill(child: Image.memory(pages.last.thumb, fit: BoxFit.cover)),
                            Positioned(
                              right: 0,
                              bottom: 0,
                              child: Container(
                                color: Colors.black87,
                                padding: const EdgeInsets.symmetric(horizontal: 4),
                                child: Text('${pages.length}', style: const TextStyle(color: Colors.white)),
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
            ),
            const Spacer(),
            _Shutter(
              label: l.scanShutter,
              progress: _auto && _s.mode.autoCapture ? _tracker.progress : 0,
              busy: _capturing,
              onPressed: _ctrl == null ? null : () => _capture(),
            ),
            const Spacer(),
            SizedBox(
              width: 64,
              child: pages.isEmpty
                  ? null
                  : TextButton(
                      onPressed: _review,
                      child: Text(l.scanDone, style: const TextStyle(color: Colors.white)),
                    ),
            ),
          ],
        ),
      );
    },
  );
}

class _Shutter extends StatelessWidget {
  const _Shutter({required this.label, required this.progress, required this.busy, required this.onPressed});
  final String label;
  final double progress;
  final bool busy;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      button: true,
      label: label,
      child: GestureDetector(
        onTap: busy ? null : onPressed,
        child: SizedBox(
          width: 76,
          height: 76,
          child: Stack(
            alignment: Alignment.center,
            children: [
              SizedBox.expand(
                child: CircularProgressIndicator(
                  value: busy ? null : progress,
                  strokeWidth: 4,
                  color: Colors.white,
                  backgroundColor: Colors.white24,
                ),
              ),
              Container(
                width: 60,
                height: 60,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: onPressed == null ? Colors.white38 : Colors.white,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

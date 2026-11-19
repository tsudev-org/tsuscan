import 'package:flutter/foundation.dart';

import '../../data/library_repository.dart';
import '../../data/scan_engine.dart';

/// Chế độ quét: quyết định bộ lọc mặc định, có cắt theo mép không, có tự chụp không, số mặt mỗi lần.
enum ScanMode {
  document(ScanFilter.auto, crop: true, autoCapture: true),
  idCard(ScanFilter.color, crop: true, autoCapture: true, sides: 2),
  businessCard(ScanFilter.color, crop: true, autoCapture: true),
  whiteboard(ScanFilter.whiteboard, crop: true, autoCapture: false),
  photo(ScanFilter.original, crop: false, autoCapture: false);

  const ScanMode(this.filter, {required this.crop, required this.autoCapture, this.sides = 1});

  /// Bộ lọc mặc định khi chụp ở chế độ này.
  final ScanFilter filter;

  /// Cắt theo tứ giác dò được (Ảnh: giữ cả khung).
  final bool crop;

  /// Cho phép tự chụp khi khung ổn định (Bảng trắng: tứ giác thường không rõ ⇒ chụp tay).
  final bool autoCapture;

  /// Số mặt mỗi giấy tờ (CCCD: trước + sau).
  final int sides;

  static ScanMode parse(String? raw) => ScanMode.values.firstWhere((m) => m.name == raw, orElse: () => document);
}

/// Quyết định tự chụp: chỉ bấm khi tứ giác đủ lớn, đủ tin cậy và đứng yên qua [stableFrames] khung liên tiếp; sau một
/// lần chụp thì đợi tài liệu đổi (dời đi hoặc mất khỏi khung) mới chụp tiếp - chụp loạt không chụp trùng một trang.
final class AutoCaptureTracker {
  AutoCaptureTracker({
    this.stableFrames = 6,
    this.maxDrift = 0.012,
    this.minConfidence = 0.5,
    this.minArea = 0.12,
    this.rearmDrift = 0.08,
  });

  /// Số khung liên tiếp phải đứng yên.
  final int stableFrames;

  /// Dịch chuyển góc tối đa giữa hai khung (tỉ lệ khung) để coi là đứng yên.
  final double maxDrift;
  final double minConfidence;

  /// Diện tích tối thiểu (tỉ lệ khung): tài liệu quá xa thì chữ không đủ nét.
  final double minArea;

  /// Sau khi chụp, tài liệu phải dời ít nhất chừng này (hoặc biến mất) mới được chụp tiếp.
  final double rearmDrift;

  ScanQuad? _last;
  ScanQuad? _captured;
  int _still = 0;

  /// Tiến độ 0..1 để vẽ vòng đếm trên nút chụp.
  double get progress => (_still / stableFrames).clamp(0.0, 1.0);

  /// Đang chờ tài liệu đổi sau một lần chụp.
  bool get waitingForChange => _captured != null;

  /// Đưa một khung vào; trả `true` khi nên chụp ngay.
  bool feed(ScanQuad? q) {
    if (q == null) {
      _last = null;
      _still = 0;
      // Mất tài liệu khỏi khung ⇒ trang mới có thể đặt vào.
      _captured = null;
      return false;
    }
    final cap = _captured;
    if (cap != null) {
      if (drift(cap, q) < rearmDrift) {
        _last = q;
        return false;
      }
      _captured = null;
      _still = 0;
    }
    final good = q.confidence >= minConfidence && q.area >= minArea;
    final last = _last;
    _last = q;
    if (!good || last == null || drift(last, q) > maxDrift) {
      _still = good ? 1 : 0;
      return false;
    }
    _still++;
    if (_still >= stableFrames) {
      _captured = q;
      _still = 0;
      return true;
    }
    return false;
  }

  /// Đã chụp tay: đợi tài liệu đổi như sau một lần tự chụp.
  void markCaptured(ScanQuad? q) {
    _captured = q;
    _still = 0;
  }

  void reset() {
    _last = null;
    _captured = null;
    _still = 0;
  }

  /// Dịch chuyển lớn nhất của một góc giữa hai tứ giác.
  static double drift(ScanQuad a, ScanQuad b) {
    var m = 0.0;
    for (var i = 0; i < 8; i++) {
      final d = (a.corners[i] - b.corners[i]).abs();
      if (d > m) m = d;
    }
    return m;
  }
}

/// Một trang trong phiên quét, chưa lưu. Chỉ giữ JPEG gốc (vài MB) - ảnh giải mã (36 MB với 12 MP) chỉ sống trong
/// lúc dò/sửa/lưu rồi bị xóa trắng.
final class ScanDraftPage {
  ScanDraftPage({required this.original, required this.quad, required this.filter, required this.thumb});

  final Uint8List original;

  /// `null` = giữ cả khung.
  final ScanQuad? quad;
  final ScanFilter filter;

  /// Ảnh xem trước đã nắn + lọc.
  final Uint8List thumb;

  ScanDraftPage copyWith({ScanQuad? Function()? quad, ScanFilter? filter, Uint8List? thumb}) => ScanDraftPage(
    original: original,
    quad: quad == null ? this.quad : quad(),
    filter: filter ?? this.filter,
    thumb: thumb ?? this.thumb,
  );
}

/// Cạnh dài ảnh xem trước trong danh sách trang.
const kThumbSide = 480;

/// Các trang đã chụp trong một phiên (chụp loạt), trước khi lưu thành tài liệu.
final class ScanSession extends ChangeNotifier {
  ScanSession(this.engine, this.mode);

  final ScanEngine engine;
  ScanMode mode;
  final _pages = <ScanDraftPage>[];
  bool _disposed = false;

  List<ScanDraftPage> get pages => List.unmodifiable(_pages);
  int get length => _pages.length;
  bool get isEmpty => _pages.isEmpty;

  /// Giải mã [bytes], dò mép (nếu chế độ cắt), dựng ảnh xem trước rồi thêm vào cuối. [hint] là tứ giác của khung
  /// xem trước, dùng khi dò trên ảnh đầy đủ không ra.
  Future<ScanDraftPage> addCapture(Uint8List bytes, {ScanQuad? hint}) async {
    final page = await prepare(engine, bytes, mode, hint: hint);
    if (_disposed) {
      _wipe(page);
      throw StateError('phiên quét đã đóng');
    }
    _pages.add(page);
    notifyListeners();
    return page;
  }

  static Future<ScanDraftPage> prepare(ScanEngine engine, Uint8List bytes, ScanMode mode, {ScanQuad? hint}) async {
    final cap = await engine.open(bytes);
    try {
      ScanQuad? quad;
      if (mode.crop && engine.supportsDetection) {
        quad = await cap.detect() ?? (hint != null && hint.isConvex ? hint : null);
      }
      final thumb = await cap.render(quad, mode.filter, kThumbSide);
      return ScanDraftPage(original: bytes, quad: quad, filter: mode.filter, thumb: thumb);
    } finally {
      cap.dispose();
    }
  }

  void replace(int index, ScanDraftPage page) {
    _pages[index] = page;
    notifyListeners();
  }

  void remove(int index) {
    _wipe(_pages.removeAt(index));
    notifyListeners();
  }

  void move(int from, int to) {
    final p = _pages.removeAt(from);
    _pages.insert(to.clamp(0, _pages.length), p);
    notifyListeners();
  }

  /// Ghi mọi trang vào [documentId] theo thứ tự; gọi [onProgress] sau mỗi trang. Ảnh gốc bị xóa trắng sau khi lưu.
  Future<void> saveTo(LibraryRepository repo, String documentId, {void Function(int done)? onProgress}) async {
    for (var i = 0; i < _pages.length; i++) {
      final p = _pages[i];
      final cap = await engine.open(p.original);
      try {
        await repo.addScannedPage(documentId, cap, p.quad, p.filter);
      } finally {
        cap.dispose();
      }
      onProgress?.call(i + 1);
    }
    clear();
  }

  void clear() {
    for (final p in _pages) {
      _wipe(p);
    }
    _pages.clear();
    if (!_disposed) notifyListeners();
  }

  static void _wipe(ScanDraftPage p) {
    // Dart không bảo đảm xóa mọi bản sao, nhưng không để ảnh giấy tờ nằm nguyên trong bộ đệm đã bỏ.
    p.original.fillRange(0, p.original.length, 0);
    p.thumb.fillRange(0, p.thumb.length, 0);
  }

  @override
  void dispose() {
    clear();
    _disposed = true;
    super.dispose();
  }
}

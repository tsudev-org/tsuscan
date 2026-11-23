import 'package:flutter/foundation.dart';

import '../bridge/generated/api/scan.dart' as ffi;

/// Bộ lọc trang (mã khớp `tsuscan_imaging::Filter::code`; test `scan_engine_test.dart` chặn lệch).
enum ScanFilter {
  original('original'),
  auto('auto'),
  color('color'),
  gray('gray'),
  bw('bw'),
  shadow('shadow'),
  whiteboard('whiteboard');

  const ScanFilter(this.code);
  final String code;
}

/// Tứ giác tài liệu: 8 số chuẩn hóa 0..1 theo ảnh đúng chiều hiển thị, thứ tự TL, TR, BR, BL.
final class ScanQuad {
  ScanQuad(List<double> corners, this.confidence) : corners = List.unmodifiable(corners) {
    if (corners.length != 8) throw ArgumentError.value(corners.length, 'corners', 'cần 8 số');
  }

  /// Cả khung (không cắt).
  static final full = ScanQuad(const [0, 0, 1, 0, 1, 1, 0, 1], 1);

  final List<double> corners;
  final double confidence;

  ({double x, double y}) point(int i) => (x: corners[2 * i], y: corners[2 * i + 1]);

  /// Diện tích (tỉ lệ khung) theo công thức dây giày.
  double get area {
    var s = 0.0;
    for (var i = 0; i < 4; i++) {
      final a = point(i);
      final b = point((i + 1) % 4);
      s += a.x * b.y - b.x * a.y;
    }
    return s.abs() / 2;
  }

  ScanQuad withPoint(int i, double x, double y) {
    final c = [...corners];
    c[2 * i] = x.clamp(0.0, 1.0);
    c[2 * i + 1] = y.clamp(0.0, 1.0);
    return ScanQuad(c, confidence);
  }

  /// Bốn góc tạo thành tứ giác lồi, đúng chiều kim đồng hồ (không tự cắt) - điều kiện để nắn.
  bool get isConvex {
    var sign = 0;
    for (var i = 0; i < 4; i++) {
      final a = point(i);
      final b = point((i + 1) % 4);
      final c = point((i + 2) % 4);
      final cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
      if (cross.abs() < 1e-9) return false;
      final s = cross > 0 ? 1 : -1;
      if (sign == 0) {
        sign = s;
      } else if (s != sign) {
        return false;
      }
    }
    return area > 0.01;
  }
}

/// Một QR/mã vạch đọc được. [text] là dữ liệu KHÔNG tin cậy: chỉ hiển thị dạng chữ thuần và sao chép, không tự mở
/// liên kết hay thực thi gì.
final class ScannedCode {
  const ScannedCode(this.format, this.text);
  final String format;
  final String text;

  @override
  bool operator ==(Object other) => other is ScannedCode && other.format == format && other.text == text;

  @override
  int get hashCode => Object.hash(format, text);
}

/// Ảnh vừa chụp, đã giải mã trong lõi. Gọi [dispose] ngay khi xong: bộ đệm điểm ảnh bị xóa trắng.
abstract interface class ScanCapture {
  int get width;
  int get height;

  /// Dò tài liệu trên ảnh đầy đủ; `null` khi không thấy.
  Future<ScanQuad?> detect();

  /// Chế độ Sách: tìm gáy trong vùng [quad] (`null` = cả khung), trả 2 tứ giác trang trái, phải; `null` khi không
  /// giống ảnh hai trang.
  Future<List<ScanQuad>?> splitBook(ScanQuad? quad);

  /// Đọc QR/mã vạch trên cả ảnh.
  Future<List<ScannedCode>> readCodes();

  /// Nắn theo [quad] (`null` = giữ cả khung) + lọc; JPEG cạnh dài ≤ [maxSide] để xem trước.
  Future<Uint8List> render(ScanQuad? quad, ScanFilter filter, int maxSide);

  void dispose();
}

/// Lõi xử lý quét của nền tảng.
abstract interface class ScanEngine {
  /// Có dò mép trên khung xem trước được không (web xem trước: không).
  bool get supportsDetection;

  /// Giải mã ảnh chụp/ảnh nhập.
  Future<ScanCapture> open(Uint8List bytes);

  /// Dò trên kênh Y của một khung xem trước. [rotation]: độ xoay cảm biến → hiển thị (0/90/180/270).
  Future<ScanQuad?> detectPreview(Uint8List luma, int width, int height, int rowStride, int rotation);

  /// Đọc QR/mã vạch trên kênh Y đầy đủ của một khung xem trước (chế độ QR).
  Future<List<ScannedCode>> readCodesPreview(Uint8List luma, int width, int height, int rowStride);
}

List<ScannedCode> _codes(List<ffi.ScanBarcode> raw) => [for (final b in raw) ScannedCode(b.format, b.text)];

/// Bản thật qua lõi Rust.
final class RustScanEngine implements ScanEngine {
  const RustScanEngine();

  @override
  bool get supportsDetection => true;

  @override
  Future<ScanCapture> open(Uint8List bytes) async => RustScanCapture(await ffi.CapturedImage.decode(bytes: bytes));

  @override
  Future<ScanQuad?> detectPreview(Uint8List luma, int width, int height, int rowStride, int rotation) async {
    // Chỉ gửi mỗi hàng thứ `step` qua cầu (xem `preview_row_step` bên Rust): chuyển trọn 1,5 MB mỗi khung tốn
    // 15-50 ms trên máy tầm trung, gom hàng bằng chép khối gần như miễn phí.
    final step = ffi.previewRowStep(width: width, height: height);
    final packed = packLumaRows(luma, width, height, rowStride, step);
    final q = await ffi.detectPreview(
      luma: packed,
      width: width,
      height: height,
      rowStride: width,
      rowStep: step,
      rotation: rotation,
    );
    return q == null ? null : ScanQuad(q.corners, q.confidence);
  }

  @override
  Future<List<ScannedCode>> readCodesPreview(Uint8List luma, int width, int height, int rowStride) async =>
      _codes(await ffi.scanBarcodesPreview(luma: luma, width: width, height: height, rowStride: rowStride));
}

/// Chép các hàng 0, [step], 2·[step]... của mặt phẳng [luma] (mỗi hàng bắt đầu ở `i * rowStride`, có thể có đệm;
/// hàng cuối có thể thiếu phần đệm) thành bộ đệm liền `width` byte mỗi hàng. Hàng thiếu byte (khung hỏng) dừng sớm:
/// lõi Rust sẽ từ chối vì thiếu hàng.
@visibleForTesting
Uint8List packLumaRows(Uint8List luma, int width, int height, int rowStride, int step) {
  final rows = (height + step - 1) ~/ step;
  final out = Uint8List(rows * width);
  for (var r = 0; r < rows; r++) {
    final src = r * step * rowStride;
    if (src + width > luma.length) return Uint8List.sublistView(out, 0, r * width);
    out.setRange(r * width, (r + 1) * width, luma, src);
  }
  return out;
}

/// Ảnh chụp giữ trong Rust (opaque).
final class RustScanCapture implements ScanCapture {
  RustScanCapture(this.inner);
  final ffi.CapturedImage inner;

  @override
  int get width => inner.width;
  @override
  int get height => inner.height;

  @override
  Future<ScanQuad?> detect() async {
    final q = await inner.detect();
    return q == null ? null : ScanQuad(q.corners, q.confidence);
  }

  @override
  Future<List<ScanQuad>?> splitBook(ScanQuad? quad) async {
    final r = await inner.splitBook(corners: quad?.corners ?? const []);
    return r?.map((q) => ScanQuad(q.corners, q.confidence)).toList();
  }

  @override
  Future<List<ScannedCode>> readCodes() async => _codes(await inner.barcodes());

  @override
  Future<Uint8List> render(ScanQuad? quad, ScanFilter filter, int maxSide) =>
      inner.render(corners: quad?.corners ?? const [], filter: filter.code, maxSide: maxSide);

  @override
  void dispose() {
    if (!inner.isDisposed) inner.dispose();
  }
}

/// Web xem trước và test: không dò, không nắn - trả lại nguyên ảnh.
final class MemoryScanEngine implements ScanEngine {
  const MemoryScanEngine();

  @override
  bool get supportsDetection => false;

  @override
  Future<ScanCapture> open(Uint8List bytes) async => MemoryScanCapture(Uint8List.fromList(bytes));

  @override
  Future<ScanQuad?> detectPreview(Uint8List luma, int width, int height, int rowStride, int rotation) async => null;

  @override
  Future<List<ScannedCode>> readCodesPreview(Uint8List luma, int width, int height, int rowStride) async => const [];
}

final class MemoryScanCapture implements ScanCapture {
  MemoryScanCapture(this.bytes);
  final Uint8List bytes;

  @override
  int get width => 0;
  @override
  int get height => 0;
  @override
  Future<ScanQuad?> detect() async => null;
  @override
  Future<List<ScanQuad>?> splitBook(ScanQuad? quad) async => null;
  @override
  Future<List<ScannedCode>> readCodes() async => const [];
  @override
  Future<Uint8List> render(ScanQuad? quad, ScanFilter filter, int maxSide) async => Uint8List.fromList(bytes);
  @override
  void dispose() => bytes.fillRange(0, bytes.length, 0);
}

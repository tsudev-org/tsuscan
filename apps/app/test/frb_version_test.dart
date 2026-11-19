// flutter_rust_bridge kiểm lúc khởi động mã sinh phải TRÙNG bản thư viện Dart đang chạy; lệch là app dừng ở màn lỗi trên
// MỌI nền tảng (sự cố lotus-share 26.11.1801, phiếu điều phối 20261119-AP1 bài học 11). Test chặn ở CI:
// mã sinh = pubspec.lock = Cargo.lock = codegen ghim trong scripts/gen-bridge.sh.
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

String _version(String path, RegExp pattern) {
  final match = pattern.firstMatch(File(path).readAsStringSync());
  expect(match, isNotNull, reason: 'không thấy flutter_rust_bridge trong $path');
  return match!.group(1)!;
}

void main() {
  test('mã sinh flutter_rust_bridge trùng bản Dart, bản Rust và codegen ghim', () {
    final codegen = _version(
      'lib/src/bridge/generated/frb_generated.dart',
      RegExp(r"String get codegenVersion => '([^']+)';"),
    );
    final dart = _version('pubspec.lock', RegExp(r'\n  flutter_rust_bridge:\n(?:    .*\n)*?    version: "([^"]+)"'));
    final rust = _version('../../Cargo.lock', RegExp(r'name = "flutter_rust_bridge"\nversion = "([^"]+)"'));
    final pinned = _version('../../scripts/gen-bridge.sh', RegExp(r'want="([^"]+)"'));
    expect(dart, codegen, reason: 'pubspec.lock có $dart nhưng mã sinh là $codegen');
    expect(rust, codegen, reason: 'Cargo.lock có $rust nhưng mã sinh là $codegen');
    expect(pinned, codegen, reason: 'scripts/gen-bridge.sh ghim $pinned nhưng mã sinh là $codegen');
  });
}

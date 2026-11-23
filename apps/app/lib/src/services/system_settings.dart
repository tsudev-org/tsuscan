import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

/// Kênh hệ thống tối thiểu (`MainActivity.kt`, `AppDelegate.swift`).
const _channel = MethodChannel('com.tsudev.tsuscan/system');

/// Có mở được trang cài đặt của app trên nền tảng này không (Android, iOS).
bool get canOpenAppSettings =>
    !kIsWeb && (defaultTargetPlatform == TargetPlatform.android || defaultTargetPlatform == TargetPlatform.iOS);

/// Mở trang cài đặt của CHÍNH app (bật lại quyền đã bị từ chối hẳn). `false` khi không mở được.
Future<bool> openAppSettings() async {
  if (!canOpenAppSettings) return false;
  try {
    return await _channel.invokeMethod<bool>('openAppSettings') ?? false;
  } on PlatformException {
    return false;
  } on MissingPluginException {
    return false;
  }
}

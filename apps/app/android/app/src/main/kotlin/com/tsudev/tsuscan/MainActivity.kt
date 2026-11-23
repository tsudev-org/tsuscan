package com.tsudev.tsuscan

import android.content.ActivityNotFoundException
import android.content.Intent
import android.net.Uri
import android.provider.Settings
import io.flutter.embedding.android.FlutterFragmentActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

// local_auth (sinh trắc học) cần FragmentActivity.
class MainActivity : FlutterFragmentActivity() {
    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        // Kênh hệ thống tối thiểu: chỉ mở trang thông tin app của CHÍNH app này (bật lại quyền camera đã bị từ chối
        // hẳn). Không cần quyền nào, không nhận tham số từ Dart.
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, "com.tsudev.tsuscan/system")
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "openAppSettings" -> {
                        val intent = Intent(
                            Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
                            Uri.fromParts("package", packageName, null),
                        )
                        try {
                            startActivity(intent)
                            result.success(true)
                        } catch (_: ActivityNotFoundException) {
                            result.success(false)
                        }
                    }
                    else -> result.notImplemented()
                }
            }
    }
}

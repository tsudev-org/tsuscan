import 'package:flutter/material.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'src/app.dart';
import 'src/bootstrap/platform.dart';
import 'src/services/settings_store.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final platform = await loadPlatform();
  final settings = SettingsStore(await SharedPreferences.getInstance());
  runApp(TsuScanApp(platform: platform, settings: settings));
}

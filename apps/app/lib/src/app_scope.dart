import 'package:flutter/widgets.dart';

import 'bootstrap/platform.dart';
import 'data/library_repository.dart';
import 'services/app_lock.dart';
import 'services/settings_store.dart';

/// Phụ thuộc dùng chung của app, truyền xuống cây widget.
class AppScope extends InheritedWidget {
  const AppScope({
    super.key,
    required this.platform,
    required this.settings,
    required this.lock,
    required this.repository,
    required this.changes,
    required super.child,
  });

  final PlatformBindings platform;
  final SettingsStore settings;
  final AppLockController lock;
  final LibraryRepository repository;

  /// Tăng mỗi khi dữ liệu thư viện đổi ⇒ các màn đang mở tải lại.
  final LibraryChanges changes;

  static AppScope of(BuildContext context) {
    final s = context.dependOnInheritedWidgetOfExactType<AppScope>();
    assert(s != null, 'AppScope chưa được đặt ở gốc cây widget');
    return s!;
  }

  @override
  bool updateShouldNotify(AppScope old) => repository != old.repository || settings != old.settings || lock != old.lock;
}

/// Bộ đếm thay đổi dữ liệu thư viện.
class LibraryChanges extends ChangeNotifier {
  int _rev = 0;
  int get revision => _rev;
  void bump() {
    _rev++;
    notifyListeners();
  }
}

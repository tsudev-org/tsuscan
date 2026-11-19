import 'dart:io';

Future<void> deleteTempFile(String path) async {
  try {
    final f = File(path);
    if (f.existsSync()) await f.delete();
  } on FileSystemException {
    // Bỏ qua: xem `temp_file.dart`.
  }
}

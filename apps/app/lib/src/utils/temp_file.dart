import 'temp_file_native.dart' if (dart.library.js_interop) 'temp_file_web.dart' as impl;

/// Xóa tệp tạm (ảnh camera ghi vào cache riêng của app) ngay sau khi đã đọc vào bộ nhớ: ảnh giấy tờ không được nằm
/// không mã hóa trên đĩa. Lỗi xóa bị nuốt (tệp đã mất/không quyền) - cache của app vẫn bị HĐH dọn.
Future<void> deleteTempFile(String path) => impl.deleteTempFile(path);

// ignore: unused_import
import 'package:intl/intl.dart' as intl;

import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Vietnamese (`vi`).
class AppLocalizationsVi extends AppLocalizations {
  AppLocalizationsVi([String locale = 'vi']) : super(locale);

  @override
  String get appTitle => 'Tsu Scan';

  @override
  String get navLibrary => 'Tài liệu';

  @override
  String get navScan => 'Quét';

  @override
  String get navSearch => 'Tìm kiếm';

  @override
  String get navSettings => 'Cài đặt';

  @override
  String get actionCancel => 'Hủy';

  @override
  String get actionSave => 'Lưu';

  @override
  String get actionCreate => 'Tạo';

  @override
  String get actionDelete => 'Xóa';

  @override
  String get actionRename => 'Đổi tên';

  @override
  String get actionMove => 'Chuyển thư mục';

  @override
  String get actionRetry => 'Thử lại';

  @override
  String get actionClose => 'Đóng';

  @override
  String get libraryAllDocuments => 'Tất cả tài liệu';

  @override
  String get libraryRootFolder => 'Chưa xếp thư mục';

  @override
  String get libraryFolders => 'Thư mục';

  @override
  String get libraryTags => 'Nhãn';

  @override
  String get libraryNewDocument => 'Tài liệu mới';

  @override
  String get libraryNewFolder => 'Thư mục mới';

  @override
  String get libraryNewTag => 'Nhãn mới';

  @override
  String get libraryImportImages => 'Nhập ảnh';

  @override
  String get libraryEmptyTitle => 'Chưa có tài liệu nào';

  @override
  String get libraryEmptyBody =>
      'Tạo tài liệu mới hoặc nhập ảnh có sẵn. Mọi dữ liệu được mã hóa và chỉ nằm trên máy này.';

  @override
  String get libraryNoMatch => 'Không có tài liệu nào khớp bộ lọc';

  @override
  String get libraryClearFilter => 'Bỏ lọc';

  @override
  String libraryPages(int count) {
    String _temp0 = intl.Intl.pluralLogic(count, locale: localeName, other: '$count trang', zero: 'Chưa có trang');
    return '$_temp0';
  }

  @override
  String libraryUpdated(String time) {
    return 'Sửa lúc $time';
  }

  @override
  String get dialogDocumentTitle => 'Tên tài liệu';

  @override
  String get dialogFolderName => 'Tên thư mục';

  @override
  String get dialogTagName => 'Tên nhãn';

  @override
  String dialogDefaultDocumentTitle(String date) {
    return 'Tài liệu $date';
  }

  @override
  String get dialogNameRequired => 'Nhập tên (tối đa 200 ký tự)';

  @override
  String get confirmDeleteDocumentTitle => 'Xóa tài liệu?';

  @override
  String confirmDeleteDocumentBody(String title) {
    return '\"$title\" và mọi trang của nó sẽ bị xóa hẳn khỏi máy này. Không thể hoàn tác.';
  }

  @override
  String get confirmDeleteFolderTitle => 'Xóa thư mục?';

  @override
  String confirmDeleteFolderBody(String name) {
    return 'Tài liệu trong \"$name\" được giữ lại và chuyển lên thư mục cha.';
  }

  @override
  String get confirmDeleteTagTitle => 'Xóa nhãn?';

  @override
  String confirmDeleteTagBody(String name) {
    return 'Nhãn \"$name\" sẽ được gỡ khỏi mọi tài liệu.';
  }

  @override
  String get confirmDeletePageTitle => 'Xóa trang?';

  @override
  String get confirmDeletePageBody => 'Trang này sẽ bị xóa hẳn khỏi máy. Không thể hoàn tác.';

  @override
  String get documentNoPages => 'Tài liệu chưa có trang. Thêm ảnh để bắt đầu.';

  @override
  String get documentAddPages => 'Thêm trang từ ảnh';

  @override
  String documentPageLabel(int number) {
    return 'Trang $number';
  }

  @override
  String get documentTagsTitle => 'Gắn nhãn';

  @override
  String get documentMoveRoot => 'Thư mục gốc';

  @override
  String documentImported(int count) {
    String _temp0 = intl.Intl.pluralLogic(count, locale: localeName, other: 'Đã thêm $count trang');
    return '$_temp0';
  }

  @override
  String documentImportSkipped(int count) {
    String _temp0 = intl.Intl.pluralLogic(count, locale: localeName, other: '$count ảnh không đọc được đã bị bỏ qua');
    return '$_temp0';
  }

  @override
  String get documentOcrPending => 'Chưa có chữ nhận dạng (OCR có ở bản sau).';

  @override
  String get pagerPerPage => '/ trang';

  @override
  String pagerSummary(int from, int to, int total) {
    final intl.NumberFormat fromNumberFormat = intl.NumberFormat.decimalPattern(localeName);
    final String fromString = fromNumberFormat.format(from);
    final intl.NumberFormat toNumberFormat = intl.NumberFormat.decimalPattern(localeName);
    final String toString = toNumberFormat.format(to);
    final intl.NumberFormat totalNumberFormat = intl.NumberFormat.decimalPattern(localeName);
    final String totalString = totalNumberFormat.format(total);

    return '$fromString-$toString / $totalString';
  }

  @override
  String get pagerEmpty => '0 / 0';

  @override
  String get pagerPrevious => 'Trước';

  @override
  String get pagerNext => 'Sau';

  @override
  String get pagerPageSizeLabel => 'Số bản ghi mỗi trang';

  @override
  String get scanTitle => 'Quét tài liệu';

  @override
  String get scanComingBody =>
      'Quét bằng camera có tự dò mép, tự chụp và chụp loạt sẽ có ở bản cập nhật kế tiếp. Lúc này bạn có thể nhập ảnh đã chụp: ảnh được mã hóa ngay trên máy, siêu dữ liệu (vị trí GPS, máy chụp) bị loại bỏ.';

  @override
  String get scanImportToNew => 'Nhập ảnh thành tài liệu mới';

  @override
  String get searchHint => 'Tìm theo tên hoặc chữ trong trang (gõ không dấu được)';

  @override
  String get searchMinChars => 'Nhập ít nhất 2 ký tự';

  @override
  String searchNoResults(String query) {
    return 'Không tìm thấy kết quả cho \"$query\"';
  }

  @override
  String get searchAccentLess => 'Khớp không dấu';

  @override
  String get settingsAppearance => 'Giao diện';

  @override
  String get settingsThemeLight => 'Sáng';

  @override
  String get settingsThemeDark => 'Tối';

  @override
  String get settingsThemeSystem => 'Hệ thống';

  @override
  String get settingsLanguage => 'Ngôn ngữ';

  @override
  String get settingsLanguageSystem => 'Theo hệ thống';

  @override
  String get settingsSecurity => 'Bảo mật';

  @override
  String get settingsAppLock => 'Khóa ứng dụng bằng mã PIN';

  @override
  String get settingsAppLockBody => 'Hỏi mã PIN khi mở app hoặc quay lại sau 1 phút';

  @override
  String get settingsBiometric => 'Mở khóa bằng sinh trắc học';

  @override
  String get settingsBiometricUnavailable => 'Thiết bị này không hỗ trợ hoặc chưa cài sinh trắc học';

  @override
  String get settingsChangePin => 'Đổi mã PIN';

  @override
  String get settingsStorageNote =>
      'Dữ liệu được mã hóa bằng khóa lưu trong kho bí mật của hệ điều hành và không rời khỏi máy.';

  @override
  String get settingsAbout => 'Giới thiệu';

  @override
  String settingsVersion(String app, String core) {
    return 'Phiên bản $app · lõi $core';
  }

  @override
  String get settingsLicense => 'Mã nguồn mở AGPL-3.0';

  @override
  String get settingsLicenses => 'Giấy phép bên thứ ba';

  @override
  String get pinSetTitle => 'Đặt mã PIN';

  @override
  String get pinConfirmTitle => 'Nhập lại mã PIN';

  @override
  String get pinHint => '4-12 chữ số';

  @override
  String get pinMismatch => 'Hai lần nhập không khớp';

  @override
  String get pinInvalid => 'Mã PIN gồm 4-12 chữ số';

  @override
  String get lockTitle => 'Tsu Scan đang khóa';

  @override
  String get lockEnterPin => 'Nhập mã PIN';

  @override
  String get lockUnlock => 'Mở khóa';

  @override
  String get lockUseBiometric => 'Dùng sinh trắc học';

  @override
  String get lockBiometricReason => 'Mở khóa Tsu Scan';

  @override
  String lockWrongPin(int count) {
    return 'Sai mã PIN ($count lần)';
  }

  @override
  String lockWait(int seconds) {
    return 'Thử lại sau $seconds giây';
  }

  @override
  String get startupOpening => 'Đang mở kho tài liệu…';

  @override
  String get startupWebNotice =>
      'Bản web đang thử nghiệm: dữ liệu chỉ nằm trong bộ nhớ và mất khi đóng tab. Bản web đầy đủ (mã hóa, lưu bền) sẽ có sau.';

  @override
  String get startupErrorTitle => 'Không mở được kho tài liệu';

  @override
  String get startupKeyMissing =>
      'Khóa giải mã trong kho bí mật của máy không còn (có thể do khôi phục máy hoặc gỡ cài đặt dở). Dữ liệu cũ không thể đọc lại nếu thiếu khóa.';

  @override
  String get startupResetLibrary => 'Tạo kho mới (xóa dữ liệu cũ)';

  @override
  String get startupResetConfirm => 'Toàn bộ tài liệu cũ trên máy này sẽ bị xóa hẳn. Tiếp tục?';

  @override
  String get errorWrongKey => 'Khóa giải mã không khớp với kho trên máy.';

  @override
  String get errorNotFound => 'Mục này không còn tồn tại.';

  @override
  String get errorInvalidInput => 'Dữ liệu nhập không hợp lệ.';

  @override
  String get errorUnsupportedImage => 'Ảnh không đọc được hoặc định dạng chưa hỗ trợ (JPEG, PNG, WebP).';

  @override
  String get errorImageTooLarge => 'Ảnh quá lớn (tối đa 100 megapixel).';

  @override
  String get errorStorage => 'Lỗi lưu trữ trên máy (đầy bộ nhớ hoặc thiếu quyền).';

  @override
  String get errorUnknown => 'Đã xảy ra lỗi không mong muốn.';

  @override
  String pagerItems(int count) {
    final intl.NumberFormat countNumberFormat = intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(count, locale: localeName, other: '$countString mục');
    return '$_temp0';
  }

  @override
  String pagerAnnounce(int from, int to, int total) {
    return 'Đang hiển thị $from đến $to trong $total mục';
  }

  @override
  String pagerFiltered(String range, int all) {
    final intl.NumberFormat allNumberFormat = intl.NumberFormat.decimalPattern(localeName);
    final String allString = allNumberFormat.format(all);

    return '$range (lọc từ $allString)';
  }
}

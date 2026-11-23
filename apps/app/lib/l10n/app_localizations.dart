import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_en.dart';
import 'app_localizations_vi.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'l10n/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale) : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations)!;
  }

  static const LocalizationsDelegate<AppLocalizations> delegate = _AppLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates = <LocalizationsDelegate<dynamic>>[
    delegate,
    GlobalMaterialLocalizations.delegate,
    GlobalCupertinoLocalizations.delegate,
    GlobalWidgetsLocalizations.delegate,
  ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[Locale('en'), Locale('vi')];

  /// No description provided for @appTitle.
  ///
  /// In vi, this message translates to:
  /// **'Tsu Scan'**
  String get appTitle;

  /// No description provided for @navLibrary.
  ///
  /// In vi, this message translates to:
  /// **'Tài liệu'**
  String get navLibrary;

  /// No description provided for @navScan.
  ///
  /// In vi, this message translates to:
  /// **'Quét'**
  String get navScan;

  /// No description provided for @navSearch.
  ///
  /// In vi, this message translates to:
  /// **'Tìm kiếm'**
  String get navSearch;

  /// No description provided for @navSettings.
  ///
  /// In vi, this message translates to:
  /// **'Cài đặt'**
  String get navSettings;

  /// No description provided for @actionCancel.
  ///
  /// In vi, this message translates to:
  /// **'Hủy'**
  String get actionCancel;

  /// No description provided for @actionSave.
  ///
  /// In vi, this message translates to:
  /// **'Lưu'**
  String get actionSave;

  /// No description provided for @actionCreate.
  ///
  /// In vi, this message translates to:
  /// **'Tạo'**
  String get actionCreate;

  /// No description provided for @actionDelete.
  ///
  /// In vi, this message translates to:
  /// **'Xóa'**
  String get actionDelete;

  /// No description provided for @actionRename.
  ///
  /// In vi, this message translates to:
  /// **'Đổi tên'**
  String get actionRename;

  /// No description provided for @actionMove.
  ///
  /// In vi, this message translates to:
  /// **'Chuyển thư mục'**
  String get actionMove;

  /// No description provided for @actionRetry.
  ///
  /// In vi, this message translates to:
  /// **'Thử lại'**
  String get actionRetry;

  /// No description provided for @actionClose.
  ///
  /// In vi, this message translates to:
  /// **'Đóng'**
  String get actionClose;

  /// No description provided for @libraryAllDocuments.
  ///
  /// In vi, this message translates to:
  /// **'Tất cả tài liệu'**
  String get libraryAllDocuments;

  /// No description provided for @libraryRootFolder.
  ///
  /// In vi, this message translates to:
  /// **'Chưa xếp thư mục'**
  String get libraryRootFolder;

  /// No description provided for @libraryFolders.
  ///
  /// In vi, this message translates to:
  /// **'Thư mục'**
  String get libraryFolders;

  /// No description provided for @libraryTags.
  ///
  /// In vi, this message translates to:
  /// **'Nhãn'**
  String get libraryTags;

  /// No description provided for @libraryNewDocument.
  ///
  /// In vi, this message translates to:
  /// **'Tài liệu mới'**
  String get libraryNewDocument;

  /// No description provided for @libraryNewFolder.
  ///
  /// In vi, this message translates to:
  /// **'Thư mục mới'**
  String get libraryNewFolder;

  /// No description provided for @libraryNewTag.
  ///
  /// In vi, this message translates to:
  /// **'Nhãn mới'**
  String get libraryNewTag;

  /// No description provided for @libraryImportImages.
  ///
  /// In vi, this message translates to:
  /// **'Nhập ảnh'**
  String get libraryImportImages;

  /// No description provided for @libraryEmptyTitle.
  ///
  /// In vi, this message translates to:
  /// **'Chưa có tài liệu nào'**
  String get libraryEmptyTitle;

  /// No description provided for @libraryEmptyBody.
  ///
  /// In vi, this message translates to:
  /// **'Tạo tài liệu mới hoặc nhập ảnh có sẵn. Mọi dữ liệu được mã hóa và chỉ nằm trên máy này.'**
  String get libraryEmptyBody;

  /// No description provided for @libraryNoMatch.
  ///
  /// In vi, this message translates to:
  /// **'Không có tài liệu nào khớp bộ lọc'**
  String get libraryNoMatch;

  /// No description provided for @libraryClearFilter.
  ///
  /// In vi, this message translates to:
  /// **'Bỏ lọc'**
  String get libraryClearFilter;

  /// No description provided for @libraryPages.
  ///
  /// In vi, this message translates to:
  /// **'{count, plural, =0{Chưa có trang} other{{count} trang}}'**
  String libraryPages(int count);

  /// No description provided for @libraryUpdated.
  ///
  /// In vi, this message translates to:
  /// **'Sửa lúc {time}'**
  String libraryUpdated(String time);

  /// No description provided for @dialogDocumentTitle.
  ///
  /// In vi, this message translates to:
  /// **'Tên tài liệu'**
  String get dialogDocumentTitle;

  /// No description provided for @dialogFolderName.
  ///
  /// In vi, this message translates to:
  /// **'Tên thư mục'**
  String get dialogFolderName;

  /// No description provided for @dialogTagName.
  ///
  /// In vi, this message translates to:
  /// **'Tên nhãn'**
  String get dialogTagName;

  /// No description provided for @dialogDefaultDocumentTitle.
  ///
  /// In vi, this message translates to:
  /// **'Tài liệu {date}'**
  String dialogDefaultDocumentTitle(String date);

  /// No description provided for @dialogNameRequired.
  ///
  /// In vi, this message translates to:
  /// **'Nhập tên (tối đa 200 ký tự)'**
  String get dialogNameRequired;

  /// No description provided for @confirmDeleteDocumentTitle.
  ///
  /// In vi, this message translates to:
  /// **'Xóa tài liệu?'**
  String get confirmDeleteDocumentTitle;

  /// No description provided for @confirmDeleteDocumentBody.
  ///
  /// In vi, this message translates to:
  /// **'\"{title}\" và mọi trang của nó sẽ bị xóa hẳn khỏi máy này. Không thể hoàn tác.'**
  String confirmDeleteDocumentBody(String title);

  /// No description provided for @confirmDeleteFolderTitle.
  ///
  /// In vi, this message translates to:
  /// **'Xóa thư mục?'**
  String get confirmDeleteFolderTitle;

  /// No description provided for @confirmDeleteFolderBody.
  ///
  /// In vi, this message translates to:
  /// **'Tài liệu trong \"{name}\" được giữ lại và chuyển lên thư mục cha.'**
  String confirmDeleteFolderBody(String name);

  /// No description provided for @confirmDeleteTagTitle.
  ///
  /// In vi, this message translates to:
  /// **'Xóa nhãn?'**
  String get confirmDeleteTagTitle;

  /// No description provided for @confirmDeleteTagBody.
  ///
  /// In vi, this message translates to:
  /// **'Nhãn \"{name}\" sẽ được gỡ khỏi mọi tài liệu.'**
  String confirmDeleteTagBody(String name);

  /// No description provided for @confirmDeletePageTitle.
  ///
  /// In vi, this message translates to:
  /// **'Xóa trang?'**
  String get confirmDeletePageTitle;

  /// No description provided for @confirmDeletePageBody.
  ///
  /// In vi, this message translates to:
  /// **'Trang này sẽ bị xóa hẳn khỏi máy. Không thể hoàn tác.'**
  String get confirmDeletePageBody;

  /// No description provided for @documentNoPages.
  ///
  /// In vi, this message translates to:
  /// **'Tài liệu chưa có trang. Thêm ảnh để bắt đầu.'**
  String get documentNoPages;

  /// No description provided for @documentAddPages.
  ///
  /// In vi, this message translates to:
  /// **'Thêm trang từ ảnh'**
  String get documentAddPages;

  /// No description provided for @documentPageLabel.
  ///
  /// In vi, this message translates to:
  /// **'Trang {number}'**
  String documentPageLabel(int number);

  /// No description provided for @documentTagsTitle.
  ///
  /// In vi, this message translates to:
  /// **'Gắn nhãn'**
  String get documentTagsTitle;

  /// No description provided for @documentMoveRoot.
  ///
  /// In vi, this message translates to:
  /// **'Thư mục gốc'**
  String get documentMoveRoot;

  /// No description provided for @documentImported.
  ///
  /// In vi, this message translates to:
  /// **'{count, plural, other{Đã thêm {count} trang}}'**
  String documentImported(int count);

  /// No description provided for @documentImportSkipped.
  ///
  /// In vi, this message translates to:
  /// **'{count, plural, other{{count} ảnh không đọc được đã bị bỏ qua}}'**
  String documentImportSkipped(int count);

  /// No description provided for @documentOcrPending.
  ///
  /// In vi, this message translates to:
  /// **'Chưa có chữ nhận dạng (OCR có ở bản sau).'**
  String get documentOcrPending;

  /// No description provided for @pagerPerPage.
  ///
  /// In vi, this message translates to:
  /// **'/ trang'**
  String get pagerPerPage;

  /// No description provided for @pagerSummary.
  ///
  /// In vi, this message translates to:
  /// **'{from}-{to} / {total}'**
  String pagerSummary(int from, int to, int total);

  /// No description provided for @pagerEmpty.
  ///
  /// In vi, this message translates to:
  /// **'0 / 0'**
  String get pagerEmpty;

  /// No description provided for @pagerPrevious.
  ///
  /// In vi, this message translates to:
  /// **'Trước'**
  String get pagerPrevious;

  /// No description provided for @pagerNext.
  ///
  /// In vi, this message translates to:
  /// **'Sau'**
  String get pagerNext;

  /// No description provided for @pagerPageSizeLabel.
  ///
  /// In vi, this message translates to:
  /// **'Số bản ghi mỗi trang'**
  String get pagerPageSizeLabel;

  /// No description provided for @scanTitle.
  ///
  /// In vi, this message translates to:
  /// **'Quét tài liệu'**
  String get scanTitle;

  /// No description provided for @searchHint.
  ///
  /// In vi, this message translates to:
  /// **'Tìm theo tên hoặc chữ trong trang (gõ không dấu được)'**
  String get searchHint;

  /// No description provided for @searchMinChars.
  ///
  /// In vi, this message translates to:
  /// **'Nhập ít nhất 2 ký tự'**
  String get searchMinChars;

  /// No description provided for @searchNoResults.
  ///
  /// In vi, this message translates to:
  /// **'Không tìm thấy kết quả cho \"{query}\"'**
  String searchNoResults(String query);

  /// No description provided for @searchAccentLess.
  ///
  /// In vi, this message translates to:
  /// **'Khớp không dấu'**
  String get searchAccentLess;

  /// No description provided for @settingsAppearance.
  ///
  /// In vi, this message translates to:
  /// **'Giao diện'**
  String get settingsAppearance;

  /// No description provided for @settingsThemeLight.
  ///
  /// In vi, this message translates to:
  /// **'Sáng'**
  String get settingsThemeLight;

  /// No description provided for @settingsThemeDark.
  ///
  /// In vi, this message translates to:
  /// **'Tối'**
  String get settingsThemeDark;

  /// No description provided for @settingsThemeSystem.
  ///
  /// In vi, this message translates to:
  /// **'Hệ thống'**
  String get settingsThemeSystem;

  /// No description provided for @settingsLanguage.
  ///
  /// In vi, this message translates to:
  /// **'Ngôn ngữ'**
  String get settingsLanguage;

  /// No description provided for @settingsLanguageSystem.
  ///
  /// In vi, this message translates to:
  /// **'Theo hệ thống'**
  String get settingsLanguageSystem;

  /// No description provided for @settingsSecurity.
  ///
  /// In vi, this message translates to:
  /// **'Bảo mật'**
  String get settingsSecurity;

  /// No description provided for @settingsAppLock.
  ///
  /// In vi, this message translates to:
  /// **'Khóa ứng dụng bằng mã PIN'**
  String get settingsAppLock;

  /// No description provided for @settingsAppLockBody.
  ///
  /// In vi, this message translates to:
  /// **'Hỏi mã PIN khi mở app hoặc quay lại sau 1 phút'**
  String get settingsAppLockBody;

  /// No description provided for @settingsBiometric.
  ///
  /// In vi, this message translates to:
  /// **'Mở khóa bằng sinh trắc học'**
  String get settingsBiometric;

  /// No description provided for @settingsBiometricUnavailable.
  ///
  /// In vi, this message translates to:
  /// **'Thiết bị này không hỗ trợ hoặc chưa cài sinh trắc học'**
  String get settingsBiometricUnavailable;

  /// No description provided for @settingsChangePin.
  ///
  /// In vi, this message translates to:
  /// **'Đổi mã PIN'**
  String get settingsChangePin;

  /// No description provided for @settingsStorageNote.
  ///
  /// In vi, this message translates to:
  /// **'Dữ liệu được mã hóa bằng khóa lưu trong kho bí mật của hệ điều hành và không rời khỏi máy.'**
  String get settingsStorageNote;

  /// No description provided for @settingsAbout.
  ///
  /// In vi, this message translates to:
  /// **'Giới thiệu'**
  String get settingsAbout;

  /// No description provided for @settingsVersion.
  ///
  /// In vi, this message translates to:
  /// **'Phiên bản {app} · lõi {core}'**
  String settingsVersion(String app, String core);

  /// No description provided for @settingsLicense.
  ///
  /// In vi, this message translates to:
  /// **'Mã nguồn mở AGPL-3.0'**
  String get settingsLicense;

  /// No description provided for @settingsLicenses.
  ///
  /// In vi, this message translates to:
  /// **'Giấy phép bên thứ ba'**
  String get settingsLicenses;

  /// No description provided for @pinSetTitle.
  ///
  /// In vi, this message translates to:
  /// **'Đặt mã PIN'**
  String get pinSetTitle;

  /// No description provided for @pinConfirmTitle.
  ///
  /// In vi, this message translates to:
  /// **'Nhập lại mã PIN'**
  String get pinConfirmTitle;

  /// No description provided for @pinHint.
  ///
  /// In vi, this message translates to:
  /// **'4-12 chữ số'**
  String get pinHint;

  /// No description provided for @pinMismatch.
  ///
  /// In vi, this message translates to:
  /// **'Hai lần nhập không khớp'**
  String get pinMismatch;

  /// No description provided for @pinInvalid.
  ///
  /// In vi, this message translates to:
  /// **'Mã PIN gồm 4-12 chữ số'**
  String get pinInvalid;

  /// No description provided for @lockTitle.
  ///
  /// In vi, this message translates to:
  /// **'Tsu Scan đang khóa'**
  String get lockTitle;

  /// No description provided for @lockEnterPin.
  ///
  /// In vi, this message translates to:
  /// **'Nhập mã PIN'**
  String get lockEnterPin;

  /// No description provided for @lockUnlock.
  ///
  /// In vi, this message translates to:
  /// **'Mở khóa'**
  String get lockUnlock;

  /// No description provided for @lockUseBiometric.
  ///
  /// In vi, this message translates to:
  /// **'Dùng sinh trắc học'**
  String get lockUseBiometric;

  /// No description provided for @lockBiometricReason.
  ///
  /// In vi, this message translates to:
  /// **'Mở khóa Tsu Scan'**
  String get lockBiometricReason;

  /// No description provided for @lockWrongPin.
  ///
  /// In vi, this message translates to:
  /// **'Sai mã PIN ({count} lần)'**
  String lockWrongPin(int count);

  /// No description provided for @lockWait.
  ///
  /// In vi, this message translates to:
  /// **'Thử lại sau {seconds} giây'**
  String lockWait(int seconds);

  /// No description provided for @startupOpening.
  ///
  /// In vi, this message translates to:
  /// **'Đang mở kho tài liệu…'**
  String get startupOpening;

  /// No description provided for @startupWebNotice.
  ///
  /// In vi, this message translates to:
  /// **'Bản web đang thử nghiệm: dữ liệu chỉ nằm trong bộ nhớ và mất khi đóng tab. Bản web đầy đủ (mã hóa, lưu bền) sẽ có sau.'**
  String get startupWebNotice;

  /// No description provided for @startupErrorTitle.
  ///
  /// In vi, this message translates to:
  /// **'Không mở được kho tài liệu'**
  String get startupErrorTitle;

  /// No description provided for @startupKeyMissing.
  ///
  /// In vi, this message translates to:
  /// **'Khóa giải mã trong kho bí mật của máy không còn (có thể do khôi phục máy hoặc gỡ cài đặt dở). Dữ liệu cũ không thể đọc lại nếu thiếu khóa.'**
  String get startupKeyMissing;

  /// No description provided for @startupResetLibrary.
  ///
  /// In vi, this message translates to:
  /// **'Tạo kho mới (xóa dữ liệu cũ)'**
  String get startupResetLibrary;

  /// No description provided for @startupResetConfirm.
  ///
  /// In vi, this message translates to:
  /// **'Toàn bộ tài liệu cũ trên máy này sẽ bị xóa hẳn. Tiếp tục?'**
  String get startupResetConfirm;

  /// No description provided for @errorWrongKey.
  ///
  /// In vi, this message translates to:
  /// **'Khóa giải mã không khớp với kho trên máy.'**
  String get errorWrongKey;

  /// No description provided for @errorNotFound.
  ///
  /// In vi, this message translates to:
  /// **'Mục này không còn tồn tại.'**
  String get errorNotFound;

  /// No description provided for @errorInvalidInput.
  ///
  /// In vi, this message translates to:
  /// **'Dữ liệu nhập không hợp lệ.'**
  String get errorInvalidInput;

  /// No description provided for @errorUnsupportedImage.
  ///
  /// In vi, this message translates to:
  /// **'Ảnh không đọc được hoặc định dạng chưa hỗ trợ (JPEG, PNG, WebP).'**
  String get errorUnsupportedImage;

  /// No description provided for @errorImageTooLarge.
  ///
  /// In vi, this message translates to:
  /// **'Ảnh quá lớn (tối đa 100 megapixel).'**
  String get errorImageTooLarge;

  /// No description provided for @errorStorage.
  ///
  /// In vi, this message translates to:
  /// **'Lỗi lưu trữ trên máy (đầy bộ nhớ hoặc thiếu quyền).'**
  String get errorStorage;

  /// No description provided for @errorUnknown.
  ///
  /// In vi, this message translates to:
  /// **'Đã xảy ra lỗi không mong muốn.'**
  String get errorUnknown;

  /// No description provided for @pagerItems.
  ///
  /// In vi, this message translates to:
  /// **'{count, plural, other{{count} mục}}'**
  String pagerItems(int count);

  /// No description provided for @pagerAnnounce.
  ///
  /// In vi, this message translates to:
  /// **'Đang hiển thị {from} đến {to} trong {total} mục'**
  String pagerAnnounce(int from, int to, int total);

  /// No description provided for @pagerFiltered.
  ///
  /// In vi, this message translates to:
  /// **'{range} (lọc từ {all})'**
  String pagerFiltered(String range, int all);

  /// No description provided for @scanIntro.
  ///
  /// In vi, this message translates to:
  /// **'Chọn chế độ rồi chụp. Mép giấy được dò và nắn thẳng ngay trên máy; ảnh không rời khỏi máy và được mã hóa khi lưu.'**
  String get scanIntro;

  /// No description provided for @scanStartCamera.
  ///
  /// In vi, this message translates to:
  /// **'Chụp bằng camera'**
  String get scanStartCamera;

  /// No description provided for @scanImport.
  ///
  /// In vi, this message translates to:
  /// **'Nhập ảnh có sẵn'**
  String get scanImport;

  /// No description provided for @scanCameraUnsupported.
  ///
  /// In vi, this message translates to:
  /// **'Máy này chưa quét bằng camera trong app được. Hãy nhập ảnh có sẵn - mép giấy vẫn được dò và nắn thẳng.'**
  String get scanCameraUnsupported;

  /// No description provided for @scanModeLabel.
  ///
  /// In vi, this message translates to:
  /// **'Chế độ quét'**
  String get scanModeLabel;

  /// No description provided for @scanModeDocument.
  ///
  /// In vi, this message translates to:
  /// **'Tài liệu'**
  String get scanModeDocument;

  /// No description provided for @scanModeIdCard.
  ///
  /// In vi, this message translates to:
  /// **'CCCD 2 mặt'**
  String get scanModeIdCard;

  /// No description provided for @scanModeBusinessCard.
  ///
  /// In vi, this message translates to:
  /// **'Danh thiếp'**
  String get scanModeBusinessCard;

  /// No description provided for @scanModeWhiteboard.
  ///
  /// In vi, this message translates to:
  /// **'Bảng trắng'**
  String get scanModeWhiteboard;

  /// No description provided for @scanModePhoto.
  ///
  /// In vi, this message translates to:
  /// **'Ảnh'**
  String get scanModePhoto;

  /// No description provided for @scanPermissionDenied.
  ///
  /// In vi, this message translates to:
  /// **'Tsu Scan chưa được phép dùng camera. Bật quyền Camera cho Tsu Scan trong Cài đặt của máy rồi thử lại, hoặc nhập ảnh có sẵn.'**
  String get scanPermissionDenied;

  /// No description provided for @scanOpenSettings.
  ///
  /// In vi, this message translates to:
  /// **'Mở Cài đặt'**
  String get scanOpenSettings;

  /// No description provided for @scanModeBook.
  ///
  /// In vi, this message translates to:
  /// **'Sách'**
  String get scanModeBook;

  /// No description provided for @scanModeCodes.
  ///
  /// In vi, this message translates to:
  /// **'QR/mã vạch'**
  String get scanModeCodes;

  /// No description provided for @scanHintBook.
  ///
  /// In vi, this message translates to:
  /// **'Đặt hai trang sách nằm ngang trong khung'**
  String get scanHintBook;

  /// No description provided for @scanHintCodes.
  ///
  /// In vi, this message translates to:
  /// **'Đưa mã QR hoặc mã vạch vào khung'**
  String get scanHintCodes;

  /// No description provided for @scanCodeCopy.
  ///
  /// In vi, this message translates to:
  /// **'Sao chép'**
  String get scanCodeCopy;

  /// No description provided for @scanCodeCopied.
  ///
  /// In vi, this message translates to:
  /// **'Đã sao chép'**
  String get scanCodeCopied;

  /// No description provided for @scanCodesNone.
  ///
  /// In vi, this message translates to:
  /// **'Không tìm thấy mã QR hay mã vạch nào.'**
  String get scanCodesNone;

  /// No description provided for @scanCodesTitle.
  ///
  /// In vi, this message translates to:
  /// **'{count, plural, other{Đọc được {count} mã}}'**
  String scanCodesTitle(int count);

  /// No description provided for @scanNoCamera.
  ///
  /// In vi, this message translates to:
  /// **'Không tìm thấy camera trên máy này.'**
  String get scanNoCamera;

  /// No description provided for @scanCameraError.
  ///
  /// In vi, this message translates to:
  /// **'Không mở được camera. Đóng các app khác đang dùng camera rồi thử lại.'**
  String get scanCameraError;

  /// No description provided for @scanHintFind.
  ///
  /// In vi, this message translates to:
  /// **'Đưa tài liệu vào khung'**
  String get scanHintFind;

  /// No description provided for @scanHintHold.
  ///
  /// In vi, this message translates to:
  /// **'Giữ yên…'**
  String get scanHintHold;

  /// No description provided for @scanHintNext.
  ///
  /// In vi, this message translates to:
  /// **'Đã chụp - đặt trang tiếp theo'**
  String get scanHintNext;

  /// No description provided for @scanHintManual.
  ///
  /// In vi, this message translates to:
  /// **'Bấm nút chụp khi sẵn sàng'**
  String get scanHintManual;

  /// No description provided for @scanSideFront.
  ///
  /// In vi, this message translates to:
  /// **'Mặt trước'**
  String get scanSideFront;

  /// No description provided for @scanSideBack.
  ///
  /// In vi, this message translates to:
  /// **'Mặt sau'**
  String get scanSideBack;

  /// No description provided for @scanAutoCapture.
  ///
  /// In vi, this message translates to:
  /// **'Tự chụp'**
  String get scanAutoCapture;

  /// No description provided for @scanFlashOff.
  ///
  /// In vi, this message translates to:
  /// **'Đèn: tắt'**
  String get scanFlashOff;

  /// No description provided for @scanFlashAuto.
  ///
  /// In vi, this message translates to:
  /// **'Đèn: tự động'**
  String get scanFlashAuto;

  /// No description provided for @scanFlashTorch.
  ///
  /// In vi, this message translates to:
  /// **'Đèn: luôn bật'**
  String get scanFlashTorch;

  /// No description provided for @scanShutter.
  ///
  /// In vi, this message translates to:
  /// **'Chụp'**
  String get scanShutter;

  /// No description provided for @scanPageCount.
  ///
  /// In vi, this message translates to:
  /// **'{count, plural, other{{count} trang}}'**
  String scanPageCount(int count);

  /// No description provided for @scanDone.
  ///
  /// In vi, this message translates to:
  /// **'Xong'**
  String get scanDone;

  /// No description provided for @scanDiscardTitle.
  ///
  /// In vi, this message translates to:
  /// **'Bỏ các trang đã chụp?'**
  String get scanDiscardTitle;

  /// No description provided for @scanDiscardBody.
  ///
  /// In vi, this message translates to:
  /// **'{count, plural, other{{count} trang chưa lưu sẽ bị xóa khỏi máy.}}'**
  String scanDiscardBody(int count);

  /// No description provided for @scanDiscard.
  ///
  /// In vi, this message translates to:
  /// **'Bỏ'**
  String get scanDiscard;

  /// No description provided for @scanReviewTitle.
  ///
  /// In vi, this message translates to:
  /// **'Duyệt trang'**
  String get scanReviewTitle;

  /// No description provided for @scanAddMore.
  ///
  /// In vi, this message translates to:
  /// **'Chụp thêm'**
  String get scanAddMore;

  /// No description provided for @scanSaveDocument.
  ///
  /// In vi, this message translates to:
  /// **'Lưu tài liệu'**
  String get scanSaveDocument;

  /// No description provided for @scanSaving.
  ///
  /// In vi, this message translates to:
  /// **'Đang lưu {done}/{total}…'**
  String scanSaving(int done, int total);

  /// No description provided for @scanProcessing.
  ///
  /// In vi, this message translates to:
  /// **'Đang xử lý ảnh…'**
  String get scanProcessing;

  /// No description provided for @scanPageN.
  ///
  /// In vi, this message translates to:
  /// **'Trang {n}'**
  String scanPageN(int n);

  /// No description provided for @scanEmptyReview.
  ///
  /// In vi, this message translates to:
  /// **'Chưa có trang nào.'**
  String get scanEmptyReview;

  /// No description provided for @scanEditTitle.
  ///
  /// In vi, this message translates to:
  /// **'Chỉnh trang'**
  String get scanEditTitle;

  /// No description provided for @scanEditPage.
  ///
  /// In vi, this message translates to:
  /// **'Chỉnh góc và bộ lọc'**
  String get scanEditPage;

  /// No description provided for @scanDeletePage.
  ///
  /// In vi, this message translates to:
  /// **'Xóa trang'**
  String get scanDeletePage;

  /// No description provided for @scanMoveUp.
  ///
  /// In vi, this message translates to:
  /// **'Lên trước'**
  String get scanMoveUp;

  /// No description provided for @scanMoveDown.
  ///
  /// In vi, this message translates to:
  /// **'Xuống sau'**
  String get scanMoveDown;

  /// No description provided for @scanFullFrame.
  ///
  /// In vi, this message translates to:
  /// **'Cả khung'**
  String get scanFullFrame;

  /// No description provided for @scanRedetect.
  ///
  /// In vi, this message translates to:
  /// **'Dò lại'**
  String get scanRedetect;

  /// No description provided for @scanNotFound.
  ///
  /// In vi, this message translates to:
  /// **'Không dò được mép giấy - kéo bốn góc bằng tay.'**
  String get scanNotFound;

  /// No description provided for @scanInvalidQuad.
  ///
  /// In vi, this message translates to:
  /// **'Bốn góc phải tạo thành một tứ giác lồi.'**
  String get scanInvalidQuad;

  /// No description provided for @scanShowResult.
  ///
  /// In vi, this message translates to:
  /// **'Xem kết quả'**
  String get scanShowResult;

  /// No description provided for @scanShowCorners.
  ///
  /// In vi, this message translates to:
  /// **'Chỉnh góc'**
  String get scanShowCorners;

  /// No description provided for @scanFilterLabel.
  ///
  /// In vi, this message translates to:
  /// **'Bộ lọc'**
  String get scanFilterLabel;

  /// No description provided for @scanFilterOriginal.
  ///
  /// In vi, this message translates to:
  /// **'Gốc'**
  String get scanFilterOriginal;

  /// No description provided for @scanFilterAuto.
  ///
  /// In vi, this message translates to:
  /// **'Tự động'**
  String get scanFilterAuto;

  /// No description provided for @scanFilterColor.
  ///
  /// In vi, this message translates to:
  /// **'Màu'**
  String get scanFilterColor;

  /// No description provided for @scanFilterGray.
  ///
  /// In vi, this message translates to:
  /// **'Xám'**
  String get scanFilterGray;

  /// No description provided for @scanFilterBw.
  ///
  /// In vi, this message translates to:
  /// **'Đen trắng'**
  String get scanFilterBw;

  /// No description provided for @scanFilterShadow.
  ///
  /// In vi, this message translates to:
  /// **'Khử bóng'**
  String get scanFilterShadow;

  /// No description provided for @scanFilterWhiteboard.
  ///
  /// In vi, this message translates to:
  /// **'Bảng trắng'**
  String get scanFilterWhiteboard;

  /// No description provided for @scanCornerTopLeft.
  ///
  /// In vi, this message translates to:
  /// **'Góc trên bên trái'**
  String get scanCornerTopLeft;

  /// No description provided for @scanCornerTopRight.
  ///
  /// In vi, this message translates to:
  /// **'Góc trên bên phải'**
  String get scanCornerTopRight;

  /// No description provided for @scanCornerBottomRight.
  ///
  /// In vi, this message translates to:
  /// **'Góc dưới bên phải'**
  String get scanCornerBottomRight;

  /// No description provided for @scanCornerBottomLeft.
  ///
  /// In vi, this message translates to:
  /// **'Góc dưới bên trái'**
  String get scanCornerBottomLeft;

  /// No description provided for @scanCornerHint.
  ///
  /// In vi, this message translates to:
  /// **'Kéo để chỉnh; dùng phím mũi tên khi đang chọn'**
  String get scanCornerHint;
}

class _AppLocalizationsDelegate extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) => <String>['en', 'vi'].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {
  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'en':
      return AppLocalizationsEn();
    case 'vi':
      return AppLocalizationsVi();
  }

  throw FlutterError(
    'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
    'an issue with the localizations generation tool. Please file an issue '
    'on GitHub with a reproducible sample app and the gen-l10n configuration '
    'that was used.',
  );
}

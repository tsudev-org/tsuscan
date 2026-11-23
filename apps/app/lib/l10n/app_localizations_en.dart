// ignore: unused_import
import 'package:intl/intl.dart' as intl;

import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get appTitle => 'Tsu Scan';

  @override
  String get navLibrary => 'Documents';

  @override
  String get navScan => 'Scan';

  @override
  String get navSearch => 'Search';

  @override
  String get navSettings => 'Settings';

  @override
  String get actionCancel => 'Cancel';

  @override
  String get actionSave => 'Save';

  @override
  String get actionCreate => 'Create';

  @override
  String get actionDelete => 'Delete';

  @override
  String get actionRename => 'Rename';

  @override
  String get actionMove => 'Move to folder';

  @override
  String get actionRetry => 'Retry';

  @override
  String get actionClose => 'Close';

  @override
  String get libraryAllDocuments => 'All documents';

  @override
  String get libraryRootFolder => 'Unfiled';

  @override
  String get libraryFolders => 'Folders';

  @override
  String get libraryTags => 'Tags';

  @override
  String get libraryNewDocument => 'New document';

  @override
  String get libraryNewFolder => 'New folder';

  @override
  String get libraryNewTag => 'New tag';

  @override
  String get libraryImportImages => 'Import images';

  @override
  String get libraryEmptyTitle => 'No documents yet';

  @override
  String get libraryEmptyBody =>
      'Create a new document or import existing images. Everything is encrypted and stays on this device.';

  @override
  String get libraryNoMatch => 'No documents match this filter';

  @override
  String get libraryClearFilter => 'Clear filter';

  @override
  String libraryPages(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count pages',
      one: '1 page',
      zero: 'No pages',
    );
    return '$_temp0';
  }

  @override
  String libraryUpdated(String time) {
    return 'Edited $time';
  }

  @override
  String get dialogDocumentTitle => 'Document name';

  @override
  String get dialogFolderName => 'Folder name';

  @override
  String get dialogTagName => 'Tag name';

  @override
  String dialogDefaultDocumentTitle(String date) {
    return 'Document $date';
  }

  @override
  String get dialogNameRequired => 'Enter a name (up to 200 characters)';

  @override
  String get confirmDeleteDocumentTitle => 'Delete document?';

  @override
  String confirmDeleteDocumentBody(String title) {
    return '\"$title\" and all of its pages will be permanently deleted from this device. This cannot be undone.';
  }

  @override
  String get confirmDeleteFolderTitle => 'Delete folder?';

  @override
  String confirmDeleteFolderBody(String name) {
    return 'Documents in \"$name\" are kept and moved to the parent folder.';
  }

  @override
  String get confirmDeleteTagTitle => 'Delete tag?';

  @override
  String confirmDeleteTagBody(String name) {
    return 'The tag \"$name\" will be removed from every document.';
  }

  @override
  String get confirmDeletePageTitle => 'Delete page?';

  @override
  String get confirmDeletePageBody => 'This page will be permanently deleted from this device. This cannot be undone.';

  @override
  String get documentNoPages => 'This document has no pages yet. Add images to get started.';

  @override
  String get documentAddPages => 'Add pages from images';

  @override
  String documentPageLabel(int number) {
    return 'Page $number';
  }

  @override
  String get documentTagsTitle => 'Tags';

  @override
  String get documentMoveRoot => 'Top level';

  @override
  String documentImported(int count) {
    String _temp0 = intl.Intl.pluralLogic(count, locale: localeName, other: 'Added $count pages', one: 'Added 1 page');
    return '$_temp0';
  }

  @override
  String documentImportSkipped(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count unreadable images were skipped',
      one: '1 unreadable image was skipped',
    );
    return '$_temp0';
  }

  @override
  String get documentOcrPending => 'No recognized text yet (OCR arrives in a later version).';

  @override
  String get pagerPerPage => '/ page';

  @override
  String pagerSummary(int from, int to, int total) {
    final intl.NumberFormat fromNumberFormat = intl.NumberFormat.decimalPattern(localeName);
    final String fromString = fromNumberFormat.format(from);
    final intl.NumberFormat toNumberFormat = intl.NumberFormat.decimalPattern(localeName);
    final String toString = toNumberFormat.format(to);
    final intl.NumberFormat totalNumberFormat = intl.NumberFormat.decimalPattern(localeName);
    final String totalString = totalNumberFormat.format(total);

    return '$fromString-$toString of $totalString';
  }

  @override
  String get pagerEmpty => '0 of 0';

  @override
  String get pagerPrevious => 'Previous';

  @override
  String get pagerNext => 'Next';

  @override
  String get pagerPageSizeLabel => 'Records per page';

  @override
  String get scanTitle => 'Scan documents';

  @override
  String get searchHint => 'Search titles or page text (accents optional)';

  @override
  String get searchMinChars => 'Type at least 2 characters';

  @override
  String searchNoResults(String query) {
    return 'No results for \"$query\"';
  }

  @override
  String get searchAccentLess => 'Accent-insensitive match';

  @override
  String get settingsAppearance => 'Appearance';

  @override
  String get settingsThemeLight => 'Light';

  @override
  String get settingsThemeDark => 'Dark';

  @override
  String get settingsThemeSystem => 'System';

  @override
  String get settingsLanguage => 'Language';

  @override
  String get settingsLanguageSystem => 'System default';

  @override
  String get settingsSecurity => 'Security';

  @override
  String get settingsAppLock => 'Lock app with a PIN';

  @override
  String get settingsAppLockBody => 'Ask for the PIN on launch or after 1 minute in the background';

  @override
  String get settingsBiometric => 'Unlock with biometrics';

  @override
  String get settingsBiometricUnavailable => 'Biometrics are unavailable or not set up on this device';

  @override
  String get settingsChangePin => 'Change PIN';

  @override
  String get settingsStorageNote =>
      'Your data is encrypted with a key kept in the operating system\'s secure storage and never leaves this device.';

  @override
  String get settingsAbout => 'About';

  @override
  String settingsVersion(String app, String core) {
    return 'Version $app · core $core';
  }

  @override
  String get settingsLicense => 'Open source under AGPL-3.0';

  @override
  String get settingsLicenses => 'Third-party licenses';

  @override
  String get pinSetTitle => 'Set a PIN';

  @override
  String get pinConfirmTitle => 'Confirm your PIN';

  @override
  String get pinHint => '4-12 digits';

  @override
  String get pinMismatch => 'The PINs do not match';

  @override
  String get pinInvalid => 'A PIN must be 4-12 digits';

  @override
  String get lockTitle => 'Tsu Scan is locked';

  @override
  String get lockEnterPin => 'Enter your PIN';

  @override
  String get lockUnlock => 'Unlock';

  @override
  String get lockUseBiometric => 'Use biometrics';

  @override
  String get lockBiometricReason => 'Unlock Tsu Scan';

  @override
  String lockWrongPin(int count) {
    return 'Wrong PIN ($count attempts)';
  }

  @override
  String lockWait(int seconds) {
    return 'Try again in $seconds seconds';
  }

  @override
  String get startupOpening => 'Opening your library…';

  @override
  String get startupWebNotice =>
      'The web version is a preview: data lives in memory only and is lost when the tab closes. The full web version (encrypted, persistent) comes later.';

  @override
  String get startupErrorTitle => 'Could not open your library';

  @override
  String get startupKeyMissing =>
      'The decryption key in this device\'s secure storage is gone (for example after a device restore or an incomplete uninstall). The old data cannot be read without it.';

  @override
  String get startupResetLibrary => 'Create a new library (erase old data)';

  @override
  String get startupResetConfirm => 'All old documents on this device will be permanently erased. Continue?';

  @override
  String get errorWrongKey => 'The decryption key does not match the library on this device.';

  @override
  String get errorNotFound => 'This item no longer exists.';

  @override
  String get errorInvalidInput => 'The input is not valid.';

  @override
  String get errorUnsupportedImage => 'The image is unreadable or its format is not supported (JPEG, PNG, WebP).';

  @override
  String get errorImageTooLarge => 'The image is too large (100 megapixels max).';

  @override
  String get errorStorage => 'A storage error occurred (disk full or no permission).';

  @override
  String get errorUnknown => 'An unexpected error occurred.';

  @override
  String pagerItems(int count) {
    final intl.NumberFormat countNumberFormat = intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(count, locale: localeName, other: '$countString items', one: '1 item');
    return '$_temp0';
  }

  @override
  String pagerAnnounce(int from, int to, int total) {
    return 'Showing $from to $to of $total items';
  }

  @override
  String pagerFiltered(String range, int all) {
    final intl.NumberFormat allNumberFormat = intl.NumberFormat.decimalPattern(localeName);
    final String allString = allNumberFormat.format(all);

    return '$range (filtered from $allString)';
  }

  @override
  String get scanIntro =>
      'Pick a mode and shoot. Page edges are detected and straightened on this device; images never leave it and are encrypted when saved.';

  @override
  String get scanStartCamera => 'Scan with camera';

  @override
  String get scanImport => 'Import existing photos';

  @override
  String get scanCameraUnsupported =>
      'In-app camera scanning is not available on this device yet. Import existing photos instead - edges are still detected and straightened.';

  @override
  String get scanModeLabel => 'Scan mode';

  @override
  String get scanModeDocument => 'Document';

  @override
  String get scanModeIdCard => 'ID card (2 sides)';

  @override
  String get scanModeBusinessCard => 'Business card';

  @override
  String get scanModeWhiteboard => 'Whiteboard';

  @override
  String get scanModePhoto => 'Photo';

  @override
  String get scanPermissionDenied =>
      'Tsu Scan is not allowed to use the camera. Turn on the Camera permission for Tsu Scan in your device settings and try again, or import existing photos.';

  @override
  String get scanOpenSettings => 'Open settings';

  @override
  String get scanModeBook => 'Book';

  @override
  String get scanModeCodes => 'QR/barcode';

  @override
  String get scanHintBook => 'Place both book pages side by side in the frame';

  @override
  String get scanHintCodes => 'Point at a QR code or barcode';

  @override
  String get scanCodeCopy => 'Copy';

  @override
  String get scanCodeCopied => 'Copied';

  @override
  String get scanCodesNone => 'No QR code or barcode found.';

  @override
  String scanCodesTitle(int count) {
    String _temp0 = intl.Intl.pluralLogic(count, locale: localeName, other: '$count codes found', one: '1 code found');
    return '$_temp0';
  }

  @override
  String get scanNoCamera => 'No camera found on this device.';

  @override
  String get scanCameraError => 'Could not open the camera. Close other apps using it and try again.';

  @override
  String get scanHintFind => 'Fit the document in the frame';

  @override
  String get scanHintHold => 'Hold still…';

  @override
  String get scanHintNext => 'Captured - place the next page';

  @override
  String get scanHintManual => 'Press the shutter when ready';

  @override
  String get scanSideFront => 'Front side';

  @override
  String get scanSideBack => 'Back side';

  @override
  String get scanAutoCapture => 'Auto capture';

  @override
  String get scanFlashOff => 'Flash: off';

  @override
  String get scanFlashAuto => 'Flash: auto';

  @override
  String get scanFlashTorch => 'Flash: always on';

  @override
  String get scanShutter => 'Capture';

  @override
  String scanPageCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(count, locale: localeName, other: '$count pages', one: '1 page');
    return '$_temp0';
  }

  @override
  String get scanDone => 'Done';

  @override
  String get scanDiscardTitle => 'Discard captured pages?';

  @override
  String scanDiscardBody(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count unsaved pages will be deleted.',
      one: '1 unsaved page will be deleted.',
    );
    return '$_temp0';
  }

  @override
  String get scanDiscard => 'Discard';

  @override
  String get scanReviewTitle => 'Review pages';

  @override
  String get scanAddMore => 'Capture more';

  @override
  String get scanSaveDocument => 'Save document';

  @override
  String scanSaving(int done, int total) {
    return 'Saving $done/$total…';
  }

  @override
  String get scanProcessing => 'Processing images…';

  @override
  String scanPageN(int n) {
    return 'Page $n';
  }

  @override
  String get scanEmptyReview => 'No pages yet.';

  @override
  String get scanEditTitle => 'Edit page';

  @override
  String get scanEditPage => 'Adjust corners and filter';

  @override
  String get scanDeletePage => 'Delete page';

  @override
  String get scanMoveUp => 'Move up';

  @override
  String get scanMoveDown => 'Move down';

  @override
  String get scanFullFrame => 'Full frame';

  @override
  String get scanRedetect => 'Detect again';

  @override
  String get scanNotFound => 'No page edges found - drag the four corners by hand.';

  @override
  String get scanInvalidQuad => 'The four corners must form a convex shape.';

  @override
  String get scanShowResult => 'Show result';

  @override
  String get scanShowCorners => 'Adjust corners';

  @override
  String get scanFilterLabel => 'Filter';

  @override
  String get scanFilterOriginal => 'Original';

  @override
  String get scanFilterAuto => 'Auto';

  @override
  String get scanFilterColor => 'Color';

  @override
  String get scanFilterGray => 'Grayscale';

  @override
  String get scanFilterBw => 'Black & white';

  @override
  String get scanFilterShadow => 'Remove shadows';

  @override
  String get scanFilterWhiteboard => 'Whiteboard';

  @override
  String get scanCornerTopLeft => 'Top-left corner';

  @override
  String get scanCornerTopRight => 'Top-right corner';

  @override
  String get scanCornerBottomRight => 'Bottom-right corner';

  @override
  String get scanCornerBottomLeft => 'Bottom-left corner';

  @override
  String get scanCornerHint => 'Drag to adjust; use arrow keys when focused';

  @override
  String get confirmTrashDocumentTitle => 'Move to trash?';

  @override
  String confirmTrashDocumentBody(String title, int days) {
    return '\"$title\" stays in the trash for $days days and is then permanently deleted. You can restore it before then.';
  }

  @override
  String get actionMoveToTrash => 'Move to trash';

  @override
  String get actionUndo => 'Undo';

  @override
  String get actionRestore => 'Restore';

  @override
  String get actionDeleteForever => 'Delete forever';

  @override
  String trashMoved(String title) {
    return 'Moved \"$title\" to the trash';
  }

  @override
  String trashRestored(String title) {
    return 'Restored \"$title\"';
  }

  @override
  String get trashTitle => 'Trash';

  @override
  String get trashEmpty => 'The trash is empty';

  @override
  String trashHint(int days) {
    return 'Documents in the trash are permanently deleted after $days days.';
  }

  @override
  String trashPurgeOn(String date) {
    return 'Deleted for good on $date';
  }

  @override
  String get trashEmptyAll => 'Empty trash';

  @override
  String get confirmEmptyTrashTitle => 'Empty the trash?';

  @override
  String confirmEmptyTrashBody(int count) {
    return '$count documents will be permanently deleted from this device. This cannot be undone.';
  }

  @override
  String get confirmPurgeTitle => 'Delete forever?';

  @override
  String confirmPurgeBody(String title) {
    return '\"$title\" and all of its pages will be permanently deleted from this device. This cannot be undone.';
  }

  @override
  String get pageActions => 'Page actions';

  @override
  String get pageRotateRight => 'Rotate right';

  @override
  String get pageRotateLeft => 'Rotate left';

  @override
  String get pageMoveEarlier => 'Move earlier';

  @override
  String get pageMoveLater => 'Move later';

  @override
  String get pageMoveToDocument => 'Move to another document…';

  @override
  String get pageSplitFromHere => 'Split into a new document from here';

  @override
  String pageMovedTo(String title) {
    return 'Moved the page to \"$title\"';
  }

  @override
  String documentSplitTitle(String title) {
    return '$title (part 2)';
  }

  @override
  String documentSplitDone(int count, String title) {
    return 'Moved $count pages to \"$title\"';
  }

  @override
  String get documentMergeInto => 'Merge another document into this…';

  @override
  String get documentPickTitle => 'Choose a document';

  @override
  String get documentNoOthers => 'There are no other documents';

  @override
  String documentMerged(int count, String title) {
    return 'Merged $count pages from \"$title\"';
  }
}

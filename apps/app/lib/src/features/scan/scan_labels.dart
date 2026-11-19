import 'package:flutter/material.dart';

import '../../../l10n/app_localizations.dart';
import '../../data/scan_engine.dart';
import 'scan_session.dart';

extension ScanModeLabel on ScanMode {
  String label(AppLocalizations l) => switch (this) {
    ScanMode.document => l.scanModeDocument,
    ScanMode.idCard => l.scanModeIdCard,
    ScanMode.businessCard => l.scanModeBusinessCard,
    ScanMode.whiteboard => l.scanModeWhiteboard,
    ScanMode.photo => l.scanModePhoto,
  };

  IconData get icon => switch (this) {
    ScanMode.document => Icons.description_outlined,
    ScanMode.idCard => Icons.badge_outlined,
    ScanMode.businessCard => Icons.contact_mail_outlined,
    ScanMode.whiteboard => Icons.dashboard_outlined,
    ScanMode.photo => Icons.photo_outlined,
  };
}

extension ScanFilterLabel on ScanFilter {
  String label(AppLocalizations l) => switch (this) {
    ScanFilter.original => l.scanFilterOriginal,
    ScanFilter.auto => l.scanFilterAuto,
    ScanFilter.color => l.scanFilterColor,
    ScanFilter.gray => l.scanFilterGray,
    ScanFilter.bw => l.scanFilterBw,
    ScanFilter.shadow => l.scanFilterShadow,
    ScanFilter.whiteboard => l.scanFilterWhiteboard,
  };
}

/// Tên góc cho trình đọc màn hình (thứ tự TL, TR, BR, BL).
String cornerLabel(AppLocalizations l, int i) => switch (i) {
  0 => l.scanCornerTopLeft,
  1 => l.scanCornerTopRight,
  2 => l.scanCornerBottomRight,
  _ => l.scanCornerBottomLeft,
};

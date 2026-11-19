import 'package:flutter_test/flutter_test.dart';
import 'package:tsuscan/src/utils/format.dart';
import 'package:tsuscan/src/utils/vi_text.dart';

void main() {
  group('viNormalize', () {
    test('bỏ dấu, chữ thường, "đ" thành "d", gộp khoảng trắng', () {
      expect(viNormalize('  Đường   Lê LỢI '), 'duong le loi');
    });

    test('dấu tổ hợp rời (NFD) cũng bị bỏ', () {
      expect(viNormalize('Hợp'), 'hop');
    });
  });

  group('viHighlightRanges', () {
    String spans(String text, String q) => viHighlightRanges(text, q).map((r) => text.substring(r.$1, r.$2)).join('|');

    test('gõ không dấu tô sáng chữ có dấu', () {
      expect(spans('Hợp đồng thuê nhà', 'hop dong'), 'Hợp|đồng');
    });

    test('tô cả dấu tổ hợp của ký tự cuối', () {
      const nfd = 'Hợp';
      expect(spans(nfd, 'hop'), nfd);
    });

    test('vị trí đúng khi có ký tự ngoài BMP', () {
      expect(spans('📄 Hóa đơn', 'hoa'), 'Hóa');
    });

    test('không khớp thì rỗng', () {
      expect(viHighlightRanges('Biên bản', 'hợp'), isEmpty);
    });
  });

  test('định dạng ngày giờ HH:mm DD/MM/YYYY', () {
    expect(formatDateTimeVN(DateTime(2027, 2, 1, 9, 5)), '09:05 01/02/2027');
    expect(formatDateVN(DateTime(2027, 2, 1)), '01/02/2027');
  });
}

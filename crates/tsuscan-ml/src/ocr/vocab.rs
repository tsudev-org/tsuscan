//! Bảng ký tự của AsterNet v1.0 (asterxlabs, giấy phép MIT), sinh từ `vocab.json` ở commit
//! `aec29ab0aaf26061e861cb4b3a301988744c5fa3`. Chỉ số 0 là ký hiệu trống CTC, chuỗi dưới đây bắt đầu từ chỉ số 1.

/// Số lớp đầu ra (gồm ký hiệu trống).
pub const ASTERNET_CLASSES: usize = 227;

/// Ký tự theo chỉ số lớp 1..N.
pub const ASTERNET_CHARS: &str = "aàáảãạăằắẳẵặâầấẩẫậbcdđeèéẻẽẹêềếểễệfghiìíỉĩịjklmnoòóỏõọôồốổỗộơờớởỡợpqrstuùúủũụưừứửữựvwxyỳýỷỹỵzAÀÁẢÃẠĂẰẮẲẴẶÂẦẤẨẪẬBCDĐEÈÉẺẼẸÊỀẾỂỄỆFGHIÌÍỈĨỊJKLMNOÒÓỎÕỌÔỒỐỔỖỘƠỜỚỞỠỢPQRSTUÙÚỦŨỤƯỪỨỬỮỰVWXYỲÝỶỸỴZ0123456789 ,.-:;\"'?!()[]{}/%+$&=*#@_\\<>°";

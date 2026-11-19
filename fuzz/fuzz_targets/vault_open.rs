//! Bản mã/tệp trang/HLC/cụm từ khôi phục từ máy chủ hoặc đĩa (dữ liệu không tin cậy): chỉ được trả Err/Ok, không panic,
//! không cấp phát theo độ dài khai báo. Khóa cố định: hầu hết đầu vào trượt xác thực - đích chính là bộ phân tích.
#![no_main]

use libfuzzer_sys::fuzz_target;
use tsuscan_vault::envelope::{self, Kind};
use tsuscan_vault::{Key32, hlc, keys, stream};

fuzz_target!(|data: &[u8]| {
    let key = Key32::from_bytes([7; 32]);
    let _ = envelope::open(&key, Kind::Record, b"ctx", data);
    let _ = envelope::open(&key, Kind::Key, b"ctx", data);
    let _ = keys::unwrap(&key, "dk|x", data);
    let mut out = Vec::new();
    let _ = stream::decrypt(&key, b"fk|x", data, &mut out);
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = hlc::Hlc::decode(s);
        let _ = keys::RecoveryPhrase::parse(s);
    }
});

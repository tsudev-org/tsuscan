//! Đọc mã vạch trên ảnh giải mã từ byte bất kỳ: không panic/tràn bộ nhớ.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(img) = tsuscan_imaging::decode(data) {
        let _ = tsuscan_doc::barcode::scan(&img);
    }
});

//! Giải mã byte bất kỳ: chỉ được trả Err, không panic/tràn bộ nhớ.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = tsuscan_imaging::decode(data);
});

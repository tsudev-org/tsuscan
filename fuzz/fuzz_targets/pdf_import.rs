//! Nhập PDF/ảnh bất kỳ (dữ liệu không tin cậy): chỉ được trả Err/Ok, không panic/tràn bộ nhớ.
//! libFuzzer biến panic (kể cả panic bị `catch_unwind` chặn trong crate) thành lỗi ⇒ bắt cả lỗi của `hayro`.
#![no_main]

use libfuzzer_sys::fuzz_target;
use tsuscan_doc::pdf::read::ImportOptions;

fuzz_target!(|data: &[u8]| {
    let opts = ImportOptions {
        dpi: 36.0,
        password: String::new(),
        pages: vec![0],
    };
    let _ = tsuscan_doc::pdf::read::page_count(data, "");
    let _ = tsuscan_doc::import::import(data, &opts);
});

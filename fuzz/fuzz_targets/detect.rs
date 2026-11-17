//! Dò mép + nắn trên ảnh xám tùy ý (2 byte đầu = kích thước) và tứ giác tùy ý.
#![no_main]

use image::GrayImage;
use libfuzzer_sys::fuzz_target;
use tsuscan_imaging::{DetectOptions, Point, Quad, detect_luma, warp};

fuzz_target!(|data: &[u8]| {
    if data.len() < 3 + 32 {
        return;
    }
    let w = u32::from(data[0]) + 16;
    let h = u32::from(data[1]) + 16;
    let px = &data[2 + 32..];
    let img = GrayImage::from_fn(w, h, |x, y| image::Luma([px[((y * w + x) as usize) % px.len()]]));
    let _ = detect_luma(&img, DetectOptions::default());

    let c: Vec<f64> = data[2..34].chunks(4).map(|b| f64::from(f32::from_le_bytes([b[0], b[1], b[2], b[3]]))).collect();
    let pts = [Point::new(c[0], c[1]), Point::new(c[2], c[3]), Point::new(c[4], c[5]), Point::new(c[6], c[7])];
    if let Ok(q) = Quad::from_unordered(pts) {
        let rgb = image::DynamicImage::ImageLuma8(img).to_rgb8();
        let opts = warp::WarpOptions { max_side: 256, ..warp::WarpOptions::default() };
        let _ = warp::warp_rgb(&rgb, &q, opts);
    }
});

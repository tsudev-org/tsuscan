# Bộ mẫu dò mép `testdata/edges`

Ảnh tài liệu **tổng hợp** có nhãn tứ giác, sinh tất định theo seed bằng `crates/tsuscan-imaging/src/synth.rs`
(tính năng `synth`). Không có giấy tờ thật của người thật: trang chứa chữ giả (nét ngẫu nhiên mô phỏng chữ Latin có
dấu tiếng Việt), tiêu đề, khung bảng.

## Cấu hình

| Tham số | Giá trị |
| --- | --- |
| Seed | `0..N`, loại nền xoay vòng theo `seed % 5` |
| Nền | gỗ có vân, vải dệt, bàn trắng gần màu giấy (ca khó), bàn tối, đá/thảm nhiều hạt |
| Khung | 1280x720, 960x720, 720x960, 1024x768 |
| Trang | 70% A4, 15% Letter, 15% thẻ ID-1 có màu; phủ 20-60% khung; xoay ±25°; lệch phối cảnh ±8% |
| Hiệu ứng | ánh sáng không đều (80%), bóng tay chồng mép (40%), lóa (30%), mờ chuyển động 2-6 px (30%), nhiễu σ 2-7 (70%) |

Test đo `crates/tsuscan-imaging/tests/edges.rs` **sinh lại toàn bộ lúc chạy** (`TSUSCAN_EDGES_N`, mặc định 150) và
chặn khi IoU trung bình < 0,90 hoặc dưới 95% ảnh đạt IoU ≥ 0,80 (`docs/PLAN.md`). Không hạ ngưỡng khi đổi thuật toán.

## Ảnh trong thư mục này

`*.jpg` + `labels.json` chỉ là **ảnh minh họa** (12 seed đầu, thu nửa kích thước) để xem nhanh bằng mắt; test không
đọc chúng. Sinh lại:

```bash
cargo run -p tsuscan-cli -- synth --seed 0 -n 12 --scale 0.5 -o testdata/edges
```

Ảnh chụp thật qua adb bổ sung ở P6 (giấy tờ giả in ra, không dùng giấy tờ thật).

## Chẩn đoán

```bash
cargo run --release -p tsuscan-imaging --example diag -- 0 50 /tmp/khung   # IoU + thời gian CPU từng ảnh
```

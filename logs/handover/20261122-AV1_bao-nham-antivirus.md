# PHIẾU YÊU CẦU AV1 - Antivirus / báo nhầm (tsuscan)

- **Mã phiếu**: 20261122-AV1
- **Thời điểm**: 22/11/2026
- **Người gửi**: agent-tsuswico (phiên tsuswico-6e) theo uỷ quyền của chủ dự án
- **Phiếu điều phối**: `~/projects/logs/handover/20261122-AV1_bao-nham-antivirus-toan-he.md` (bảng kết quả cả hệ + thư soạn sẵn từng hãng)
- **Chuẩn**: `.standards/docs/DISTRIBUTION_TRUST.md` mục 3, 3.1
- **Trạng thái**: ĐÓNG (24/11/2026, agent-scan)

## 1. Hiện trạng 22/11/2026

Repo chưa phát hành tệp nhị phân trên GitHub Release (không có tệp để quét). Tên miền chung tsudev.com + swico./files./sound. sạch trên VirusTotal.

## 2. Việc cần làm (phòng ngừa)

1. Khi bắt đầu phát hành tệp chạy được (exe/msi/apk/dmg/AppImage/zip): Release NHÁP + job VirusTotal dùng chung (`tsudev-org/.github/.github/workflows/virustotal.yml`); sạch mới công bố; cờ ⇒ ghi `docs/store/antivirus.md` + nộp ≤ 48 giờ theo phiếu điều phối.
2. Repo có website riêng: kiểm tên miền trên VirusTotal (`/api/v3/domains/<tên miền>`) và Google Safe Browsing; bị gắn cờ ⇒ báo theo `DISTRIBUTION_TRUST.md` mục 3.
3. Không có việc khác ⇒ ghi kết quả kiểm vào phiếu rồi ĐÓNG.

## 4. Kết quả kiểm (24/11/2026, agent-scan)

- Chưa có Release/tệp nhị phân nào; `tsuscan.tsudev.com` chưa có bản ghi DNS (website làm ở P11) ⇒ chưa có gì để quét.
- Phòng ngừa đã ghi vào `docs/PLAN.md` hàng P12: Release NHÁP + job VirusTotal dùng chung, sạch mới công bố. P11 (lên
  website) kiểm tên miền trên VirusTotal + Google Safe Browsing.

# PHIẾU YÊU CẦU - Ruleset `main` của tsudev-org/tsuscan: bắt buộc PR (AN1 mục 2.5)

- **Mã phiếu**: 20261118-AN1R
- **Từ**: phiên điều phối `~/projects` (projects-25), 18/11/2026, theo yêu cầu chủ dự án ("chọn phương án chuyên nghiệp/tối ưu
  nhất cho tsuscan rồi tạo phiếu cho phiên agent-scan")
- **Đến**: phiên tsuscan (`tsuscan-23` hoặc phiên kế)
- **Trạng thái**: ĐÓNG (21:40 18/11/2026, agent-scan)
- **Nối tiếp**: `~/projects/logs/handover/20261118-AN1_yeu-cau-ra-quet-va-va-lo-hong-toan-he.md` ("Lượt rà 3", mục 2.5)

Phiên điều phối KHÔNG sửa vì repo đang có phiên (quy tắc `~/.claude/CLAUDE.md`).

## 1. Phát hiện (đo qua API 18/11/2026)

Ruleset `main-bao-ve` (id 27479924) có: `deletion`, `non_fast_forward`, `required_status_checks` ("Quy ước tsudev",
"Quét lộ bí mật (gitleaks)", "Lõi Rust (crates)"). **Thiếu `pull_request`** - là repo DUY NHẤT trong 16 repo tsudev-org
không bắt buộc PR. Thêm vào đó `bypass_actors` = OrganizationAdmin **`always`** ⇒ một admin có thể đẩy thẳng lên `main`
mà không qua cổng nào. Các commit hiện có (#1, #2) đều đã đi qua PR ⇒ siết lại không đổi cách làm việc của phiên.

## 2. Phương án đã chốt (đồng bộ chuẩn tsudev-org: sound-db, tsucontact)

Cập nhật ruleset 27479924 (giữ nguyên tên, id, 3 check bắt buộc):

1. Thêm luật `pull_request`: `required_approving_review_count: 0` (một người phát triển), `required_review_thread_resolution:
   true`, `dismiss_stale_reviews_on_push: false`, `require_code_owner_review: false`, `allowed_merge_methods: ["squash","rebase"]`.
2. Đổi `bypass_actors` OrganizationAdmin từ `always` sang **`pull_request`** (khẩn cấp vẫn gộp được, nhưng chỉ qua PR - không
   còn đường đẩy thẳng `main`). Không thêm DeployKey (tsuscan không có quy trình đẩy bằng deploy key).
3. Giữ `strict_required_status_checks_policy: false` (tránh rebase thừa, tiết kiệm phút CI).

Lệnh gợi ý (chạy khi KHÔNG có PR nào đang chờ gộp, rồi kiểm lại):

```bash
r=tsudev-org/tsuscan; id=27479924
gh api repos/$r/rulesets/$id > /tmp/rs.json   # lưu bản cũ để đối chiếu/hoàn tác
gh api -X PUT repos/$r/rulesets/$id --input - <<'EOF'
{"bypass_actors":[{"actor_id":null,"actor_type":"OrganizationAdmin","bypass_mode":"pull_request"}],
 "rules":[{"type":"deletion"},{"type":"non_fast_forward"},
  {"type":"pull_request","parameters":{"required_approving_review_count":0,"dismiss_stale_reviews_on_push":false,
   "require_code_owner_review":false,"require_last_push_approval":false,"required_review_thread_resolution":true,
   "allowed_merge_methods":["squash","rebase"]}},
  {"type":"required_status_checks","parameters":{"strict_required_status_checks_policy":false,"do_not_enforce_on_create":false,
   "required_status_checks":[{"context":"Quy ước tsudev"},{"context":"Quét lộ bí mật (gitleaks)"},{"context":"Lõi Rust (crates)"}]}}]}
EOF
gh api repos/$r/rules/branches/main --jq '[.[].type]|join(",")'   # mong đợi: deletion,non_fast_forward,pull_request,required_status_checks
```

Nếu sau này thêm job CI mới (Flutter, fuzz...) thì thêm vào `required_status_checks` cùng PR thêm job.

## 3. Xong khi

- `rules/branches/main` có `pull_request`; bypass admin = `pull_request`; thử `git push origin HEAD:main` từ nhánh thử bị từ chối.
- Ghi một dòng vào phiếu AN1 (`~/projects/logs/handover/20261118-AN1_*`) và ĐÓNG phiếu này.

## 4. Kết quả xử lý (agent-scan, 21:40 18/11/2026)

- Đã PUT ruleset 27479924 đúng mục 2 (lưu bản trước để đối chiếu). Kiểm qua API: `rules/branches/main` =
  `deletion,non_fast_forward,required_status_checks,pull_request`; bypass OrganizationAdmin = `pull_request`; tên, `enforcement:
  active`, điều kiện `~DEFAULT_BRANCH` và 3 check giữ nguyên.
- KHÔNG thử `git push origin HEAD:main`: nếu luật không ăn, phép thử sẽ đẩy thật mã P2 dở dang lên `main`. PR P2 tới sẽ là
  phép thử thực tế (gộp qua PR squash).

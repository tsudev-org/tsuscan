#!/usr/bin/env bash
# Tải tệp mô hình AI theo models/manifest.toml và kiểm sha256 + kích thước trước khi đặt vào chỗ.
# Dùng: scripts/fetch-models.sh [mã ...]   (không truyền mã ⇒ tải tất cả)
# Biến: TSUSCAN_MODELS = thư mục đích (mặc định models/ của repo).
# Tệp sai sha256 bị xóa ngay, không bao giờ nằm lại ở thư mục đích.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$root/models/manifest.toml"
dest="${TSUSCAN_MODELS:-$root/models}"
mkdir -p "$dest"

# Đọc các khối [[model]] thành dòng: id|file|url|sha256|size
entries="$(awk '
  function flush() { if (id != "") print id "|" file "|" url "|" sha "|" size; id = file = url = sha = size = "" }
  /^\[\[model\]\]/ { flush(); next }
  /^\[/ { flush(); next }
  match($0, /^(id|file|url|sha256|size)[ \t]*=[ \t]*/) {
    key = $1; val = substr($0, RLENGTH + 1); gsub(/^"|"[ \t]*$/, "", val)
    if (key == "id") id = val; else if (key == "file") file = val; else if (key == "url") url = val
    else if (key == "sha256") sha = val; else if (key == "size") size = val
  }
  END { flush() }
' "$manifest")"

want=("$@")
fail=0
while IFS='|' read -r id file url sha size; do
  [[ -z "$id" ]] && continue
  if ((${#want[@]})); then
    skip=1; for w in "${want[@]}"; do [[ "$w" == "$id" ]] && skip=0; done
    ((skip)) && continue
  fi
  [[ "$url" == https://* ]] || { echo "LỖI $id: URL không phải HTTPS" >&2; fail=1; continue; }
  [[ "$file" != */* && "$file" != .* ]] || { echo "LỖI $id: tên tệp chứa đường dẫn" >&2; fail=1; continue; }
  target="$dest/$file"
  if [[ -f "$target" ]] && echo "$sha  $target" | sha256sum -c --status - 2>/dev/null; then
    echo "đã có  $id ($file)"; continue
  fi
  tmp="$(mktemp "$dest/.tai-XXXXXX")"
  echo "tải    $id ← $url"
  if ! curl -sSfL --proto '=https' --tlsv1.2 --retry 3 --max-filesize "$((size + 1))" -o "$tmp" "$url"; then
    rm -f "$tmp"; echo "LỖI $id: tải thất bại" >&2; fail=1; continue
  fi
  got_size="$(stat -c %s "$tmp")"
  if [[ "$got_size" != "$size" ]] || ! echo "$sha  $tmp" | sha256sum -c --status -; then
    rm -f "$tmp"; echo "LỖI $id: sai kích thước/sha256 (nhận $got_size byte) - đã xóa" >&2; fail=1; continue
  fi
  mv -f "$tmp" "$target"
  echo "xong   $id ($file, $size byte, sha256 khớp)"
done <<< "$entries"
exit "$fail"

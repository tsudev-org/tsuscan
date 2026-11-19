#!/usr/bin/env bash
# Sinh lại mã cầu Rust ↔ Dart (flutter_rust_bridge) cho apps/app + crates/tsuscan-ffi.
# CI chạy script này rồi so `git diff` - mã sinh ra phải luôn khớp API trong crates/tsuscan-ffi/src/api.
set -euo pipefail
cd "$(dirname "$0")/../apps/app"
want="2.13.0"
have="$(flutter_rust_bridge_codegen --version | awk '{print $2}')"
if [ "$have" != "$want" ]; then
  echo "Cần flutter_rust_bridge_codegen $want (đang có $have): cargo install flutter_rust_bridge_codegen --version $want --locked" >&2
  exit 1
fi
flutter_rust_bridge_codegen generate

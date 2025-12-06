#!/bin/bash
# ------------------------------------------------------------
# 用途：產生 core 目錄下所有 Rust crate 的 tarpaulin 覆蓋率報告（LCOV），並自動改名
# 用法：將本檔放在專案根目錄執行
# ------------------------------------------------------------

set -e

cd "$(dirname "$(dirname "$0")")"

for dir in core/*; do
  if [ -d "$dir" ] && [ -f "$dir/Cargo.toml" ]; then
    crate_name=$(basename "$dir")
    echo "==> 產生覆蓋率報告 $dir"
    (cd "$dir" && cargo tarpaulin --out Lcov --output-dir coverage)
    if [ -f "coverage/tarpaulin-report.html" ]; then
      mv -f "coverage/tarpaulin-report.html" "coverage/${crate_name}.html"
    fi
    if [ -f "coverage/lcov.info" ]; then
      mv -f "coverage/lcov.info" "coverage/lcov-${crate_name}.info"
    fi
  fi
done

lcov --add-tracefile "coverage/lcov-*.info" -o coverage/lcov.info

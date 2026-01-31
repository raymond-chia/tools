#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""檢查 Rust 檔案中的 use 語句是否都在檔案頂部"""

import sys
from pathlib import Path

# 固定標準輸出編碼
if sys.stdout.encoding != "utf-8":
    sys.stdout.reconfigure(encoding="utf-8")


def check_file(filepath):
    """檢查單個檔案，返回 (has_error, error_messages)"""
    with open(filepath, "r", encoding="utf-8") as f:
        lines = f.readlines()

    errors = []
    seen_non_use_code = False  # 是否已經遇到過非 use/mod 的代碼
    in_multiline_use = False  # 是否在多行 use 語句中

    for i, line in enumerate(lines):
        stripped = line.strip()

        # 跳過空行和註釋
        if not stripped or stripped.startswith("//"):
            continue

        # 檢查是否在多行 use 語句中
        if in_multiline_use:
            if ";" in stripped:
                in_multiline_use = False
            continue

        # 允許在頂部的語句
        if (
            stripped.startswith("use ")
            or stripped.startswith("mod ")
            or stripped.startswith("pub mod ")
            or stripped.startswith("#[")
        ):
            # 檢查是否是多行 use 語句
            if stripped.startswith("use ") and "{" in stripped and "}" not in stripped:
                in_multiline_use = True
            # 如果已經遇過代碼，現在又看到 use，報錯
            elif seen_non_use_code and stripped.startswith("use "):
                errors.append(f"  Line {i+1}: use 語句不在檔案頂部")
        else:
            # 遇到其他代碼
            seen_non_use_code = True

    return len(errors) > 0, errors


def main():
    # 找出所有要檢查的 .rs 檔案（排除 target 目錄）
    root = Path(".")
    rust_files = []

    for rs_file in root.rglob("*.rs"):
        # 排除 target 目錄
        if "target" in rs_file.parts:
            continue
        rust_files.append(rs_file)

    all_errors = False
    for filepath in sorted(rust_files):
        has_error, errors = check_file(filepath)
        if has_error:
            all_errors = True
            print(f"FAIL: {filepath}")
            for error in errors:
                print(error)

    if all_errors:
        print("\nWARNING: 發現 use 語句不在檔案頂部，請修正")
        return 1
    else:
        print("PASS: 所有 use 語句位置正確")
        return 0


if __name__ == "__main__":
    sys.exit(main())

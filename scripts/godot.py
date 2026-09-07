#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""建置 godot_bind 後啟動 Godot 遊戲或執行 Godot 測試。"""

import shutil
import subprocess
import sys
from pathlib import Path


def parse_mode(arguments: list[str]) -> str | None:
    if not arguments:
        return "run"
    if arguments == ["test"]:
        return "test"
    print("用法：python scripts/godot.py [test]", file=sys.stderr)
    return None


def main() -> int:
    mode = parse_mode(sys.argv[1:])
    if mode is None:
        return 2

    project_root = Path(__file__).resolve().parent.parent
    godot_project_path = project_root / "godot"

    build_result = subprocess.run(
        ["cargo", "build", "-p", "godot_bind", "--release"],
        cwd=project_root,
        check=False,
    )
    if build_result.returncode != 0:
        return build_result.returncode

    godot_executable = shutil.which("godot")
    if godot_executable is None:
        print("找不到 Godot，請確認 `godot` 已加入 PATH。", file=sys.stderr)
        return 127

    if mode == "test":
        log_path = project_root / "ignore-tmp" / "gdunit.log"
        report_path = project_root / "ignore-tmp" / "gdunit-reports"
        log_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.mkdir(parents=True, exist_ok=True)
        godot_arguments = [
            godot_executable,
            "--path",
            str(godot_project_path),
            "--log-file",
            str(log_path),
            "-s",
            "res://addons/gdUnit4/bin/GdUnitCmdTool.gd",
            "-a",
            "res://tests",
            "-c",
            "-rd",
            str(report_path),
        ]
    else:
        godot_arguments = [godot_executable, "--path", str(godot_project_path)]

    godot_result = subprocess.run(
        godot_arguments,
        cwd=project_root,
        check=False,
    )
    return godot_result.returncode


if __name__ == "__main__":
    sys.exit(main())

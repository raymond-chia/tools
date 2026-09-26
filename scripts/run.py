import argparse
from pathlib import Path
import shutil
import subprocess
import sys


ROOT_DIRECTORY = Path(__file__).resolve().parents[1]
PROJECT_DIRECTORY = ROOT_DIRECTORY / "godot"
BRIDGE_LIBRARY = "godot_bridge.dll" if sys.platform == "win32" else "libgodot_bridge.so"
GODOT_EXECUTABLE = "godot.cmd" if sys.platform == "win32" else "godot"
GDUNIT_TEST_SCRIPT = "res://addons/gdUnit4/bin/GdUnitCmdTool.gd"
GODOT_TEST_LOG = ROOT_DIRECTORY / "ignore-tmp" / "godot-tests.log"
GDUNIT_REPORT_DIRECTORY = ROOT_DIRECTORY / "ignore-tmp" / "gdunit4-reports"
GDUNIT_REPORT_PATH = "res://../ignore-tmp/gdunit4-reports"
GDUNIT_ORPHAN_WARNING = 101


def build_bridge() -> int:
    build_result = subprocess.run(
        ["cargo", "build", "--release", "-p", "godot-bridge"],
        cwd=ROOT_DIRECTORY,
    )
    if build_result.returncode != 0:
        return build_result.returncode

    shutil.copy2(
        ROOT_DIRECTORY / "target" / "release" / BRIDGE_LIBRARY,
        PROJECT_DIRECTORY / "bin" / BRIDGE_LIBRARY,
    )
    return 0


def run_game() -> int:
    result = subprocess.run([GODOT_EXECUTABLE, "--path", str(PROJECT_DIRECTORY)])
    return result.returncode


def run_editor() -> int:
    result = subprocess.run(
        [
            GODOT_EXECUTABLE,
            "--path",
            str(PROJECT_DIRECTORY),
            "--scene",
            "res://features/editor/editor.tscn",
        ]
    )
    return result.returncode


def run_godot_tests() -> int:
    GODOT_TEST_LOG.parent.mkdir(exist_ok=True)
    GDUNIT_REPORT_DIRECTORY.mkdir(exist_ok=True)
    result = subprocess.run(
        [
            GODOT_EXECUTABLE,
            "--path",
            str(PROJECT_DIRECTORY),
            "--language",
            "zh_TW",
            "--log-file",
            str(GODOT_TEST_LOG),
            "--script",
            GDUNIT_TEST_SCRIPT,
            "--add",
            "res://tests",
            "--continue",
            "--report-directory",
            GDUNIT_REPORT_PATH,
        ]
    )
    # GdUnit4 以 101 表示斷言全過、但測試期間偵測到孤立節點；警告仍保留在輸出。
    return 0 if result.returncode == GDUNIT_ORPHAN_WARNING else result.returncode


def run_rust_tests() -> int:
    format_result = subprocess.run(["cargo", "fmt"], cwd=ROOT_DIRECTORY)
    if format_result.returncode != 0:
        return format_result.returncode

    test_result = subprocess.run(["cargo", "test"], cwd=ROOT_DIRECTORY)
    return test_result.returncode


def main() -> int:
    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(dest="command")
    commands.add_parser("editor", help="啟動遊戲資料編輯器")
    test_command = commands.add_parser("test", help="執行測試")
    test_command.add_argument("target", choices=("godot", "rust"))
    parser.set_defaults(target=None)
    arguments = parser.parse_args()

    if shutil.which(GODOT_EXECUTABLE) is None:
        print(
            f"找不到 Godot 執行檔 {GODOT_EXECUTABLE}；請安裝 Godot 並將其加入 PATH。",
            file=sys.stderr,
        )
        return 1

    match (arguments.command, arguments.target):
        case ("test", "rust"):
            run = run_rust_tests
        case ("test", "godot"):
            run = run_godot_tests
        case ("editor", None):
            run = run_editor
        case (None, None):
            run = run_game

    build_result = build_bridge()
    if build_result != 0:
        return build_result
    return run()


if __name__ == "__main__":
    sys.exit(main())

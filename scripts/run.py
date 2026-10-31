import argparse
import csv
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys


ROOT_DIRECTORY = Path(__file__).resolve().parents[1]
PROJECT_DIRECTORY = ROOT_DIRECTORY / "godot"
BRIDGE_LIBRARY = {
    "win32": "godot_bridge.dll",
    "linux": "libgodot_bridge.so",
    "darwin": "libgodot_bridge.dylib",
}[sys.platform]
GODOT_EXECUTABLE = "godot.cmd" if sys.platform == "win32" else "godot"
GDUNIT_TEST_SCRIPT = "res://addons/gdUnit4/bin/GdUnitCmdTool.gd"
GODOT_TEST_LOG = ROOT_DIRECTORY / "ignore-tmp" / "godot-tests.log"
GDUNIT_REPORT_DIRECTORY = ROOT_DIRECTORY / "ignore-tmp" / "gdunit4-reports"
GDUNIT_REPORT_PATH = "res://../ignore-tmp/gdunit4-reports"
GDUNIT_ORPHAN_WARNING = 101
# Linux 以 XDG_DATA_HOME 隔離各 worker 的 user://，因此可同時執行多個 Godot。
GODOT_TEST_WORKERS = 10
GODOT_TEST_WORKER_DIRECTORY = ROOT_DIRECTORY / "ignore-tmp" / "godot-tests"
# 對應 GdUnit4 console reporter 的總結行與失敗（含 error）測試案例行。
GDUNIT_OVERALL_SUMMARY = re.compile(r"Overall Summary:.*?(?P<errors>\d+) errors \| (?P<failures>\d+) failures")
GDUNIT_FAILED_TEST = re.compile(r"^\s*(res://\S+ > \S+) FAILED", re.MULTILINE)


def sort_translation_keys() -> int:
    for path in sorted((PROJECT_DIRECTORY / "translations").glob("*.csv")):
        with path.open("r", encoding="utf-8", newline="") as source:
            lines = source.readlines()
        reader = csv.reader(lines)
        next(reader)
        header = lines[:reader.line_num]
        rows = []
        start = reader.line_num
        for row in reader:
            rows.append((row[0], lines[start:reader.line_num]))
            start = reader.line_num

        rows.sort(key=lambda row: row[0])
        with path.open("w", encoding="utf-8", newline="") as destination:
            destination.writelines(header)
            for _, record_lines in rows:
                destination.writelines(record_lines)
        print(f"已排序 {path.relative_to(ROOT_DIRECTORY)}（{len(rows)} 筆）")
    return 0


def build_bridge(*, editor: bool) -> int:
    command = ["cargo", "build", "--release", "-p", "godot-bridge"]
    if not editor:
        command.append("--no-default-features")

    build_result = subprocess.run(
        command,
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


def godot_test_command(log_file: Path, report_path: str, test_paths: list[str]) -> list[str]:
    command = [
        GODOT_EXECUTABLE,
        "--path",
        str(PROJECT_DIRECTORY),
        "--language",
        "zh_TW",
        "--log-file",
        str(log_file),
        "--script",
        GDUNIT_TEST_SCRIPT,
    ]
    for test_path in test_paths:
        command += ["--add", test_path]
    return command + ["--continue", "--report-directory", report_path]


# GdUnit4 以 101 表示斷言全過、但測試期間偵測到孤立節點；警告仍保留在輸出。
def godot_test_result(returncode: int) -> int:
    return 0 if returncode == GDUNIT_ORPHAN_WARNING else returncode


def run_godot_tests() -> int:
    if sys.platform == "linux":
        return run_parallel_godot_tests()
    GODOT_TEST_LOG.parent.mkdir(exist_ok=True)
    GDUNIT_REPORT_DIRECTORY.mkdir(exist_ok=True)
    result = subprocess.run(godot_test_command(GODOT_TEST_LOG, GDUNIT_REPORT_PATH, ["res://tests"]))
    return godot_test_result(result.returncode)


# 依測試檔輪流分配給各 worker；每個 worker 使用獨立的 user://、日誌與報告目錄。
def run_parallel_godot_tests() -> int:
    test_paths = [
        "res://" + path.relative_to(PROJECT_DIRECTORY).as_posix()
        for path in sorted((PROJECT_DIRECTORY / "tests").rglob("test_*.gd"))
    ]
    shutil.rmtree(GODOT_TEST_WORKER_DIRECTORY, ignore_errors=True)
    workers = []
    for index in range(min(GODOT_TEST_WORKERS, len(test_paths))):
        directory = GODOT_TEST_WORKER_DIRECTORY / f"worker-{index}"
        (directory / "data").mkdir(parents=True)
        (directory / "reports").mkdir()
        log_file = directory / "godot-tests.log"
        report_path = "res://../" + (directory / "reports").relative_to(ROOT_DIRECTORY).as_posix()
        process = subprocess.Popen(
            godot_test_command(log_file, report_path, test_paths[index::GODOT_TEST_WORKERS]),
            env={**os.environ, "XDG_DATA_HOME": str(directory / "data")},
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        workers.append((process, log_file))

    result = 0
    errors = 0
    failures = 0
    failed_tests = []
    abnormal_workers = []
    for process, log_file in workers:
        returncode = godot_test_result(process.wait())
        log = log_file.read_text(encoding="utf-8", errors="replace")
        print(log)
        print(f"{log_file.relative_to(ROOT_DIRECTORY)}：exit {returncode}")
        if result == 0:
            result = returncode
        summary = GDUNIT_OVERALL_SUMMARY.search(log)
        if summary is not None:
            errors += int(summary["errors"])
            failures += int(summary["failures"])
        failed_tests += GDUNIT_FAILED_TEST.findall(log)
        if returncode != 0:
            abnormal_workers.append(f"{log_file.parent.name}（exit {returncode}）")

    print("===== Godot 測試總結 =====")
    if result == 0:
        print("✅ 全部通過")
        return result
    print(f"❌ 有測試失敗：{failures} failures、{errors} errors")
    for failed_test in failed_tests:
        print(f"  {failed_test}")
    print(f"異常 worker：{'、'.join(abnormal_workers)}")
    return result


def run_rust_tests() -> int:
    format_result = subprocess.run(["cargo", "fmt"], cwd=ROOT_DIRECTORY)
    if format_result.returncode != 0:
        return format_result.returncode

    test_result = subprocess.run(["cargo", "test"], cwd=ROOT_DIRECTORY)
    return test_result.returncode


def main() -> int:
    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(dest="command")
    commands.add_parser("sort-tr", help="依 key 排序翻譯 CSV")
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

    # 所有 command 分派只能在此 match 處理，禁止在其他位置另行分派。
    match (arguments.command, arguments.target):
        case ("test", "rust"):
            run = run_rust_tests
        case ("test", "godot"):
            run = run_godot_tests
        case ("editor", None):
            run = run_editor
        case ("sort-tr", None):
            return sort_translation_keys()
        case (None, None):
            run = run_game

    build_result = build_bridge(editor=arguments.command is not None)
    if build_result != 0:
        return build_result
    return run()


if __name__ == "__main__":
    sys.exit(main())

from pathlib import Path
import shutil
import subprocess
import sys


ROOT_DIRECTORY = Path(__file__).resolve().parents[1]
PROJECT_DIRECTORY = ROOT_DIRECTORY / "godot"
BRIDGE_LIBRARY = "godot_bridge.dll"
GODOT_TEST_SCRIPT = "res://tests/run.gd"
GODOT_TEST_LOG = ROOT_DIRECTORY / "ignore-tmp" / "godot-tests.log"


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
    result = subprocess.run(["godot.cmd", "--path", str(PROJECT_DIRECTORY)])
    return result.returncode


def run_godot_tests() -> int:
    GODOT_TEST_LOG.parent.mkdir(exist_ok=True)
    result = subprocess.run(
        [
            "godot.cmd",
            "--headless",
            "--path",
            str(PROJECT_DIRECTORY),
            "--log-file",
            str(GODOT_TEST_LOG),
            "--script",
            GODOT_TEST_SCRIPT,
        ]
    )
    return result.returncode


def main() -> int:
    arguments = sys.argv[1:]
    if arguments not in ([], ["test", "godot"]):
        print("用法：run.py [test godot]", file=sys.stderr)
        return 2

    build_result = build_bridge()
    if build_result != 0:
        return build_result

    if arguments == ["test", "godot"]:
        return run_godot_tests()
    return run_game()


if __name__ == "__main__":
    sys.exit(main())

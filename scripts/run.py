from pathlib import Path
import shutil
import subprocess
import sys


ROOT_DIRECTORY = Path(__file__).resolve().parents[1]
PROJECT_DIRECTORY = ROOT_DIRECTORY / "godot"
BRIDGE_LIBRARY = "godot_bridge.dll"


def main() -> int:
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
    result = subprocess.run(["godot.cmd", "--path", str(PROJECT_DIRECTORY)])
    return result.returncode


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""
用途: 添加 Rust 依賴到 workspace，並自動獲取最新版本
用法: uv run add-dep.py <套件名稱>
例如: uv run add-dep.py serde
"""

import sys
import json
import re
from pathlib import Path
from urllib.request import urlopen
from urllib.error import URLError


def get_latest_version(package_name: str) -> str:
    """
    從 crates.io API 查詢套件的最新版本

    Args:
        package_name: 套件名稱（如 serde）

    Returns:
        版本號字符串（如 1.0.0）

    Raises:
        Exception: 如果找不到套件或網路錯誤
    """
    url = f"https://crates.io/api/v1/crates/{package_name}"

    print(f"正在查詢 {package_name} 的最新版本...")

    try:
        with urlopen(url, timeout=10) as response:
            data = json.loads(response.read().decode('utf-8'))
            version = data.get('crate', {}).get('max_version')

            if not version:
                raise Exception(f"找不到套件 {package_name}")

            return version
    except URLError as e:
        raise Exception(f"無法連接到 crates.io: {e}")
    except json.JSONDecodeError:
        raise Exception("無法解析 crates.io 響應")


def find_workspace_root() -> Path:
    """
    找到 workspace 根目錄（包含 Cargo.toml 的目錄）

    Returns:
        workspace 根目錄的 Path 對象

    Raises:
        Exception: 如果找不到 Cargo.toml
    """
    # scripts/add-dep.py 的上一層就是 workspace root
    script_dir = Path(__file__).parent
    workspace_root = script_dir.parent
    cargo_toml = workspace_root / "Cargo.toml"

    if not cargo_toml.exists():
        raise Exception(f"錯誤: 找不到 workspace Cargo.toml ({cargo_toml})")

    return workspace_root


def update_cargo_toml(cargo_toml_path: Path, package_name: str, version: str) -> None:
    """
    更新 Cargo.toml，添加或更新 [workspace.dependencies] 中的套件

    Args:
        cargo_toml_path: Cargo.toml 的路徑
        package_name: 套件名稱
        version: 版本號
    """
    # 讀取 Cargo.toml 內容
    content = cargo_toml_path.read_text(encoding='utf-8')

    # 檢查是否已有 [workspace.dependencies] 部分
    if '[workspace.dependencies]' not in content:
        print("新增 [workspace.dependencies] 部分...")
        content = content.rstrip() + '\n\n[workspace.dependencies]\n'

    # 檢查套件是否已存在
    pattern = f"^{re.escape(package_name)}\\s*="
    if re.search(pattern, content, re.MULTILINE):
        print(f"\033[33m套件已存在，正在更新版本...\033[0m")
        # 替換現有的版本
        content = re.sub(
            f"^{re.escape(package_name)}\\s*=.*$",
            f'{package_name} = "{version}"',
            content,
            flags=re.MULTILINE
        )
    else:
        print("添加新套件到 [workspace.dependencies]...")
        # 在 [workspace.dependencies] 後面添加新行
        content = re.sub(
            r'(\[workspace\.dependencies\])',
            f'\\1\n{package_name} = "{version}"',
            content,
            count=1
        )

    # 寫回 Cargo.toml
    cargo_toml_path.write_text(content, encoding='utf-8')


def verify_update(cargo_toml_path: Path, package_name: str, version: str) -> None:
    """
    驗證更新是否成功

    Args:
        cargo_toml_path: Cargo.toml 的路徑
        package_name: 套件名稱
        version: 版本號

    Raises:
        Exception: 如果驗證失敗
    """
    content = cargo_toml_path.read_text(encoding='utf-8')
    pattern = f'^{re.escape(package_name)} = "{re.escape(version)}"'

    if re.search(pattern, content, re.MULTILINE):
        print(f"\033[32m✓ 驗證成功: 已寫入 {package_name} = \"{version}\"\033[0m")
    else:
        raise Exception("驗證失敗: 寫入 Cargo.toml 未成功")


def main():
    """主程序"""
    # 檢查參數
    if len(sys.argv) != 2:
        print("用法: uv run add-dep.py <套件名稱>")
        print("例如: uv run add-dep.py serde")
        sys.exit(1)

    package_name = sys.argv[1]

    try:
        # 步驟 1: 查詢最新版本
        version = get_latest_version(package_name)
        print(f"\033[32m找到版本: {version}\033[0m")

        # 步驟 2: 找到 workspace 根目錄
        workspace_root = find_workspace_root()
        cargo_toml_path = workspace_root / "Cargo.toml"

        # 步驟 3: 更新 Cargo.toml
        update_cargo_toml(cargo_toml_path, package_name, version)
        print(f"\033[32m已更新 workspace Cargo.toml\033[0m")

        # 步驟 4: 驗證更新
        verify_update(cargo_toml_path, package_name, version)

        # 完成
        print()
        print("\033[36m現在你可以執行:\033[0m")
        print(f"\033[36m  cargo add -p board {package_name}\033[0m")

    except Exception as e:
        print(f"\033[31m錯誤: {e}\033[0m", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()

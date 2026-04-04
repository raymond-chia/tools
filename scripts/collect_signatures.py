"""掃描 Rust 原始碼，蒐集所有 fn 簽名（到 { 或 ; 為止）。

用法：
    python scripts/collect_signatures.py [目錄...]

預設掃描 core/board 與 editor。
"""

import re
from pathlib import Path

# 匹配 fn 定義開頭（只匹配 pub fn / pub(crate) fn，不匹配私有函數）
FN_PATTERN = re.compile(
    r"^(\s*)pub(\([^)]*\))?\s+(async\s+)?(const\s+)?(unsafe\s+)?fn\s+\w+"
)

# 匹配 macro_rules! 的匯出行（如 pub(crate) use get_component;）
MACRO_USE_PATTERN = re.compile(
    r"^(\s*)pub(\([^)]*\))?\s+use\s+(\w+)\s*;"
)


def extract_signatures(filepath: Path) -> list[str]:
    """從單一檔案提取所有 fn 簽名。"""
    text = filepath.read_text(encoding="utf-8")
    lines = text.splitlines()

    # 先收集檔案中所有 macro_rules! 定義的名稱
    macro_names = set()
    for line in lines:
        m = re.match(r"^\s*macro_rules!\s+(\w+)", line)
        if m:
            macro_names.add(m.group(1))

    signatures = []
    i = 0
    while i < len(lines):
        line = lines[i]
        # 匹配 macro_rules! 的匯出行
        macro_match = MACRO_USE_PATTERN.match(line)
        if macro_match and macro_match.group(3) in macro_names:
            raw = line.strip()
            signatures.append(f"{raw} (macro)")
            i += 1
            continue
        if FN_PATTERN.match(line):
            # 蒐集簽名直到遇到 { 或 ;
            sig_lines = []
            j = i
            while j < len(lines):
                sig_lines.append(lines[j])
                if "{" in lines[j] or lines[j].rstrip().endswith(";"):
                    break
                j += 1

            # 組合並清理
            raw = " ".join(l.strip() for l in sig_lines)
            # 移除 { 及之後的內容
            raw = re.sub(r"\s*\{.*$", "", raw)
            # 移除 where 子句（簡化顯示）
            raw = re.sub(r"\s+where\s+.*$", "", raw)
            # 壓縮多餘空白
            raw = re.sub(r"\s+", " ", raw).strip()

            signatures.append(raw)
            i = j + 1
        else:
            i += 1

    return signatures


def main():
    root = Path(__file__).resolve().parent.parent
    dirs = [root / "core" / "board", root / "editor"]

    rs_files = []
    for d in dirs:
        rs_files.extend(sorted(d.rglob("*.rs")))

    for filepath in rs_files:
        sigs = extract_signatures(filepath)
        if not sigs:
            continue
        rel = filepath.relative_to(root)
        print(f"\n### {rel}")
        for sig in sigs:
            print(f"- `{sig}`")


if __name__ == "__main__":
    main()

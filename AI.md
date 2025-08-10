# 規則

- 不要在本 markdown 提到 sim mode

## core

- function 盡量回傳 Result
- Result Err 要定義在 enum

# Rust 專案結構說明

本文件記錄目前專案目錄結構，供未來 AI 修改與參考。

## 根目錄

- `.gitignore`：Git 忽略設定
- `.rooignore`：AI 禁止存取設定（已自動忽略）
- `README.md`：專案說明文件
- `.vscode/`：VSCode 編輯器設定
- `core/`：共用函式庫目錄
- `editor/`：工具專案目錄

---

## core 目錄

- `towebai.bat`：批次檔，將 core 目錄下所有 .rs 檔案內容合併輸出為 `towebai.rs`，便於整合分析或外部工具使用。

### chess-lib

- `Cargo.toml`、`Cargo.lock`：Rust 專案設定
- `src/`
  - `battle.rs`：戰鬥邏輯
  - `board.rs`：棋盤邏輯
  - `lib.rs`：主函式庫入口
  - `unit.rs`：單位邏輯
  - `action/`
    - `algo.rs`：行動演算法（如路徑搜尋、Dijkstra 等知名演算法）
    - `ai.rs`：AI 決策模組（負責非玩家方單位行為決策，包含 simple_ai_control 等介面）
    - `mod.rs`：行動模組
    - `movement.rs`：移動邏輯
    - `skill.rs`：技能邏輯
- `tests/`
  - 多個 JSON 測試資料（如 board.json、skill_max_hp.json 等）

#### action/ai.rs 說明

- **用途**：專責非玩家方單位的 AI 行為決策。
- **主要介面**：
  - `pub fn simple_ai_control(...)`：簡單版 AI，當玩家單位進入一定距離時攻擊最近目標。
  - 未來可擴充困難版（如多步規劃、技能選擇等）。
- **設計原則**：
  - AI 決策與演算法分離，algo.rs 僅存放通用演算法，ai.rs 專注於行為決策。
  - 介面設計便於主流程呼叫與未來擴充。

### dialogs-lib

- `Cargo.toml`、`Cargo.lock`
- `src/lib.rs`：對話函式庫主入口

### skills-lib

- `Cargo.toml`、`Cargo.lock`
- `src/lib.rs`：技能函式庫主入口

### test-data

- `demo-dialogs.toml`：對話測試資料

---

## editor 目錄

- `Cargo.toml`、`Cargo.lock`
- `src/`
  - `boards.rs`：棋盤相關工具
  - `common.rs`：共用工具
  - `dialogs.rs`：對話相關工具
  - `main.rs`：主程式入口
  - `skills.rs`：技能相關工具
  - `units.rs`：單位相關工具

---

## 備註

- `core/` 目錄下包含多個 Rust library，分別處理棋盤、對話、技能等功能。
- `editor/` 目錄為工具專案，整合並調用各 library。
- 測試資料與設定檔皆有明確分層，便於維護與擴充。
- `.rooignore` 內標記的目錄與檔案已自動排除於 AI 存取範圍。

---

> 本結構說明供未來 AI 進行自動化修改、分析與擴充時參考。

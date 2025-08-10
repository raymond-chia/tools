# 規則

- 不要在本 markdown 提到 sim mode

## core

### 錯誤處理規範（專案標準）

- core 目錄下各 crate 必須以 `enum` 定義錯誤型別，並 derive [`thiserror::Error`](https://docs.rs/thiserror)。
- 重要錯誤應攜帶 function name 及 context（如參數、狀態等），以利追蹤與除錯。
- Option 場景應視情況改為 Result，避免以 None 隱藏錯誤，並確保錯誤傳遞時保留來源（source）。
- 錯誤訊息建議使用 `#[error("...")]` 屬性，訊息可帶參數與來源資訊，並鼓勵使用 `{source}` 以自動串接上下游錯誤。
- 本規範為專案標準，所有錯誤處理皆須遵循。

#### 範例

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ChessError {
    #[error("Invalid move in {func}: {reason}")]
    InvalidMove {
        func: &'static str,
        reason: String,
    },
    #[error("IO error in {func}: {source}")]
    Io {
        func: &'static str,
        #[source]
        source: std::io::Error,
    },
    #[error("Unknown error")]
    Unknown,
}

// 使用範例
fn do_something() -> Result<(), ChessError> {
    let func = "do_something";
    // ...邏輯
    Err(ChessError::InvalidMove {
        func,
        reason: "out of bound".to_string(),
    })
}
```

> 註：
>
> - 若需包裝外部錯誤，請於 enum variant 加上 `#[source]` 並攜帶來源錯誤。
> - 建議錯誤訊息攜帶 function name、參數、狀態等 context。
> - Option 場景若有錯誤語意，請改用 Result 並明確回傳錯誤型別。
> - 錯誤傳遞時應保留來源（source），以利追蹤。

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

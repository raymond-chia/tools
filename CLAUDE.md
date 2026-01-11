# CLAUDE.md

戰術回合制 RPG 遊戲（Rust）。

## 專案結構

- **core/chess-lib**: 核心遊戲邏輯（戰鬥系統、移動、技能施放、AI）
- **core/skills-lib**: 技能定義、效果類型
- **core/object-lib**: 物件類型定義（ObjectType、Orientation）
- **core/dialogs-lib**: 對話系統
- **editor**: egui GUI 編輯器（地圖、單位、技能編輯）

## 基本指令

```bash
# 測試
cargo.exe test
```

## 程式碼規範

**語言**: 繁體中文（程式碼、註解、文件）

- **禁止向後相容**
  - 直接刪除未使用的程式碼，不要保留
  - 如果程式碼未使用，完全刪除它
- 盡量列出所有 match arm, 確保未來新增會更新到
- 使用 match 取代 let else

**核心規則**:

- 型別安全、完整錯誤處理（錯誤訊息包含豐富上下文）
- 禁止 magic numbers/strings
- 不確定時詢問使用者，不要自行決定
- `use` 語句放在檔案頂部

**測試**:

- 不寫測試給 `ai.rs`、`editor` crate、inner functions
- 只有在副作用難以測試時才修改程式碼邏輯

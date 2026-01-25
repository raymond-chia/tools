# 通用規則

- **語言**: 繁體中文（註解、文件），禁止簡體中文
- 撰寫計畫的時候不要添加程式碼，請保持精簡
- 使用 match 而不要 let else
- match arm 使用編譯器的 exhaustiveness checking 保護，避免未來忘記添加 match arm
- 確保型別安全，有完整錯誤處理（錯誤訊息包含豐富上下文）
- 禁止 magic numbers/strings
- `use` 語句放在檔案頂部，不要放在 function 裡面
- **Fail fast**：在函數開頭進行所有驗證和檢查，不在執行中間檢查

# 本專案

- 本專案是戰術回合制 RPG 遊戲（Rust）。
- 禁止向後相容
- 使用 test driven (TDD)
  - 測試請集中在 core\board\tests 的子資料夾
- 禁止查看 bak 開頭的資料夾或檔案

## 基本指令

```bash
# 測試
cargo test
```

## **Struct/Enum 設計**

- 所有 struct/enum 都必須 derive Debug
- **不要預先設計**：只補現在用到的 derive、方法、錯誤類型
  - 只補齊必要的 derive，不嘗試預先 derive
  - 有需要時再添加，不要預測未來需求

## **測試**:

- 不替以下撰寫測試: `editor` crate、inner functions
- 只有在副作用難以測試時才修改程式碼邏輯
- **視覺化測試資料**：所有測試都應該盡量用視覺化方式呈現測試資料
  - 使用 ASCII art 或圖示化方式展示棋盤狀態
  - 讓測試資料一目瞭然，便於理解測試意圖

## 分層設計

### 戰棋遊戲架構設計文件

#### 核心設計原則

1. 數據驅動設計
   所有遊戲內容（單位、技能、狀態效果）用外部資料定義
   使用 TOML 格式存儲
   運行時載入和解析
   邏輯代碼只處理「如何執行」，不寫死「執行什麼」
2. ECS 架構
   使用 bevy_ecs 管理所有遊戲狀態
   Component 只存資料，不含邏輯
   System 處理邏輯，通過 Query 操作 Component

### Error 層（types/error.rs）

- **責任**：集中管理所有錯誤類型
- **設計**：所有業務邏輯的錯誤都在 error.rs 定義，確保錯誤訊息包含豐富上下文

## 專案索引

- 如果發現 core 底下的檔案結構跟本檔案紀錄不合的時候，請更新本檔案 `專案結構`  
  不需要紀錄太細，以免需要常常變動
- 如果發現 core 底下的 function 與本檔案紀錄不合的時候，請更新本檔案 `function 集`  
  只列出公開函數（pub fn），不包含 struct/enum 定義  
  格式：`函數簽名` - 簡短說明

### 專案結構

```
core/board/
├── src/
└── tests/      - 測試
```

### function 集

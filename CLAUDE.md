# 通用規則

- **語言**: 繁體中文（註解、文件），禁止簡體中文
- 撰寫計畫的時候不要添加程式碼，請保持精簡
- 使用 match 而不要 let else
- match arm 使用編譯器的 exhaustiveness checking 保護，避免未來忘記添加 match arm
- 確保型別安全，有完整錯誤處理（錯誤訊息包含豐富上下文）
- 禁止 magic numbers/strings
- `use` 語句放在檔案頂部，不要放在 function 裡面
- **Fail fast**：在函數開頭進行所有驗證和檢查，不在執行中間檢查
- 每當 claude code 提出設計建議時，要求 claude code 先明確引用 CLAUDE.md 的具體條文來論證為什麼這個設計符合規則

# 本專案

- 本專案是戰術回合制 RPG 遊戲（Rust）。
- 禁止向後相容
- 使用 functional programming
  - 禁止使用 oop
- 使用 test driven (TDD)
  - 測試請集中在 core\board\tests 的子資料夾
- 禁止查看 bak 開頭的資料夾或檔案
- 寫完後檢查是否有違反 D:\Mega\prog\rust\tools\README-設計機制.md。如果只是尚未實作完畢，只要提示尚未實作就好。如果違反請警告使用者。

## 基本指令

```bash
# 測試
cargo test
```

## **核心設計原則**

1. 數據驅動設計
   所有遊戲內容（單位、技能、狀態效果）用外部資料定義
   使用 TOML 格式存儲
   運行時載入和解析
   邏輯代碼只處理「如何執行」，不寫死「執行什麼」
2. ECS 架構
   使用 bevy_ecs 管理所有遊戲狀態

## **Struct/Enum 設計**

- 所有 struct/enum 都必須 derive Debug
- **不要預先設計**：只補現在用到的 derive、方法、錯誤類型
  - 只補齊必要的 derive，不嘗試預先 derive
  - 有需要時再添加，不要預測未來需求

## **檔案放置原則**

### component.rs

- 存放 ECS Component：只存資料，不含業務邏輯
- 必須 derive `Component` 和 `Debug`
- 禁止出現 impl

### alias.rs

- 存放類型別名（type alias）

### primitive.rs

- 保留給基本資料類型定義（未來使用）

### logic/ 中的參數類型

- 函數參數類型放在同一個檔案中（與函數一起）
- 例：`Direction` 和 `Mover` 在 `logic/movement.rs`
- 這些類型可以組合來自 `component.rs` 的 Component

### error.rs

- 集中定義所有錯誤類型
- 確保錯誤訊息包含豐富上下文

### logic/

- 存放核心業務邏輯函數（純邏輯運算，不依賴 ECS Query）
- 可以依賴 component.rs 和 typ.rs 的類型
- 不含 Struct/Enum 定義

### system/

- 存放真正的 ECS System（通過 Query 操作 Component）
- 不存放 Struct/Enum 定義
- 函數簽名包含 Query 或系統相關參數

## **測試**:

- 不替以下撰寫測試: `editor` crate、inner functions
- 只有在副作用難以測試時才修改程式碼邏輯
- **視覺化測試資料**：所有測試都應該盡量用視覺化方式呈現測試資料
  - 使用 ASCII art 或圖示化方式展示棋盤狀態
    - 請用 load_from_ascii 解析
  - 讓測試資料一目瞭然，便於理解測試意圖
  - **使用 test_data 陣列**：多個測試案例應使用 `let test_data = [...]` 的形式，用迴圈遍歷，不要寫單一測試案例

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
│   ├── alias.rs          - 類型別名（Coord, ID）
│   ├── component.rs      - ECS Component 定義
│   ├── error.rs          - 錯誤型別定義
│   ├── loader.rs         - 資料載入（TOML 解析）
│   ├── primitive.rs      - 基本資料類型定義（未來使用）
│   ├── logic/            - 核心業務邏輯（非 ECS System）
│   │   ├── board.rs      - 棋盤驗證
│   │   └── movement.rs   - 移動邏輯
│   └── system/           - ECS System（目前未實作）
└── tests/                - 測試
    ├── board/
    │   ├── test_board.rs
    │   └── test_movement.rs
    ├── test.rs
    └── test_error.rs

editor/
└── src/
```

- **core/board**：模擬邏輯、序列化
- **editor**：編輯操作、GUI

### function 集

#### logic/board.rs

- `pub fn is_valid_position(board: Board, pos: Position) -> bool` - 驗證位置是否在棋盤邊界內

#### logic/movement.rs

- `pub fn step_in_direction(board: Board, pos: Position, direction: Direction) -> Option<Position>` - 計算從位置往方向移動一格，檢查棋盤邊界
- `pub fn manhattan_path<F>(board: Board, mover: Mover, to: Position, get_occupant: F) -> Result<Vec<Position>>` - 計算從起點到終點的移動路徑（水平+垂直），檢查碰撞

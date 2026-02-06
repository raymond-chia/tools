---
paths:
  - "core/**/*"
---

# Core/Board 規則

本檔案包含 `core/board` crate 的專屬規則。

## **核心設計原則**

1. **數據驅動設計**
   - 所有遊戲內容（單位、技能、狀態效果）用外部資料定義
   - 使用 TOML 格式存儲
   - 邏輯代碼只處理「如何執行」，不寫死「執行什麼」

2. **ECS 架構**
   - 使用 bevy_ecs 管理所有遊戲狀態

3. **測試驅動開發 (TDD)**
   - 使用 test driven development

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
- 可以依賴 component.rs 和 primitive.rs 的類型
- 不含 Struct/Enum 定義

### system/

- 存放真正的 ECS System（通過 Query 操作 Component）
- 不存放 Struct/Enum 定義
- 函數簽名包含 Query 或系統相關參數

## **測試**

- 不替以下撰寫測試：inner functions、serialize/deserialize
- 只有在副作用難以測試時才修改程式碼邏輯
- **視覺化測試資料**：所有測試都應該盡量用視覺化方式呈現測試資料
  - 使用 ASCII art 或圖示化方式展示棋盤狀態
    - 請用 load_from_ascii 解析
  - 讓測試資料一目瞭然，便於理解測試意圖
  - **使用 test_data 陣列**：多個測試案例應使用 `let test_data = [...]` 的形式，用迴圈遍歷，不要寫單一測試案例
- 測試請集中在 `core\board\tests` 的子資料夾

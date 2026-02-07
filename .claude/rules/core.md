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
   - 使用 bevy_ecs 管理所有遊戲狀態，達到 single responsibility

3. **測試驅動開發 (TDD) 與測試規則**
   - 開發方法：先寫失敗的測試，再實現邏輯使其通過
   - 使用視覺化方式呈現測試資料（ASCII art 或圖示化方式展示棋盤狀態）
     - 用 load_from_ascii 解析 ASCII art
   - 讓測試資料一目瞭然，便於理解測試意圖
   - 以下情況不需撰寫測試：inner functions、serialize/deserialize
   - **使用 test_data 陣列**：多個測試案例應使用 `let test_data = [...]` 的形式，用迴圈遍歷，不要寫單一測試案例
   - 測試請集中在 `core\board\tests` 的子資料夾

4. **自訂錯誤型別**
   - `core/` crate 為了容易解析錯誤，使用自訂 enum，不用 String、anyhow 等通用錯誤型別

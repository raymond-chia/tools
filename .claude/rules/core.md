---
paths:
  - "core/**/*"
---

# Core/Board 規則

本檔案包含 `core/board` crate 的專屬規則。

## 核心設計原則

數據驅動設計

- 所有遊戲內容（單位、技能、狀態效果）用外部資料定義
- 使用 TOML 格式存儲
- 邏輯代碼只處理「如何執行」，不寫死「執行什麼」

ECS 架構

- 使用 bevy_ecs 管理所有遊戲狀態，達到 single responsibility

自訂錯誤型別

- `core/` crate 為了容易解析錯誤，使用自訂 enum，不用 String、anyhow 等通用錯誤型別

## 開發方法（TDD）

流程：先寫失敗的測試 → 實現邏輯使其通過

測試規則

- 使用 ASCII art 視覺化棋盤狀態（用 load_from_ascii 解析）
- 讓測試資料一目瞭然，便於理解測試意圖
- 多個測試案例使用 `let test_data = [...]` + 迴圈遍歷，不要寫單一案例
- 測試集中在 core/board/tests/ 的子資料夾

不需要測試

- inner functions
- serialize/deserialize

# Core/Board 規則

本檔案包含 `core/board` crate 的專屬規則。
此檔案不設定 paths 限制，以免寫測試的時候不會載入。

## 核心設計原則

數據驅動設計

- 所有遊戲內容（單位、技能、狀態效果）用外部資料定義
- 使用 TOML 格式存儲
- 邏輯代碼只處理「如何執行」，不寫死「執行什麼」

ECS 架構

- 使用 bevy_ecs 管理所有遊戲狀態，達到 single responsibility

World 操作集中原則（ecs_logic）

- 在操作 `World` 的函數中，所有 `world` 的讀取（`get_resource`、`query`）應集中在最前面，所有寫入（`spawn`、`despawn`、`insert_resource`）應集中在最後面
- 中間只做純邏輯運算與 fail fast 驗證，不穿插任何 `world` 操作
- 此規則優先於 fail fast：即使某個驗證可以更早短路，也不能打斷 `world` 讀取的連續性

自訂錯誤型別

- `core/` crate 為了容易解析錯誤，使用自訂 enum，不用 String、anyhow 等通用錯誤型別

## 開發方法（TDD）

流程：先寫失敗的測試 → 實現邏輯使其通過

不需要測試

- inner functions
- serialize/deserialize

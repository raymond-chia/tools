# 行動後不應該能夠延遲

## 目標與範圍

讓延遲與移動互斥：移動過就不能延遲，進入延遲模式就不能移動。

不做：技能使用後禁用延遲（待技能系統實作後再處理）。

## 設計決策

- 「已行動不能延遲」是遊戲規則，放在 board crate
- 「延遲模式下不顯示移動範圍、不響應移動點擊」是顯示控制，放在 UI
- 判斷「已行動」：`movement_used > 0`（未來加技能時擴充條件）
- 延遲按鈕禁用表現：灰色禁用狀態

## 實作步驟

### board crate（`ecs_logic/turn.rs`）

1. 新增 `pub fn can_delay_current_unit(world: &mut World) -> Result<bool>` — 查詢當前單位是否可延遲（`movement_used == 0`）
2. `delay_current_unit` 內部加入同樣檢查，已行動時回傳錯誤

### UI（`editor/src/tabs/level_tab/battle.rs`）

3. `render_bottom_panel`：呼叫 `can_delay_current_unit` 決定延遲按鈕是否灰色禁用
4. `render_battlefield`：當 `is_delaying` 為 true 時，將 `reachable_positions` 設為空，不顯示移動範圍
5. `handle_mouse_click`：當 `is_delaying` 為 true 時，跳過左鍵移動邏輯

## 注意事項

- 未來實作技能系統後，`can_delay_current_unit` 的條件需擴充
- 延遲模式下仍可右鍵查看單位詳情

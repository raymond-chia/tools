# 在 Editor 戰鬥模式中整合移動功能

## 目標與範圍

在 editor 的戰鬥模式（battle.rs）中，讓當前行動單位自動顯示可移動範圍，玩家左鍵點選目標格子後執行移動。

**做什麼：**

- 進入戰鬥模式後，自動顯示當前行動單位的可移動範圍（2 MOV）
- 用不同顏色區分 1 MOV 和 2 MOV 範圍的格子
- 滑鼠懸停在可移動格子上時，顯示移動路徑預覽（路徑顏色高亮 + 顯示移動力消耗）
- 左鍵點選可移動格子後執行移動
- 移動後重新計算可移動範圍（剩餘移動力可繼續移動）

**不做什麼：**

- 不實作技能系統
- 不實作攻擊

## 設計決策

### 可移動範圍的計算

每幀重新計算，不做快取。等效能有問題再加快取。

直接使用 `board::ecs_logic::movement::get_reachable_positions(&mut world, occupant)`。該函數內部已實作 `budget = movement * 2 - movement_used` 的邏輯，天然支援 2 MOV 範圍。

區分 1 MOV 和 2 MOV 範圍：從 snapshot 的 unit_map 取得當前單位的 `movement` 和 `movement_used`，計算 `remaining_1mov = movement - movement_used`。`ReachableInfo.cost <= remaining_1mov` 為 1 MOV 範圍（移動後還能用技能），`cost > remaining_1mov` 為 2 MOV 範圍（移動+移動，不能用技能）。

`passthrough_only` 的格子不顯示高亮。

### 移動路徑預覽

滑鼠懸停在可到達格子（非 passthrough_only）上時，使用 `board::logic::movement::reconstruct_path` 計算路徑，將路徑上的格子用路徑預覽顏色高亮，並在 tooltip 中顯示移動力消耗。

### 移動執行

左鍵點選可到達格子（非 passthrough_only）時，呼叫 `board::ecs_logic::movement::execute_move(&mut world, occupant, target)`。

### 顏色常數

在 `editor/src/constants.rs` 新增：

- `BATTLEFIELD_COLOR_MOVE_1MOV` - 1 MOV 範圍內的格子顏色
- `BATTLEFIELD_COLOR_MOVE_2MOV` - 2 MOV 範圍內的格子顏色（移動+移動才能到達）
- `BATTLEFIELD_COLOR_MOVE_PATH` - 移動路徑預覽顏色

### 渲染層整合

`render_grid` 參數調整：

- `is_highlight` 重命名為 `is_border_highlight: impl Fn(Position) -> bool`（維持原行為，黃色外框）
- 新增 `get_bg_highlight: impl Fn(Position) -> Option<Color32>`（有值時填充該顏色作為格子背景覆蓋層）
- 繪製順序：先畫格子背景 → 再畫 bg_highlight → 再畫文字 → 最後畫外框

### 不修改 UI 狀態

因為每幀重新計算，不需要在 `LevelTabUIState` 中新增快取欄位。所有移動相關的資料（reachable_positions、movement、movement_used）都在 `render_form` 中即時計算。

### 操作流程

1. `render_form` 中取得當前行動單位的 occupant，呼叫 `get_reachable_positions` 計算可到達位置
2. 從 snapshot 取得當前單位的 movement 和 movement_used，計算 1 MOV / 2 MOV 分界
3. `render_grid` 時，透過 `get_bg_highlight` closure 回傳格子的背景高亮顏色（1 MOV / 2 MOV / 路徑預覽）
4. 滑鼠懸停可到達格子時，用 `reconstruct_path` 計算路徑，路徑格子用路徑顏色，tooltip 顯示消耗
5. 左鍵點選可到達格子時，呼叫 `execute_move`

## 實作步驟

1. **constants.rs**：新增 3 個移動相關顏色常數

2. **battlefield.rs**：`render_grid` 的 `is_highlight` 重命名為 `is_border_highlight`，新增 `get_bg_highlight: impl Fn(Position) -> Option<Color32>` 參數。同時將 `battlefield::is_highlight` 輔助函數重命名為 `is_border_highlight`

3. **deployment.rs / edit.rs**：更新 `render_grid` 呼叫，參數名改為 `is_border_highlight`，`get_bg_highlight` 傳入回傳 `None` 的 closure

4. **battle.rs**：
   - 在 `render_form` 或 `render_battlefield` 中，取得當前行動單位 occupant，呼叫 `get_reachable_positions`
   - 從 snapshot 取得當前單位的 movement 和 movement_used
   - 建構 `get_bg_highlight` closure：路徑預覽格子回傳路徑顏色，否則根據 cost 回傳 1 MOV / 2 MOV 顏色，passthrough_only 回傳 None
   - 滑鼠懸停時用 `reconstruct_path` 計算路徑，tooltip 顯示消耗
   - 更新 `handle_mouse_click` 處理左鍵點擊移動

## 注意事項

- editor 規則：禁止直接使用 world API，只能透過 board crate 的函數傳入 world
- `render_grid` 被 deployment.rs 和 battle.rs 共用，修改簽名需同步更新所有呼叫處
- 需要 import `board::logic::movement::ReachableInfo` 和 `board::ecs_logic::movement` 到 editor
- `reconstruct_path` 是純邏輯函數（在 `board::logic::movement` 中），不需要 world

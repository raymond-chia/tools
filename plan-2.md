# 在 battle.rs 使用 start_new_round

## 目標與範圍

在戰鬥模式中整合回合系統：進入戰鬥時初始化回合順序，顯示回合順序面板，提供結束回合/延遲按鈕，離開時回到編輯模式。

**不做**：移動功能。

## 設計決策

1. **觸發時機**：進入戰鬥模式時立即呼叫 `start_new_round`（加 TODO：未來改為玩家單位進入敵人 10 格範圍內才觸發）
2. **回合順序面板**：漂浮在戰場左側，無背景（底下露出戰場）；只顯示尚未行動的單位，越下面越早行動，當前行動單位在最底部
3. **互動功能**：點擊條目選中對應單位（棋盤高亮 + 調整 scroll_offset 置中）
4. **操作按鈕**：戰場下方放「結束回合」和「延遲」按鈕（類似 bottom panel）
5. **返回**：「返回」直接回到 `Edit` 模式，清空 world（`world = World::default()`）
6. **延遲互動**：點「延遲」按鈕進入延遲模式，面板條目之間出現可點擊的插入點，點擊插入點後執行延遲
7. **結束回合**：最底部條目移除，下一個成為當前行動單位

## 實作步驟

### 1. LevelTabUIState 新增欄位

在 `level_tab.rs` 的 `LevelTabUIState` 新增：

- `is_delaying: bool`：是否處於延遲選擇模式

### 2. deployment.rs：開始戰鬥時呼叫 start_new_round

在「開始戰鬥」按鈕 click handler 中：

- 呼叫 `board::ecs_logic::turn::start_new_round(&mut ui_state.world)`
- 失敗則顯示錯誤且不切換模式
- 加 TODO 註解：未來改為玩家單位進入敵人 10 格範圍內才觸發

### 3. battle.rs：返回編輯模式

將「返回部署」改為「返回」，click handler 中：

- `ui_state.world = World::default()`
- `ui_state.mode = LevelTabMode::Edit`

### 4. battle.rs：渲染回合順序面板（漂浮左側）

用 `egui::Area`（無 frame/背景）在戰場左側疊加回合面板：

- 從 World 查詢 `TurnOrder`（透過 `board::ecs_logic::turn::get_turn_order`）
- 顯示當前輪數 `round`
- 過濾出 `has_acted == false` 的 entries，反轉排列（最後行動的在上，當前行動的在底部）
- 每個條目顯示單位名稱（從 `Snapshot.unit_map` 用 occupant 反查）+ 陣營顏色
- 點擊條目：設定 `ui_state.selected_left_pos` 為該單位位置，並調整 `scroll_offset` 使其置中

### 5. battle.rs：延遲模式

- 點「延遲」按鈕 → 設定 `ui_state.is_delaying = true`
- 面板進入延遲模式：在每對相鄰條目之間渲染一個可點擊的插入點（如水平線或按鈕）
- 插入點只出現在當前單位（最底部）之上的位置
- 點擊插入點 → 計算對應的 `target_index`，呼叫 `delay_current_unit(world, target_index)`，然後 `is_delaying = false`

### 6. battle.rs：棋盤高亮選中單位

將 `battlefield::is_highlight(None)` 改為 `battlefield::is_highlight(ui_state.selected_left_pos)`。

### 7. battle.rs：底部操作面板

在戰場下方渲染：

- 「結束回合」按鈕 → 呼叫 `board::ecs_logic::turn::end_current_turn`，結果更新 snapshot
- 「延遲」按鈕 → 設定 `is_delaying = true`

### 8. 加 TODO 註解

在 start_new_round 呼叫處加 TODO：未來改為玩家單位進入敵人 10 格範圍內才觸發。

## 注意事項

- `start_new_round` 在 TurnOrder 已存在時回傳錯誤，返回 Edit 時清空 world 可避免此問題
- 回合面板需要從 Snapshot 的 `unit_map`（`HashMap<Position, UnitBundle>`）反查 occupant 對應的位置和名稱
- 面板漂浮無背景需注意文字可讀性（可加半透明底或文字描邊）
- 延遲的 `target_index` 需要從面板的視覺位置（反轉後）轉換回 `TurnOrder.entries` 的實際 index

# 戰鬥模式技能列表 UI

## 目標與範圍

在戰鬥模式底部面板（結束回合、延遲旁邊）新增「技能」按鈕，點擊後在按鈕上方彈出當前行動單位的技能列表。

- 可用技能正常顯示
- 不可用技能灰色，hover 顯示「缺乏魔力」
- 目前只顯示列表，不處理技能使用（點擊技能無動作）

## 設計決策

- 彈出方式：使用 `egui::Area` + `egui::Frame` 手動定位在按鈕上方。如果有問題退回 `egui::Window`
- 開關狀態：在 `LevelTabUIState` 新增 `show_skill_popup: bool`
- 關閉方式：再次點擊按鈕 toggle，或點擊 popup 外部區域關閉
- 資料來源：呼叫 `board::ecs_logic::skill::get_available_skills`

## 實作步驟

1. **`editor/src/tabs/level_tab.rs`** — `LevelTabUIState` 新增欄位 `pub show_skill_popup: bool`
2. **`editor/src/tabs/level_tab/battle.rs`** — `render_bottom_panel` 中：
   - 在延遲按鈕後新增「技能」按鈕，toggle `show_skill_popup`
   - 記錄按鈕的 `response.rect` 作為 popup 定位基準
3. **`editor/src/tabs/level_tab/battle.rs`** — 新增 `render_skill_popup` 函數：
   - 用 `egui::Area::new(...).fixed_pos(...)` 定位在按鈕上方
   - 搭配 `egui::Frame` 加背景邊框
   - 呼叫 `get_available_skills` 取得技能列表
   - 遍歷顯示：可用的正常 label，不可用的用 `ui.add_enabled(false, ...)` + hover 提示
   - 點擊外部關閉：檢查 `response.clicked_elsewhere()`

## 注意事項

- Area 定位需要知道 popup 高度才能精確放在按鈕正上方；第一幀可能位置不準，可用估算高度或接受微小偏移
- `get_available_skills` 需要 `&mut World`，需注意借用順序
- 延遲模式和技能 popup 同時開啟可能造成混亂，考慮互斥（開技能時關延遲，反之亦然）

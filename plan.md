# 戰鬥模式技能施放設計

## 背景

`editor/src/tabs/level_tab/battle.rs` 目前缺口：`handle_mouse_click` 在 `BattleAction::SkillPopup` 下沒有呼叫 `execute_skill`，技能點不出來。

## 多目標支援（core/board 現狀）

`Target` 欄位：`count`（目標數上限）、`allow_same_target`、`selection`、`selectable_filter`、`area`。
`execute_skill(world, skill_name, &[Position])` 已支援多目標，對每個目標各展開 AOE 並累計 `EffectEntry`。
`validate_skill_targets` 負責數量、重複、射程、filter 檢查。

→ **多目標邏輯已在 core，editor 不該自己追蹤 count / picked。**

## 設計決策

### 決策 1：選目標的流程採「逐次累積 + 玩家按確認」（方案 B）

玩家可以少於 count 就施放（core 允許 `1..=count`），較有彈性。達到 count 不自動觸發，必須按確認按鈕。

### 決策 2：「已選目標」狀態放在 World，core 提供四個函數

避免 editor 與 core 各自保存 `picked` 不同步，也避免 editor 需要知道 count / allow_same_target。

在 `core/board/src/ecs_types/resources.rs` 新增 resource：

```rust
pub struct SkillTargeting {
    pub skill_name: SkillName,
    pub picked: Vec<Position>,
}
```

在 `core/board/src/ecs_logic/skill.rs` 新增：

```rust
pub fn start_skill_targeting(world: &mut World, skill_name: &SkillName) -> Result<()>
pub fn add_skill_target(world: &mut World, pos: Position) -> Result<()>
pub fn get_skill_targeting(world: &World) -> Option<&SkillTargeting>
pub fn cancel_skill_targeting(world: &mut World)
```

- `start`：驗證技能存在且當前單位可用；建立 resource，清空 picked
- `add`：驗證位置在 targetable 範圍內；`allow_same_target=false` 且已存在 → 直接忽略（回 Ok）；已滿 count 則回錯；其餘情況 push 進 picked
- `get`：查詢目前狀態供 UI 渲染
- `cancel`：移除 resource

editor 按「確認」時自己 `get_skill_targeting` 取 `(skill_name, picked)` → 呼叫既有 `execute_skill` → `cancel_skill_targeting`。

## editor 側 UI

### `BattleAction`

```rust
SkillPopup { selected_skill_name: Option<SkillName> },  // 維持不變
SkillTargeting,  // 不帶資料，狀態由 world 持有
```

popup 點技能 → 呼叫 `start_skill_targeting` → `battle_action = SkillTargeting` → popup 關閉。

### 渲染

`render_battlefield` 在 `SkillTargeting` 下：

- 呼叫 `get_skill_targeting` 取 `skill_name` 與 `picked`
- `get_skill_targetable_positions(skill_name)` 高亮可選格
- hover 時 `get_skill_affected_positions` 顯示 AOE 預覽
- `picked` 中的格子用新增的 `BATTLEFIELD_COLOR_SKILL_PICKED` 標色區分

### 點擊

`handle_mouse_click` 在 `SkillTargeting` 下：

- **左鍵**：先在 editor 側檢查 `skill_targetable.contains(&pos)`，通過才呼叫 `add_skill_target`。
  - `allow_same_target=false` 下，core 內對已選過的格子直接「忽略」（不視為錯誤，也不 toggle），避免與 `allow_same_target=true` 的行為混淆。
- **右鍵**：`cancel_skill_targeting` → `battle_action = Normal`

### 確認 / 取消按鈕

底部操作面板在 `SkillTargeting` 下改顯示：

- 「確認施放（已選 N）」：`get_skill_targeting` 取 picked → `execute_skill` → `battle_log.extend(...)` → `cancel_skill_targeting` → `battle_action = Normal`
- 「取消」：`cancel_skill_targeting` → `battle_action = Normal`

picked 為空時「確認」禁用（core 會拒絕空 targets）。

## 對戰 Log

### 資料

`LevelTabUIState` 新增：

```rust
pub battle_log: Vec<EffectEntry>,
```

`EffectEntry` 從 `board::logic::skill::skill_execution` re-export（必要時公開 visibility）。

### 格式化

`battle.rs` 內：

```rust
fn format_effect_entry(entry: &EffectEntry, snapshot: &Snapshot) -> String
```

逐類型輸出一行字串。MVP 可先 `format!("{:?}", entry)`。

### 顯示位置

與「右側單位詳情面板」共用右側空間，透過一個 toggle 按鈕切換顯示 log 或 詳情。

- 在 `LevelTabUIState` 新增 `right_panel_view: RightPanelView`（`Details` / `Log`）
- 原本「右側詳情面板」的條件 `ui_state.selected_right_pos.is_some()` 改為：`Details` 模式下有選中才顯示詳情、`Log` 模式一律顯示 log
- toggle 按鈕放在右側面板頂部（或底部面板加一顆切換鈕），點擊切換兩種視圖

## 實作步驟

1. core/board：新增 `SkillTargeting` resource 與四個函數，補測試
2. core/board：確認 `EffectEntry` / `ResolvedEffect` / `CheckTarget` 為 `pub`，必要時公開
3. 更新 `core-index.md`
4. editor：`BattleAction::SkillPopup` 選定技能 → `start_skill_targeting`，新增 `SkillTargeting` 變體
5. editor：`LevelTabUIState` 加 `battle_log`
6. editor：`render_battlefield` 支援 `SkillTargeting` 下的高亮 / AOE / picked 標色
7. editor：`handle_mouse_click` 實作左鍵 add、右鍵 cancel
8. editor：底部面板加「確認 / 取消」按鈕
9. editor：加 `render_battle_log` 與 `format_effect_entry`
10. editor：`constants.rs` 加 `BATTLEFIELD_COLOR_SKILL_PICKED`
11. 更新 `editor-index.md`

## 已確認

- 多目標：方案 B（逐次累積 + 確認按鈕）
- 狀態放 world，core 提供 start / add / get / cancel 四函數
- log 與單位詳情共用右側面板，透過 toggle 切換
- `allow_same_target=false` 時點已選格子：core 忽略（不報錯、不 toggle）

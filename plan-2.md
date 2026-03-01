# 部署面板改動計畫

目標：部署點選擇完全在地圖預覽上進行，左側面板改為顯示已部署單位 + 條件顯示 ComboBox

## 改動檔案

`editor/src/tabs/level_tab/deployment.rs`

## 改動內容

### 1. `render_unit_combobox` — 修改簽名與邏輯

- 參數改為 `(ui, pos, deployed_name: &Option<String>, ui_state)`，移除 `index`
- ComboBox id 改用座標 `player_unit_selector_{x}_{y}`
- `selected_text`：有已部署單位時顯示單位名稱，否則顯示「選擇單位」
- 已部署時，選項最前面加一個「── 清除 ──」常數選項，選中後呼叫 `undeploy_unit`
- 定義常數 `CLEAR_UNIT_LABEL`

### 2. `render_player_deployment_panel` — 重寫

ScrollArea 內依序顯示：

1. **ComboBox 區塊**（條件顯示）：`selected_left_pos` 是部署點時，顯示座標標題 + 呼叫 `render_unit_combobox`
2. **已部署單位列表**（一直顯示）：遍歷 `deployment_positions`，有部署單位的顯示座標和單位名稱；無任何部署時顯示「尚未部署任何單位」

### 3. `render_deployment_slot` — 刪除

邏輯已合併到面板中，不再需要。

### 不動的部分

- `render_form`、`render_top_bar`、`render_level_info`
- `render_battlefield`、`handle_mouse_click`
- `deployed_positions`

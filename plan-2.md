# 重構 battle.rs：共用 deployment.rs 邏輯

## 目標與範圍

將 `deployment.rs` 中可共用的函數搬到 `grid.rs`（已合併 `ui.rs`），然後讓 `battle.rs` 重用這些函數。
battle 模式不需要部署點相關的 UI，但透過將 `deployment_set` 設為空 HashSet 來共用 `Snapshot` 和渲染邏輯。

## 設計決策

- **共用 Snapshot**：battle 重用同一個 `Snapshot` struct，deployment 相關欄位傳空值
- **關卡資訊**：battle 顯示關卡名稱、尺寸、敵人數量（不顯示玩家部署數/上限）
- **點擊行為**：battle 有自己的 `handle_mouse_click`，不共用
- **檔案合併**：`ui.rs` 已搬入 `grid.rs`，刪除 `ui.rs`
- **grid.rs 改名 battlefield.rs**：職責從純幾何擴展為戰場共用邏輯

## 實作步驟

### 1. 刪除 `ui.rs`，`grid.rs` 改名 `battlefield.rs`

### 2. 搬共用函數到 `battlefield.rs`

從 `deployment.rs` 搬移：

- `Snapshot` struct + `query_snapshot` fn
- `get_cell_info`、`get_tooltip_info`、`is_highlight`
- `get_faction_color`、`get_unit_abbr`
- `render_details_panel` + `render_unit_details` + `render_object_details`
- `enemy_units`

不搬（deployment 專用）：

- `handle_mouse_click` — battle 有不同的點擊邏輯
- `deployed_positions` — 只有 deployment 用到

### 3. 修改 `deployment.rs`

移除搬走的函數，引用 `super::battlefield::*`。

### 4. 重寫 `battle.rs`

- 用 `query_battle_snapshot` 建立 Snapshot（deployment 欄位為空）
- 頂部：返回按鈕
- 關卡資訊：名稱、尺寸、敵人數量
- 主要佈局：戰場 + 右側詳情面板
- battle 自己的 `handle_mouse_click`
- 圖例

### 5. 更新 `level_tab.rs` 的 mod 宣告

`mod grid` → `mod battlefield`，刪除 `mod ui`。

### 6. 更新索引文件

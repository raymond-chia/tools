---
paths:
  - "editor/**/*"
---

# Editor 專案索引

## 編輯規則

### 專案結構

禁止列舉具體的常數值，只記錄檔案與職責

例子：

- ❌ 錯誤：constants.rs - BUTTON_WIDTH = 100, PANEL_HEIGHT = 300
- ✓ 正確：constants.rs - UI 與編輯器常數定義

### Function 集簽名

保留完整簽名（pub fn、trait 方法），移除實現細節, 常數值, enum, struct

例子：

- ❌ 錯誤：`pub fn render_search_input(ui: &mut egui::Ui, query: &mut String) -> egui::Response` - 檢查輸入是否為空、建立搜尋框寬度為 200px、清除按鈕顏色
- ✓ 正確：`pub fn render_search_input(ui: &mut egui::Ui, query: &mut String) -> egui::Response` - 渲染搜尋輸入框

## 專案結構

```
editor/
├── src/
│   ├── main.rs              - 入口函數、字體設置、模組聲明
│   ├── constants.rs         - UI 與編輯器常數定義
│   ├── editor_item.rs       - EditorItem trait 定義
│   ├── editor_macros.rs     - define_editors! 巨集（自動生成編輯器結構）
│   ├── generic_editor.rs    - 泛型編輯器狀態管理（GenericEditorState<T>、EditMode）
│   ├── generic_io.rs        - 泛型 TOML I/O（載入、儲存）
│   ├── app.rs               - eframe::App trait 實現、UI 渲染
│   ├── utils/               - 通用工具模組
│   │   ├── mod.rs           - 工具模組定義和導出
│   │   ├── dnd.rs           - 拖放（DnD）相關工具
│   │   └── search.rs        - 搜尋相關工具
│   └── tabs/
│       ├── mod.rs           - 標籤頁模組定義
│       ├── object_tab.rs    - 物件編輯器（ObjectType）
│       ├── skill_tab.rs     - 技能編輯器
│       ├── unit_tab.rs      - 單位編輯器（UnitType）
│       ├── level_tab.rs     - 關卡編輯器（部署/戰鬥模式主模塊）
│       └── level_tab/
│           ├── battle.rs    - 戰鬥模式UI
│           ├── deployment.rs - 部署模式UI
│           ├── grid.rs      - 棋盤網格渲染邏輯（編輯和模擬模式）
│           └── unit_details.rs - 單位詳情展示相關函數
```

## Function 集

### editor/editor_item.rs

- `pub trait EditorItem` - 所有可編輯項目必須實現的 trait
  - `type UIState: Default` - UI 狀態關聯型別（如搜尋、拖曳等）
  - `fn name(&self) -> &str` - 取得項目名稱
  - `fn set_name(&mut self, name: String)` - 設定項目名稱
  - `fn type_name() -> &'static str` - 項目類型名稱
  - `fn type_name_plural() -> &'static str` - 複數形式
  - `fn validate(&self, all_items: &[Self], editing_index: Option<usize>) -> Result<(), String>` - 驗證項目
  - `fn after_confirm(&mut self)` - 編輯確認後的鉤子（如排序、正規化等）
- `pub fn validate_name<T: EditorItem>(item: &T, all_items: &[T], editing_index: Option<usize>) -> Result<(), String>` - 驗證項目名稱（檢查非空和重複）

### editor/generic_editor.rs

- `pub fn set_success(&mut self, msg: impl Into<String>)` - 設置成功訊息
- `pub fn set_error(&mut self, msg: impl Into<String>)` - 設置錯誤訊息
- `pub fn start_creating(&mut self)` - 開始新增項目
- `pub fn start_editing(&mut self, index: usize)` - 開始編輯項目
- `pub fn start_copying(&mut self, index: usize)` - 複製項目
- `pub fn confirm_edit(&mut self)` - 確認編輯
- `pub fn cancel_edit(&mut self)` - 取消編輯
- `pub fn delete_item(&mut self, index: usize)` - 刪除項目
- `pub fn is_editing(&self) -> bool` - 判斷是否在編輯模式
- `pub fn move_item(&mut self, from: usize, to: usize)` - 移動項目（拖曳排序用）

### editor/generic_io.rs

- `pub fn load_file<T: EditorItem>(state: &mut GenericEditorState<T>, path: &Path, data_key: &str)` - 從 TOML 檔案載入項目
- `pub fn save_file<T: EditorItem>(state: &mut GenericEditorState<T>, path: &Path, data_key: &str)` - 儲存項目到 TOML 檔案

### editor/editor_macros.rs

- `pub macro define_editors` - 生成 EditorTab 枚舉、EditorApp 結構和 new() 方法（自動載入檔案）

### editor/utils/dnd.rs

- `pub fn render_dnd_handle(ui: &mut egui::Ui, item_id: Id, index: usize, label: &str) -> Option<(usize, usize)>` - 渲染拖曳手柄，返回 (from_index, to_index)

### editor/utils/search.rs

- `pub fn render_search_input(ui: &mut egui::Ui, query: &mut String) -> egui::Response` - 渲染搜尋輸入框
- `pub fn match_search_query(item: &str, query_lower: &str) -> bool` - 判斷項目是否匹配搜尋查詢
- `pub fn filter_by_search<'a, T: AsRef<str>>(items: &'a [T], query: &str) -> Vec<&'a T>` - 根據搜尋查詢過濾列表

### editor/tabs/object_tab.rs、skill_tab.rs、unit_tab.rs

每個 tab 提供：

- `pub fn file_name() -> &'static str` - 取得檔案名稱
- `pub fn render_form(ui: &mut egui::Ui, item: &mut T)` - 渲染編輯表單
- `impl EditorItem for T` - 實現 EditorItem trait

### editor/tabs/level_tab/grid.rs

- `pub struct DragState` - 拖曳狀態
- `pub enum DraggedObject` - 拖曳物體的類型和索引
- `pub fn prepare_lookup_maps(level: &LevelType) -> (HashSet<Position>, HashMap<Position, &UnitPlacement>, HashMap<Position, &ObjectPlacement>)` - 建立查詢表以加速格子內容查詢
- `pub fn calculate_grid_dimensions(level: &LevelType) -> (f32, f32)` - 計算棋盤預覽的總尺寸
- `pub fn calculate_visible_range(scroll_offset: egui::Vec2, viewport_size: egui::Vec2, level: &LevelType) -> VisibleGridRange` - 計算可見範圍內的格子索引
- `pub fn screen_to_board_pos(screen_pos: egui::Pos2, rect: egui::Rect, level: &LevelType) -> Option<Position>` - 將螢幕座標轉換為棋盤座標
- `pub fn render_grid(ui: &mut egui::Ui, rect: egui::Rect, player_positions: &HashSet<Position>, enemy_units_map: &HashMap<Position, &UnitPlacement>, objects_map: &HashMap<Position, &ObjectPlacement>, drag_state: Option<DragState>, hovered_in_bounds: Option<Position>, visible_range: VisibleGridRange)` - 繪製棋盤格子（編輯模式）
- `pub fn render_simulation_grid(ui: &mut egui::Ui, rect: egui::Rect, level: &LevelType, player_positions: &HashSet<Position>, enemy_units_map: &HashMap<Position, &UnitPlacement>, objects_map: &HashMap<Position, &ObjectPlacement>, simulation_state: &SimulationState, visible_range: VisibleGridRange, _skills_map: &HashMap<SkillName, SkillType>, _units_map: &HashMap<TypeName, UnitType>)` - 渲染模擬戰鬥的棋盤網格
- `pub fn identify_dragged_object(level: &LevelType, pos: &Position) -> Option<DraggedObject>` - 識別被拖曳的物體及其索引
- `pub fn apply_drag_update(level: &mut LevelType, state: DragState, new_pos: Position)` - 應用拖曳更新
- `pub fn render_hover_tooltip(ui: &mut egui::Ui, level: &LevelType, rect: egui::Rect, response: &egui::Response, player_positions: &HashSet<Position>, enemy_units_map: &HashMap<Position, &UnitPlacement>, objects_map: &HashMap<Position, &ObjectPlacement>)` - 渲染懸停提示
- `pub fn render_battlefield_legend(ui: &mut egui::Ui)` - 渲染戰場圖例

### editor/tabs/level_tab/unit_details.rs

- `pub fn handle_unit_right_click(response: &egui::Response, rect: egui::Rect, level: &LevelType, player_positions: &HashSet<Position>, enemy_units_map: &HashMap<Position, &UnitPlacement>, ui_state: &mut LevelTabUIState)` - 處理右鍵點擊選擇單位
- `pub fn handle_panel_close_on_click(response: &egui::Response, rect: egui::Rect, level: &LevelType, player_positions: &HashSet<Position>, enemy_units_map: &HashMap<Position, &UnitPlacement>, ui_state: &mut LevelTabUIState)` - 處理點擊戰場空白處關閉面板
- `pub fn render_unit_details_panel(ui: &mut egui::Ui, unit_type_name: &TypeName, skills_map: &HashMap<SkillName, SkillType>, units_map: &HashMap<TypeName, UnitType>)` - 渲染單位詳情面板的內容（不含面板容器）

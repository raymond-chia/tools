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
│   ├── main.rs              - 程式進入點和初始化
│   ├── constants.rs         - UI 與編輯器常數定義
│   ├── editor_item.rs       - EditorItem trait 定義
│   ├── editor_macros.rs     - 編輯器結構自動生成巨集
│   ├── generic_editor.rs    - 泛型編輯器狀態管理
│   ├── generic_io.rs        - 泛型 TOML 檔案載入與儲存
│   ├── app.rs               - 主應用程式 UI 渲染
│   ├── utils/               - 通用工具模組
│   │   ├── mod.rs           - 工具模組定義和導出
│   │   ├── dnd.rs           - 拖放功能
│   │   └── search.rs        - 搜尋功能
│   └── tabs/
│       ├── mod.rs           - 標籤頁模組定義
│       ├── object_tab.rs    - 物件編輯器
│       ├── skill_tab.rs     - 技能編輯器
│       ├── unit_tab.rs      - 單位編輯器
│       ├── level_tab.rs     - 關卡編輯器主邏輯
│       └── level_tab/
│           ├── battle.rs      - 戰鬥模式 UI
│           ├── battlefield.rs - 戰場共用邏輯（網格、快照、詳情面板）
│           ├── deployment.rs  - 部署模式 UI
│           └── edit.rs        - 編輯模式 UI
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

MessageState 的方法：

- `pub fn set_success(&mut self, msg: impl Into<String>)` - 設置成功訊息
- `pub fn set_error(&mut self, msg: impl Into<String>)` - 設置錯誤訊息

GenericEditorState 的方法：

- `pub fn set_success(&mut self, msg: impl Into<String>)` - 設置成功訊息
- `pub fn set_error(&mut self, msg: impl Into<String>)` - 設置錯誤訊息
- `pub fn start_creating(&mut self)` - 開始新增項目
- `pub fn start_editing(&mut self, index: usize)` - 開始編輯項目
- `pub fn start_copying(&mut self, index: usize)` - 複製項目
- `pub fn confirm_edit(&mut self)` - 確認編輯（含驗證與後處理）
- `pub fn cancel_edit(&mut self)` - 取消編輯
- `pub fn delete_item(&mut self, index: usize)` - 刪除項目
- `pub fn is_editing(&self) -> bool` - 判斷是否在編輯模式
- `pub fn move_item(&mut self, from: usize, to: usize)` - 移動項目（拖曳排序用）

### editor/generic_io.rs

- `pub fn load_file<T: EditorItem>(state: &mut GenericEditorState<T>, path: &Path, data_key: &str)` - 從 TOML 檔案載入項目（通過狀態消息反映結果）
- `pub fn save_file<T: EditorItem>(state: &mut GenericEditorState<T>, path: &Path, data_key: &str)` - 儲存項目到 TOML 檔案（通過狀態消息反映結果）

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

### editor/tabs/level_tab/battlefield.rs

網格渲染與座標轉換：

- `pub struct VisibleGridRange` - 棋盤可見範圍
- `pub fn calculate_grid_dimensions(board: Board) -> egui::Vec2` - 計算棋盤預覽的總尺寸
- `pub fn calculate_visible_range(scroll_offset: egui::Vec2, viewport_size: egui::Vec2, board: Board) -> VisibleGridRange` - 計算可見範圍內的格子索引
- `pub fn screen_to_board_pos(screen_pos: egui::Pos2, rect: egui::Rect, board: Board) -> Option<Position>` - 將螢幕座標轉換為棋盤座標
- `pub fn compute_hover_pos(response: &egui::Response, rect: egui::Rect, board: Board) -> Option<Position>` - 計算滑鼠懸停時的棋盤座標
- `pub fn render_grid(ui: &mut egui::Ui, rect: egui::Rect, board: Board, scroll_offset: egui::Vec2, get_cell_info: impl Fn(Position) -> (String, Color32, Color32), is_highlight: impl Fn(Position) -> bool)` - 繪製棋盤格子
- `pub fn render_hover_tooltip(ui: &mut egui::Ui, rect: egui::Rect, hovered_pos: Position, get_tooltip_info: impl Fn(Position) -> String)` - 渲染懸停提示
- `pub fn render_battlefield_legend(ui: &mut egui::Ui)` - 渲染戰場圖例

快照與共用邏輯：

- `pub struct Snapshot` - 戰場模式所需的關卡查詢結果
- `pub fn query_snapshot(world: &mut World) -> CResult<Snapshot>` - 一次查詢所有關卡資料
- `pub fn get_cell_info(snapshot: &Snapshot) -> impl Fn(Position) -> (String, Color32, Color32)` - 取得格子顯示資訊
- `pub fn is_highlight(selected_pos: Option<Position>) -> impl Fn(Position) -> bool` - 判斷是否高亮
- `pub fn get_tooltip_info(snapshot: &Snapshot) -> impl Fn(Position) -> String` - 取得懸停提示資訊
- `pub fn render_details_panel(ui: &mut egui::Ui, pos: Position, snapshot: &Snapshot)` - 渲染詳情面板
- `pub fn enemy_units(snapshot: &Snapshot) -> impl Iterator<Item = &UnitBundle>` - 取得敵方單位
- `pub fn get_faction_color(factions: &[Faction], unit_faction_id: ID) -> Color32` - 取得陣營顏色
- `pub fn get_unit_abbr(unit_name: &str) -> String` - 取得單位名稱縮寫

### editor/tabs/level_tab/deployment.rs

- `pub fn render_form(ui: &mut egui::Ui, ui_state: &mut LevelTabUIState, message_state: &mut MessageState)` - 渲染單位部署模式表單

### editor/tabs/level_tab/battle.rs

- `pub fn render_form(ui: &mut egui::Ui, ui_state: &mut LevelTabUIState, message_state: &mut MessageState)` - 渲染戰鬥模式表單

### editor/tabs/level_tab/edit.rs

- `pub fn render_form(ui: &mut egui::Ui, level: &mut LevelType, ui_state: &mut LevelTabUIState, message_state: &mut MessageState)` - 渲染編輯模式的表單
- `pub fn file_name() -> &'static str` - 取得檔案名稱

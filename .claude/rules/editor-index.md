---
paths:
  - "editor/**/*"
---

# Editor 專案索引

本檔案包含 `editor` crate 的專案結構和 function 集。

## 維護原則

**專案結構**

- 紀錄檔案存在與主要職責，不列舉具體的常數值
- 如果有新增/移除檔案才更新

**Function 集和 Trait**

- 保留 **簽名**：pub fn、trait 方法（API 相對穩定，幫助理解「怎麼用」）
- 移除 **實現細節**（如「檢查索引有效性」、「建立目錄如需要」），這些容易變
- 移除會頻繁變動的具體值（如常數值、enum variants）

## 專案結構

⚠️ **編輯前檢查清單**（見上方「維護原則」）
- [ ] 只記錄檔案存在與主要職責？
- [ ] 沒有列舉具體的常數值？
- [ ] 新增/移除檔案時才編輯？

```
editor/
├── src/
│   ├── main.rs              - 入口函數、字體設置、模組聲明
│   ├── constants.rs         - UI 與編輯器常數定義
│   ├── editor_item.rs       - EditorItem trait 定義
│   ├── generic_editor.rs    - 泛型編輯器狀態管理（GenericEditorState<T>、EditMode）
│   ├── generic_io.rs        - 泛型 TOML I/O（載入、儲存）
│   ├── state.rs             - EditorApp 主應用狀態、EditorTab 頁籤
│   ├── app.rs               - eframe::App trait 實現、UI 渲染
│   └── tabs/
│       ├── object_tab.rs    - 物件編輯器（ObjectType）
│       ├── skill_tab.rs     - 技能編輯器
│       ├── unit_tab.rs      - 單位編輯器（UnitType）
│       └── level_tab.rs     - 關卡編輯器（尚未實作）
```

## Function 集

⚠️ **編輯前檢查清單**（見上方「維護原則」）
- [ ] 保留了完整的函數簽名？
- [ ] 移除了實現細節（如「檢查索引有效性」、「建立目錄」）？
- [ ] 沒有列舉具體的常數值或 enum variants？

### editor/editor_item.rs

- `pub trait EditorItem` - 所有可編輯項目必須實現的 trait
  - `fn name(&self) -> &str` - 取得項目名稱
  - `fn set_name(&mut self, name: String)` - 設定項目名稱
  - `fn validate(&self) -> Result<(), String>` - 驗證項目
  - `fn type_name() -> &'static str` - 項目類型名稱
  - `fn type_name_plural() -> &'static str` - 複數形式

### editor/generic_editor.rs

- `pub fn set_success(&mut self, msg: impl Into<String>)` - 設置成功訊息
- `pub fn set_error(&mut self, msg: impl Into<String>)` - 設置錯誤訊息
- `pub fn start_creating(&mut self)` - 開始新增項目
- `pub fn start_editing(&mut self, index: usize)` - 開始編輯項目
- `pub fn start_copying(&mut self, index: usize)` - 複製項目
- `pub fn confirm_edit(&mut self)` - 確認編輯
- `pub fn cancel_edit(&mut self)` - 取消編輯
- `pub fn delete_item(&mut self, index: usize)` - 刪除項目

### editor/generic_io.rs

- `pub fn load_file<T: EditorItem>(state: &mut GenericEditorState<T>, path: &Path, data_key: &str)` - 從 TOML 檔案載入項目
- `pub fn save_file<T: EditorItem>(state: &mut GenericEditorState<T>, path: &Path, data_key: &str)` - 儲存項目到 TOML 檔案

### editor/state.rs

- `impl EditorApp { pub fn new() -> Self }` - 建立編輯器並初始化狀態

### editor/tabs/object_tab.rs、skill_tab.rs、unit_tab.rs

每個 tab 提供：

- `pub fn file_name() -> &'static str` - 取得檔案名稱
- `pub fn render_form(ui: &mut egui::Ui, item: &mut T)` - 渲染編輯表單
- `impl EditorItem for T` - 實現 EditorItem trait

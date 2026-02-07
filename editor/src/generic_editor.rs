//! 泛型編輯器狀態管理

use crate::constants::COPY_SUFFIX;
use crate::editor_item::EditorItem;

/// 編輯模式
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EditMode {
    #[default]
    None, // 瀏覽模式
    Creating,       // 新增模式
    Editing(usize), // 編輯模式（儲存索引）
}

/// 泛型編輯器狀態
#[derive(Debug, Default)]
pub struct GenericEditorState<T: EditorItem> {
    /// 最後的操作訊息（成功或錯誤）
    pub message: String,
    /// 訊息是否為錯誤
    pub is_error: bool,
    /// 訊息是否可見
    pub message_visible: bool,
    /// 項目列表
    pub items: Vec<T>,
    /// 當前選中的項目索引
    pub selected_index: Option<usize>,
    /// 編輯中的項目（暫存）
    pub editing_item: Option<T>,
    /// 當前編輯模式
    pub edit_mode: EditMode,
    /// 編輯器 UI 狀態（搜尋、拖曳等）
    pub ui_state: T::UIState,
}

impl<T: EditorItem> GenericEditorState<T> {
    /// 設置成功訊息
    pub fn set_success(&mut self, msg: impl Into<String>) {
        self.message = msg.into();
        self.is_error = false;
        self.message_visible = true;
    }

    /// 設置錯誤訊息
    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.message = msg.into();
        self.is_error = true;
        self.message_visible = true;
    }

    /// 開始新增項目
    pub fn start_creating(&mut self) {
        self.editing_item = Some(T::default());
        self.edit_mode = EditMode::Creating;
    }

    /// 開始編輯項目
    pub fn start_editing(&mut self, index: usize) {
        // Fail Fast: 驗證索引
        if index >= self.items.len() {
            self.set_error(format!("無效的索引：{}", index));
            return;
        }

        self.editing_item = Some(self.items[index].clone());
        self.edit_mode = EditMode::Editing(index);
    }

    /// 複製項目
    pub fn start_copying(&mut self, index: usize) {
        // Fail Fast: 驗證索引
        if index >= self.items.len() {
            self.set_error(format!("無效的索引：{}", index));
            return;
        }

        let mut copied = self.items[index].clone();
        copied.set_name(format!("{}{}", copied.name(), COPY_SUFFIX));
        self.editing_item = Some(copied);
        self.edit_mode = EditMode::Creating;
    }

    /// 確認編輯
    pub fn confirm_edit(&mut self) {
        // Fail Fast: 驗證是否正在編輯
        if self.edit_mode == EditMode::None {
            self.set_error("目前不在編輯模式");
            return;
        }

        // Fail Fast: 驗證項目是否存在且有效
        if self.editing_item.is_none() {
            self.set_error(format!("編輯{}不存在", T::type_name()));
            self.edit_mode = EditMode::None;
            return;
        }

        if let Err(e) = self.editing_item.as_ref().unwrap().validate() {
            self.set_error(e);
            return;
        }

        // 驗證通過後的鉤子（如排序、正規化等）
        if let Some(item) = &mut self.editing_item {
            item.after_confirm();
        }

        // 此時 editing_item 已確保不為 None
        let item = self
            .editing_item
            .as_ref()
            .expect("內部邏輯錯誤：前面已驗證過，editing_item 不應為 None");

        match self.edit_mode {
            EditMode::Creating => {
                self.items.push(item.clone());
                self.set_success(format!("成功新增{}：{}", T::type_name(), item.name()));
                self.selected_index = Some(self.items.len() - 1);
            }
            EditMode::Editing(index) => {
                // Fail Fast: 驗證索引
                if index >= self.items.len() {
                    self.set_error(format!("無效的索引：{}", index));
                    self.edit_mode = EditMode::None;
                    self.editing_item = None;
                    return;
                }

                self.items[index] = item.clone();
                self.set_success(format!("成功編輯{}：{}", T::type_name(), item.name()));
                self.selected_index = Some(index);
            }
            EditMode::None => {}
        }

        self.edit_mode = EditMode::None;
        self.editing_item = None;
    }

    /// 取消編輯
    pub fn cancel_edit(&mut self) {
        self.edit_mode = EditMode::None;
        self.editing_item = None;
    }

    /// 刪除項目
    pub fn delete_item(&mut self, index: usize) {
        // Fail Fast: 驗證索引
        if index >= self.items.len() {
            self.set_error(format!("無效的索引：{}", index));
            return;
        }

        let name = self.items[index].name().to_string();
        self.items.remove(index);
        self.set_success(format!("成功刪除{}：{}", T::type_name(), name));

        // 調整選中索引
        self.selected_index = None;
    }
}

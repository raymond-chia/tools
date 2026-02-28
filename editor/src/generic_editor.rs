//! 泛型編輯器狀態管理

use crate::constants::COPY_SUFFIX;
use crate::editor_item::EditorItem;

/// 編輯模式及項目狀態
#[derive(Debug, Clone, Default)]
pub enum EditMode<T> {
    /// 瀏覽模式
    #[default]
    None,
    /// 新增模式
    Creating(T),
    /// 編輯模式（索引、項目）
    Editing(usize, T),
}

/// 訊息狀態（成功或錯誤訊息）
#[derive(Debug, Default)]
pub struct MessageState {
    /// 最後的操作訊息（成功或錯誤）
    pub message: String,
    /// 訊息是否為錯誤
    pub is_error: bool,
    /// 訊息是否可見
    pub message_visible: bool,
}

impl MessageState {
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
}

/// 泛型編輯器狀態
#[derive(Debug, Default)]
pub struct GenericEditorState<T: EditorItem> {
    /// 訊息狀態
    pub message_state: MessageState,

    /// 項目列表
    pub items: Vec<T>,
    /// 項目列表的搜尋查詢文字
    pub search_query: String,
    /// 當前選中的項目索引
    pub selected_index: Option<usize>,

    /// 當前編輯模式（包含編輯中的項目）
    pub edit_mode: EditMode<T>,

    /// 編輯器 UI 狀態（搜尋、拖曳等）
    pub ui_state: T::UIState,
}

impl<T: EditorItem> GenericEditorState<T> {
    /// 開始新增項目
    pub fn start_creating(&mut self) {
        self.edit_mode = EditMode::Creating(T::default());
    }

    /// 開始編輯項目
    pub fn start_editing(&mut self, index: usize) {
        // Fail Fast: 驗證索引
        if index >= self.items.len() {
            self.message_state
                .set_error(format!("無效的索引：{}", index));
            return;
        }

        self.edit_mode = EditMode::Editing(index, self.items[index].clone());
    }

    /// 複製項目
    pub fn start_copying(&mut self, index: usize) {
        // Fail Fast: 驗證索引
        if index >= self.items.len() {
            self.message_state
                .set_error(format!("無效的索引：{}", index));
            return;
        }

        let mut copied = self.items[index].clone();
        copied.set_name(format!("{}{}", copied.name(), COPY_SUFFIX));
        self.edit_mode = EditMode::Creating(copied);
    }

    /// 確認編輯
    pub fn confirm_edit(&mut self) {
        // Fail Fast: 提取編輯模式和項目（直接取出所有權，避免 clone）
        let edit_mode = std::mem::take(&mut self.edit_mode);
        let (editing_index, item) = match edit_mode {
            EditMode::None => {
                self.message_state.set_error("目前不在編輯模式");
                return;
            }
            EditMode::Creating(item) => (None, item),
            EditMode::Editing(idx, item) => (Some(idx), item),
        };

        // 驗證項目
        if let Err(e) = item.validate(&self.items, editing_index) {
            self.message_state.set_error(e);
            // 復原 edit_mode（validate 失敗時保留編輯狀態）
            self.edit_mode = match editing_index {
                None => EditMode::Creating(item),
                Some(idx) => EditMode::Editing(idx, item),
            };
            return;
        }

        // 驗證通過後的鉤子（如排序、正規化等）
        let mut confirmed_item = item;
        confirmed_item.after_confirm();

        // 執行相應的操作
        match editing_index {
            None => {
                // Creating
                let name = confirmed_item.name().to_string();
                self.items.push(confirmed_item);
                self.message_state
                    .set_success(format!("成功新增{}：{}", T::type_name(), name));
                self.selected_index = Some(self.items.len() - 1);
            }
            Some(index) => {
                // Editing
                // Fail Fast: 驗證索引
                if index >= self.items.len() {
                    self.message_state
                        .set_error(format!("無效的索引：{}", index));
                    return;
                }

                let name = confirmed_item.name().to_string();
                self.items[index] = confirmed_item;
                self.message_state
                    .set_success(format!("成功編輯{}：{}", T::type_name(), name));
                self.selected_index = Some(index);
            }
        }
    }

    /// 取消編輯
    pub fn cancel_edit(&mut self) {
        self.edit_mode = EditMode::None;
    }

    /// 刪除項目
    pub fn delete_item(&mut self, index: usize) {
        // Fail Fast: 驗證索引
        if index >= self.items.len() {
            self.message_state
                .set_error(format!("無效的索引：{}", index));
            return;
        }

        let name = self.items[index].name().to_string();
        self.items.remove(index);
        self.message_state
            .set_success(format!("成功刪除{}：{}", T::type_name(), name));

        // 調整選中索引
        self.selected_index = None;
    }

    /// 判斷是否在編輯模式
    pub fn is_editing(&self) -> bool {
        !matches!(self.edit_mode, EditMode::None)
    }

    /// 移動項目（拖曳排序用）
    pub fn move_item(&mut self, from: usize, to: usize) {
        // Fail Fast: 驗證索引有效性
        if from == to {
            return;
        }
        if from >= self.items.len() || to >= self.items.len() {
            return;
        }

        let item = self.items.remove(from);
        self.items.insert(to, item);

        // 因為移動會影響索引
        // 正確追蹤選中目標的索引
        self.selected_index = match self.selected_index {
            Some(sel) if sel == from => Some(to),
            Some(sel) if from < sel && sel <= to => Some(sel - 1),
            Some(sel) if to <= sel && sel < from => Some(sel + 1),
            other => other,
        };
    }
}

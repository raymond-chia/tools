//! 泛型 TOML I/O 功能

use crate::editor_item::EditorItem;
use crate::generic_editor::GenericEditorState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// 泛型 TOML 資料容器
#[derive(Debug, Serialize, Deserialize)]
struct ItemsData<T> {
    #[serde(flatten)]
    pub items_map: HashMap<String, Vec<T>>,
}

impl<T> ItemsData<T> {
    /// 建立新容器（使用指定的 key）
    pub fn new(key: &str, items: Vec<T>) -> Self {
        let mut map = HashMap::new();
        map.insert(key.to_string(), items);
        Self { items_map: map }
    }

    /// 取得項目列表
    pub fn get_items(&self, key: &str) -> Option<&Vec<T>> {
        self.items_map.get(key)
    }
}

/// 載入檔案
pub fn load_file<T: EditorItem>(
    state: &mut GenericEditorState<T>,
    path: &Path,
    data_key: &str, // "objects", "skills" 等
) {
    // Fail Fast: 檢查是否正在編輯
    if state.is_editing() {
        state.message_state.set_error("請先完成或取消當前的編輯");
        return;
    }

    // 讀取檔案
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            state
                .message_state
                .set_error(format!("載入檔案失敗：{} - {}", path.display(), e));
            return;
        }
    };

    // TOML 反序列化
    let data: ItemsData<T> = match toml::from_str(&content) {
        Ok(data) => data,
        Err(e) => {
            state
                .message_state
                .set_error(format!("解析 TOML 失敗：{}", e));
            return;
        }
    };

    // 更新狀態
    match data.get_items(data_key) {
        Some(items) => {
            state.items = items.clone();
            state.selected_index = None;
            state.message_state.set_success(format!(
                "成功載入檔案：{}（共 {} 個{}）",
                path.display(),
                state.items.len(),
                T::type_name_plural()
            ));
        }
        None => {
            state
                .message_state
                .set_error(format!("TOML 檔案中找不到 key：{}", data_key));
        }
    }
}

/// 儲存檔案
pub fn save_file<T: EditorItem>(state: &mut GenericEditorState<T>, path: &Path, data_key: &str) {
    // Fail Fast: 檢查是否正在編輯
    if state.is_editing() {
        state.message_state.set_error("請先完成或取消當前的編輯");
        return;
    }

    // 建立資料結構
    let data = ItemsData::new(data_key, state.items.clone());

    // TOML 序列化
    let content = match toml::to_string_pretty(&data) {
        Ok(content) => content,
        Err(e) => {
            state
                .message_state
                .set_error(format!("序列化 TOML 失敗：{}", e));
            return;
        }
    };

    // 確保目錄存在
    if let Some(parent) = path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            state
                .message_state
                .set_error(format!("建立目錄失敗：{} - {}", parent.display(), e));
            return;
        }
    }

    // 寫入檔案
    match fs::write(path, content) {
        Ok(_) => {
            state.message_state.set_success(format!(
                "成功儲存檔案：{}（共 {} 個{}）",
                path.display(),
                state.items.len(),
                T::type_name_plural()
            ));
        }
        Err(e) => {
            state
                .message_state
                .set_error(format!("儲存檔案失敗：{} - {}", path.display(), e));
        }
    }
}

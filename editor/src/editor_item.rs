//! 編輯器項目的通用 trait 定義

use serde::{Deserialize, Serialize};

/// 所有可編輯項目必須實現的基本 trait
pub trait EditorItem:
    Clone + Default + std::fmt::Debug + Serialize + for<'de> Deserialize<'de>
{
    /// 該編輯器的 UI 狀態類型（如搜尋、篩選等）
    /// 不需要 UI 狀態的編輯器可使用 ()
    type UIState: Default;

    /// 取得項目名稱（用於列表顯示和驗證）
    fn name(&self) -> &str;

    /// 設定項目名稱（用於複製功能）
    fn set_name(&mut self, name: String);

    /// 項目類型名稱（用於訊息顯示，如「物件」、「技能」）
    fn type_name() -> &'static str;

    /// 項目類型名稱複數形式（用於訊息顯示，如「物件」、「技能」）
    fn type_name_plural() -> &'static str {
        Self::type_name()
    }

    /// 驗證項目（confirm_edit 時呼叫）
    /// 返回 Ok(()) 表示驗證通過，Err(String) 表示驗證失敗
    fn validate(&self, all_items: &[Self], editing_index: Option<usize>) -> Result<(), String> {
        validate_name(self, all_items, editing_index)
    }

    /// 編輯確認後的鉤子（驗證通過後呼叫）
    /// 用於進行排序、正規化等操作
    fn after_confirm(&mut self) {}
}

/// 驗證項目名稱的輔助函數（用於檢查名稱非空和重複）
pub fn validate_name<T: EditorItem>(
    item: &T,
    all_items: &[T],
    editing_index: Option<usize>,
) -> Result<(), String> {
    if item.name().trim().is_empty() {
        return Err("名稱不能為空".to_string());
    }

    for (idx, existing_item) in all_items.iter().enumerate() {
        if let Some(edit_idx) = editing_index {
            if idx == edit_idx {
                continue;
            }
        }
        if existing_item.name() == item.name() {
            return Err(format!(
                "{}「{}」已存在，請使用不同的名稱",
                T::type_name(),
                item.name()
            ));
        }
    }

    Ok(())
}

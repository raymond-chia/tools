//! 搜尋輔助函數

use board::domain::alias::TypeName;

/// 渲染搜尋輸入框
pub fn render_search_input(ui: &mut egui::Ui, query: &mut String) -> egui::Response {
    ui.horizontal(|ui| {
        ui.label("搜尋：");
        ui.text_edit_singleline(query)
    })
    .inner
}

pub fn match_search_query(item: &str, query_lower: &str) -> bool {
    query_lower.is_empty() || item.to_lowercase().contains(query_lower)
}

/// 根據搜尋查詢過濾列表（支援任何可轉換為字串的類型）
pub fn filter_by_search<'a, T: AsRef<str>>(items: &'a [T], query: &str) -> Vec<&'a T> {
    let query_lower = query.to_lowercase();
    items
        .iter()
        .filter(|item| match_search_query(item.as_ref(), &query_lower))
        .collect()
}

/// 建立 ComboBox，將選項數量編入 id 以繞過 egui popup 高度快取問題
/// 參見 https://github.com/emilk/egui/issues/5225
pub fn combobox_with_dynamic_height(
    id_salt: &str,
    selected_text: &str,
    option_count: usize,
) -> egui::ComboBox {
    egui::ComboBox::from_id_salt(format!("{}_{}", id_salt, option_count))
        .selected_text(selected_text)
}

/// 在 ComboBox 中渲染過濾後的選項
/// 被過濾掉的選項以不可選的空白佔位取代，防止 egui popup 快取到較小高度後無法恢復
pub fn render_filtered_options(
    ui: &mut egui::Ui,
    visible_items: &[&TypeName],
    hidden_count: usize,
    selected_value: &mut String,
    query: &str,
) {
    if !query.is_empty() && visible_items.is_empty() {
        ui.label("找不到符合的項目");
    } else {
        for item_name in visible_items {
            ui.selectable_value(selected_value, item_name.to_string(), item_name.as_str());
        }
    }
    for _ in 0..hidden_count {
        ui.add_enabled(false, egui::Button::selectable(false, ""));
    }
}

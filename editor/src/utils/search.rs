//! 搜尋輔助函數

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

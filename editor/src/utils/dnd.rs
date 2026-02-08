//! DnD（拖放）通用工具

use egui::Id;

/// 渲染可拖曳項目的手柄，並自動處理 DnD 邏輯
///
/// # 參數
/// - `ui`: egui UI 上下文
/// - `item_id`: 項目的唯一 ID（用於 DnD，例如 `egui::Id::new("item_drag").with(index)`）
/// - `index`: 項目的索引
/// - `label`: 拖曳手柄顯示的文字（例如 "☰" 或 "⋮⋮"）
///
/// # 回傳值
/// 如果拖曳完成，返回 `Some((from_index, to_index))`；否則返回 `None`
pub fn render_dnd_handle(
    ui: &mut egui::Ui,
    item_id: Id,
    index: usize,
    label: &str,
) -> Option<(usize, usize)> {
    let drag_response = ui.dnd_drag_source(item_id, index, |ui| {
        ui.label(label);
    });

    // 檢查懸停 + 自動繪製指示線
    if let Some(dragged_idx) = drag_response.response.dnd_hover_payload::<usize>() {
        draw_dnd_indicator(ui, *dragged_idx, index);
    }

    // 檢查釋放 + 回傳結果
    drag_response
        .response
        .dnd_release_payload::<usize>()
        .map(|dragged_idx| (*dragged_idx, index))
}

/// 繪製拖曳指示線
pub(crate) fn draw_dnd_indicator(ui: &egui::Ui, dragged_idx: usize, target_idx: usize) {
    let stroke = egui::Stroke::new(2.0, egui::Color32::WHITE);
    let space_rect = ui.available_rect_before_wrap();
    let insert_y = if dragged_idx < target_idx {
        space_rect.bottom()
    } else {
        space_rect.top()
    };
    ui.painter().hline(space_rect.x_range(), insert_y, stroke);
}

//! 關卡編輯器的戰鬥模式邏輯

use super::battlefield::{self, Snapshot};
use super::{LevelTabMode, LevelTabUIState, MessageState};
use crate::constants::*;
use board::ecs_types::components::Position;
use board::error::Result as CResult;

/// 渲染戰鬥模式表單
pub fn render_form(
    ui: &mut egui::Ui,
    ui_state: &mut LevelTabUIState,
    message_state: &mut MessageState,
) {
    let snapshot = match battlefield::query_snapshot(&mut ui_state.world) {
        Ok(s) => s,
        Err(e) => {
            message_state.set_error(format!("讀取關卡資料失敗：{}", e));
            return;
        }
    };

    // 頂部：返回按鈕
    if ui.button("← 返回部署").clicked() {
        ui_state.mode = LevelTabMode::Deploy;
        return;
    }

    ui.add_space(SPACING_SMALL);

    render_level_info(ui, &snapshot);

    ui.add_space(SPACING_MEDIUM);
    ui.separator();

    // 主要佈局：戰場 + 右側詳情面板
    let mut errors = vec![];
    let height = ui.available_height();
    ui.horizontal(|ui| {
        // 預先計算右側面板寬度
        let right_panel_width = if ui_state.selected_right_pos.is_some() {
            LIST_PANEL_WIDTH + SPACING_SMALL // 面板寬度 + scroll bar
        } else {
            0.0
        };
        let center_panel_width = ui.available_width() - right_panel_width;

        // 中間：戰場預覽
        ui.vertical(|ui| {
            ui.set_height(height);
            ui.set_width(center_panel_width);
            if let Err(e) = render_battlefield(ui, &snapshot, ui_state) {
                errors.push(format!("渲染戰場失敗：{}", e));
            }
        });

        // 右側：單位詳情面板（條件顯示）
        if let Some(pos) = ui_state.selected_right_pos {
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("battle_details_panel")
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(LIST_PANEL_WIDTH);
                        battlefield::render_details_panel(ui, pos, &snapshot);
                    });
                });
        }
    });
    if !errors.is_empty() {
        message_state.set_error(errors.join("\n"));
    }
}

/// 渲染關卡資訊（不含玩家部署數/上限）
fn render_level_info(ui: &mut egui::Ui, snapshot: &Snapshot) {
    let enemy_count = battlefield::enemy_units(snapshot).count();

    egui::Grid::new("battle_level_info_grid")
        .spacing([SPACING_MEDIUM, SPACING_MEDIUM])
        .show(ui, |ui| {
            ui.label(format!("關卡名稱：{}", snapshot.level_config.name));
            ui.separator();
            ui.label(format!(
                "尺寸：{}×{}",
                snapshot.board.width, snapshot.board.height
            ));
            ui.separator();
            ui.label(format!("敵人數量：{}", enemy_count));
        });
}

/// 渲染戰場預覽
fn render_battlefield(
    ui: &mut egui::Ui,
    snapshot: &Snapshot,
    ui_state: &mut LevelTabUIState,
) -> CResult<()> {
    let board = snapshot.board;
    let scroll_output = egui::ScrollArea::both()
        .auto_shrink([false; 2])
        .id_salt("battle_battlefield")
        .show(ui, |ui| {
            let total_size = battlefield::calculate_grid_dimensions(board);
            let (rect, response) = ui.allocate_exact_size(total_size, egui::Sense::click());

            let hovered_pos = battlefield::compute_hover_pos(&response, rect, board);

            // 渲染網格
            let get_cell_info_fn = battlefield::get_cell_info(snapshot);
            let is_highlight_fn = battlefield::is_highlight(None);
            battlefield::render_grid(
                ui,
                rect,
                board,
                ui_state.scroll_offset,
                get_cell_info_fn,
                is_highlight_fn,
            );
            if let Some(hovered_pos) = hovered_pos {
                handle_mouse_click(&response, hovered_pos, snapshot, ui_state);
                let get_tooltip_info_fn = battlefield::get_tooltip_info(snapshot);
                battlefield::render_hover_tooltip(ui, rect, hovered_pos, get_tooltip_info_fn);
            }

            ui.add_space(SPACING_SMALL);
            battlefield::render_battlefield_legend(ui);
        });
    // 儲存滾動位置
    ui_state.scroll_offset = scroll_output.state.offset;

    Ok(())
}

/// 處理棋盤點擊事件（戰鬥模式）
/// 右鍵：選擇有單位或物件的位置顯示詳情
fn handle_mouse_click(
    response: &egui::Response,
    clicked_pos: Position,
    snapshot: &Snapshot,
    ui_state: &mut LevelTabUIState,
) {
    if response.secondary_clicked() {
        if snapshot.unit_map.contains_key(&clicked_pos)
            || snapshot.object_map.contains_key(&clicked_pos)
        {
            ui_state.selected_right_pos = Some(clicked_pos);
        } else {
            ui_state.selected_right_pos = None;
        }
    }
}

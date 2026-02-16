//! 關卡編輯器的戰鬥模式邏輯

use super::grid::render_simulation_grid;
use super::unit_details::{handle_unit_right_click, render_unit_details_side_panel};
use super::{
    LevelTabMode, LevelTabUIState, calculate_grid_dimensions, calculate_visible_range,
    prepare_lookup_maps, render_battlefield_legend, render_hover_tooltip,
};
use crate::constants::*;
use board::loader_schema::LevelType;

/// 渲染戰鬥模式的表單（只顯示戰場和 hover tip）
pub fn render_battle_form(ui: &mut egui::Ui, level: &LevelType, ui_state: &mut LevelTabUIState) {
    // 頂部：返回按鈕
    if ui.button("← 返回部署").clicked() {
        ui_state.mode = LevelTabMode::Deploy;
    }

    ui.add_space(SPACING_SMALL);

    // 預先計算面板寬度（如果面板存在）
    let panel_width = if ui_state.temp_unit_name.is_some() {
        LIST_PANEL_WIDTH + SPACING_SMALL // 面板寬度 + 分隔符
    } else {
        0.0
    };
    let height = ui.available_height();

    // 水平分割佈局：戰場 + 單位詳情面板
    ui.horizontal(|ui| {
        // 左側：戰場（使用剩餘空間）
        ui.vertical(|ui| {
            ui.set_height(height);
            ui.set_max_width(ui.available_width() - panel_width);

            let scroll_output = egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .id_salt("battle_battlefield")
                .show(ui, |ui| {
                    let (total_width, total_height) = calculate_grid_dimensions(level);

                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(total_width, total_height),
                        egui::Sense::click(),
                    );

                    // 計算可見範圍
                    let viewport_size = ui.clip_rect().size();
                    let visible_range =
                        calculate_visible_range(ui_state.scroll_offset, viewport_size, level);

                    // 建立查詢表
                    let (player_positions, enemy_units_map, objects_map) =
                        prepare_lookup_maps(level);

                    // 渲染網格
                    render_simulation_grid(
                        ui,
                        rect,
                        level,
                        &player_positions,
                        &enemy_units_map,
                        &objects_map,
                        &ui_state.simulation_state,
                        visible_range,
                        &ui_state.skills_map,
                        &ui_state.units_map,
                    );

                    // 渲染 hover tooltip
                    render_hover_tooltip(
                        ui,
                        level,
                        rect,
                        &response,
                        &player_positions,
                        &enemy_units_map,
                        &objects_map,
                    );

                    // 處理右鍵點擊選擇單位
                    handle_unit_right_click(
                        &response,
                        rect,
                        level,
                        &player_positions,
                        &enemy_units_map,
                        ui_state,
                    );
                });

            // 儲存滾動位置
            ui_state.scroll_offset = scroll_output.state.offset;
        });

        // 右側：單位詳情面板（條件顯示）
        if let Some(unit_name) = &ui_state.temp_unit_name.clone() {
            ui.separator();
            render_unit_details_side_panel(ui, unit_name, ui_state);
        }
    });

    ui.add_space(SPACING_SMALL);
    render_battlefield_legend(ui);
}

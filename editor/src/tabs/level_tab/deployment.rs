//! 關卡編輯器的模擬戰鬥模式邏輯

use super::{
    LevelTabMode, LevelTabUIState, calculate_grid_dimensions, calculate_visible_range, grid,
    prepare_lookup_maps, render_battlefield_legend, render_hover_tooltip, screen_to_board_pos,
    unit_details::{handle_unit_right_click, render_unit_details_side_panel},
};
use crate::constants::*;
use crate::utils::search::{filter_by_search, render_search_input};
use board::component::Position;
use board::loader_schema::LevelType;

/// 渲染單位部署模式的表單
pub fn render_deployment_form(
    ui: &mut egui::Ui,
    level: &LevelType,
    ui_state: &mut LevelTabUIState,
) {
    // 頂部：按鈕區
    ui.horizontal(|ui| {
        if ui.button("← 返回編輯").clicked() {
            ui_state.mode = LevelTabMode::Edit;
        }

        ui.separator();

        // 檢查是否至少部署了 1 個單位
        let can_battle = !ui_state.simulation_state.deployed_units.is_empty();
        if ui
            .add_enabled(can_battle, egui::Button::new("開始戰鬥"))
            .clicked()
        {
            ui_state.mode = LevelTabMode::Battle;
        }
    });

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .id_salt("all")
        .show(ui, |ui| {
            // 關卡資訊顯示
            render_level_info(ui, level, ui_state);

            ui.add_space(SPACING_MEDIUM);
            ui.separator();

            let height = ui.available_height();

            // 主要內容區：分左右兩欄
            ui.horizontal(|ui| {
                // 左欄：玩家部署面板
                ui.vertical(|ui| {
                    ui.set_height(height);
                    ui.set_width(LIST_PANEL_WIDTH);
                    render_player_deployment_panel(ui, level, ui_state);
                });

                ui.separator();

                // 右欄：戰場預覽
                ui.vertical(|ui| {
                    render_battlefield_simulation_preview(ui, level, ui_state);
                });
            });
        });
}

/// 渲染關卡資訊顯示
fn render_level_info(ui: &mut egui::Ui, level: &LevelType, ui_state: &mut LevelTabUIState) {
    egui::Grid::new("level_info_grid")
        .spacing([SPACING_MEDIUM, SPACING_MEDIUM])
        .show(ui, |ui| {
            ui.label(format!("關卡名稱：{}", level.name));
            ui.separator();
            ui.label(format!(
                "尺寸：{}×{}",
                level.board_width, level.board_height
            ));

            ui.end_row();

            let deployed_count = ui_state.simulation_state.deployed_units.len();
            ui.label(format!(
                "玩家部署：{} / {}",
                deployed_count, level.max_player_units
            ));
            ui.separator();
            ui.label(format!("敵人數量：{}", level.enemy_units.len()));
        });
}

/// 渲染玩家部署面板（左側列表）
fn render_player_deployment_panel(
    ui: &mut egui::Ui,
    level: &LevelType,
    ui_state: &mut LevelTabUIState,
) {
    ui.heading("玩家部署");

    // Fail fast：檢查是否有可用單位
    if ui_state.available_units.is_empty() {
        ui.label("⚠️ 尚未定義任何單位，無法部署");
        return;
    }

    ui.add_space(SPACING_MEDIUM);
    ui.separator();

    // 渲染部署點列表
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .id_salt("deployment_list")
        .show(ui, |ui| {
            for (index, pos) in level.player_placement_positions.iter().enumerate() {
                render_deployment_slot(ui, index, *pos, level, ui_state);
            }
        });
}

/// 渲染單個部署槽（部署點）
fn render_deployment_slot(
    ui: &mut egui::Ui,
    index: usize,
    pos: Position,
    level: &LevelType,
    ui_state: &mut LevelTabUIState,
) {
    // 先取得已部署單位的名稱副本
    let deployed_unit_name = ui_state
        .simulation_state
        .deployed_units
        .get(&index)
        .cloned();
    let is_selected = ui_state.simulation_state.selected_deployment_point == Some(index);

    let mut clear_clicked = false;
    let mut select_clicked = false;

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(format!("#{} ({}, {})", index + 1, pos.x, pos.y));

            // 已部署：顯示單位名稱 + 清除按鈕
            match deployed_unit_name {
                Some(unit_name) => {
                    ui.label(format!("✓ {}", unit_name));
                    if ui.button("清除").clicked() {
                        clear_clicked = true;
                    }
                }
                None => {
                    // 未部署：顯示選擇按鈕
                    if ui.button("選擇單位").clicked() {
                        select_clicked = true;
                    }
                }
            }
        });

        // 如果選中這個部署點，顯示 ComboBox
        if is_selected {
            render_unit_combobox(ui, index, ui_state, level);
        }
    });

    // 在閉包外處理狀態更新
    if clear_clicked {
        ui_state.simulation_state.deployed_units.remove(&index);
        ui_state.simulation_state.selected_deployment_point = None;
    }
    if select_clicked {
        ui_state.simulation_state.selected_deployment_point = Some(index);
    }

    ui.add_space(SPACING_SMALL);
}

/// 渲染單位選擇 ComboBox（集成搜尋）
fn render_unit_combobox(
    ui: &mut egui::Ui,
    index: usize,
    ui_state: &mut LevelTabUIState,
    level: &LevelType,
) {
    let mut selected_value = ui_state
        .simulation_state
        .deployed_units
        .get(&index)
        .cloned()
        .unwrap_or_default();

    egui::ComboBox::from_id_salt(format!("player_unit_selector_{}", index))
        .selected_text(if selected_value.is_empty() {
            "選擇單位"
        } else {
            &selected_value
        })
        .height(COMBOBOX_MIN_HEIGHT)
        .show_ui(ui, |ui| {
            ui.set_min_width(COMBOBOX_MIN_WIDTH);

            // 搜尋輸入框（使用全域搜尋字串）
            let response = render_search_input(ui, &mut ui_state.unit_search_query);
            ui.memory_mut(|mem| mem.request_focus(response.id));
            ui.separator();

            // 過濾選項
            let visible_units =
                filter_by_search(&ui_state.available_units, &ui_state.unit_search_query);
            for unit_name in visible_units {
                ui.selectable_value(&mut selected_value, unit_name.clone(), unit_name);
            }
        });

    // 如果選擇了單位，更新部署狀態
    if !selected_value.is_empty() {
        // 檢查是否達到上限
        let deployed_count = ui_state.simulation_state.deployed_units.len();
        let can_deploy = ui_state
            .simulation_state
            .deployed_units
            .contains_key(&index)
            || (deployed_count as u32) < level.max_player_units;

        if can_deploy {
            ui_state
                .simulation_state
                .deployed_units
                .insert(index, selected_value);
            ui_state.simulation_state.selected_deployment_point = None;
        }
    }
}

/// 渲染戰場預覽（模擬戰鬥模式）
fn render_battlefield_simulation_preview(
    ui: &mut egui::Ui,
    level: &LevelType,
    ui_state: &mut LevelTabUIState,
) {
    ui.heading("戰場預覽");

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
                .id_salt("simulation_battlefield")
                .show(ui, |ui| {
                    let (total_width, total_height) = calculate_grid_dimensions(level);

                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(total_width, total_height),
                        egui::Sense::click(),
                    );

                    // 處理點擊事件（選擇部署點）
                    if response.clicked() {
                        if let Some(clicked_pos) = response
                            .interact_pointer_pos()
                            .and_then(|p| screen_to_board_pos(p, rect, level))
                        {
                            handle_deployment_point_click(clicked_pos, level, ui_state);
                        }
                    }

                    // 計算可見範圍
                    let viewport_size = ui.clip_rect().size();
                    let visible_range =
                        calculate_visible_range(ui_state.scroll_offset, viewport_size, level);

                    // 建立查詢表
                    let (player_positions, enemy_units_map, objects_map) =
                        prepare_lookup_maps(level);

                    // 渲染網格（模擬模式專用）
                    grid::render_simulation_grid(
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

/// 處理棋盤上的部署點點擊事件
fn handle_deployment_point_click(
    clicked_pos: Position,
    level: &LevelType,
    ui_state: &mut LevelTabUIState,
) {
    // Fail fast：檢查點擊位置是否為玩家部署點
    let deployment_index = level
        .player_placement_positions
        .iter()
        .position(|pos| *pos == clicked_pos);

    match deployment_index {
        Some(index) => {
            ui_state.simulation_state.selected_deployment_point = Some(index);
        }
        None => {
            // 點擊非部署點，取消選擇
            ui_state.simulation_state.selected_deployment_point = None;
        }
    }
}

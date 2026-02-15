//! 關卡編輯器的模擬戰鬥模式邏輯

use super::{
    LevelTabMode, LevelTabUIState, SimulationState, VisibleGridRange, calculate_grid_dimensions,
    calculate_visible_range, prepare_lookup_maps, render_battlefield_legend, render_hover_tooltip,
    screen_to_board_pos,
};
use crate::constants::*;
use crate::utils::search::{filter_by_search, render_search_input};
use board::alias::TypeName;
use board::component::Position;
use board::loader_schema::{LevelType, ObjectPlacement, UnitPlacement};
use std::collections::{HashMap, HashSet};

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
        .show(ui, |ui| {
            // 關卡資訊顯示
            render_level_info(ui, level, ui_state);

            ui.add_space(SPACING_MEDIUM);
            ui.separator();

            // 主要內容區：分左右兩欄
            ui.horizontal(|ui| {
                // 左欄：玩家部署面板
                ui.vertical(|ui| {
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
        .min_scrolled_height(LIST_PANEL_MIN_HEIGHT)
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

    let scroll_output = egui::ScrollArea::both()
        .auto_shrink([false; 2])
        .max_width(ui.available_width() - SPACING_MEDIUM)
        .min_scrolled_height(LIST_PANEL_MIN_HEIGHT)
        .id_salt("simulation_battlefield")
        .show(ui, |ui| {
            let (total_width, total_height) = calculate_grid_dimensions(level);

            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(total_width, total_height), egui::Sense::click());

            // 處理點擊事件
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
            let (player_positions, enemy_units_map, objects_map) = prepare_lookup_maps(level);

            // 渲染網格（模擬模式專用）
            render_simulation_grid(
                ui,
                rect,
                level,
                &player_positions,
                &enemy_units_map,
                &objects_map,
                &ui_state.simulation_state,
                visible_range,
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
        });

    // 儲存滾動位置
    ui_state.scroll_offset = scroll_output.state.offset;

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

/// 渲染模擬戰鬥的棋盤網格（與編輯模式不同的視覺反饋）
pub fn render_simulation_grid(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    level: &LevelType,
    player_positions: &HashSet<Position>,
    enemy_units_map: &HashMap<Position, &UnitPlacement>,
    objects_map: &HashMap<Position, &ObjectPlacement>,
    simulation_state: &SimulationState,
    visible_range: VisibleGridRange,
) {
    let painter = ui.painter();
    for y in visible_range.min.y..visible_range.max.y {
        for x in visible_range.min.x..visible_range.max.x {
            let pos = Position { x, y };

            // 計算格子位置
            let cell_x = rect.min.x + x as f32 * (BATTLEFIELD_CELL_SIZE + BATTLEFIELD_GRID_SPACING);
            let cell_y = rect.min.y + y as f32 * (BATTLEFIELD_CELL_SIZE + BATTLEFIELD_GRID_SPACING);
            let cell_rect = egui::Rect::from_min_size(
                egui::pos2(cell_x, cell_y),
                egui::vec2(BATTLEFIELD_CELL_SIZE, BATTLEFIELD_CELL_SIZE),
            );

            // 判斷是否為玩家部署點
            let deployment_index = player_positions
                .contains(&pos)
                .then(|| {
                    level
                        .player_placement_positions
                        .iter()
                        .position(|p| *p == pos)
                })
                .flatten();

            // 決定格子內容與背景顏色
            let (cell_text, bg_color) = match deployment_index {
                Some(index) => {
                    // 玩家部署點：根據部署狀態顯示
                    match simulation_state.deployed_units.get(&index) {
                        Some(unit_name) => {
                            let abbrev = unit_name.chars().take(2).collect::<TypeName>();
                            (abbrev, BATTLEFIELD_COLOR_PLAYER)
                        }
                        None => ("".to_string(), BATTLEFIELD_COLOR_PLAYER),
                    }
                }
                None => {
                    // 非部署點：顯示敵人或物件
                    if let Some(unit) = enemy_units_map.get(&pos) {
                        let abbrev = unit.unit_type_name.chars().take(2).collect::<TypeName>();
                        (abbrev, BATTLEFIELD_COLOR_ENEMY)
                    } else if let Some(obj) = objects_map.get(&pos) {
                        let abbrev = obj.object_type_name.chars().take(2).collect::<TypeName>();
                        (abbrev, BATTLEFIELD_COLOR_OBJECT)
                    } else {
                        ("".to_string(), BATTLEFIELD_COLOR_EMPTY)
                    }
                }
            };

            // 繪製格子背景
            painter.rect_filled(cell_rect, 0.0, bg_color);

            // 繪製文本
            painter.text(
                cell_rect.center(),
                egui::Align2::CENTER_CENTER,
                cell_text,
                egui::FontId::proportional(BATTLEFIELD_TEXT_SIZE),
                egui::Color32::BLACK,
            );

            // 選中高亮：綠色外邊框
            if simulation_state.selected_deployment_point == deployment_index {
                painter.rect_stroke(
                    cell_rect,
                    0.0,
                    egui::Stroke::new(BATTLEFIELD_STROKE_WIDTH, BATTLEFIELD_COLOR_PLAYER_SELECTED),
                    egui::epaint::StrokeKind::Outside,
                );
            }
        }
    }
}

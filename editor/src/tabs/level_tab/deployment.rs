//! 關卡編輯器的單位部署模式邏輯

use super::battlefield::{self, Snapshot};
use super::{LevelTabMode, LevelTabUIState, MessageState};
use crate::constants::*;
use crate::utils::search::{
    combobox_with_dynamic_height, filter_by_search, render_filtered_options, render_search_input,
};
use board::ecs_types::components::Position;
use board::error::Result as CResult;

/// 渲染單位部署模式表單
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
    let deployed_count = deployed_positions(&snapshot).count();

    // 頂部：按鈕區
    render_top_bar(ui, deployed_count, ui_state);
    match ui_state.mode {
        LevelTabMode::Deploy => {}
        _ => return, // 模式已切換，提前返回
    }

    ui.add_space(SPACING_SMALL);

    render_level_info(ui, &snapshot, deployed_count);

    ui.add_space(SPACING_MEDIUM);
    ui.separator();

    // 主要內容區：分左中右三部分
    let mut errors = vec![];
    let height = ui.available_height();
    ui.horizontal(|ui| {
        // 左側：玩家部署面板
        ui.vertical(|ui| {
            ui.set_height(height);
            ui.set_width(LIST_PANEL_WIDTH);
            if let Err(e) = render_player_deployment_panel(ui, &snapshot, ui_state) {
                errors.extend(e);
            }
        });

        ui.separator();

        // 中間：戰場預覽
        // 預先計算單位詳細資訊寬度（如果存在）
        let right_panel_width = if ui_state.selected_right_pos.is_some() {
            LIST_PANEL_WIDTH + SPACING_SMALL // 面板寬度 + scroll bar
        } else {
            0.0
        };
        let center_panel_width = ui.available_width() - right_panel_width;
        ui.vertical(|ui| {
            ui.set_width(center_panel_width);
            if let Err(e) = render_battlefield(ui, &snapshot, ui_state) {
                errors.push(format!("渲染戰場失敗：{}", e));
            }
        });

        // 右側：單位詳情面板（條件顯示）
        if let Some(pos) = ui_state.selected_right_pos {
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("details_panel")
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

fn render_top_bar(ui: &mut egui::Ui, deployed_count: usize, ui_state: &mut LevelTabUIState) {
    ui.horizontal(|ui| {
        if ui.button("← 返回編輯").clicked() {
            ui_state.mode = LevelTabMode::Edit;
            return;
        }

        ui.separator();

        let has_deployed = deployed_count > 0;
        if ui
            .add_enabled(has_deployed, egui::Button::new("開始戰鬥"))
            .clicked()
        {
            ui_state.mode = LevelTabMode::Battle;
            return;
        }
    });
}

/// 渲染關卡資訊顯示
fn render_level_info(ui: &mut egui::Ui, snapshot: &Snapshot, deployed_count: usize) {
    let enemy_count = battlefield::enemy_units(snapshot).count();

    egui::Grid::new("level_info_grid")
        .spacing([SPACING_MEDIUM, SPACING_MEDIUM])
        .show(ui, |ui| {
            ui.label(format!("關卡名稱：{}", snapshot.level_config.name));
            ui.separator();
            ui.label(format!(
                "尺寸：{}×{}",
                snapshot.board.width, snapshot.board.height
            ));

            ui.end_row();

            ui.label(format!(
                "玩家部署：{} / {}",
                deployed_count, snapshot.max_player_units
            ));
            ui.separator();
            ui.label(format!("敵人數量：{}", enemy_count));
        });
}

/// 渲染玩家部署面板（左側列表）
fn render_player_deployment_panel(
    ui: &mut egui::Ui,
    snapshot: &Snapshot,
    ui_state: &mut LevelTabUIState,
) -> Result<(), Vec<String>> {
    ui.heading("玩家部署");

    ui.add_space(SPACING_MEDIUM);
    ui.separator();

    // Fail fast：檢查是否有可用單位
    if ui_state.available_units.is_empty() {
        ui.label("⚠️ 尚未定義任何單位，無法部署");
        return Ok(());
    }

    // 渲染部署資訊
    let mut errors = vec![];
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .id_salt("deployment_list")
        .show(ui, |ui| {
            // ComboBox 區塊：選中部署點時才顯示
            let selected_deploy_pos = ui_state
                .selected_left_pos
                .filter(|pos| snapshot.deployment_set.contains(pos));

            if let Some(pos) = selected_deploy_pos {
                let deployed_name = snapshot
                    .unit_map
                    .get(&pos)
                    .map(|bundle| bundle.occupant_type_name.0.clone());

                ui.group(|ui| {
                    ui.label(format!("部署點 ({}, {})", pos.x, pos.y));
                    if let Err(e) = render_unit_combobox(ui, pos, &deployed_name, ui_state) {
                        errors.push(format!("部署點 ({}, {}) 錯誤：{}", pos.x, pos.y, e));
                    }
                });

                ui.add_space(SPACING_MEDIUM);
                ui.separator();
            } else {
                ui.label("請在地圖上點擊部署點");
                ui.add_space(SPACING_MEDIUM);
                ui.separator();
            }

            ui.add_space(SPACING_SMALL);

            // 已部署單位列表（一直顯示）
            ui.label("已部署單位：");
            let mut has_deployed = false;
            for pos in &snapshot.deployment_positions {
                if let Some(bundle) = snapshot.unit_map.get(pos) {
                    has_deployed = true;
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "({}, {}) — {}",
                            pos.x, pos.y, bundle.occupant_type_name.0
                        ));
                    });
                }
            }
            if !has_deployed {
                ui.label("尚未部署任何單位");
            }
        });

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// 渲染單位選擇 ComboBox（集成搜尋），選擇後直接部署
fn render_unit_combobox(
    ui: &mut egui::Ui,
    pos: Position,
    deployed_name: &Option<String>,
    ui_state: &mut LevelTabUIState,
) -> CResult<()> {
    let mut selected_value = String::new();

    let unit_names: Vec<String> = ui_state
        .available_units
        .iter()
        .map(|u| u.name.clone())
        .collect();

    let display_text = match deployed_name {
        Some(name) => name.as_str(),
        None => "選擇單位",
    };

    combobox_with_dynamic_height(
        &format!("player_unit_selector_{}_{}", pos.x, pos.y),
        display_text,
        unit_names.len(),
    )
    .show_ui(ui, |ui| {
        // 搜尋輸入框（使用全域搜尋字串）
        let response = render_search_input(ui, &mut ui_state.unit_search_query);
        ui.memory_mut(|mem| mem.request_focus(response.id));
        ui.separator();

        // 已部署時，加「清除」選項
        if deployed_name.is_some() {
            ui.selectable_value(&mut selected_value, CLEAR_LABEL.to_string(), CLEAR_LABEL);
            ui.separator();
        }

        // 過濾選項
        let visible_units = filter_by_search(&unit_names, &ui_state.unit_search_query);
        render_filtered_options(
            ui,
            &visible_units,
            // 預留清除的高度
            unit_names.len() + 1 - visible_units.len(),
            &mut selected_value,
            &ui_state.unit_search_query,
        );
    });

    if selected_value == CLEAR_LABEL {
        board::ecs_logic::deployment::undeploy_unit(&mut ui_state.world, pos)?;
    } else if !selected_value.is_empty() {
        board::ecs_logic::deployment::deploy_unit(&mut ui_state.world, &selected_value, pos)?;
    }
    Ok(())
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
        .id_salt("battlefield")
        .show(ui, |ui| {
            let total_size = battlefield::calculate_grid_dimensions(board);
            let (rect, response) = ui.allocate_exact_size(total_size, egui::Sense::click());

            let hovered_pos = battlefield::compute_hover_pos(&response, rect, board);

            // 渲染網格
            let get_cell_info_fn = battlefield::get_cell_info(snapshot);
            let is_highlight_fn = battlefield::is_highlight(ui_state.selected_left_pos);
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

/// 處理棋盤點擊事件
/// 左鍵：點擊部署點則選擇，否則取消選擇
/// 右鍵：點擊有單位的位置則顯示詳情，否則取消選擇
fn handle_mouse_click(
    response: &egui::Response,
    clicked_pos: Position,
    snapshot: &Snapshot,
    ui_state: &mut LevelTabUIState,
) {
    if response.clicked() {
        // 左鍵：選擇部署點
        if snapshot.deployment_set.contains(&clicked_pos) {
            ui_state.selected_left_pos = Some(clicked_pos);
        } else {
            ui_state.selected_left_pos = None;
        }
    }
    if response.secondary_clicked() {
        // 右鍵：選擇有單位或物件的位置
        if snapshot.unit_map.contains_key(&clicked_pos)
            || snapshot.object_map.contains_key(&clicked_pos)
        {
            ui_state.selected_right_pos = Some(clicked_pos);
        } else {
            ui_state.selected_right_pos = None;
        }
    }
}

/// 取得部署點上的已部署單位位置
fn deployed_positions(snapshot: &Snapshot) -> impl Iterator<Item = &Position> {
    snapshot
        .unit_map
        .keys()
        .filter(|pos| snapshot.deployment_set.contains(pos))
}

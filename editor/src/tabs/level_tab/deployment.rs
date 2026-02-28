//! 關卡編輯器的單位部署模式邏輯

use super::battlefield::{self, Snapshot};
use super::{LevelTabMode, LevelTabUIState, MessageState};
use crate::constants::*;
use crate::utils::search::{filter_by_search, render_search_input};
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

    // 渲染部署點列表
    let mut errors = vec![];
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .id_salt("deployment_list")
        .show(ui, |ui| {
            for (index, pos) in snapshot.deployment_positions.iter().enumerate() {
                if let Err(e) = render_deployment_slot(ui, index, *pos, snapshot, ui_state) {
                    errors.push(format!("部署槽 #{} 錯誤：{}", index + 1, e));
                }
            }
        });

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// 渲染單個部署槽（部署點）
fn render_deployment_slot(
    ui: &mut egui::Ui,
    index: usize,
    pos: Position,
    snapshot: &Snapshot,
    ui_state: &mut LevelTabUIState,
) -> CResult<()> {
    let deployed_unit_name = snapshot
        .unit_map
        .get(&pos)
        .map(|bundle| bundle.occupant_type_name.0.clone());
    let is_selected = ui_state.selected_left_pos == Some(pos);

    let mut clear_clicked = false;
    let mut select_clicked = false;
    let mut combobox_error: Option<board::error::Error> = None;

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(format!("#{} ({}, {})", index + 1, pos.x, pos.y));

            // 已部署：顯示單位名稱 + 清除按鈕
            match &deployed_unit_name {
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
            if let Err(e) = render_unit_combobox(ui, index, pos, ui_state) {
                combobox_error = Some(e);
            }
        }
    });

    // 在閉包外處理狀態更新
    if let Some(e) = combobox_error {
        return Err(e);
    }
    if clear_clicked {
        board::ecs_logic::deployment::undeploy_unit(&mut ui_state.world, pos)?;
    }
    if select_clicked {
        ui_state.selected_left_pos = Some(pos);
    }

    ui.add_space(SPACING_SMALL);
    Ok(())
}

/// 渲染單位選擇 ComboBox（集成搜尋），選擇後直接部署
fn render_unit_combobox(
    ui: &mut egui::Ui,
    index: usize,
    pos: Position,
    ui_state: &mut LevelTabUIState,
) -> CResult<()> {
    let mut selected_value = String::new();

    let unit_names: Vec<String> = ui_state
        .available_units
        .iter()
        .map(|u| u.name.clone())
        .collect();

    egui::ComboBox::from_id_salt(format!("player_unit_selector_{}", index))
        .selected_text("選擇單位")
        .height(COMBOBOX_MIN_HEIGHT)
        .show_ui(ui, |ui| {
            ui.set_min_width(COMBOBOX_MIN_WIDTH);

            // 搜尋輸入框（使用全域搜尋字串）
            let response = render_search_input(ui, &mut ui_state.unit_search_query);
            ui.memory_mut(|mem| mem.request_focus(response.id));
            ui.separator();

            // 過濾選項
            let visible_units = filter_by_search(&unit_names, &ui_state.unit_search_query);
            for unit_name in visible_units {
                ui.selectable_value(&mut selected_value, unit_name.clone(), unit_name);
            }
        });

    if !selected_value.is_empty() {
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

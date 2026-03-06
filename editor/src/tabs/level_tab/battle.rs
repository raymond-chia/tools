//! 關卡編輯器的戰鬥模式邏輯

use super::battlefield::{self, Snapshot};
use super::{LevelTabMode, LevelTabUIState, MessageState};
use crate::constants::*;
use board::ecs_types::components::{Occupant, Position};
use board::ecs_types::resources::TurnOrder;
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

    let turn_order = match board::ecs_logic::turn::get_turn_order(&mut ui_state.world) {
        Ok(t) => t.clone(),
        Err(e) => {
            message_state.set_error(format!("讀取回合資料失敗：{}", e));
            return;
        }
    };

    // 頂部：返回按鈕
    if ui.button("← 返回").clicked() {
        ui_state.mode = LevelTabMode::Edit;
        return;
    }

    ui.add_space(SPACING_SMALL);

    render_level_info(ui, &snapshot);

    ui.add_space(SPACING_MEDIUM);
    ui.separator();

    // 主要佈局：回合面板 + 戰場 + 右側詳情面板
    let mut errors = vec![];
    let height = ui.available_height() - BOTTOM_PANEL_HEIGHT;
    ui.horizontal(|ui| {
        // 左側：回合順序面板
        ui.vertical(|ui| {
            ui.set_height(height);
            ui.set_width(TURN_PANEL_WIDTH);
            if let Err(e) = render_turn_order_panel(ui, &snapshot, &turn_order, ui_state) {
                errors.push(e);
            }
        });

        ui.separator();

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

    ui.separator();

    // 底部操作面板
    if let Err(e) = render_bottom_panel(ui, ui_state) {
        errors.push(e);
    }

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

            ui.end_row();

            ui.label(format!(
                "尺寸：{}×{}",
                snapshot.board.width, snapshot.board.height
            ));
            ui.separator();
            ui.label(format!("敵人數量：{}", enemy_count));
        });
}

/// 渲染回合順序面板（左側）
fn render_turn_order_panel(
    ui: &mut egui::Ui,
    snapshot: &Snapshot,
    turn_order: &TurnOrder,
    ui_state: &mut LevelTabUIState,
) -> Result<(), String> {
    ui.label(format!("第 {} 輪", turn_order.round));
    ui.add_space(SPACING_SMALL);

    ui.heading("回合順序");
    ui.add_space(SPACING_SMALL);
    ui.separator();

    // 過濾出尚未行動的 entries，反轉排列（當前行動在底部）
    // 反轉：最後行動的在上，當前行動的在底部
    let pending_entries: Vec<_> = turn_order
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| !entry.has_acted)
        .rev()
        .collect();

    let mut error = Ok(());
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .id_salt("turn_order_panel")
        .show(ui, |ui| {
            for (real_idx, entry) in pending_entries {
                let is_current = real_idx == turn_order.current_index;

                // 延遲模式：在非當前條目之間顯示插入點
                if ui_state.is_delaying && !is_current {
                    // 插入點只出現在當前單位（最底部）之上
                    if ui.button("── 插入 ──").clicked() {
                        // 我們需要的 target_index 是這個 entry 在 entries 中的 real_idx
                        if let Err(e) = board::ecs_logic::turn::delay_current_unit(
                            &mut ui_state.world,
                            real_idx,
                        ) {
                            error = Err(format!("延遲失敗：{}", e));
                        }
                        // 無論成功與否，都關閉延遲模式（因為玩家已經點了插入）
                        ui_state.is_delaying = false;
                        return;
                    }
                }

                // 渲染條目
                let unit_info = match find_unit_info_by_occupant(&entry.occupant, snapshot) {
                    Ok(info) => info,
                    Err(e) => {
                        error = Err(e);
                        return;
                    }
                };
                let label = format!(
                    "{}\n(INI:{} + {}={})",
                    unit_info.name, entry.initiative, entry.roll, entry.total
                );
                let response = ui.colored_label(unit_info.faction_color, label);

                // 點擊條目：選中對應單位並置中
                if response.clicked() {
                    ui_state.selected_left_pos = Some(unit_info.position);
                    ui_state.pending_center_pos = Some(unit_info.position);
                }
            }
        });
    error
}

/// 渲染底部操作面板
fn render_bottom_panel(ui: &mut egui::Ui, ui_state: &mut LevelTabUIState) -> Result<(), String> {
    let mut error = Ok(());
    ui.horizontal(|ui| {
        ui.set_height(BOTTOM_PANEL_HEIGHT - SPACING_SMALL); // 預留一些空隙

        if ui.button("結束回合").clicked() {
            if let Err(e) = board::ecs_logic::turn::end_current_turn(&mut ui_state.world) {
                error = Err(format!("結束回合失敗：{}", e));
                return;
            }
            ui_state.is_delaying = false;
            return;
        }

        ui.separator();

        let delay_label = if ui_state.is_delaying {
            "取消延遲"
        } else {
            "延遲"
        };
        if ui.button(delay_label).clicked() {
            ui_state.is_delaying = !ui_state.is_delaying;
            return;
        }
    });
    error
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
        .horizontal_scroll_offset(ui_state.scroll_offset.x)
        .vertical_scroll_offset(ui_state.scroll_offset.y)
        .show(ui, |ui| {
            let total_size = battlefield::calculate_grid_dimensions(board);
            let (rect, response) = ui.allocate_exact_size(total_size, egui::Sense::click());

            let hovered_pos = battlefield::compute_hover_pos(&response, rect, board);

            // 渲染網格（高亮選中的單位）
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
    // 處理延遲置中
    if let Some(pos) = ui_state.pending_center_pos.take() {
        let cell_stride = BATTLEFIELD_CELL_SIZE + BATTLEFIELD_GRID_SPACING;
        let target_x = pos.x as f32 * cell_stride + BATTLEFIELD_CELL_SIZE / 2.0;
        let target_y = pos.y as f32 * cell_stride + BATTLEFIELD_CELL_SIZE / 2.0;
        let viewport = scroll_output.inner_rect.size();
        ui_state.scroll_offset = egui::vec2(
            (target_x - viewport.x / 2.0).max(0.0),
            (target_y - viewport.y / 2.0).max(0.0),
        );
    } else {
        ui_state.scroll_offset = scroll_output.state.offset;
    }

    Ok(())
}

// ==================== 輔助函數 ====================

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

/// 從 snapshot 中反查 occupant 對應的單位資訊
struct UnitInfo {
    name: String,
    faction_color: egui::Color32,
    position: Position,
}

fn find_unit_info_by_occupant(
    occupant: &Occupant,
    snapshot: &Snapshot,
) -> Result<UnitInfo, String> {
    for (pos, bundle) in &snapshot.unit_map {
        if bundle.occupant == *occupant {
            let faction_color =
                battlefield::get_faction_color(&snapshot.level_config.factions, bundle.faction.0);
            return Ok(UnitInfo {
                name: bundle.occupant_type_name.0.clone(),
                faction_color,
                position: *pos,
            });
        }
    }
    Err(format!("在 snapshot 中找不到佔據者: {:?}", occupant))
}

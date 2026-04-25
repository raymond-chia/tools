//! 關卡編輯器的戰鬥模式邏輯

use super::battlefield::{self, CellHighlight, Snapshot};
use super::{BattleAction, LevelTabMode, LevelTabUIState, MessageState, RightPanelView};
use crate::constants::*;
use board::ecs_types::components::{Occupant, Position};
use board::ecs_types::resources::TurnOrder;
use board::error::Result as CResult;
use board::logic::movement::ReachableInfo;
use board::logic::skill::skill_execution::{
    CheckDetail, CheckResult, CheckTarget, EffectEntry, ResolvedEffect,
};
use std::collections::{HashMap, HashSet};

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
    // 需要包含 separator 高度, margin, padding, ...
    let height = ui.available_height() - BOTTOM_PANEL_HEIGHT - SPACING_SMALL * 5.0;
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
        let right_panel_width = LIST_PANEL_WIDTH + SPACING_SMALL * 5.0; // 面板寬度 + scroll bar
        let center_panel_width = ui.available_width() - right_panel_width;

        // 中間：戰場預覽
        ui.vertical(|ui| {
            ui.set_height(height);
            ui.set_width(center_panel_width);
            if let Err(e) = render_battlefield(ui, &snapshot, &turn_order, ui_state) {
                errors.push(format!("渲染戰場失敗：{}", e));
            }
        });

        // 右側面板：依 RightPanelView 切換詳情 / log
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt("battle_right_panel")
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.set_width(LIST_PANEL_WIDTH - SPACING_SMALL);
                    render_right_panel_toggle(ui, &mut ui_state.right_panel_view);
                    ui.separator();
                    match ui_state.right_panel_view {
                        RightPanelView::Details => match ui_state.selected_right_pos {
                            Some(pos) => battlefield::render_details_panel(ui, pos, &snapshot),
                            None => {
                                ui.label("（右鍵點擊單位或物件以顯示詳情）");
                            }
                        },
                        RightPanelView::Log => {
                            render_battle_log(ui, &ui_state.battle_log, &snapshot);
                        }
                    }
                });
            });
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

    ui.horizontal(|ui| {
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
                if ui_state.battle_action == BattleAction::Delaying && !is_current {
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
                        ui_state.battle_action = BattleAction::Normal;
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
        // 預留一些空隙
        let height = BOTTOM_PANEL_HEIGHT;
        ui.set_height(height);

        let height = height - SPACING_SMALL * 2.0;
        let button_size = egui::vec2(BOTTOM_PANEL_BUTTON_WIDTH, height);

        let is_skill_mode = ui_state.battle_action == BattleAction::SkillMode;

        // 第一顆：結束回合（SkillMode 下 disabled）
        let mut end_turn_clicked = false;
        ui.add_enabled_ui(!is_skill_mode, |ui| {
            if ui
                .add_sized(
                    button_size,
                    egui::Button::new("結束回合").wrap_mode(egui::TextWrapMode::Wrap),
                )
                .clicked()
            {
                end_turn_clicked = true;
            }
        });
        if end_turn_clicked {
            if let Err(e) = board::ecs_logic::turn::end_current_turn(&mut ui_state.world) {
                error = Err(format!("結束回合失敗：{}", e));
                return;
            }
            ui_state.battle_action = BattleAction::Normal;
            return;
        }

        ui.separator();

        // 第二顆：延遲（SkillMode 下點擊會先 cancel targeting 再進入 Delaying）
        let can_delay = match board::ecs_logic::turn::can_delay_current_unit(&mut ui_state.world) {
            Ok(v) => v,
            Err(e) => {
                error = Err(format!("查詢可否延遲失敗：{}", e));
                return;
            }
        };
        let (label, next_action) = if ui_state.battle_action == BattleAction::Delaying {
            ("取消延遲", BattleAction::Normal)
        } else {
            ("延遲", BattleAction::Delaying)
        };
        let mut delay_clicked = false;
        ui.add_enabled_ui(can_delay, |ui| {
            if ui
                .add_sized(
                    button_size,
                    egui::Button::new(label).wrap_mode(egui::TextWrapMode::Wrap),
                )
                .clicked()
            {
                delay_clicked = true;
            }
        });
        if delay_clicked {
            board::ecs_logic::skill::cancel_skill_targeting(&mut ui_state.world);
            ui_state.battle_action = next_action;
            return;
        }

        ui.separator();

        // 第三顆：技能 / 取消（就地切換，寬度相同）
        let can_use_skill =
            match board::ecs_logic::skill::can_use_skill_current_unit(&mut ui_state.world) {
                Ok(v) => v,
                Err(e) => {
                    error = Err(format!("查詢可否使用技能失敗：{}", e));
                    return;
                }
            };
        let (label, next_action) = if is_skill_mode {
            ("取消", BattleAction::Normal)
        } else {
            ("技能", BattleAction::SkillMode)
        };
        let mut skill_clicked = false;
        let mut skill_response = None;
        ui.add_enabled_ui(can_use_skill || is_skill_mode, |ui| {
            let response = ui.add_sized(
                button_size,
                egui::Button::new(label).wrap_mode(egui::TextWrapMode::Wrap),
            );
            if response.clicked() {
                skill_clicked = true;
            }
            skill_response = Some(response);
        });
        if skill_clicked {
            board::ecs_logic::skill::cancel_skill_targeting(&mut ui_state.world);
            ui_state.battle_action = next_action;
            return;
        }

        // SkillMode 下常駐技能 popup，錨定在第三顆按鈕上方
        if is_skill_mode {
            if let Some(response) = skill_response {
                if let Err(e) = render_skill_popup(ui, ui_state, response.rect) {
                    error = Err(e);
                }
            }
        }
    });
    error
}

/// 渲染戰場預覽
fn render_battlefield(
    ui: &mut egui::Ui,
    snapshot: &Snapshot,
    turn_order: &TurnOrder,
    ui_state: &mut LevelTabUIState,
) -> CResult<()> {
    let board = snapshot.board;

    // SkillMode 下從 world 讀取當前選中技能與 picked（resource 不存在代表尚未選技能）
    let (selected_skill, picked_positions) = match ui_state.battle_action {
        BattleAction::SkillMode => {
            match board::ecs_logic::query::get_skill_targeting(&ui_state.world) {
                Ok(t) => (Some(t.skill_name.clone()), t.picked.clone()),
                Err(_) => (None, Vec::new()),
            }
        }
        _ => (None, Vec::new()),
    };
    let skill_targetable: HashSet<Position> = match &selected_skill {
        Some(skill_name) => board::ecs_logic::skill::get_skill_targetable_positions(
            &mut ui_state.world,
            skill_name,
        )?
        .into_iter()
        .collect(),
        None => HashSet::new(),
    };
    let picked_set: HashSet<Position> = picked_positions.into_iter().collect();

    // 取得當前行動單位的可移動範圍
    let current_occupant = board::ecs_logic::turn::get_current_unit(turn_order).ok();
    let (reachable_positions, remaining_1mov, current_pos) = match current_occupant {
        Some(occupant) if ui_state.battle_action == BattleAction::Normal => {
            let reachable =
                board::ecs_logic::movement::get_reachable_positions(&mut ui_state.world, occupant)?;
            let unit_bundle = snapshot
                .unit_map
                .values()
                .find(|b| b.occupant == occupant)
                .ok_or_else(|| board::error::BoardError::OccupantNotFound { occupant })?;
            let cost_used = match &unit_bundle.action_state {
                board::ecs_types::components::ActionState::Moved { cost } => *cost as i32,
                board::ecs_types::components::ActionState::Done => {
                    unit_bundle.attributes.movement_point.0 * 2
                }
            };
            let remaining = unit_bundle.attributes.movement_point.0 - cost_used;
            (reachable, remaining, Some(unit_bundle.position))
        }
        _ => (HashMap::new(), 0, None),
    };

    let mut error = Ok(());
    let scroll_output = egui::ScrollArea::both()
        .auto_shrink([false; 2])
        .id_salt("battle_battlefield")
        .horizontal_scroll_offset(ui_state.scroll_offset.x)
        .vertical_scroll_offset(ui_state.scroll_offset.y)
        .show(ui, |ui| {
            let total_size = battlefield::calculate_grid_dimensions(board);
            let (rect, response) = ui.allocate_exact_size(total_size, egui::Sense::click());

            let hovered_pos = battlefield::compute_hover_pos(&response, rect, board);

            // 計算技能 AOE 預覽（懸停在可攻擊位置時）
            let skill_all_filtered_positions = match (&selected_skill, hovered_pos) {
                (Some(skill_name), Some(hover)) if skill_targetable.contains(&hover) => {
                    let preview = board::ecs_logic::skill::get_skill_affected_positions(
                        &mut ui_state.world,
                        skill_name,
                        hover,
                    );
                    let preview = match preview {
                        Ok(preview) => preview,
                        Err(e) => {
                            error = Err(e);
                            return;
                        }
                    };
                    preview.filtered_positions.into_iter().collect()
                }
                _ => HashSet::new(),
            };

            // 計算路徑預覽（懸停時）
            let preview_path = preview_path(current_pos, hovered_pos, &reachable_positions);

            // 渲染網格（加上可移動範圍高亮）
            let get_cell_info_fn = battlefield::get_cell_info(snapshot);
            let get_cell_highlight_fn = get_cell_highlight(
                ui_state.selected_right_pos,
                preview_path,
                &reachable_positions,
                remaining_1mov,
                &skill_targetable,
                &skill_all_filtered_positions,
                &picked_set,
            );

            battlefield::render_grid(
                ui,
                rect,
                board,
                ui_state.scroll_offset,
                get_cell_info_fn,
                get_cell_highlight_fn,
            );
            if let Some(hovered_pos) = hovered_pos {
                error = handle_mouse_click(
                    &response,
                    hovered_pos,
                    snapshot,
                    ui_state,
                    &reachable_positions,
                );
                let get_tooltip_info_fn =
                    get_tooltip_info_with_movement(&reachable_positions, snapshot, remaining_1mov);
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

    error
}

/// 渲染技能列表彈出面板
fn render_skill_popup(
    ui: &mut egui::Ui,
    ui_state: &mut LevelTabUIState,
    button_rect: egui::Rect,
) -> Result<(), String> {
    let skills = match board::ecs_logic::skill::get_available_skills(&mut ui_state.world) {
        Ok(s) => s,
        Err(e) => return Err(format!("取得技能列表失敗：{}", e)),
    };
    let targeting = board::ecs_logic::query::get_skill_targeting(&ui_state.world)
        .ok()
        .cloned();
    let current_skill = targeting.as_ref().map(|t| t.skill_name.clone());
    // 只有多目標技能才顯示「確認施放」按鈕
    let confirm_info = match targeting {
        Some(t) if t.max_count != SINGLE_TARGET_COUNT => {
            Some((t.skill_name.clone(), t.picked.clone(), !t.picked.is_empty()))
        }
        _ => None,
    };

    let mut clicked_skill = None;
    let mut confirm_clicked_payload = None;
    let popup_pos = egui::pos2(button_rect.left(), button_rect.top());
    egui::Area::new(egui::Id::new("skill_popup"))
        .fixed_pos(popup_pos)
        .pivot(egui::Align2::LEFT_BOTTOM)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_width(button_rect.width());
                if let Some((skill_name, picked, enabled)) = &confirm_info {
                    let label = format!("確認施放\n（已選 {}）", picked.len());
                    ui.add_enabled_ui(*enabled, |ui| {
                        if ui.button(label).clicked() {
                            confirm_clicked_payload = Some((skill_name.clone(), picked.clone()));
                        }
                    });
                    ui.separator();
                }
                ui.horizontal_top(|ui| {
                    let item_size = egui::vec2(BOTTOM_PANEL_BUTTON_WIDTH, 0.0);
                    for skill in &skills {
                        if skill.usable {
                            let is_selected = current_skill.as_ref() == Some(&skill.name);
                            if ui
                                .add_sized(
                                    item_size,
                                    egui::Button::selectable(is_selected, &skill.name),
                                )
                                .clicked()
                            {
                                clicked_skill = Some(skill.name.clone());
                            }
                        } else {
                            let label = egui::Label::new(
                                egui::RichText::new(&skill.name).color(egui::Color32::GRAY),
                            );
                            ui.add_sized(item_size, label).on_hover_text("缺乏魔力");
                        }
                    }
                    if skills.is_empty() {
                        ui.label("無可用技能");
                    }
                });
            });
        });

    if let Some((skill_name, picked)) = confirm_clicked_payload {
        let entries =
            board::ecs_logic::skill::execute_skill(&mut ui_state.world, &skill_name, &picked)
                .map_err(|e| format!("施放技能失敗：{}", e))?;
        ui_state.battle_log.extend(entries);
        ui_state.right_panel_view = RightPanelView::Log;
        board::ecs_logic::skill::cancel_skill_targeting(&mut ui_state.world);
        ui_state.battle_action = BattleAction::Normal;
        return Ok(());
    }

    if let Some(name) = clicked_skill {
        board::ecs_logic::skill::start_skill_targeting(&mut ui_state.world, &name)
            .map_err(|e| format!("開始技能選目標失敗：{}", e))?;
    }

    Ok(())
}

// ==================== 輔助函數 ====================

fn preview_path(
    src: Option<Position>,
    dst: Option<Position>,
    reachable_positions: &HashMap<Position, ReachableInfo>,
) -> HashSet<Position> {
    let (src, dst) = match (src, dst) {
        (Some(src), Some(dst)) => (src, dst),
        _ => return HashSet::new(),
    };
    match reachable_positions.get(&dst) {
        Some(info) => {
            // 目的地不能停留
            if info.passthrough_only {
                return HashSet::new();
            }
        }
        // 目的地不在可達範圍內
        None => {
            return HashSet::new();
        }
    };
    board::logic::movement::reconstruct_path(&reachable_positions, src, dst)
        .into_iter()
        .collect()
}

fn get_cell_highlight<'a>(
    selected_pos: Option<Position>,
    preview_path: HashSet<Position>,
    reachable_positions: &'a HashMap<Position, ReachableInfo>,
    remaining_1mov: i32,
    skill_targetable: &'a HashSet<Position>,
    skill_all_filtered_positions: &'a HashSet<Position>,
    picked_set: &'a HashSet<Position>,
) -> impl Fn(Position) -> CellHighlight + 'a {
    move |pos: Position| -> CellHighlight {
        let border = if skill_targetable.contains(&pos) {
            Some(BATTLEFIELD_COLOR_SKILL_RED)
        } else if selected_pos == Some(pos) {
            Some(BATTLEFIELD_COLOR_HIGHLIGHT)
        } else {
            None
        };

        let bg = if picked_set.contains(&pos) {
            Some(BATTLEFIELD_COLOR_SKILL_PICKED)
        } else if skill_all_filtered_positions.contains(&pos) {
            Some(BATTLEFIELD_COLOR_SKILL_RED)
        } else if preview_path.contains(&pos) {
            Some(BATTLEFIELD_COLOR_MOVE_PATH)
        } else if let Some(info) = reachable_positions.get(&pos) {
            if info.passthrough_only {
                None
            } else if (info.cost as i32) <= remaining_1mov {
                Some(BATTLEFIELD_COLOR_MOVE_1MOV)
            } else {
                Some(BATTLEFIELD_COLOR_MOVE_2MOV)
            }
        } else {
            None
        };

        CellHighlight { border, bg }
    }
}

/// 處理棋盤點擊事件（戰鬥模式）
/// 左鍵：移動當前行動單位到可到達的位置
/// 右鍵：選擇有單位或物件的位置顯示詳情
fn handle_mouse_click(
    response: &egui::Response,
    clicked_pos: Position,
    snapshot: &Snapshot,
    ui_state: &mut LevelTabUIState,
    reachable_positions: &HashMap<Position, board::logic::movement::ReachableInfo>,
) -> CResult<()> {
    if response.clicked() {
        match ui_state.battle_action {
            BattleAction::Normal => {
                // 左鍵：執行移動
                match reachable_positions.get(&clicked_pos) {
                    Some(info) => {
                        if !info.passthrough_only {
                            board::ecs_logic::movement::execute_move(
                                &mut ui_state.world,
                                clicked_pos,
                            )?;
                            ui_state.selected_left_pos = Some(clicked_pos);
                        }
                    }
                    _ => {}
                }
            }
            BattleAction::Delaying => {}
            BattleAction::SkillMode => {
                // 左鍵：若已選技能，嘗試新增目標（editor 先判斷在可攻擊範圍內）
                let (skill_name, max_count) =
                    match board::ecs_logic::query::get_skill_targeting(&ui_state.world) {
                        Ok(t) => (t.skill_name.clone(), t.max_count),
                        Err(_) => return Ok(()),
                    };
                let targetable: HashSet<Position> =
                    board::ecs_logic::skill::get_skill_targetable_positions(
                        &mut ui_state.world,
                        &skill_name,
                    )?
                    .into_iter()
                    .collect();
                if targetable.contains(&clicked_pos) {
                    board::ecs_logic::skill::add_skill_target(&mut ui_state.world, clicked_pos)?;
                    if max_count == SINGLE_TARGET_COUNT {
                        let picked = board::ecs_logic::query::get_skill_targeting(&ui_state.world)?
                            .picked
                            .clone();
                        let entries = board::ecs_logic::skill::execute_skill(
                            &mut ui_state.world,
                            &skill_name,
                            &picked,
                        )?;
                        ui_state.battle_log.extend(entries);
                        ui_state.right_panel_view = RightPanelView::Log;
                        board::ecs_logic::skill::cancel_skill_targeting(&mut ui_state.world);
                        ui_state.battle_action = BattleAction::Normal;
                    }
                }
            }
        }
    }
    if response.secondary_clicked() {
        match ui_state.battle_action {
            BattleAction::SkillMode => {
                // 右鍵：取消技能模式
                board::ecs_logic::skill::cancel_skill_targeting(&mut ui_state.world);
                ui_state.battle_action = BattleAction::Normal;
            }
            _ => {}
        }

        // 右鍵：選擇詳情並切換到詳情面板
        if snapshot.unit_map.contains_key(&clicked_pos)
            || snapshot.object_map.contains_key(&clicked_pos)
        {
            ui_state.selected_right_pos = Some(clicked_pos);
            ui_state.right_panel_view = RightPanelView::Details;
        } else {
            ui_state.selected_right_pos = None;
        }
    }
    Ok(())
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
            let faction_color = battlefield::get_faction_color(
                &snapshot.level_config.factions,
                bundle.unit_faction.0,
            );
            return Ok(UnitInfo {
                name: bundle.occupant_type_name.0.clone(),
                faction_color,
                position: *pos,
            });
        }
    }
    Err(format!("在 snapshot 中找不到佔據者: {:?}", occupant))
}

/// 取得懸停提示，並加上移動花費資訊
fn get_tooltip_info_with_movement<'a>(
    reachable: &'a HashMap<Position, board::logic::movement::ReachableInfo>,
    snapshot: &'a Snapshot,
    remaining_1mov: i32,
) -> impl Fn(Position) -> String + 'a {
    let base_tooltip = battlefield::get_tooltip_info(snapshot);
    move |pos| -> String {
        let base_info = base_tooltip(pos);

        if let Some(info) = reachable.get(&pos) {
            if !info.passthrough_only {
                let mov_type = if (info.cost as i32) <= remaining_1mov {
                    "1 MOV"
                } else {
                    "2 MOV"
                };
                return format!("{}\n移動花費：{} ({})", base_info, info.cost, mov_type);
            }
        }

        base_info
    }
}

/// 渲染右側面板切換鈕（詳情 / log）
fn render_right_panel_toggle(ui: &mut egui::Ui, view: &mut RightPanelView) {
    ui.horizontal(|ui| {
        ui.selectable_value(view, RightPanelView::Details, "詳情");
        ui.selectable_value(view, RightPanelView::Log, "Log");
    });
}

/// 渲染戰鬥 log 面板
fn render_battle_log(ui: &mut egui::Ui, log: &[EffectEntry], snapshot: &Snapshot) {
    ui.heading("戰鬥 Log");
    ui.add_space(SPACING_SMALL);
    if log.is_empty() {
        ui.label("（尚無紀錄）");
        return;
    }
    egui::ScrollArea::vertical()
        .id_salt("battle_log_scroll")
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            for entry in log {
                render_effect_entry(ui, entry, snapshot);
                ui.separator();
            }
        });
}

/// 渲染單筆效果條目（多行）
fn render_effect_entry(ui: &mut egui::Ui, entry: &EffectEntry, snapshot: &Snapshot) {
    let caster_str = match find_unit_name_by_id(entry.caster, snapshot) {
        Some(name) => name,
        None => format!("單位#{}", entry.caster),
    };
    let target_str = format_check_target(&entry.target, snapshot);
    ui.add(
        egui::Label::new(format!(
            "{} 對 {} 使用 {}",
            caster_str, target_str, entry.skill_name
        ))
        .wrap(),
    );
    ui.add(
        egui::Label::new(format!(
            "判定：{}",
            format_check(&entry.check, entry.check_detail.as_ref())
        ))
        .wrap(),
    );
    ui.add(egui::Label::new(format!("效果：{}", format_effect(&entry.effect))).wrap());
}

fn format_check_target(target: &CheckTarget, snapshot: &Snapshot) -> String {
    match target {
        CheckTarget::Unit(id) => match find_unit_name_by_id(*id, snapshot) {
            Some(name) => format!("單位 {}", name),
            None => format!("單位#{}", id),
        },
        CheckTarget::Position(pos) => format!("位置({}, {})", pos.x, pos.y),
    }
}

fn format_check(check: &CheckResult, detail: Option<&CheckDetail>) -> String {
    let result_str = match check {
        CheckResult::Auto => "自動命中".to_string(),
        CheckResult::Hit { crit } => {
            if *crit {
                "爆擊命中".to_string()
            } else {
                "命中".to_string()
            }
        }
        CheckResult::Block { crit } => {
            if *crit {
                "爆擊被格擋".to_string()
            } else {
                "被格擋".to_string()
            }
        }
        CheckResult::Evade => "閃避".to_string(),
        CheckResult::Resisted => "抵抗".to_string(),
        CheckResult::Affected => "生效".to_string(),
    };
    match detail {
        None => result_str,
        Some(d) => format!(
            "{} [{} 命中 {} vs {} 閃避 {} / 格擋 {}, 爆 {}%, 骰 {}]",
            result_str,
            d.accuracy_source,
            d.attacker_accuracy,
            d.defense_type,
            d.defender_evasion,
            d.defender_block,
            d.crit_rate,
            d.roll,
        ),
    }
}

fn format_effect(effect: &ResolvedEffect) -> String {
    match effect {
        ResolvedEffect::NoEffect => "無效果".to_string(),
        ResolvedEffect::HpChange { final_amount, .. } => format!("HP 變化 {}", final_amount),
        ResolvedEffect::SpawnObject { object_type } => format!("產生物件 {}", object_type),
        ResolvedEffect::ApplyBuff(name) => format!("施加狀態 {}", name),
    }
}

fn find_unit_name_by_id(id: board::domain::alias::ID, snapshot: &Snapshot) -> Option<String> {
    for bundle in snapshot.unit_map.values() {
        if let Occupant::Unit(unit_id) = bundle.occupant {
            if unit_id == id {
                return Some(bundle.occupant_type_name.0.clone());
            }
        }
    }
    None
}

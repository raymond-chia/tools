//! 關卡編輯器的戰鬥模式邏輯

use super::battlefield::{self, CellHighlight, Snapshot};
use super::{BattleAction, LevelTabMode, LevelTabUIState, MessageState, RightPanelView};
use crate::constants::*;
use board::domain::alias::SkillName;
use board::domain::battle_log::{LogCheck, LogCheckDetail, LogEffect, LogEvent, LogTarget};
use board::domain::core_types::PendingReaction;
use board::ecs_logic::reaction::ProcessReactionResult;
use board::ecs_types::components::{Occupant, Position};
use board::ecs_types::resources::TurnOrder;
use board::error::Result as CResult;
use board::logic::movement::ReachableInfo;
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
                    ui.set_width(LIST_PANEL_WIDTH - SPACING_SMALL * 5.0);
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
                            match board::ecs_logic::query::get_battle_log(&ui_state.world) {
                                Ok(log) => render_battle_log(ui, log),
                                Err(e) => {
                                    ui.label(format!("讀取戰鬥 log 失敗：{}", e));
                                }
                            }
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
    let pending = board::ecs_logic::reaction::get_pending_reactions(&ui_state.world);
    if !pending.is_empty() {
        return render_reaction_panel(ui, ui_state, &pending);
    }

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

            // 計算路徑風險預覽（懸停在可停留目標時）：危險地面 + 藉機攻擊觸發格
            let path_hazards = match hovered_pos {
                Some(hover) if !preview_path.is_empty() => {
                    match board::ecs_logic::movement::preview_move_path(&mut ui_state.world, hover)
                    {
                        Ok(preview_hazard) => collect_path_hazards(&preview_hazard, &preview_path),
                        Err(e) => {
                            error = Err(e);
                            return;
                        }
                    }
                }
                _ => HashSet::new(),
            };

            // 計算命中率預覽（SkillMode 懸停在可攻擊目標時）
            let hit_preview_text = match (&selected_skill, hovered_pos) {
                (Some(skill_name), Some(hover)) if skill_targetable.contains(&hover) => {
                    match board::ecs_logic::skill::preview_hit_probabilities(
                        &mut ui_state.world,
                        skill_name,
                        hover,
                    ) {
                        Ok(Some(preview)) => Some(format_hit_preview(&preview)),
                        Ok(None) => None,
                        Err(e) => {
                            error = Err(e);
                            return;
                        }
                    }
                }
                _ => None,
            };

            // 渲染網格（加上可移動範圍高亮）
            let get_cell_info_fn = battlefield::get_cell_info(snapshot);
            let get_cell_highlight_fn = get_cell_highlight(
                ui_state.selected_right_pos,
                &preview_path,
                &reachable_positions,
                remaining_1mov,
                &skill_targetable,
                &skill_all_filtered_positions,
                &picked_set,
                &path_hazards,
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
                let get_tooltip_with_hit_fn = |pos: Position| -> String {
                    let base = get_tooltip_info_fn(pos);
                    match &hit_preview_text {
                        Some(text) => format!("{}\n{}", base, text),
                        None => base,
                    }
                };
                battlefield::render_hover_tooltip(ui, rect, hovered_pos, get_tooltip_with_hit_fn);
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
                        // 第一行技能名、第二行費用
                        let button_text = format!("{}\n費用 {}", skill.name, skill.cost);
                        if skill.usable {
                            let is_selected = current_skill.as_ref() == Some(&skill.name);
                            if ui
                                .add_sized(
                                    item_size,
                                    egui::Button::selectable(is_selected, button_text),
                                )
                                .clicked()
                            {
                                clicked_skill = Some(skill.name.clone());
                            }
                        } else {
                            let label = egui::Label::new(
                                egui::RichText::new(button_text).color(egui::Color32::GRAY),
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
        board::ecs_logic::battle_log::append_skill_log(&mut ui_state.world, &entries)
            .map_err(|e| format!("產生技能 log 失敗：{}", e))?;
        board::ecs_logic::turn::resolve_deaths(&mut ui_state.world)
            .map_err(|e| format!("處理死亡失敗：{}", e))?;
        ui_state.right_panel_view = RightPanelView::Log;
        board::ecs_logic::skill::cancel_skill_targeting(&mut ui_state.world);
        ui_state.battle_action = BattleAction::Normal;
        sync_reaction_decisions(ui_state);
        return Ok(());
    }

    if let Some(name) = clicked_skill {
        board::ecs_logic::skill::start_skill_targeting(&mut ui_state.world, &name)
            .map_err(|e| format!("開始技能選目標失敗：{}", e))?;
    }

    Ok(())
}

// ==================== 輔助函數 ====================

/// 重建懸停目標的有序移動路徑（含起點）；目標不可停留或不可達時回空
fn preview_path(
    src: Option<Position>,
    dst: Option<Position>,
    reachable_positions: &HashMap<Position, ReachableInfo>,
) -> Vec<Position> {
    let (src, dst) = match (src, dst) {
        (Some(src), Some(dst)) => (src, dst),
        _ => return Vec::new(),
    };
    match reachable_positions.get(&dst) {
        Some(info) => {
            // 目的地不能停留
            if info.passthrough_only {
                return Vec::new();
            }
        }
        // 目的地不在可達範圍內
        None => {
            return Vec::new();
        }
    };
    board::logic::movement::reconstruct_path(&reachable_positions, src, dst)
}

/// 蒐集路徑風險格：危險地面 + 藉機攻擊觸發格（移動者被打的那一格）
///
/// 危險地面直接取 hazard_positions；藉機攻擊以 from_index 對映到 path 上
/// 移動者離開的那格，即移動者遭攻擊時所在的位置（非攻擊者所在格）。
fn collect_path_hazards(
    preview: &board::ecs_logic::movement::MovePathPreview,
    path: &[Position],
) -> HashSet<Position> {
    let mut hazards: HashSet<Position> = preview.hazard_positions.iter().copied().collect();
    for reaction in &preview.reactions {
        // from_index 由 core 依同一條 path 算出，必定落在範圍內
        hazards.insert(path[reaction.from_index]);
    }
    hazards
}

fn get_cell_highlight<'a>(
    selected_pos: Option<Position>,
    preview_path: &'a [Position],
    reachable_positions: &'a HashMap<Position, ReachableInfo>,
    remaining_1mov: i32,
    skill_targetable: &'a HashSet<Position>,
    skill_all_filtered_positions: &'a HashSet<Position>,
    picked_set: &'a HashSet<Position>,
    path_hazards: &'a HashSet<Position>,
) -> impl Fn(Position) -> CellHighlight + 'a {
    move |pos: Position| -> CellHighlight {
        let border = if skill_targetable.contains(&pos) {
            Some(BATTLEFIELD_COLOR_SKILL_RED)
        } else if path_hazards.contains(&pos) {
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
                            board::ecs_logic::movement::plan_move(
                                &mut ui_state.world,
                                clicked_pos,
                            )?;
                            board::ecs_logic::movement::advance_move(&mut ui_state.world)?;
                            ui_state.selected_left_pos = Some(clicked_pos);
                            sync_reaction_decisions(ui_state);
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
                        board::ecs_logic::battle_log::append_skill_log(
                            &mut ui_state.world,
                            &entries,
                        )?;
                        board::ecs_logic::turn::resolve_deaths(&mut ui_state.world)?;
                        ui_state.right_panel_view = RightPanelView::Log;
                        board::ecs_logic::skill::cancel_skill_targeting(&mut ui_state.world);
                        ui_state.battle_action = BattleAction::Normal;
                        sync_reaction_decisions(ui_state);
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

/// 將命中率預覽格式化為 tooltip 文字（命中/格擋/閃避/爆擊率 + 命中值來源明細）
fn format_hit_preview(preview: &board::ecs_logic::skill::HitPreview) -> String {
    let prob = &preview.probabilities;
    let acc = &preview.breakdowns.attacker_accuracy;

    // 命中值來源逐項（省略為 0 的加成，保留基礎與最終）
    let mut sources = vec![format!("基礎 {}", acc.base)];
    if acc.skill_bonus != 0 {
        sources.push(format!("技能 {:+}", acc.skill_bonus));
    }
    if acc.flanking_bonus != 0 {
        sources.push(format!("側翼 {:+}", acc.flanking_bonus));
    }
    if acc.adjacent_penalty != 0 {
        sources.push(format!("相鄰 {:+}", acc.adjacent_penalty));
    }

    format!(
        "命中 {}% / 格擋 {}% / 閃避 {}%\n爆擊 {}%\n命中值：{} = {}",
        prob.hit,
        prob.block,
        prob.evade,
        prob.crit,
        sources.join(" "),
        acc.total,
    )
}

/// 渲染右側面板切換鈕（詳情 / log）
fn render_right_panel_toggle(ui: &mut egui::Ui, view: &mut RightPanelView) {
    ui.horizontal(|ui| {
        ui.selectable_value(view, RightPanelView::Details, "詳情");
        ui.selectable_value(view, RightPanelView::Log, "Log");
    });
}

/// 渲染戰鬥 log 面板（讀取 core 提供的 LogEvent 序列，自帶名稱快照）
fn render_battle_log(ui: &mut egui::Ui, log: &[LogEvent]) {
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
            for event in log {
                render_log_event(ui, event);
                ui.separator();
            }
        });
}

/// 渲染單筆 log 事件（多行）
fn render_log_event(ui: &mut egui::Ui, event: &LogEvent) {
    match event {
        LogEvent::Skill {
            caster,
            skill_name,
            target,
            check,
            check_detail,
            effect,
        } => {
            ui.add(
                egui::Label::new(format!(
                    "{} 對 {} 使用 {}",
                    caster,
                    format_log_target(target),
                    skill_name
                ))
                .wrap(),
            );
            ui.add(
                egui::Label::new(format!(
                    "判定：{}",
                    format_log_check(check, check_detail.as_ref())
                ))
                .wrap(),
            );
            ui.add(egui::Label::new(format!("效果：{}", format_log_effect(effect))).wrap());
        }
        LogEvent::Reaction {
            reactor,
            trigger,
            skill_name,
            target,
            check,
            check_detail,
            effect,
        } => {
            ui.add(
                egui::Label::new(format!(
                    "{} 反應 {}，對 {} 使用 {}",
                    reactor,
                    trigger,
                    format_log_target(target),
                    skill_name
                ))
                .wrap(),
            );
            ui.add(
                egui::Label::new(format!(
                    "判定：{}",
                    format_log_check(check, check_detail.as_ref())
                ))
                .wrap(),
            );
            ui.add(egui::Label::new(format!("效果：{}", format_log_effect(effect))).wrap());
        }
        LogEvent::Death { unit } => {
            ui.add(egui::Label::new(format!("{} 死亡", unit)).wrap());
        }
    }
}

fn format_log_target(target: &LogTarget) -> String {
    match target {
        LogTarget::Unit { name } => format!("單位 {}", name),
        LogTarget::Object { name } => format!("物件 {}", name),
        LogTarget::EmptyGround => "空地".to_string(),
    }
}

fn format_log_check(check: &LogCheck, detail: Option<&LogCheckDetail>) -> String {
    let result_str = match check {
        LogCheck::Auto => "自動命中".to_string(),
        LogCheck::Hit { crit } => {
            if *crit {
                "爆擊命中".to_string()
            } else {
                "命中".to_string()
            }
        }
        LogCheck::Block { crit } => {
            if *crit {
                "爆擊被格擋".to_string()
            } else {
                "被格擋".to_string()
            }
        }
        LogCheck::Evade => "閃避".to_string(),
        LogCheck::Resisted => "抵抗".to_string(),
        LogCheck::Affected => "生效".to_string(),
    };
    match detail {
        None => result_str,
        Some(d) => format!(
            "{} [{} 命中 {} + 骰 {} = {} vs {} 閃避 {} / 格擋 {} (閃+格 {}), 爆 {}%]",
            result_str,
            d.accuracy_source,
            d.breakdowns.attacker_accuracy.total,
            d.roll,
            d.breakdowns.attacker_accuracy.total + d.roll,
            d.defense_type,
            d.breakdowns.defender_evasion.total,
            d.breakdowns.defender_block.total,
            d.breakdowns.defender_evasion.total + d.breakdowns.defender_block.total,
            d.breakdowns.crit,
        ),
    }
}

fn format_log_effect(effect: &LogEffect) -> String {
    match effect {
        LogEffect::None => "無效果".to_string(),
        LogEffect::HpChange { amount } => format!("HP 變化 {}", amount),
        LogEffect::SpawnObject { object_type } => format!("產生物件 {}", object_type),
        LogEffect::ApplyBuff { buff_name } => format!("施加狀態 {}", buff_name),
    }
}

/// 同步反應決策草稿：依照最新 pending 更新 decisions，保留已有選擇，補入新反應者，移除消失者
fn sync_reaction_decisions(ui_state: &mut LevelTabUIState) {
    let pending = board::ecs_logic::reaction::get_pending_reactions(&ui_state.world);
    let decisions = &mut ui_state.reaction_decision.decisions;

    // 移除不在 pending 中的反應者
    decisions.retain(|(occupant, _)| pending.iter().any(|r| r.reactor == *occupant));

    // 補入 pending 中新出現的反應者（保持 pending 原始順序新增到尾端）
    for reaction in &pending {
        if !decisions
            .iter()
            .any(|(occupant, _)| *occupant == reaction.reactor)
        {
            decisions.push((reaction.reactor, None));
        }
    }
}

/// 渲染反應決策面板（取代底部操作面板）
fn render_reaction_panel(
    ui: &mut egui::Ui,
    ui_state: &mut LevelTabUIState,
    pending: &[PendingReaction],
) -> Result<(), String> {
    let mut confirm_clicked = false;
    let error = Ok(());

    ui.horizontal(|ui| {
        ui.set_height(BOTTOM_PANEL_HEIGHT);

        ui.label("反應：");

        // 每個 decision 條目：反應者名稱 + 技能下拉 + 上移/下移
        let decision_count = ui_state.reaction_decision.decisions.len();
        let mut swap_indices: Option<(usize, usize)> = None;

        for idx in 0..decision_count {
            let (occupant, _) = ui_state.reaction_decision.decisions[idx];

            let reactor_name = find_reactor_name(occupant, pending);
            let available_skills = pending
                .iter()
                .find(|r| r.reactor == occupant)
                .map(|r| r.available_skills.as_slice())
                .unwrap_or(&[]);

            ui.group(|ui| {
                ui.label(&reactor_name);

                // 技能下拉：None = 跳過
                let selected_skill = &mut ui_state.reaction_decision.decisions[idx].1;
                let selected_label = selected_skill
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("跳過");
                egui::ComboBox::from_id_salt(format!("reaction_skill_{}", idx))
                    .selected_text(selected_label)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(selected_skill.is_none(), "跳過")
                            .clicked()
                        {
                            *selected_skill = None;
                        }
                        for skill_name in available_skills {
                            let is_selected = selected_skill.as_ref() == Some(skill_name);
                            if ui
                                .selectable_label(is_selected, skill_name.as_str())
                                .clicked()
                            {
                                *selected_skill = Some(skill_name.clone());
                            }
                        }
                    });

                // 排序按鈕
                ui.horizontal(|ui| {
                    let up_enabled = idx > 0;
                    let down_enabled = idx + 1 < decision_count;
                    ui.add_enabled_ui(up_enabled, |ui| {
                        if ui.small_button("↑").clicked() {
                            swap_indices = Some((idx - 1, idx));
                        }
                    });
                    ui.add_enabled_ui(down_enabled, |ui| {
                        if ui.small_button("↓").clicked() {
                            swap_indices = Some((idx, idx + 1));
                        }
                    });
                });
            });
        }

        if let Some((a, b)) = swap_indices {
            ui_state.reaction_decision.decisions.swap(a, b);
        }

        ui.separator();

        let button_size = egui::vec2(
            BOTTOM_PANEL_BUTTON_WIDTH,
            BOTTOM_PANEL_HEIGHT - SPACING_SMALL * 2.0,
        );
        if ui
            .add_sized(
                button_size,
                egui::Button::new("確認反應").wrap_mode(egui::TextWrapMode::Wrap),
            )
            .clicked()
        {
            confirm_clicked = true;
        }
    });

    if confirm_clicked {
        let decisions: Vec<(Occupant, SkillName)> = ui_state
            .reaction_decision
            .decisions
            .iter()
            .filter_map(|(occupant, skill)| skill.clone().map(|s| (*occupant, s)))
            .collect();

        board::ecs_logic::reaction::set_reactions(&mut ui_state.world, decisions)
            .map_err(|e| format!("設定反應失敗：{}", e))?;

        ui_state.reaction_decision.decisions.clear();

        loop {
            match board::ecs_logic::reaction::process_reactions(&mut ui_state.world)
                .map_err(|e| format!("執行反應失敗：{}", e))?
            {
                ProcessReactionResult::Executed { effects, trigger } => {
                    board::ecs_logic::battle_log::append_reaction_log(
                        &mut ui_state.world,
                        trigger,
                        &effects,
                    )
                    .map_err(|e| format!("產生反應 log 失敗：{}", e))?;
                    board::ecs_logic::turn::resolve_deaths(&mut ui_state.world)
                        .map_err(|e| format!("處理死亡失敗：{}", e))?;
                    ui_state.right_panel_view = RightPanelView::Log;
                }
                ProcessReactionResult::NeedDecision => {
                    sync_reaction_decisions(ui_state);
                    break;
                }
                ProcessReactionResult::Done => {
                    board::ecs_logic::movement::force_advance_move(&mut ui_state.world)
                        .map_err(|e| format!("繼續移動失敗：{}", e))?;
                    break;
                }
            }
        }
    }

    error
}

/// 從 pending 列表中找到反應者的顯示名稱（目前以 ID 顯示，pending 不含型別名）
fn find_reactor_name(occupant: Occupant, _pending: &[PendingReaction]) -> String {
    match occupant {
        Occupant::Unit(id) => format!("單位#{}", id),
        Occupant::Object(id) => format!("物件#{}", id),
    }
}

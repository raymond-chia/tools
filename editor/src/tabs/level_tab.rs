//! 關卡編輯器 tab

mod battle;
mod deployment;
mod grid;
mod unit_details;

use crate::constants::*;
use crate::editor_item::{EditorItem, validate_name};
use crate::utils::search::{filter_by_search, render_search_input};
use board::alias::{Coord, SkillName, TypeName};
use board::component::Position;
use board::loader_schema::{LevelType, ObjectPlacement, SkillType, UnitPlacement, UnitType};
use std::collections::{HashMap, HashSet};

// 公開從 grid 模組導出的項目
pub use grid::{
    DragState, apply_drag_update, calculate_grid_dimensions, calculate_visible_range,
    identify_dragged_object, is_position_in_bounds, prepare_lookup_maps, render_battlefield_legend,
    render_hover_tooltip, screen_to_board_pos,
};
// 公開從 unit_details 模組導出的項目
#[allow(unused_imports)]
pub use unit_details::{handle_unit_right_click, render_unit_details_side_panel};

/// 關卡編輯器的模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LevelTabMode {
    /// 編輯模式
    #[default]
    Edit,
    /// 部署模式
    Deploy,
    /// 戰鬥模式
    Battle,
}

/// 模擬戰鬥的狀態
#[derive(Debug, Default)]
pub struct SimulationState {
    /// 已部署的玩家單位：Map<部署點索引, 單位類型名稱>
    pub deployed_units: HashMap<usize, TypeName>,

    /// 當前選中的部署點索引（用於顯示 ComboBox）
    pub selected_deployment_point: Option<usize>,
}

/// 關卡編輯器的 UI 狀態
#[derive(Debug, Default)]
pub struct LevelTabUIState {
    pub available_units: Vec<TypeName>,
    pub available_objects: Vec<TypeName>,

    pub unit_search_query: TypeName,
    pub object_search_query: TypeName,

    pub drag_state: Option<DragState>,
    pub scroll_offset: egui::Vec2,

    /// 當前標籤頁的模式
    pub mode: LevelTabMode,
    /// 模擬戰鬥的狀態
    pub simulation_state: SimulationState,
    /// 臨時儲存右鍵菜單中要顯示的單位名稱
    pub temp_unit_name: Option<TypeName>,

    /// 技能映射表：用於計算單位屬性
    pub skills_map: HashMap<SkillName, SkillType>,
    /// 單位映射表：用於查找單位定義
    pub units_map: HashMap<TypeName, UnitType>,
}

// ==================== EditorItem 實作 ====================

impl EditorItem for LevelType {
    type UIState = LevelTabUIState;

    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn type_name() -> &'static str {
        "關卡"
    }

    fn validate(&self, all_items: &[Self], editing_index: Option<usize>) -> Result<(), String> {
        validate_name(self, all_items, editing_index)?;

        if self.board_width == 0 || self.board_height == 0 {
            return Err("棋盤尺寸必須大於 0".to_string());
        }
        if self.max_player_units == 0 {
            return Err("人數上限必須大於 0".to_string());
        }
        if (self.player_placement_positions.len() as u32) < self.max_player_units {
            return Err(format!(
                "玩家放置點數量 ({}) 少於上限 ({})",
                self.player_placement_positions.len(),
                self.max_player_units
            ));
        }

        // 檢查玩家部署點超出棋盤範圍
        for (idx, pos) in self.player_placement_positions.iter().enumerate() {
            check_position_in_bounds(&self, *pos, idx + 1, "玩家部署點")?;
        }

        // 檢查敵人位置超出棋盤範圍
        for (idx, unit) in self.enemy_units.iter().enumerate() {
            check_position_in_bounds(&self, unit.position, idx + 1, "敵人")?;
        }

        // 檢查物件位置超出棋盤範圍
        for (idx, obj) in self.object_placements.iter().enumerate() {
            check_position_in_bounds(&self, obj.position, idx + 1, "物件")?;
        }

        // 檢查玩家部署點互相重複
        let player_positions_set: HashSet<Position> =
            self.player_placement_positions.iter().cloned().collect();
        if player_positions_set.len() != self.player_placement_positions.len() {
            return Err("玩家部署點存在重複位置".to_string());
        }

        // 檢查敵人位置互相重複
        let enemy_positions_set: HashSet<Position> =
            self.enemy_units.iter().map(|u| u.position).collect();
        if enemy_positions_set.len() != self.enemy_units.len() {
            return Err("敵人位置存在重複".to_string());
        }

        // 檢查玩家部署點與敵人位置不重複
        if !player_positions_set.is_disjoint(&enemy_positions_set) {
            return Err("玩家部署點和敵人位置存在重複".to_string());
        }

        Ok(())
    }

    fn after_confirm(&mut self) {
        // 按位置排序（X 座標優先，再按 Y 座標）
        self.player_placement_positions
            .sort_by_key(|pos| (pos.x, pos.y));
        self.enemy_units
            .sort_by_key(|unit| (unit.position.x, unit.position.y));
        self.object_placements
            .sort_by_key(|obj| (obj.position.x, obj.position.y));
    }
}

/// 取得關卡的檔案名稱
pub fn file_name() -> &'static str {
    "levels"
}

// ==================== 本地輔助函數 ====================

fn check_position_in_bounds(
    level: &LevelType,
    pos: Position,
    index: usize,
    label: &str,
) -> Result<(), String> {
    if !is_position_in_bounds(level, pos) {
        return Err(format!(
            "{} #{} ({}, {}) 超出棋盤範圍 (寬: {}, 高: {})",
            label, index, pos.x, pos.y, level.board_width, level.board_height
        ));
    }
    Ok(())
}

/// 在 ComboBox 中渲染過濾後的選項
fn render_filtered_options(
    ui: &mut egui::Ui,
    visible_items: &[&TypeName],
    selected_value: &mut String,
    query: &str,
) {
    if !query.is_empty() && visible_items.is_empty() {
        ui.label("找不到符合的項目");
    } else {
        for item_name in visible_items {
            ui.selectable_value(selected_value, item_name.to_string(), item_name.as_str());
        }
    }
}

// ==================== 表單渲染 ====================

/// 渲染關卡編輯表單
pub fn render_form(ui: &mut egui::Ui, level: &mut LevelType, ui_state: &mut LevelTabUIState) {
    match ui_state.mode {
        LevelTabMode::Edit => render_edit_form(ui, level, ui_state),
        LevelTabMode::Deploy | LevelTabMode::Battle => {
            // 繪製半透明遮罩，完全遮蔽背景
            let viewport = ui.ctx().viewport_rect();
            ui.painter()
                .rect_filled(viewport, 0.0, egui::Color32::from_black_alpha(200));

            // 根據模式決定窗口標題和渲染函數
            match ui_state.mode {
                LevelTabMode::Deploy => {
                    egui::Window::new("🎮 單位部署")
                        .fixed_pos(viewport.min)
                        .fixed_size(viewport.size())
                        .resizable(false)
                        .collapsible(false)
                        .show(ui.ctx(), |ui| {
                            deployment::render_deployment_form(ui, level, ui_state);
                        });
                }
                LevelTabMode::Battle => {
                    egui::Window::new("⚔️ 模擬戰鬥")
                        .fixed_pos(viewport.min)
                        .fixed_size(viewport.size())
                        .resizable(false)
                        .collapsible(false)
                        .show(ui.ctx(), |ui| {
                            battle::render_battle_form(ui, level, ui_state);
                        });
                }
                _ => unreachable!(),
            }
        }
    }
}

/// 渲染編輯模式的表單
fn render_edit_form(ui: &mut egui::Ui, level: &mut LevelType, ui_state: &mut LevelTabUIState) {
    // 基本資訊區
    ui.horizontal(|ui| {
        ui.label("名稱：");
        ui.text_edit_singleline(&mut level.name);
    });

    ui.horizontal(|ui| {
        ui.label("棋盤寬度：");
        ui.add(
            egui::DragValue::new(&mut level.board_width)
                .speed(DRAG_VALUE_SPEED)
                .range(1..=Coord::MAX),
        );
        ui.add_space(SPACING_SMALL);
        ui.label("棋盤高度：");
        ui.add(
            egui::DragValue::new(&mut level.board_height)
                .speed(DRAG_VALUE_SPEED)
                .range(1..=Coord::MAX),
        );
    });

    ui.add_space(SPACING_MEDIUM);
    ui.separator();

    // 玩家放置點配置區
    ui.vertical(|ui| {
        ui.label("玩家人數上限：");
        ui.add(
            egui::DragValue::new(&mut level.max_player_units)
                .speed(DRAG_VALUE_SPEED)
                .range(1..=6),
        );
        ui.add_space(SPACING_SMALL);
        ui.heading("玩家放置點");
        render_placement_positions_list(ui, &mut level.player_placement_positions);
    });

    ui.add_space(SPACING_MEDIUM);
    ui.separator();

    // 敵人單位配置區
    ui.heading("敵人單位配置");
    render_unit_placement_list(
        ui,
        &mut level.enemy_units,
        &ui_state.available_units,
        &mut ui_state.unit_search_query,
    );

    ui.add_space(SPACING_MEDIUM);
    ui.separator();

    // 物件配置區
    ui.heading("物件配置");
    render_object_placement_list(
        ui,
        &mut level.object_placements,
        &ui_state.available_objects,
        &mut ui_state.object_search_query,
    );

    ui.add_space(SPACING_MEDIUM);
    ui.separator();

    // 戰場預覽區
    render_battlefield_preview(ui, level, ui_state);
}

/// 渲染玩家放置點列表
fn render_placement_positions_list(ui: &mut egui::Ui, positions: &mut Vec<Position>) {
    if ui.button("新增放置點").clicked() {
        positions.push(Position::default());
    }

    let mut to_remove = None;
    for (index, position) in positions.iter_mut().enumerate() {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(format!("#{}", index + 1));
                if ui.button("刪除").clicked() {
                    to_remove = Some(index);
                }

                ui.separator();

                ui.label("X：");
                ui.add(
                    egui::DragValue::new(&mut position.x)
                        .speed(DRAG_VALUE_SPEED)
                        .range(0..=Coord::MAX),
                );
                ui.label("Y：");
                ui.add(
                    egui::DragValue::new(&mut position.y)
                        .speed(DRAG_VALUE_SPEED)
                        .range(0..=Coord::MAX),
                );
            });
        });
        ui.add_space(SPACING_SMALL);
    }

    if let Some(index) = to_remove {
        positions.remove(index);
    }
}

/// 渲染單位配置列表
fn render_unit_placement_list(
    ui: &mut egui::Ui,
    placements: &mut Vec<UnitPlacement>,
    available_units: &[TypeName],
    unit_search_query: &mut TypeName,
) {
    if ui.button("新增單位").clicked() {
        placements.push(UnitPlacement::default());
    }

    let mut to_remove = None;
    for (index, placement) in placements.iter_mut().enumerate() {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(format!("#{}", index + 1));
                if ui.button("刪除").clicked() {
                    to_remove = Some(index);
                }

                ui.separator();

                ui.label("X：");
                ui.add(
                    egui::DragValue::new(&mut placement.position.x)
                        .speed(DRAG_VALUE_SPEED)
                        .range(0..=Coord::MAX),
                );
                ui.label("Y：");
                ui.add(
                    egui::DragValue::new(&mut placement.position.y)
                        .speed(DRAG_VALUE_SPEED)
                        .range(0..=Coord::MAX),
                );

                ui.separator();

                ui.label("單位類型：");
                if available_units.is_empty() {
                    ui.label("（尚未定義任何單位）");
                } else {
                    egui::ComboBox::from_id_salt(format!("unit_placement_{}", index))
                        .selected_text(if placement.unit_type_name.is_empty() {
                            "選擇單位"
                        } else {
                            &placement.unit_type_name
                        })
                        .height(COMBOBOX_MIN_HEIGHT)
                        .show_ui(ui, |ui| {
                            ui.set_min_width(COMBOBOX_MIN_WIDTH);

                            let response = render_search_input(ui, unit_search_query);
                            ui.memory_mut(|mem| mem.request_focus(response.id));
                            ui.separator();
                            let visible_units =
                                filter_by_search(available_units, unit_search_query);
                            render_filtered_options(
                                ui,
                                &visible_units,
                                &mut placement.unit_type_name,
                                unit_search_query,
                            );
                        });
                }
            });
        });
        ui.add_space(SPACING_SMALL);
    }

    if let Some(index) = to_remove {
        placements.remove(index);
    }
}

/// 渲染物件配置列表
fn render_object_placement_list(
    ui: &mut egui::Ui,
    placements: &mut Vec<ObjectPlacement>,
    available_objects: &[TypeName],
    object_search_query: &mut TypeName,
) {
    if ui.button("新增物件").clicked() {
        placements.push(ObjectPlacement::default());
    }

    let mut to_remove = None;
    for (index, placement) in placements.iter_mut().enumerate() {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(format!("#{}", index + 1));
                if ui.button("刪除").clicked() {
                    to_remove = Some(index);
                }

                ui.separator();

                ui.label("X：");
                ui.add(
                    egui::DragValue::new(&mut placement.position.x)
                        .speed(DRAG_VALUE_SPEED)
                        .range(0..=Coord::MAX),
                );
                ui.label("Y：");
                ui.add(
                    egui::DragValue::new(&mut placement.position.y)
                        .speed(DRAG_VALUE_SPEED)
                        .range(0..=Coord::MAX),
                );

                ui.separator();

                ui.label("物件類型：");
                if available_objects.is_empty() {
                    ui.label("（尚未定義任何物件）");
                } else {
                    egui::ComboBox::from_id_salt(format!("object_placement_{}", index))
                        .selected_text(if placement.object_type_name.is_empty() {
                            "選擇物件"
                        } else {
                            &placement.object_type_name
                        })
                        .height(COMBOBOX_MIN_HEIGHT)
                        .show_ui(ui, |ui| {
                            ui.set_min_width(COMBOBOX_MIN_WIDTH);

                            let response = render_search_input(ui, object_search_query);
                            ui.memory_mut(|mem| mem.request_focus(response.id));
                            ui.separator();
                            let visible_objects =
                                filter_by_search(available_objects, object_search_query);
                            render_filtered_options(
                                ui,
                                &visible_objects,
                                &mut placement.object_type_name,
                                object_search_query,
                            );
                        });
                }
            });
        });
        ui.add_space(SPACING_SMALL);
    }

    if let Some(index) = to_remove {
        placements.remove(index);
    }
}

/// 渲染戰場預覽，支持拖曳修改位置
fn render_battlefield_preview(
    ui: &mut egui::Ui,
    level: &mut LevelType,
    ui_state: &mut LevelTabUIState,
) {
    ui.horizontal(|ui| {
        ui.heading("戰場預覽");

        if ui.button("🎮 開始部署").clicked() {
            ui_state.mode = LevelTabMode::Deploy;
            ui_state.simulation_state = SimulationState::default();
        }
    });

    let scroll_output = egui::ScrollArea::both()
        .auto_shrink([false; 2])
        .max_width(ui.available_width() - SPACING_MEDIUM)
        .min_scrolled_height(LIST_PANEL_MIN_HEIGHT)
        .show(ui, |ui: &mut egui::Ui| {
            let (total_width, total_height) = calculate_grid_dimensions(level);

            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(total_width, total_height),
                egui::Sense::click_and_drag(),
            );

            let mut drag_state = ui_state.drag_state;

            // 檢測拖曳開始
            if response.drag_started() {
                if let Some(pos) = response
                    .hover_pos()
                    .and_then(|p| screen_to_board_pos(p, rect, level))
                {
                    if let Some(dragged) = identify_dragged_object(level, &pos) {
                        drag_state = Some(DragState { object: dragged });
                    }
                }
            }

            // 計算拖曳預覽位置
            let hovered_in_bounds = if drag_state.is_some() {
                response
                    .hover_pos()
                    .and_then(|p| screen_to_board_pos(p, rect, level))
            } else {
                None
            };

            // 檢測拖曳結束（當拖曳停止且有拖曳狀態時）
            if !response.dragged() && drag_state.is_some() {
                if let Some(state) = drag_state {
                    if let Some(new_pos) = hovered_in_bounds {
                        apply_drag_update(level, state, new_pos);
                    }
                }
                drag_state = None;
            }

            // 保存拖曳狀態
            ui_state.drag_state = drag_state;

            // 計算可見範圍（視口裁剪優化）
            let viewport_size = ui.clip_rect().size();
            let visible_range =
                calculate_visible_range(ui_state.scroll_offset, viewport_size, level);

            // 在更新後重新建立 lookup maps
            let (player_positions, enemy_units_map, objects_map) = prepare_lookup_maps(level);

            grid::render_grid(
                ui,
                rect,
                &player_positions,
                &enemy_units_map,
                &objects_map,
                drag_state,
                hovered_in_bounds,
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

    // 儲存滾動位置供下一幀使用
    ui_state.scroll_offset = scroll_output.state.offset;

    ui.add_space(SPACING_SMALL);
    render_battlefield_legend(ui);
}

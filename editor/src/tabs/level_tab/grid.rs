//! 棋盤網格渲染邏輯

use super::SimulationState;
use crate::constants::*;
use board::alias::{Coord, SkillName, TypeName};
use board::component::Position;
use board::loader_schema::{LevelType, ObjectPlacement, SkillType, UnitPlacement, UnitType};
use std::collections::{HashMap, HashSet};

/// 棋盤可見範圍（用於視口裁剪）
#[derive(Clone, Copy, Debug)]
pub struct VisibleGridRange {
    pub min: Position,
    pub max: Position,
}

/// 拖曳物體的類型和索引
#[derive(Clone, Copy, Debug)]
pub enum DraggedObject {
    Player(usize),
    Enemy(usize),
    Object(usize),
}

/// 拖曳狀態
#[derive(Clone, Copy, Debug)]
pub struct DragState {
    pub object: DraggedObject,
}

// ==================== 準備層 ====================

/// 建立查詢表以加速格子內容查詢
pub fn prepare_lookup_maps(
    level: &LevelType,
) -> (
    HashSet<Position>,
    HashMap<Position, &UnitPlacement>,
    HashMap<Position, &ObjectPlacement>,
) {
    let player_positions: HashSet<Position> =
        level.player_placement_positions.iter().cloned().collect();
    let enemy_units_map: HashMap<Position, &UnitPlacement> =
        level.enemy_units.iter().map(|u| (u.position, u)).collect();
    let objects_map: HashMap<Position, &ObjectPlacement> = level
        .object_placements
        .iter()
        .map(|o| (o.position, o))
        .collect();

    (player_positions, enemy_units_map, objects_map)
}

/// 計算棋盤預覽的總尺寸
pub fn calculate_grid_dimensions(level: &LevelType) -> (f32, f32) {
    let total_width = level.board_width as f32 * (BATTLEFIELD_CELL_SIZE + BATTLEFIELD_GRID_SPACING)
        - BATTLEFIELD_GRID_SPACING;
    let total_height = level.board_height as f32
        * (BATTLEFIELD_CELL_SIZE + BATTLEFIELD_GRID_SPACING)
        - BATTLEFIELD_GRID_SPACING;

    (total_width, total_height)
}

/// 計算可見範圍內的格子索引（用於視口裁剪）
pub fn calculate_visible_range(
    scroll_offset: egui::Vec2,
    viewport_size: egui::Vec2,
    level: &LevelType,
) -> VisibleGridRange {
    let cell_size = BATTLEFIELD_CELL_SIZE + BATTLEFIELD_GRID_SPACING;

    let min_x = (scroll_offset.x / cell_size).floor().max(0.0) as Coord;
    let max_x = ((scroll_offset.x + viewport_size.x) / cell_size)
        .ceil()
        .min(level.board_width as f32) as Coord;

    let min_y = (scroll_offset.y / cell_size).floor().max(0.0) as Coord;
    let max_y = ((scroll_offset.y + viewport_size.y) / cell_size)
        .ceil()
        .min(level.board_height as f32) as Coord;

    VisibleGridRange {
        min: Position { x: min_x, y: min_y },
        max: Position { x: max_x, y: max_y },
    }
}

// ==================== 座標轉換層 ====================

/// 將螢幕座標轉換為棋盤座標
pub fn screen_to_board_pos(
    screen_pos: egui::Pos2,
    rect: egui::Rect,
    level: &LevelType,
) -> Option<Position> {
    let rel_x = screen_pos.x - rect.min.x;
    let rel_y = screen_pos.y - rect.min.y;

    if rel_x < 0.0 || rel_y < 0.0 {
        return None;
    }

    let x = (rel_x / (BATTLEFIELD_CELL_SIZE + BATTLEFIELD_GRID_SPACING)) as Coord;
    let y = (rel_y / (BATTLEFIELD_CELL_SIZE + BATTLEFIELD_GRID_SPACING)) as Coord;

    let pos = Position { x, y };
    is_position_in_bounds(level, pos).then_some(pos)
}

/// 檢查位置是否在棋盤邊界內
pub fn is_position_in_bounds(level: &LevelType, pos: Position) -> bool {
    pos.x < level.board_width && pos.y < level.board_height
}

// ==================== 渲染層 ====================

/// 繪製棋盤格子，支持拖曳（編輯模式）
pub fn render_grid(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    player_positions: &HashSet<Position>,
    enemy_units_map: &HashMap<Position, &UnitPlacement>,
    objects_map: &HashMap<Position, &ObjectPlacement>,
    drag_state: Option<DragState>,
    hovered_in_bounds: Option<Position>,
    visible_range: VisibleGridRange,
) {
    let painter = ui.painter();
    for y in visible_range.min.y..visible_range.max.y {
        for x in visible_range.min.x..visible_range.max.x {
            let pos = Position { x, y };

            // 計算每個格子的左上角座標
            let cell_x = rect.min.x + x as f32 * (BATTLEFIELD_CELL_SIZE + BATTLEFIELD_GRID_SPACING);
            let cell_y = rect.min.y + y as f32 * (BATTLEFIELD_CELL_SIZE + BATTLEFIELD_GRID_SPACING);
            let cell_rect = egui::Rect::from_min_size(
                egui::pos2(cell_x, cell_y),
                egui::vec2(BATTLEFIELD_CELL_SIZE, BATTLEFIELD_CELL_SIZE),
            );

            // 判斷拖曳預覽位置
            let is_drag_target = drag_state.is_some() && hovered_in_bounds == Some(pos);

            // 決定格子內容與背景顏色
            let (cell_text, bg_color) = if player_positions.contains(&pos) {
                ("".to_string(), BATTLEFIELD_COLOR_PLAYER)
            } else if let Some(unit) = enemy_units_map.get(&pos) {
                let abbrev = unit.unit_type_name.chars().take(2).collect::<TypeName>();
                (abbrev, BATTLEFIELD_COLOR_ENEMY)
            } else if let Some(obj) = objects_map.get(&pos) {
                let abbrev = obj.object_type_name.chars().take(2).collect::<TypeName>();
                (abbrev, BATTLEFIELD_COLOR_OBJECT)
            } else {
                ("".to_string(), BATTLEFIELD_COLOR_EMPTY)
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

            // 拖曳預覽：外框高亮目標位置
            if is_drag_target {
                painter.rect_stroke(
                    cell_rect,
                    0.0,
                    egui::Stroke::new(STROKE_WIDTH, BATTLEFIELD_COLOR_DRAG_HIGHLIGHT),
                    egui::epaint::StrokeKind::Outside,
                );
            }
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
    _skills_map: &HashMap<SkillName, SkillType>,
    _units_map: &HashMap<TypeName, UnitType>,
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
                    egui::Stroke::new(STROKE_WIDTH, BATTLEFIELD_COLOR_PLAYER_SELECTED),
                    egui::epaint::StrokeKind::Outside,
                );
            }
        }
    }
}

/// 渲染懸停提示
pub fn render_hover_tooltip(
    ui: &mut egui::Ui,
    level: &LevelType,
    rect: egui::Rect,
    response: &egui::Response,
    player_positions: &HashSet<Position>,
    enemy_units_map: &HashMap<Position, &UnitPlacement>,
    objects_map: &HashMap<Position, &ObjectPlacement>,
) {
    if let Some(hover_pos) = response.hover_pos() {
        let rel_x = hover_pos.x - rect.min.x;
        let rel_y = hover_pos.y - rect.min.y;

        let hover_x = (rel_x / (BATTLEFIELD_CELL_SIZE + BATTLEFIELD_GRID_SPACING)) as Coord;
        let hover_y = (rel_y / (BATTLEFIELD_CELL_SIZE + BATTLEFIELD_GRID_SPACING)) as Coord;

        let hovered_pos = Position {
            x: hover_x,
            y: hover_y,
        };

        // 邊界檢查
        if !is_position_in_bounds(level, hovered_pos) {
            return;
        }

        // 根據該格子內容顯示懸停文本
        let hover_text = if player_positions.contains(&hovered_pos) {
            format!("({}, {}) 玩家放置點", hover_x, hover_y)
        } else if let Some(unit) = enemy_units_map.get(&hovered_pos) {
            format!("({}, {}) {}", hover_x, hover_y, unit.unit_type_name.clone())
        } else if let Some(obj) = objects_map.get(&hovered_pos) {
            format!(
                "({}, {}) {}",
                hover_x,
                hover_y,
                obj.object_type_name.clone()
            )
        } else {
            format!("({}, {})", hover_x, hover_y)
        };

        // 計算文本寬度
        let galley = ui.painter().layout_no_wrap(
            hover_text.clone(),
            egui::FontId::default(),
            egui::Color32::BLACK,
        );
        let text_width = galley.rect.width();
        let tooltip_width = text_width * 3.0;

        let viewport_rect = ui.ctx().viewport_rect();
        let tooltip_x = if hover_pos.x + tooltip_width + SPACING_MEDIUM > viewport_rect.right() {
            // 右邊太窄，顯示在左邊
            hover_pos.x - tooltip_width - SPACING_MEDIUM
        } else {
            // 右邊有足夠空間，顯示在右邊
            hover_pos.x + SPACING_MEDIUM
        };

        egui::Area::new(egui::Id::new("battlefield_hover_tooltip"))
            .fixed_pos(egui::pos2(tooltip_x, hover_pos.y + SPACING_MEDIUM))
            .show(ui.ctx(), |ui| {
                ui.set_max_width(tooltip_width);
                ui.label(
                    egui::RichText::new(&hover_text)
                        .color(egui::Color32::BLACK)
                        .background_color(egui::Color32::GRAY),
                );
            });
    }
}

/// 渲染戰場圖例
pub fn render_battlefield_legend(ui: &mut egui::Ui) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label("圖例：");

            // 玩家放置點
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(BATTLEFIELD_CELL_SIZE, BATTLEFIELD_CELL_SIZE),
                egui::Sense::empty(),
            );
            ui.painter()
                .rect_filled(rect, 0.0, BATTLEFIELD_COLOR_PLAYER);
            ui.label("玩家放置點");

            ui.label("｜");

            // 敵人單位
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(BATTLEFIELD_CELL_SIZE, BATTLEFIELD_CELL_SIZE),
                egui::Sense::empty(),
            );
            ui.painter().rect_filled(rect, 0.0, BATTLEFIELD_COLOR_ENEMY);
            ui.label("敵人單位");

            ui.label("｜");

            // 物件
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(BATTLEFIELD_CELL_SIZE, BATTLEFIELD_CELL_SIZE),
                egui::Sense::empty(),
            );
            ui.painter()
                .rect_filled(rect, 0.0, BATTLEFIELD_COLOR_OBJECT);
            ui.label("物件");
        });
    });
}

// ==================== 交互層 ====================

/// 識別被拖曳的物體及其索引
pub fn identify_dragged_object(level: &LevelType, pos: &Position) -> Option<DraggedObject> {
    // 優先檢查玩家部署點
    for (idx, player_pos) in level.player_placement_positions.iter().enumerate() {
        if player_pos == pos {
            return Some(DraggedObject::Player(idx));
        }
    }

    // 檢查敵人單位
    for (idx, unit) in level.enemy_units.iter().enumerate() {
        if unit.position == *pos {
            return Some(DraggedObject::Enemy(idx));
        }
    }

    // 檢查物件
    for (idx, obj) in level.object_placements.iter().enumerate() {
        if obj.position == *pos {
            return Some(DraggedObject::Object(idx));
        }
    }

    None
}

/// 應用拖曳更新
pub fn apply_drag_update(level: &mut LevelType, state: DragState, new_pos: Position) {
    match state.object {
        DraggedObject::Player(idx) => {
            if idx < level.player_placement_positions.len() {
                level.player_placement_positions[idx] = new_pos;
            }
        }
        DraggedObject::Enemy(idx) => {
            if idx < level.enemy_units.len() {
                level.enemy_units[idx].position = new_pos;
            }
        }
        DraggedObject::Object(idx) => {
            if idx < level.object_placements.len() {
                level.object_placements[idx].position = new_pos;
            }
        }
    }
}

//! 關卡編輯器 tab

use crate::constants::*;
use crate::editor_item::EditorItem;
use board::alias::{Coord, TypeName};
use board::component::Position;
use board::loader_schema::{LevelType, ObjectPlacement, UnitPlacement};
use std::collections::{HashMap, HashSet};

// ==================== EditorItem 實作 ====================

impl EditorItem for LevelType {
    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn type_name() -> &'static str {
        "關卡"
    }

    fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("名稱不能為空".to_string());
        }
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
        Ok(())
    }
}

/// 取得關卡的檔案名稱
pub fn file_name() -> &'static str {
    "levels"
}

// ==================== 表單渲染 ====================

/// 建立查詢表以加速格子內容查詢
fn prepare_lookup_maps(
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
fn calculate_grid_dimensions(level: &LevelType) -> (f32, f32) {
    let total_width = level.board_width as f32 * (BATTLEFIELD_CELL_SIZE + BATTLEFIELD_GRID_SPACING)
        - BATTLEFIELD_GRID_SPACING;
    let total_height = level.board_height as f32
        * (BATTLEFIELD_CELL_SIZE + BATTLEFIELD_GRID_SPACING)
        - BATTLEFIELD_GRID_SPACING;

    (total_width, total_height)
}

/// 繪製棋盤格子
fn render_grid(
    ui: &mut egui::Ui,
    level: &LevelType,
    rect: egui::Rect,
    player_positions: &HashSet<Position>,
    enemy_units_map: &HashMap<Position, &UnitPlacement>,
    objects_map: &HashMap<Position, &ObjectPlacement>,
) {
    let painter = ui.painter();
    for y in 0..level.board_height {
        for x in 0..level.board_width {
            let pos = Position { x, y };

            // 計算每個格子的左上角座標
            let cell_x = rect.min.x + x as f32 * (BATTLEFIELD_CELL_SIZE + BATTLEFIELD_GRID_SPACING);
            let cell_y = rect.min.y + y as f32 * (BATTLEFIELD_CELL_SIZE + BATTLEFIELD_GRID_SPACING);
            let cell_rect = egui::Rect::from_min_size(
                egui::pos2(cell_x, cell_y),
                egui::vec2(BATTLEFIELD_CELL_SIZE, BATTLEFIELD_CELL_SIZE),
            );

            // 決定格子內容與背景顏色
            let (cell_text, bg_color) = if player_positions.contains(&pos) {
                ("".to_string(), BATTLEFIELD_COLOR_PLAYER)
            } else if let Some(unit) = enemy_units_map.get(&pos) {
                let abbrev = unit.unit_type_name.chars().take(2).collect::<String>();
                (abbrev, BATTLEFIELD_COLOR_ENEMY)
            } else if let Some(obj) = objects_map.get(&pos) {
                let abbrev = obj.object_type_name.chars().take(2).collect::<String>();
                (abbrev, BATTLEFIELD_COLOR_OBJECT)
            } else {
                ("".to_string(), BATTLEFIELD_COLOR_EMPTY)
            };

            // 繪製格子
            painter.rect_filled(cell_rect, 0.0, bg_color);
            painter.text(
                cell_rect.center(),
                egui::Align2::CENTER_CENTER,
                cell_text,
                egui::FontId::proportional(BATTLEFIELD_TEXT_SIZE),
                egui::Color32::BLACK,
            );
        }
    }
}

/// 渲染懸停提示
fn render_hover_tooltip(
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

        // 邊界檢查
        if hover_x >= level.board_width && hover_y >= level.board_height {
            return;
        }

        let hovered_pos = Position {
            x: hover_x,
            y: hover_y,
        };

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
fn render_battlefield_legend(ui: &mut egui::Ui) {
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

/// 渲染戰場預覽
fn render_battlefield_preview(ui: &mut egui::Ui, level: &LevelType) {
    ui.heading("戰場預覽");

    let (player_positions, enemy_units_map, objects_map) = prepare_lookup_maps(level);
    let (total_width, total_height) = calculate_grid_dimensions(level);

    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(total_width, total_height), egui::Sense::hover());

    render_grid(
        ui,
        level,
        rect,
        &player_positions,
        &enemy_units_map,
        &objects_map,
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

    ui.add_space(SPACING_SMALL);
    render_battlefield_legend(ui);
}

/// 渲染關卡編輯表單
pub fn render_form(
    ui: &mut egui::Ui,
    level: &mut LevelType,
    available_units: &[TypeName],
    available_objects: &[TypeName],
) {
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
    render_unit_placement_list(ui, &mut level.enemy_units, available_units);

    ui.add_space(SPACING_MEDIUM);
    ui.separator();

    // 物件配置區
    ui.heading("物件配置");
    render_object_placement_list(ui, &mut level.object_placements, available_objects);

    ui.add_space(SPACING_MEDIUM);
    ui.separator();

    // 戰場預覽區
    render_battlefield_preview(ui, level);
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
    available_units: &[String],
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
                        .show_ui(ui, |ui| {
                            for unit_name in available_units {
                                ui.selectable_value(
                                    &mut placement.unit_type_name,
                                    unit_name.clone(),
                                    unit_name,
                                );
                            }
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
    available_objects: &[String],
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
                        .show_ui(ui, |ui| {
                            for object_name in available_objects {
                                ui.selectable_value(
                                    &mut placement.object_type_name,
                                    object_name.clone(),
                                    object_name,
                                );
                            }
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

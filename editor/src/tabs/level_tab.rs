//! 關卡編輯器 tab

use crate::constants::*;
use crate::editor_item::EditorItem;
use board::alias::{Coord, TypeName};
use board::component::Position;
use board::loader_schema::{LevelType, ObjectPlacement, UnitPlacement};

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
        ui.add_space(SPACING_SMALL);
        ui.label("玩家人數上限：");
        ui.add(
            egui::DragValue::new(&mut level.max_player_units)
                .speed(DRAG_VALUE_SPEED)
                .range(1..=6),
        );
    });

    ui.add_space(SPACING_MEDIUM);
    ui.separator();

    // 玩家放置點配置區
    ui.heading("玩家放置點");
    render_placement_positions_list(ui, &mut level.player_placement_positions);

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

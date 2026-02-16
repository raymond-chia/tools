//! 單位詳情展示相關的共用函數

use super::{LevelTabUIState, SimulationState, screen_to_board_pos};
use crate::constants::{LIST_PANEL_WIDTH, SPACING_SMALL};
use board::alias::{SkillName, TypeName};
use board::component::Position;
use board::loader_schema::{LevelType, SkillType, TriggerEvent, UnitPlacement, UnitType};
use std::collections::{HashMap, HashSet};

/// 處理右鍵點擊選擇單位
pub fn handle_unit_right_click(
    response: &egui::Response,
    rect: egui::Rect,
    level: &LevelType,
    player_positions: &HashSet<Position>,
    enemy_units_map: &HashMap<Position, &UnitPlacement>,
    ui_state: &mut LevelTabUIState,
) {
    // Fail fast：檢查是否右鍵點擊
    if !response.secondary_clicked() {
        return;
    }

    let clicked_pos = match response
        .interact_pointer_pos()
        .and_then(|p| screen_to_board_pos(p, rect, level))
    {
        None => return,
        Some(pos) => pos,
    };

    if let Some(unit_type_name) = get_unit_at_position(
        clicked_pos,
        level,
        player_positions,
        enemy_units_map,
        &ui_state.simulation_state,
    ) {
        ui_state.temp_unit_name = Some(unit_type_name);
    }
}

/// 渲染單位詳情側邊面板（含容器、標題、關閉按鈕）
pub fn render_unit_details_side_panel(
    ui: &mut egui::Ui,
    unit_name: &TypeName,
    ui_state: &mut LevelTabUIState,
) {
    ui.vertical(|ui| {
        ui.set_width(LIST_PANEL_WIDTH);

        // 標題列（含關閉按鈕）
        ui.horizontal(|ui| {
            ui.heading("單位詳情");
            if ui.button("X 關閉").clicked() {
                ui_state.temp_unit_name = None;
            }
        });
        ui.separator();

        // 內容區（可滾動）
        egui::ScrollArea::both()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                render_unit_details_panel(ui, unit_name, &ui_state.skills_map, &ui_state.units_map);
            });
    });
}

/// 渲染單位詳情面板的內容（不含面板容器）
fn render_unit_details_panel(
    ui: &mut egui::Ui,
    unit_type_name: &TypeName,
    skills_map: &HashMap<SkillName, SkillType>,
    units_map: &HashMap<TypeName, UnitType>,
) {
    // Fail fast：獲取單位定義
    let unit_def = match units_map.get(unit_type_name) {
        Some(unit) => unit,
        None => {
            ui.label(format!("⚠️ 單位定義未找到: {}", unit_type_name));
            return;
        }
    };

    ui.label(unit_type_name);
    ui.separator();

    // 計算屬性
    let attributes = match board::logic::unit_attributes::calculate_attributes(
        &unit_def.skills,
        &[],
        skills_map,
    ) {
        Ok(attrs) => attrs,
        Err(e) => {
            ui.label(format!("⚠️ 屬性計算失敗: {:?}", e));
            return;
        }
    };

    // 顯示被動技能
    let passive_skills: Vec<&SkillName> = unit_def
        .skills
        .iter()
        .filter_map(|skill_name| {
            skills_map.get(skill_name).and_then(|skill| {
                if skill.trigger == TriggerEvent::Passive {
                    Some(skill_name)
                } else {
                    None
                }
            })
        })
        .collect();

    if !passive_skills.is_empty() {
        ui.label("被動技能:");
        for skill_name in passive_skills {
            ui.label(format!("  • {}", &skill_name));
        }
        ui.separator();
    }

    // 顯示所有屬性
    ui.label("屬性:");
    ui.vertical(|ui| {
        ui.set_width(ui.available_width() - SPACING_SMALL * 5.0);

        ui.label(format!("HP: {}", attributes.hp));
        ui.label(format!("MP: {}", attributes.mp));
        ui.label(format!("Initiative: {}", attributes.initiative));
        ui.label(format!("Hit: {}", attributes.hit));
        ui.label(format!("Evasion: {}", attributes.evasion));
        ui.label(format!("Block: {}", attributes.block));
        ui.label(format!("Block Protection: {}", attributes.block_protection));
        ui.label(format!("Physical Attack: {}", attributes.physical_attack));
        ui.label(format!("Magical Attack: {}", attributes.magical_attack));
        ui.label(format!("Magical DC: {}", attributes.magical_dc));
        ui.label(format!("Fortitude: {}", attributes.fortitude));
        ui.label(format!("Reflex: {}", attributes.reflex));
        ui.label(format!("Will: {}", attributes.will));
        ui.label(format!("Movement: {}", attributes.movement));
        ui.label(format!(
            "Opportunity Attacks: {}",
            attributes.opportunity_attacks
        ));
    });
}

/// 取得指定位置的單位類型名稱（玩家或敵人）
fn get_unit_at_position(
    clicked_pos: Position,
    level: &LevelType,
    _player_positions: &HashSet<Position>,
    enemy_units_map: &HashMap<Position, &UnitPlacement>,
    simulation_state: &SimulationState,
) -> Option<TypeName> {
    // 檢查是否為玩家部署點
    if let Some(index) = level
        .player_placement_positions
        .iter()
        .position(|pos| *pos == clicked_pos)
    {
        if let Some(unit_type_name) = simulation_state.deployed_units.get(&index) {
            return Some(unit_type_name.clone());
        }
    }

    // 檢查是否為敵人
    if let Some(enemy_unit) = enemy_units_map.get(&clicked_pos) {
        return Some(enemy_unit.unit_type_name.clone());
    }

    None
}

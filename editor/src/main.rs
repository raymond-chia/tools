//! PF2e 戰術戰鬥編輯器
//!
//! 使用文字表示單位的簡單網格戰術戰鬥編輯器

use core::{
    abilities::{Ability, AbilityScores},
    character::{Ancestry, Character, CharacterClass},
    combat::{Attack, CombatUnit, DamageDice, DamageType, Position},
};
use eframe::egui;

const GRID_SIZE: usize = 20;
const CELL_SIZE: f32 = 40.0;

/// 戰鬥遭遇狀態
struct CombatState {
    units: Vec<CombatUnit>,
    current_turn: usize,
    selected_unit: Option<usize>,
    target_unit: Option<usize>,
    combat_log: Vec<String>,
    movement_mode: bool,
}

impl CombatState {
    fn new() -> Self {
        Self {
            units: Vec::new(),
            current_turn: 0,
            selected_unit: None,
            target_unit: None,
            combat_log: Vec::new(),
            movement_mode: false,
        }
    }

    fn add_unit(&mut self, unit: CombatUnit) {
        self.units.push(unit);
    }

    fn current_unit(&self) -> Option<&CombatUnit> {
        self.units.get(self.current_turn)
    }

    fn current_unit_mut(&mut self) -> Option<&mut CombatUnit> {
        self.units.get_mut(self.current_turn)
    }

    fn end_turn(&mut self) {
        self.log(format!(
            "{}'s turn ended",
            self.current_unit()
                .map(|u| u.character.name.as_str())
                .unwrap_or("Unknown")
        ));

        self.current_turn = (self.current_turn + 1) % self.units.len();

        if let Some(unit) = self.current_unit_mut() {
            unit.start_turn();
        }

        self.log(format!(
            "{}'s turn begins",
            self.current_unit()
                .map(|u| u.character.name.as_str())
                .unwrap_or("Unknown")
        ));

        self.movement_mode = false;
    }

    fn log(&mut self, message: String) {
        self.combat_log.push(message);
        if self.combat_log.len() > 50 {
            self.combat_log.remove(0);
        }
    }

    /// 執行打擊動作
    fn perform_strike(&mut self, attacker_idx: usize, target_idx: usize) {
        if attacker_idx >= self.units.len() || target_idx >= self.units.len() {
            return;
        }

        // 檢查攻擊者是否還有剩餘動作
        let attacker = &self.units[attacker_idx];
        if attacker.actions_remaining < 1 {
            self.log(format!(
                "{} has no actions remaining!",
                attacker.character.name
            ));
            return;
        }

        // 檢查單位是否相鄰
        let attacker_pos = self.units[attacker_idx].position;
        let target_pos = self.units[target_idx].position;

        if !attacker_pos.is_adjacent(&target_pos) {
            self.log("Target is too far away!".to_string());
            return;
        }

        // 獲取攻擊加成
        let attacker = &self.units[attacker_idx];
        let str_mod = attacker
            .character
            .ability_scores
            .modifier(Ability::Strength);
        let map_penalty = attacker.map.penalty();
        let attack_bonus = str_mod + attacker.character.level + map_penalty;

        let attacker_name = attacker.character.name.clone();

        // 創建基礎攻擊
        let damage_dice = DamageDice::new(1, 8, DamageType::Slashing);
        let attack = Attack::new("Strike".to_string(), attack_bonus, damage_dice, str_mod);

        let target_ac = self.units[target_idx].character.armor_class;
        let target_name = self.units[target_idx].character.name.clone();

        // 進行攻擊
        let result = attack.attack(target_ac);

        // 記錄攻擊
        self.log(format!(
            "{} attacks {} (rolled {})",
            attacker_name, target_name, result.attack_roll.total
        ));

        if let Some(damage) = &result.damage {
            let crit = if result.critical_hit() {
                " (CRITICAL!)"
            } else {
                ""
            };
            self.log(format!(
                "  Hit{}: {} {} damage",
                crit,
                damage.amount,
                damage.damage_type.name()
            ));

            // 施加傷害
            self.units[target_idx].character.take_damage(damage.amount);
            self.log(format!(
                "  {} HP: {}/{}",
                target_name,
                self.units[target_idx].character.current_hp,
                self.units[target_idx].character.max_hp
            ));

            if !self.units[target_idx].character.is_alive() {
                self.log(format!("  {} is defeated!", target_name));
            }
        } else {
            self.log("  Miss!".to_string());
        }

        // 使用動作並記錄攻擊以計算 MAP
        let mut error_msg = None;
        if let Some(attacker) = self.units.get_mut(attacker_idx) {
            if let Err(e) = attacker.use_action(1) {
                error_msg = Some(format!("Error using action: {}", e));
            }
            attacker.map.record_attack();
        }
        if let Some(msg) = error_msg {
            self.log(msg);
        }
    }

    /// 執行跨步動作
    fn perform_stride(&mut self, unit_idx: usize, target_pos: Position) {
        if unit_idx >= self.units.len() {
            return;
        }

        let unit = &mut self.units[unit_idx];
        let unit_name = unit.character.name.clone();

        match unit.stride(target_pos) {
            Ok(()) => {
                self.log(format!(
                    "{} moves to ({}, {})",
                    unit_name, target_pos.x, target_pos.y
                ));
            }
            Err(e) => {
                self.log(format!("{} cannot move: {}", unit_name, e));
            }
        }
    }

    /// 獲取單位可移動位置
    fn get_movable_positions(&self, unit_idx: usize) -> Vec<Position> {
        if unit_idx >= self.units.len() {
            return Vec::new();
        }

        let unit = &self.units[unit_idx];
        let speed = unit.character.ancestry.speed();
        let current_pos = unit.position;

        let mut positions = Vec::new();

        // 檢查移動範圍內的所有位置
        let range = (speed / 5) as i32; // 將英尺轉換為格數
        for dx in -range..=range {
            for dy in -range..=range {
                let new_pos = Position::new(current_pos.x + dx, current_pos.y + dy);

                // 檢查是否在網格範圍內
                if new_pos.x >= 0
                    && new_pos.x < GRID_SIZE as i32
                    && new_pos.y >= 0
                    && new_pos.y < GRID_SIZE as i32
                {
                    // 檢查是否在移動距離內
                    if current_pos.distance_to(&new_pos) <= speed {
                        // 檢查是否沒有被其他單位佔據
                        if !self.units.iter().any(|u| u.position == new_pos) {
                            positions.push(new_pos);
                        }
                    }
                }
            }
        }

        positions
    }
}

/// 主應用程式
struct TacticalEditor {
    combat: CombatState,
    hover_pos: Option<Position>,
}

impl Default for TacticalEditor {
    fn default() -> Self {
        let mut combat = CombatState::new();

        // 創建測試用的範例角色
        let fighter_abilities = AbilityScores::new(18, 14, 14, 10, 12, 10).unwrap();
        let fighter = Character::new(
            "Fighter".to_string(),
            Ancestry::Human,
            CharacterClass::Fighter,
            fighter_abilities,
        )
        .unwrap();

        let goblin_abilities = AbilityScores::new(14, 16, 12, 10, 10, 8).unwrap();
        let goblin = Character::new(
            "Goblin".to_string(),
            Ancestry::Goblin,
            CharacterClass::Rogue,
            goblin_abilities,
        )
        .unwrap();

        let wizard_abilities = AbilityScores::new(10, 14, 12, 18, 12, 10).unwrap();
        let wizard = Character::new(
            "Wizard".to_string(),
            Ancestry::Elf,
            CharacterClass::Wizard,
            wizard_abilities,
        )
        .unwrap();

        combat.add_unit(CombatUnit::new(fighter, Position::new(5, 5)));
        combat.add_unit(CombatUnit::new(goblin, Position::new(10, 10)));
        combat.add_unit(CombatUnit::new(wizard, Position::new(3, 8)));

        // 開始第一回合
        if let Some(unit) = combat.current_unit_mut() {
            unit.start_turn();
        }
        combat.log("Combat begins!".to_string());

        Self {
            combat,
            hover_pos: None,
        }
    }
}

impl eframe::App for TacticalEditor {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::right("info_panel").show(ctx, |ui| {
            ui.heading("Combat Info");

            if let Some(current) = self.combat.current_unit() {
                ui.separator();
                ui.label(format!("Current Turn: {}", current.character.name));
                ui.label(format!("Actions: {}/3", current.actions_remaining));
                ui.label(format!("MAP: {}", current.map.penalty()));
                ui.label(format!(
                    "HP: {}/{}",
                    current.character.current_hp, current.character.max_hp
                ));
                ui.label(format!("Speed: {} ft", current.character.ancestry.speed()));

                ui.separator();
                if ui.button("End Turn").clicked() {
                    self.combat.end_turn();
                }
            }

            ui.separator();
            ui.heading("Units");
            for (idx, unit) in self.combat.units.iter().enumerate() {
                let is_current = idx == self.combat.current_turn;
                let prefix = if is_current { "▶ " } else { "  " };

                if ui
                    .selectable_label(
                        self.combat.selected_unit == Some(idx),
                        format!(
                            "{}{} ({}/{}) @({},{})",
                            prefix,
                            unit.character.name,
                            unit.character.current_hp,
                            unit.character.max_hp,
                            unit.position.x,
                            unit.position.y
                        ),
                    )
                    .clicked()
                {
                    self.combat.selected_unit = Some(idx);
                    self.combat.movement_mode = false;
                }
            }

            ui.separator();
            ui.heading("Actions");

            if let Some(selected) = self.combat.selected_unit {
                if selected == self.combat.current_turn {
                    // 移動動作
                    ui.checkbox(&mut self.combat.movement_mode, "Movement Mode");

                    if self.combat.movement_mode {
                        ui.label("Click a highlighted square to move");
                    } else {
                        // 攻擊動作
                        if let Some(target) = self.combat.target_unit {
                            if ui.button("Strike (1 action)").clicked() {
                                self.combat.perform_strike(selected, target);
                            }
                        }
                    }
                }
            }

            ui.separator();
            ui.heading("Combat Log");
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for msg in &self.combat.combat_log {
                        ui.label(msg);
                    }
                });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Tactical Grid");

            let (response, painter) = ui.allocate_painter(
                egui::Vec2::new(GRID_SIZE as f32 * CELL_SIZE, GRID_SIZE as f32 * CELL_SIZE),
                egui::Sense::click(),
            );

            let rect = response.rect;

            // 繪製網格
            for x in 0..=GRID_SIZE {
                let x_pos = rect.left() + x as f32 * CELL_SIZE;
                painter.line_segment(
                    [
                        egui::pos2(x_pos, rect.top()),
                        egui::pos2(x_pos, rect.bottom()),
                    ],
                    egui::Stroke::new(1.0, egui::Color32::GRAY),
                );
            }

            for y in 0..=GRID_SIZE {
                let y_pos = rect.top() + y as f32 * CELL_SIZE;
                painter.line_segment(
                    [
                        egui::pos2(rect.left(), y_pos),
                        egui::pos2(rect.right(), y_pos),
                    ],
                    egui::Stroke::new(1.0, egui::Color32::GRAY),
                );
            }

            // 在移動模式下獲取可移動位置
            let movable_positions = if self.combat.movement_mode {
                if let Some(selected) = self.combat.selected_unit {
                    self.combat.get_movable_positions(selected)
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };

            // 高亮可移動位置
            for pos in &movable_positions {
                let cell_rect = egui::Rect::from_min_size(
                    egui::pos2(
                        rect.left() + pos.x as f32 * CELL_SIZE,
                        rect.top() + pos.y as f32 * CELL_SIZE,
                    ),
                    egui::vec2(CELL_SIZE, CELL_SIZE),
                );
                painter.rect_filled(
                    cell_rect,
                    0.0,
                    egui::Color32::from_rgba_unmultiplied(100, 255, 100, 80),
                );
            }

            // 處理滑鼠懸停和點擊
            if let Some(hover_pos) = response.hover_pos() {
                let x = ((hover_pos.x - rect.left()) / CELL_SIZE).floor() as i32;
                let y = ((hover_pos.y - rect.top()) / CELL_SIZE).floor() as i32;

                if x >= 0 && x < GRID_SIZE as i32 && y >= 0 && y < GRID_SIZE as i32 {
                    let grid_pos = Position::new(x, y);
                    self.hover_pos = Some(grid_pos);

                    // 高亮懸停的格子
                    let cell_rect = egui::Rect::from_min_size(
                        egui::pos2(
                            rect.left() + x as f32 * CELL_SIZE,
                            rect.top() + y as f32 * CELL_SIZE,
                        ),
                        egui::vec2(CELL_SIZE, CELL_SIZE),
                    );
                    painter.rect_filled(
                        cell_rect,
                        0.0,
                        egui::Color32::from_rgba_unmultiplied(100, 100, 255, 50),
                    );

                    // 處理點擊
                    if response.clicked() {
                        if self.combat.movement_mode {
                            // 嘗試移動
                            if let Some(selected) = self.combat.selected_unit {
                                if movable_positions.contains(&grid_pos) {
                                    self.combat.perform_stride(selected, grid_pos);
                                }
                            }
                        } else {
                            // 嘗試選擇/目標單位
                            let mut found = false;
                            for (idx, unit) in self.combat.units.iter().enumerate() {
                                if unit.position == grid_pos {
                                    if Some(idx) == self.combat.selected_unit {
                                        self.combat.target_unit = None;
                                    } else {
                                        self.combat.target_unit = Some(idx);
                                    }
                                    found = true;
                                    break;
                                }
                            }
                            if !found {
                                self.combat.target_unit = None;
                            }
                        }
                    }
                }
            } else {
                self.hover_pos = None;
            }

            // 繪製單位
            for (idx, unit) in self.combat.units.iter().enumerate() {
                let is_current = idx == self.combat.current_turn;
                let is_selected = self.combat.selected_unit == Some(idx);
                let is_target = self.combat.target_unit == Some(idx);
                let is_alive = unit.character.is_alive();

                let cell_rect = egui::Rect::from_min_size(
                    egui::pos2(
                        rect.left() + unit.position.x as f32 * CELL_SIZE,
                        rect.top() + unit.position.y as f32 * CELL_SIZE,
                    ),
                    egui::vec2(CELL_SIZE, CELL_SIZE),
                );

                // 背景顏色
                let bg_color = if !is_alive {
                    egui::Color32::DARK_RED
                } else if is_current {
                    egui::Color32::from_rgb(100, 255, 100)
                } else if is_target {
                    egui::Color32::from_rgb(255, 100, 100)
                } else if is_selected {
                    egui::Color32::from_rgb(100, 200, 255)
                } else {
                    egui::Color32::from_rgb(200, 200, 200)
                };

                painter.rect_filled(cell_rect, 4.0, bg_color);

                // 當前回合的邊框
                if is_current {
                    painter.rect_stroke(
                        cell_rect,
                        4.0,
                        egui::Stroke::new(3.0, egui::Color32::GREEN),
                        egui::epaint::StrokeKind::Outside,
                    );
                }

                // 繪製單位符號
                painter.text(
                    cell_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    unit.symbol(),
                    egui::FontId::proportional(24.0),
                    egui::Color32::BLACK,
                );

                // 繪製 HP 條
                let hp_ratio = unit.character.current_hp as f32 / unit.character.max_hp as f32;
                let hp_bar_rect = egui::Rect::from_min_size(
                    egui::pos2(cell_rect.left() + 2.0, cell_rect.bottom() - 6.0),
                    egui::vec2((CELL_SIZE - 4.0) * hp_ratio, 4.0),
                );
                painter.rect_filled(hp_bar_rect, 1.0, egui::Color32::RED);
            }
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    use std::path::PathBuf;

    // 初始化 i18n 系統
    let locales_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent() // 從 editor/ 往上到 tools/
        .unwrap()
        .join("locales");

    // 設定預設語言為繁體中文
    core::i18n::init(&locales_dir, "zh-TW").expect("Failed to initialize i18n");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native(
        "PF2e Tactical Combat Editor",
        options,
        Box::new(|_cc| Ok(Box::new(TacticalEditor::default()))),
    )
}

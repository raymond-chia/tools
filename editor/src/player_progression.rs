// 玩家進度編輯器模組
use crate::common::*;
use chess_lib::*;
use egui::*;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use skills_lib::*;
use std::collections::{BTreeMap, BTreeSet};
use std::io;

/// 每個戰場對應的玩家進度（roster）
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct PlayerProgressionData {
    #[serde(default)]
    pub boards: BTreeMap<BoardID, PlayerProgress>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct PlayerProgress {
    pub roster: BTreeMap<UnitTemplateType, Unit>,
}

/// 玩家在某戰場的 roster 狀態（可依需求擴充）
/// 玩家單位資料，技能分為主動與被動
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Unit {
    pub unit_type: UnitTemplateType,
    pub active_skills: BTreeSet<SkillID>,
    pub passive_skills: BTreeSet<SkillID>,
    /// 行動優先值，預設為 0
    #[serde(default)]
    pub initiative: u32,
}

#[derive(Debug, Default)]
pub struct PlayerProgressionEditor {
    // 其他編輯器的資料
    unit_templates: IndexMap<UnitTemplateType, UnitTemplate>,
    skills: BTreeMap<SkillID, Skill>,
    active_skill_ids: Vec<SkillID>,
    passive_skill_ids: Vec<SkillID>,
    // 本編輯器的資料
    selected_board: Option<BoardID>,
    // 新增單位暫存欄位
    new_unit_type: Option<UnitTemplateType>,
    data: PlayerProgressionData,
    has_unsaved_changes: bool,
    status_message: Option<(String, bool)>,
}

impl PlayerProgressionEditor {
    pub fn new() -> Self {
        let mut editor = Self::default();
        editor.reload();
        editor
    }

    /// 重新載入玩家進度資料，並根據 BOARDS_FILE 補齊所有 boards
    pub fn reload(&mut self) {
        // 1. 解析 boards 檔案，取得所有 board id
        let board_map: BTreeMap<String, BoardConfig> = from_file(BOARDS_FILE).unwrap_or_default();
        let board_ids: Vec<String> = board_map.keys().cloned().collect();

        // 2. 讀取玩家進度資料
        let mut data = match from_file::<_, PlayerProgressionData>(PROGRESSION_FILE) {
            Ok(d) => d,
            Err(err) => {
                self.set_status(format!("載入玩家進度失敗: {}", err), true);
                return;
            }
        };

        // 3. 補齊 boards
        for board_id in &board_ids {
            data.boards
                .entry(board_id.clone())
                .or_insert_with(PlayerProgress::default);
        }

        // 4. 移除多餘的 boards（只保留 boards 檔案中有的）
        data.boards.retain(|k, _| board_ids.contains(k));
        self.data = data;

        // 5. 載入 unit_templates（參考 boards.rs）
        self.unit_templates = match crate::units::load_unit_templates(UNIT_TEMPLATES_FILE) {
            Ok(unit_templates) => unit_templates
                .into_iter()
                .map(|u| (u.name.clone(), u))
                .collect(),
            Err(err) => {
                self.set_status(format!("載入單位失敗: {}", err), true);
                return;
            }
        };

        // 6. 載入 skills（參考 boards.rs）
        match load_skills(SKILLS_FILE) {
            Ok((skills, active_skill_ids, passive_skill_ids)) => {
                self.skills = skills;
                self.active_skill_ids = active_skill_ids;
                self.passive_skill_ids = passive_skill_ids;
            }
            Err(err) => {
                self.skills = BTreeMap::new();
                self.active_skill_ids = Vec::new();
                self.passive_skill_ids = Vec::new();
                self.set_status(format!("載入技能失敗: {}", err), true);
                return;
            }
        }

        self.set_status("已重新載入玩家進度".to_string(), false);
    }

    pub fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("player_progression_left_panel")
            .default_width(220.0)
            .show(ctx, |ui| {
                self.show_side_panel(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_central_panel(ui);
        });

        self.show_status_message(ctx);
    }

    fn show_side_panel(&mut self, ui: &mut Ui) {
        ui.heading("玩家進度管理");
        ui.horizontal(|ui| {
            if ui.button("重新載入").clicked() {
                self.reload();
            }
            if ui.button("儲存進度").clicked() {
                match save_progression(&self.data) {
                    Ok(_) => {
                        self.set_status("儲存成功".to_string(), false);
                        self.has_unsaved_changes = false;
                    }
                    Err(e) => {
                        self.set_status(format!("儲存失敗: {}", e), true);
                    }
                }
            }
        });
        ui.separator();
        ui.heading("戰場玩家進度列表");
        if self.data.boards.is_empty() {
            ui.label("尚未載入玩家進度資料或無任何戰場進度");
            return;
        }
        for (board_id, _) in &self.data.boards {
            let selected = self.selected_board.as_ref() == Some(board_id);
            if ui.selectable_label(selected, board_id).clicked() {
                self.selected_board = Some(board_id.clone());
            }
        }
    }

    fn show_central_panel(&mut self, ui: &mut Ui) {
        let board_id = match &self.selected_board {
            Some(board_id) => board_id,
            None => {
                ui.heading("請在左側選擇要編輯的戰場");
                return;
            }
        };

        ui.heading(format!("編輯戰場: {board_id} 的 Roster"));

        let progress = self.data.boards.get_mut(board_id).unwrap();

        ui.separator();

        // 新增單位區塊
        ui.horizontal(|ui| {
            ui.label("新增單位：");
            let unit_types: Vec<_> = self.unit_templates.keys().collect();
            let mut selected_idx = self
                .new_unit_type
                .as_ref()
                .and_then(|t| unit_types.iter().position(|x| *x == t))
                .unwrap_or(0);
            egui::ComboBox::from_id_salt("new_unit_type_combo")
                .selected_text(
                    unit_types
                        .get(selected_idx)
                        .map(|s| s.as_str())
                        .unwrap_or("請選擇單位"),
                )
                .show_ui(ui, |ui| {
                    for (i, typ) in unit_types.iter().enumerate() {
                        let resp = ui.selectable_value(&mut selected_idx, i, *typ);
                        if resp.changed() {
                            self.new_unit_type = Some((*typ).clone());
                        }
                    }
                });
            if ui.button("新增").clicked() {
                if let Some(typ) = unit_types.get(selected_idx) {
                    progress.roster.insert(
                        (*typ).clone(),
                        Unit {
                            unit_type: (*typ).clone(),
                            ..Default::default()
                        },
                    );
                    self.has_unsaved_changes = true;
                }
            }
        });

        ui.heading("單位列表");
        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                if progress.roster.is_empty() {
                    ui.label("此戰場尚無單位");
                    return;
                }
                let mut to_remove_unit: Option<UnitTemplateType> = None;
                let mut to_edit_unit: Option<Unit> = None;
                for (typ, unit) in progress.roster.iter() {
                    // BTreeSet 的 iter() 回傳不可變參考，無法直接編輯 unit
                    // 若需編輯，需複製出來再處理

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(format!("種類: {}", typ));
                            if ui.small_button("x").on_hover_text("刪除此單位").clicked() {
                                to_remove_unit = Some(typ.clone());
                            }
                        });

                        // 主動技能換行
                        ui.vertical(|ui| {
                            if unit.active_skills.is_empty() {
                                ui.label("主動: -");
                            } else {
                                ui.label("主動:");
                                egui::ScrollArea::horizontal()
                                    .id_salt(format!("active_skills_scroll_{}", typ))
                                    .max_height(32.0)
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            for skill in unit.active_skills.iter() {
                                                ui.label(skill.as_str());
                                                if ui
                                                    .small_button("x")
                                                    .on_hover_text("移除主動技能")
                                                    .clicked()
                                                {
                                                    let mut new_unit = unit.clone();
                                                    new_unit.active_skills.remove(skill);
                                                    to_edit_unit = Some(new_unit);
                                                }
                                            }
                                        });
                                    });
                            }
                            let mut add_active_skill: Option<SkillID> = None;
                            egui::ComboBox::from_id_salt(format!(
                                "unit_active_skill_combo_{}",
                                typ
                            ))
                            .selected_text("新增主動技能")
                            .show_ui(ui, |ui| {
                                for skill_id in self.active_skill_ids.iter() {
                                    if ui.selectable_label(false, skill_id.as_str()).clicked() {
                                        add_active_skill = Some(skill_id.clone());
                                    }
                                }
                            });
                            if let Some(skill_id) = add_active_skill {
                                if !unit.active_skills.contains(&skill_id) {
                                    let mut new_unit = unit.clone();
                                    new_unit.active_skills.insert(skill_id);
                                    to_edit_unit = Some(new_unit);
                                }
                            }
                        });

                        // 被動技能換行
                        ui.vertical(|ui| {
                            if unit.passive_skills.is_empty() {
                                ui.label("被動: -");
                            } else {
                                ui.label("被動:");
                                egui::ScrollArea::horizontal()
                                    .id_salt(format!("passive_skills_scroll_{}", typ))
                                    .max_height(32.0)
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            for skill in unit.passive_skills.iter() {
                                                ui.label(skill.as_str());
                                                if ui
                                                    .small_button("x")
                                                    .on_hover_text("移除被動技能")
                                                    .clicked()
                                                {
                                                    let mut new_unit = unit.clone();
                                                    new_unit.passive_skills.remove(skill);
                                                    to_edit_unit = Some(new_unit);
                                                }
                                            }
                                        });
                                    });
                            }
                            let mut add_passive_skill: Option<SkillID> = None;
                            egui::ComboBox::from_id_salt(format!(
                                "unit_passive_skill_combo_{}",
                                typ
                            ))
                            .selected_text("新增被動技能")
                            .show_ui(ui, |ui| {
                                for skill_id in self.passive_skill_ids.iter() {
                                    if ui.selectable_label(false, skill_id.as_str()).clicked() {
                                        add_passive_skill = Some(skill_id.clone());
                                    }
                                }
                            });
                            if let Some(skill_id) = add_passive_skill {
                                if !unit.passive_skills.contains(&skill_id) {
                                    let mut new_unit = unit.clone();
                                    new_unit.passive_skills.insert(skill_id);
                                    to_edit_unit = Some(new_unit);
                                }
                            }
                        });
                    });
                }
                if let Some(typ) = to_remove_unit {
                    progress.roster.remove(&typ);
                    self.has_unsaved_changes = true;
                }
                if let Some(new_unit) = to_edit_unit {
                    let typ = new_unit.unit_type.clone();
                    progress.roster.remove(&typ);
                    progress.roster.insert(typ, new_unit);
                    self.has_unsaved_changes = true;
                }
            });
    }

    fn show_status_message(&mut self, ctx: &Context) {
        if let Some((message, is_error)) = &self.status_message {
            show_status_message(ctx, message, *is_error);
        }
    }

    fn set_status(&mut self, msg: String, is_error: bool) {
        self.status_message = Some((msg, is_error));
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.has_unsaved_changes
    }
}

fn save_progression(data: &PlayerProgressionData) -> io::Result<()> {
    to_file(PROGRESSION_FILE, data)
}

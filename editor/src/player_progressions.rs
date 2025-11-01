// 玩家進度編輯器模組
use crate::common::*;
use chess_lib::*;
use egui::*;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use skills_lib::*;
use std::collections::{BTreeMap, HashSet};
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
/// 玩家單位資料，技能依 tag 分類
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Unit {
    pub unit_type: UnitTemplateType,
    #[serde(with = "skill_by_tags_key_map")]
    pub skills: SkillByTags,
}

#[derive(Debug, Default)]
pub struct PlayerProgressionEditor {
    // 其他編輯器的資料
    unit_templates: IndexMap<UnitTemplateType, UnitTemplate>,
    skills: BTreeMap<SkillID, Skill>,
    skill_group: SkillByTags,
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
        let board_map: BTreeMap<String, BoardConfig> = from_file(boards_file()).unwrap_or_default();
        let board_ids: Vec<String> = board_map.keys().cloned().collect();

        // 2. 讀取玩家進度資料
        let mut data = match from_file::<_, PlayerProgressionData>(progressions_file()) {
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
        self.unit_templates = match crate::units::load_unit_templates(unit_templates_file()) {
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
        match from_file::<_, BTreeMap<SkillID, Skill>>(skills_file()) {
            Err(err) => {
                self.skills = BTreeMap::new();
                self.skill_group = SkillByTags::new();
                self.set_status(format!("載入技能失敗: {}", err), true);
                return;
            }
            Ok(skills) => {
                let skill_group = match must_group_skills_by_tags(&skills) {
                    Err(err) => {
                        self.skills = BTreeMap::new();
                        self.skill_group = SkillByTags::new();
                        self.set_status(format!("解析技能分類失敗: {}", err), true);
                        return;
                    }
                    Ok(skill_group) => skill_group,
                };
                self.skills = skills;
                self.skill_group = skill_group;
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
                match self.validate_skills_exist() {
                    Ok(_) => match save_progression(&self.data) {
                        Ok(_) => {
                            self.set_status("儲存成功".to_string(), false);
                            self.has_unsaved_changes = false;
                        }
                        Err(e) => {
                            self.set_status(format!("儲存失敗: {}", e), true);
                        }
                    },
                    Err(msg) => {
                        self.set_status(format!("儲存失敗：{}", msg), true);
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

        // 強制中央內容最小寬度
        ui.set_min_width(ui.available_width());

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
        egui::ScrollArea::vertical().show(ui, |ui| {
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

                    for (tag_tuple, all_skill_ids) in self.skill_group.iter() {
                        let empty_vec: Vec<SkillID> = Vec::new();
                        let skill_ids = unit.skills.get(tag_tuple).unwrap_or(&empty_vec);
                        ui.vertical(|ui| {
                            let label = format!(
                                "分類: ─── {:?}-{:?}-{:?} ───",
                                tag_tuple.0, tag_tuple.1, tag_tuple.2
                            );
                            ui.label(label);
                            // 已有技能列表 + 移除技能
                            egui::ScrollArea::horizontal()
                                .id_salt(format!(
                                    "skills_scroll_{:?}_{:?}_{:?}_{}",
                                    tag_tuple.0, tag_tuple.1, tag_tuple.2, typ
                                ))
                                .max_height(32.0)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        for (idx, skill) in skill_ids.iter().enumerate() {
                                            ui.label(skill);
                                            if ui
                                                .small_button("x")
                                                .on_hover_text("移除技能")
                                                .clicked()
                                            {
                                                let mut new_unit = unit.clone();
                                                if let Some(new_unit_skills) =
                                                    new_unit.skills.get_mut(tag_tuple)
                                                {
                                                    new_unit_skills.remove(idx);
                                                    if new_unit_skills.is_empty() {
                                                        new_unit.skills.remove(tag_tuple);
                                                    }
                                                }
                                                to_edit_unit = Some(new_unit);
                                            }
                                        }
                                    });
                                });
                            // 可新增技能選項
                            let owned_skill_ids: HashSet<&SkillID> =
                                unit.skills.values().flat_map(|v| v).collect();
                            let unowned_skill_ids: Vec<SkillID> = all_skill_ids
                                .iter()
                                .filter(|skill_id| !owned_skill_ids.contains(skill_id))
                                .cloned()
                                .collect();
                            let mut add_skill: Option<SkillID> = None;
                            egui::ComboBox::from_id_salt(format!(
                                "unit_skill_combo_{:?}_{:?}_{:?}_{}",
                                tag_tuple.0, tag_tuple.1, tag_tuple.2, typ
                            ))
                            .selected_text("新增技能")
                            .show_ui(ui, |ui| {
                                for skill_id in unowned_skill_ids.iter() {
                                    if ui.selectable_label(false, skill_id).clicked() {
                                        add_skill = Some(skill_id.clone());
                                    }
                                }
                            });
                            if let Some(skill_id) = add_skill {
                                let mut new_unit = unit.clone();
                                new_unit
                                    .skills
                                    .entry(tag_tuple.clone())
                                    .or_insert_with(Vec::new)
                                    .push(skill_id);
                                to_edit_unit = Some(new_unit);
                            }
                        });
                    }
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

    /// 檢查所有進度中的技能是否存在於 skills 資料表
    fn validate_skills_exist(&self) -> Result<(), String> {
        let mut missing: Vec<(String, String, String)> = Vec::new();
        for (board_id, progress) in &self.data.boards {
            for (unit_type, unit) in &progress.roster {
                for skill_ids in unit.skills.values() {
                    for skill_id in skill_ids {
                        if !self.skills.contains_key(skill_id) {
                            missing.push((board_id.clone(), unit_type.clone(), skill_id.clone()));
                        }
                    }
                }
            }
        }
        if !missing.is_empty() {
            let mut msg = String::from("下列技能不存在：\n");
            for (board, unit, skill) in missing {
                msg.push_str(&format!(
                    "戰場: {}, 單位: {}, 技能: {}\n",
                    board, unit, skill
                ));
            }
            return Err(msg);
        }
        Ok(())
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
    to_file(progressions_file(), data)
}

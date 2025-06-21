use crate::{
    common::{from_file, show_status_message, to_file},
    skills::SkillsData,
};
use egui::{Button, Ui};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io,
};

const UNIT_TYPES_FILE: &str = "unit-types.toml";
const SKILLS_FILE: &str = "../shared-lib/test-data/ignore-skills.toml";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnitType {
    pub name: String,
    pub skills: BTreeSet<String>,
}

#[derive(Default)]
pub struct UnitEditor {
    unit_types: BTreeMap<String, UnitType>,
    skills: Vec<String>,
    selected_unit: Option<String>,
    selected_skill: String,
    has_unsaved_changes: bool,
    status_message: Option<(String, bool)>,
}

pub fn load_unit_types(path: &str) -> io::Result<BTreeSet<UnitType>> {
    #[derive(serde::Deserialize)]
    struct UnitTypeConfig {
        unit_types: BTreeSet<UnitType>,
    }
    return from_file::<_, UnitTypeConfig>(path).map(|config| config.unit_types);
}

impl UnitEditor {
    pub fn new() -> Self {
        let mut editor = Self::default();
        editor.reload();
        editor
    }

    pub fn reload(&mut self) {
        // 重新載入 unit_types
        match load_unit_types(UNIT_TYPES_FILE) {
            Ok(unit_types) => {
                self.unit_types = unit_types
                    .into_iter()
                    .map(|unit| (unit.name.clone(), unit))
                    .collect();
                if let Some(selected) = &self.selected_unit {
                    if !self.unit_types.contains_key(selected) {
                        // 如果選中的單位不存在，則清除選中狀態
                        self.selected_unit = None;
                    }
                }
            }
            Err(err) => {
                self.set_status(format!("載入單位類型失敗: {}", err), true);
                return;
            }
        }
        // 重新載入 skills
        match SkillsData::from_file(SKILLS_FILE) {
            Ok(skills_data) => {
                self.skills = skills_data
                    .skills
                    .into_iter()
                    .map(|(name, _)| name)
                    .collect();
                if self.skills.is_empty() {
                    self.selected_skill.clear();
                } else if !self.skills.contains(&self.selected_skill) {
                    self.selected_skill = self.skills.first().cloned().unwrap_or_default();
                }
            }
            Err(err) => {
                self.set_status(format!("載入技能失敗: {}", err), true);
                return;
            }
        }
        self.set_status("已重新載入 unit_types 與 skills".to_string(), false);
    }

    pub fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("unit_list_panel")
            .default_width(200.0)
            .show(ctx, |ui| {
                self.show_unit_list(ui);
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_unit_editor(ui);
        });
        self.show_status_message(ctx);
    }

    fn show_unit_list(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            if ui.button("重新載入 unit_types & skills").clicked() {
                self.reload();
            }
            ui.heading("單位列表");
        });
        if ui.button("新增單位").clicked() {
            let new_unit = UnitType {
                name: String::new(),
                skills: BTreeSet::new(),
            };
            self.unit_types
                .insert(new_unit.name.clone(), new_unit.clone());
            self.selected_unit = Some(new_unit.name.clone());
            self.has_unsaved_changes = true;
        }
        let mut to_copy = None;
        let mut to_delete = None;
        for name in self.unit_types.keys() {
            let selected = self.selected_unit == Some(name.clone());
            let button = Button::new(name).fill(if selected {
                egui::Color32::DARK_GRAY
            } else {
                egui::Color32::TRANSPARENT
            });
            if ui.add(button).clicked() {
                self.selected_unit = Some(name.clone());
            }
            ui.horizontal(|ui| {
                if ui.button("複製").clicked() {
                    to_copy = Some(name.clone());
                }
                if ui.button("刪除").clicked() {
                    to_delete = Some(name.clone());
                }
            });
        }
        if let Some(name) = to_copy {
            let mut new_unit = self.unit_types.get(&name).cloned().unwrap();
            new_unit.name.push_str("_copy");
            let new_unit = new_unit;
            self.unit_types
                .insert(new_unit.name.clone(), new_unit.clone());
            self.selected_unit = Some(new_unit.name.clone());
            self.has_unsaved_changes = true;
        }
        if let Some(name) = to_delete {
            self.unit_types.remove(&name);
            self.selected_unit = None;
            self.has_unsaved_changes = true;
        }
    }

    fn show_unit_editor(&mut self, ui: &mut Ui) {
        if let Some(orig_name) = self.selected_unit.clone() {
            let Some(mut unit) = self.unit_types.get(&orig_name).cloned() else {
                return;
            };
            ui.heading("單位編輯");
            ui.label("名稱（含等級）：");
            ui.text_edit_singleline(&mut unit.name);
            ui.label("技能：");
            egui::ComboBox::from_id_salt("add_skill_combo")
                .selected_text(format!("選擇技能: {}", &self.selected_skill))
                .show_ui(ui, |ui| {
                    for skill in &self.skills {
                        if ui.button(skill).clicked() {
                            self.selected_skill = skill.clone();
                        }
                    }
                });
            if ui.button("新增技能").clicked() {
                unit.skills.insert(self.selected_skill.clone());
                self.has_unsaved_changes = true;
            }
            let mut deleted = None;
            ui.horizontal(|ui| {
                for skill in &unit.skills {
                    ui.label(skill);
                    if ui.button("移除").clicked() {
                        deleted = Some(skill.clone());
                    }
                }
            });
            if let Some(deleted) = deleted {
                unit.skills.remove(&deleted);
                self.has_unsaved_changes = true;
            }

            // 判斷名稱是否有變更
            let name_changed = unit.name != orig_name;
            if name_changed && !self.unit_types.contains_key(&unit.name) {
                self.unit_types.remove(&orig_name);
                self.selected_unit = Some(unit.name.clone());
                self.unit_types.insert(unit.name.clone(), unit.clone());
            } else if !name_changed {
                self.unit_types.insert(unit.name.clone(), unit.clone());
            }
            if ui.button("儲存").clicked() {
                if let Err(e) = self.save_unit_types(UNIT_TYPES_FILE) {
                    self.set_status(format!("儲存失敗: {}", e), true);
                } else {
                    self.set_status("儲存成功".to_string(), false);
                    self.has_unsaved_changes = false;
                }
            }
        }
    }

    fn save_unit_types(&self, path: &str) -> Result<(), io::Error> {
        #[derive(serde::Serialize)]
        struct UnitTypeConfig<'a> {
            unit_types: BTreeSet<&'a UnitType>,
        }
        let config = UnitTypeConfig {
            unit_types: self.unit_types.values().collect(),
        };
        return to_file(path, &config);
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.has_unsaved_changes
    }

    pub fn set_status(&mut self, msg: String, is_error: bool) {
        self.status_message = Some((msg, is_error));
    }

    fn show_status_message(&mut self, ctx: &egui::Context) {
        if let Some((message, is_error)) = &self.status_message {
            show_status_message(ctx, message, *is_error);
        }
    }
}

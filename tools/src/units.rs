use crate::{common::*, skills::SkillsData};
use chess_lib::{UnitTemplate, UnitTemplateType};
use egui::{Button, Ui};
use std::io;

#[derive(Default)]
pub struct UnitsEditor {
    // 需要指定順序
    unit_templates: Vec<UnitTemplate>,
    skills: Vec<String>,
    selected_unit: Option<UnitTemplateType>,
    selected_skill: String,
    has_unsaved_changes: bool,
    status_message: Option<(String, bool)>,
}

pub fn load_unit_templates(path: &str) -> io::Result<Vec<UnitTemplate>> {
    #[derive(serde::Deserialize)]
    struct UnitTemplatesConfig {
        unit_templates: Vec<UnitTemplate>,
    }
    return from_file::<_, UnitTemplatesConfig>(path).map(|config| config.unit_templates);
}

impl UnitsEditor {
    pub fn new() -> Self {
        let mut editor = Self::default();
        editor.reload();
        editor
    }

    pub fn reload(&mut self) {
        // 重新載入 unit_templates
        match load_unit_templates(UNIT_TEMPLATES_FILE) {
            Ok(unit_templates) => {
                self.unit_templates = unit_templates;
                if let Some(selected) = &self.selected_unit {
                    if !self.unit_templates.iter().any(|u| &u.name == selected) {
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
        self.set_status("已重新載入 unit_templates 與 skills".to_string(), false);
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
            if ui.button("重新載入 unit_templates & skills").clicked() {
                self.reload();
            }
            ui.heading("單位列表");
        });
        if ui.button("新增單位").clicked() {
            let new_unit = UnitTemplate::default();
            self.unit_templates.push(new_unit.clone());
            self.selected_unit = Some(new_unit.name.clone());
            self.has_unsaved_changes = true;
        }
        let mut to_copy = None;
        let mut to_delete = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (idx, unit) in self.unit_templates.iter().enumerate() {
                let name = &unit.name;
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
                        to_copy = Some(idx);
                    }
                    if ui.button("刪除").clicked() {
                        to_delete = Some(idx);
                    }
                });
            }
        });
        // 新增刪除不會同時發生
        if let Some(idx) = to_copy {
            let mut new_unit = self.unit_templates[idx].clone();
            new_unit.name.push_str("_copy");
            let new_unit_name = new_unit.name.clone();
            self.unit_templates.insert(idx + 1, new_unit);
            self.selected_unit = Some(new_unit_name);
            self.has_unsaved_changes = true;
        } else if let Some(idx) = to_delete {
            self.unit_templates.remove(idx);
            self.selected_unit = None;
            self.has_unsaved_changes = true;
        }
    }

    fn show_unit_editor(&mut self, ui: &mut Ui) {
        let Some(orig_name) = self.selected_unit.clone() else {
            return;
        };
        let idx = self.unit_templates.iter().position(|u| u.name == orig_name);
        let Some(idx) = idx else {
            return;
        };
        let mut unit = self.unit_templates[idx].clone();
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
        if name_changed && !self.unit_templates.iter().any(|u| u.name == unit.name) {
            self.selected_unit = Some(unit.name.clone());
            self.unit_templates[idx] = unit;
        } else if !name_changed {
            self.unit_templates[idx] = unit;
        }
        if ui.button("儲存").clicked() {
            if let Err(e) = self.save_unit_templates(UNIT_TEMPLATES_FILE) {
                self.set_status(format!("儲存失敗: {}", e), true);
            } else {
                self.set_status("儲存成功".to_string(), false);
                self.has_unsaved_changes = false;
            }
        }
    }

    fn save_unit_templates(&self, path: &str) -> Result<(), io::Error> {
        #[derive(serde::Serialize)]
        struct UnitTemplatesConfig<'a> {
            unit_templates: &'a Vec<UnitTemplate>,
        }
        let config = UnitTemplatesConfig {
            unit_templates: &self.unit_templates,
        };
        return to_file(path, &config);
    }

    pub fn set_status(&mut self, msg: String, is_error: bool) {
        self.status_message = Some((msg, is_error));
    }

    fn show_status_message(&mut self, ctx: &egui::Context) {
        if let Some((message, is_error)) = &self.status_message {
            show_status_message(ctx, message, *is_error);
        }
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.has_unsaved_changes
    }
}

use crate::{common::*, skills::SkillsData};
use chess_lib::{UnitTemplate, UnitTemplateType};
use egui::*;
use std::io;

#[derive(Default)]
pub struct UnitsEditor {
    // 需要指定順序
    unit_templates: Vec<UnitTemplate>,
    active_skill_ids: Vec<String>,
    passive_skill_ids: Vec<String>,
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
                let is_selected_exist = self.selected_unit.as_ref().map_or(false, |selected| {
                    self.unit_templates.iter().any(|u| &u.name == selected)
                });
                if !is_selected_exist {
                    // 如果選中的單位不存在，則清除選中狀態
                    self.selected_unit = None;
                }
            }
            Err(err) => {
                self.set_status(format!("載入單位類型失敗: {}", err), true);
                return;
            }
        }
        // 重新載入 skills，並分類主動/被動
        match SkillsData::from_file(SKILLS_FILE) {
            Ok(skills_data) => {
                let mut active_skill_ids = Vec::new();
                let mut passive_skill_ids = Vec::new();
                for (id, skill) in &skills_data.skills {
                    if skill.tags.contains(&skills_lib::Tag::Active) {
                        active_skill_ids.push(id.clone());
                    } else if skill.tags.contains(&skills_lib::Tag::Passive) {
                        passive_skill_ids.push(id.clone());
                    }
                }
                active_skill_ids.sort();
                passive_skill_ids.sort();
                self.active_skill_ids = active_skill_ids;
                self.passive_skill_ids = passive_skill_ids;
                // 維持選取行為
                let all_skills: Vec<String> = self
                    .active_skill_ids
                    .iter()
                    .chain(self.passive_skill_ids.iter())
                    .cloned()
                    .collect();
                if all_skills.is_empty() {
                    self.selected_skill.clear();
                } else if !all_skills.contains(&self.selected_skill) {
                    self.selected_skill = all_skills.first().cloned().unwrap_or_default();
                }
            }
            Err(err) => {
                self.set_status(format!("載入技能失敗: {}", err), true);
                return;
            }
        }
        self.set_status("已重新載入 unit_templates 與 skills".to_string(), false);
    }

    pub fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        SidePanel::left("unit_list_panel")
            .default_width(200.0)
            .show(ctx, |ui| {
                if ui.button("重新載入").clicked() {
                    self.reload();
                }
                if ui.button("儲存").clicked() {
                    if let Err(e) = self.save_unit_templates(UNIT_TEMPLATES_FILE) {
                        self.set_status(format!("儲存失敗: {e}"), true);
                    } else {
                        self.set_status("儲存成功".to_string(), false);
                        self.has_unsaved_changes = false;
                    }
                }
                self.show_unit_list(ui);
            });
        CentralPanel::default().show(ctx, |ui| {
            self.show_unit_editor(ui);
        });
        self.show_status_message(ctx);
    }

    fn show_unit_list(&mut self, ui: &mut Ui) {
        ui.heading("單位列表");
        if ui.button("新增單位").clicked() {
            let new_unit = UnitTemplate::default();
            self.unit_templates.push(new_unit.clone());
            self.selected_unit = Some(new_unit.name.clone());
            self.has_unsaved_changes = true;
        }
        let mut to_copy = None;
        let mut to_delete = None;
        let mut to_move_up = None;
        let mut to_move_down = None;
        let mut to_select = None;
        ScrollArea::vertical().show(ui, |ui| {
            for (idx, unit) in self.unit_templates.iter().enumerate() {
                let name = &unit.name;
                let selected = self.selected_unit == Some(name.clone());
                let button = Button::new(name).fill(if selected {
                    ui.style().visuals.selection.bg_fill
                } else {
                    ui.style().visuals.widgets.noninteractive.bg_fill
                });
                if ui.add(button).clicked() {
                    to_select = Some(name.clone());
                }
                ui.horizontal(|ui| {
                    if ui.button("複製").clicked() {
                        to_copy = Some(idx);
                    }
                    if ui.button("刪除").clicked() {
                        to_delete = Some(idx);
                    }
                    // 排序按鈕
                    if idx > 0 && ui.button("↑").clicked() {
                        to_move_up = Some(idx);
                    }
                    if idx + 1 < self.unit_templates.len() && ui.button("↓").clicked() {
                        to_move_down = Some(idx);
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
        // 排序操作（for 迴圈外執行 swap）
        if let Some(idx) = to_move_up {
            self.unit_templates.swap(idx, idx - 1);
            self.has_unsaved_changes = true;
        } else if let Some(idx) = to_move_down {
            self.unit_templates.swap(idx, idx + 1);
            self.has_unsaved_changes = true;
        }
        // 選取操作（for 迴圈外執行）
        if let Some(name) = to_select {
            self.selected_unit = Some(name);
        }
    }

    fn show_unit_editor(&mut self, ui: &mut Ui) {
        let orig_name = match self.selected_unit.clone() {
            None => return,
            Some(orig_name) => orig_name,
        };
        let idx = self.unit_templates.iter().position(|u| u.name == orig_name);
        let idx = match idx {
            None => return,
            Some(idx) => idx,
        };
        let unit = &mut self.unit_templates[idx];
        ui.heading("單位編輯");
        ui.label("名稱（含等級）：");
        ui.text_edit_singleline(&mut unit.name);
        self.selected_unit = Some(unit.name.clone());
        ui.label("技能：");
        ComboBox::from_id_salt("add_skill_combo")
            .selected_text(format!("選擇技能: {}", &self.selected_skill))
            .show_ui(ui, |ui| {
                ui.label("─── 主動技能 ───");
                for skill in &self.active_skill_ids {
                    if ui.button(skill).clicked() {
                        self.selected_skill = skill.clone();
                    }
                }
                ui.separator();
                ui.label("─── 被動技能 ───");
                for skill in &self.passive_skill_ids {
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

    fn show_status_message(&mut self, ctx: &Context) {
        if let Some((message, is_error)) = &self.status_message {
            show_status_message(ctx, message, *is_error);
        }
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.has_unsaved_changes
    }
}

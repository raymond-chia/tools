use crate::common::*;
use eframe::{Frame, egui};
use egui::{Button, DragValue, ScrollArea, Separator, Ui};
use serde::{Deserialize, Serialize};
use skills_lib::*;
use std::collections::{BTreeMap, HashMap};
use std::io;
use std::path::{Path, PathBuf};
use strum::IntoEnumIterator;

/// 技能資料集
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SkillsData {
    #[serde(flatten)]
    pub skills: HashMap<String, Skill>,
}

impl SkillsData {
    pub fn from_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let skills = from_file(path)?;
        return Ok(Self { skills });
    }

    fn save_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let sorted_skills: BTreeMap<_, _> = self.skills.clone().into_iter().collect();
        return to_file(path, &sorted_skills);
    }

    /// 新增技能
    fn create_skill(&mut self, skill_id: &str) -> Result<(), String> {
        if self.skills.contains_key(skill_id) {
            return Err("技能 ID 已存在".to_string());
        }
        self.skills.insert(skill_id.to_string(), Skill::default());
        Ok(())
    }

    /// 刪除技能
    fn delete_skill(&mut self, skill_id: &str) -> Result<(), String> {
        if !self.skills.contains_key(skill_id) {
            return Err("找不到指定的技能".to_string());
        }
        self.skills.remove(skill_id);
        Ok(())
    }

    /// 檢查技能的合法性
    fn validate(skill: &Skill) -> Result<(), String> {
        if skill.effects.len() == 0 {
            return Err("技能必須至少有一個效果".to_string());
        }

        if skill.range.0 > skill.range.1 {
            return Err("技能範圍的最小值不能大於最大值".to_string());
        }

        // 檢查標籤的互斥條件
        // 條件1: active, passive 只能擇一
        let has_passive = skill.tags.contains(&Tag::Passive);
        let has_active = skill.tags.contains(&Tag::Active);
        if has_passive && has_active {
            return Err("技能不能同時是主動 (Active) 和被動 (Passive)".to_string());
        }

        // 條件2: single, area 只能擇一
        let has_single = skill.tags.contains(&Tag::Single);
        let has_area = skill.tags.contains(&Tag::Area);
        if has_single && has_area {
            return Err("技能不能同時是單體 (Single) 和範圍 (Area)".to_string());
        }

        // 條件3: caster, melee, ranged 只能擇一
        let has_caster = skill.tags.contains(&Tag::Caster);
        let has_melee = skill.tags.contains(&Tag::Melee);
        let has_ranged = skill.tags.contains(&Tag::Ranged);
        let range_count = [has_caster, has_melee, has_ranged]
            .iter()
            .filter(|&&b| b)
            .count();
        if range_count > 1 {
            return Err("技能的作用範圍 (Caster/Melee/Ranged) 只能擇一".to_string());
        }

        // 檢查單體技能
        if skill.tags.contains(&Tag::Single) {
            match &skill.effects[0] {
                Effect::Hp { shape, .. }
                | Effect::Burn { shape, .. }
                | Effect::MovePoints { shape, .. } => {
                    if shape != &Shape::Point {
                        return Err("單體技能的效果形狀必須是點".to_string());
                    }
                }
            }
        }

        // 檢查範圍技能
        if skill.tags.contains(&Tag::Area) {
            match &skill.effects[0] {
                Effect::Hp { shape, .. }
                | Effect::Burn { shape, .. }
                | Effect::MovePoints { shape, .. } => match shape {
                    Shape::Point => {
                        return Err("範圍技能的效果形狀不能是點".to_string());
                    }
                    Shape::Circle(radius) => {
                        if *radius < 2 {
                            return Err("範圍技能的效果形狀半徑不能小於 2".to_string());
                        }
                    }
                    Shape::Line(length) => {
                        if *length < 2 {
                            return Err("範圍技能的效果形狀長度不能小於 2".to_string());
                        }
                    }
                    Shape::Cone(radius, angle) => {
                        if *radius < 2 && *angle < 90 {
                            return Err(
                                "範圍技能的效果形狀半徑不能小於 2 同時角度又小於 90".to_string()
                            );
                        }
                    }
                },
            }
        }

        // 檢查施法者技能
        if skill.tags.contains(&Tag::Caster) {
            if skill.range.0 != 0 || skill.range.1 != 0 {
                return Err("施法者技能的範圍必須是 (0, 0)".to_string());
            }
            match &skill.effects[0] {
                Effect::Hp { target_type, .. }
                | Effect::Burn { target_type, .. }
                | Effect::MovePoints { target_type, .. } => {
                    if target_type != &TargetType::Caster {
                        return Err("施法者技能的目標類型必須是施法者".to_string());
                    }
                }
            }
        }

        // effect 跟 tag 需要一起存在
        let checklist = [
            (
                skill.effects.iter().any(|e| match e {
                    Effect::Hp { value, .. } => *value < 0,
                    _ => false,
                }),
                Tag::Attack,
                "攻擊 tag 需要有 HP 效果",
            ),
            (
                skill.effects.iter().any(|e| match e {
                    Effect::Hp { value, .. } => *value > 0,
                    _ => false,
                }),
                Tag::Heal,
                "治療 tag 需要有 HP 效果",
            ),
        ];
        for (check, tag, msg) in checklist {
            if check ^ skill.tags.contains(&tag) {
                return Err(msg.to_string());
            }
        }

        // effect 需要有對應的 tag
        let checklist = [(
            skill.effects.iter().any(|e| match e {
                Effect::Burn { .. } => true,
                _ => false,
            }),
            Tag::Fire,
            "燃燒 debuff 需要有火焰 tag",
        )];
        for (check, tag, msg) in checklist {
            if check && !skill.tags.contains(&tag) {
                return Err(msg.to_string());
            }
        }

        Ok(())
    }
}

pub struct SkillsEditor {
    skills_data: SkillsData,
    has_unsaved_changes_flag: bool, // 記錄自上次存檔後是否有修改
    current_file_path: Option<PathBuf>,
    status_message: Option<(String, bool)>, // message, is_error
    //
    new_skill_id: String,
    selected_skill: Option<String>,
    //
    show_add_effect_popup: bool,
    show_confirmation_dialog: bool,
    confirmation_action: ConfirmationAction,
}

impl crate::common::New for SkillsEditor {
    fn new() -> Self {
        return Self::new();
    }
}

impl SkillsEditor {
    pub fn new() -> Self {
        // 嘗試自動載入寫死的檔案
        let (skills_data, current_file_path, err) = match SkillsData::from_file(SKILLS_FILE) {
            Ok(skills_data) => {
                let current_file_path = Some(std::path::PathBuf::from(SKILLS_FILE));
                (skills_data, current_file_path, None)
            }
            Err(err) => (SkillsData::default(), None, Some(err)),
        };
        let mut result = Self {
            skills_data,
            current_file_path,
            ..Default::default()
        };
        if let Some(err) = err {
            result.set_status(err.to_string(), true);
        }
        return result;
    }
}

#[derive(Debug, Clone)]
enum ConfirmationAction {
    None,
    DeleteSkill(String),
    DeleteEffect(String, usize),
}

impl Default for SkillsEditor {
    fn default() -> Self {
        Self {
            skills_data: SkillsData {
                skills: HashMap::new(),
            },
            has_unsaved_changes_flag: false,
            current_file_path: None,
            status_message: None,
            selected_skill: None,
            new_skill_id: String::new(),
            show_add_effect_popup: false,
            show_confirmation_dialog: false,
            confirmation_action: ConfirmationAction::None,
        }
    }
}

impl FileOperator<PathBuf> for SkillsEditor {
    fn current_file_path(&self) -> Option<PathBuf> {
        self.current_file_path.clone()
    }
    fn load_file(&mut self, path: PathBuf) {
        self.load_file(path);
    }
    fn save_file(&mut self, path: PathBuf) {
        self.save_file(path);
    }
    fn set_status(&mut self, status: String, is_error: bool) {
        self.set_status(status, is_error);
    }
}

impl SkillsEditor {
    pub fn update(&mut self, ctx: &egui::Context, _: &mut Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                show_file_menu(ui, self);
            });
        });

        egui::SidePanel::left("skills_list_panel")
            .default_width(200.0)
            .show(ctx, |ui| {
                self.show_skills_list(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_skill_editor(ui);
        });

        self.show_add_effect_popup(ctx);
        self.show_confirmation_dialog(ctx);
        self.show_status_message(ctx);
    }

    fn show_skills_list(&mut self, ui: &mut Ui) {
        ui.heading("技能列表");

        ui.horizontal(|ui| {
            ui.label("新增技能 ID:");
            ui.text_edit_singleline(&mut self.new_skill_id);
            if ui.button("新增").clicked() {
                self.create_skill();
            }
        });

        ui.add_space(10.0);

        ScrollArea::vertical().show(ui, |ui| {
            // 收集所有技能 ID 並按字母順序排序
            let mut skill_ids: Vec<_> = self.skills_data.skills.keys().collect();
            skill_ids.sort(); // 按字母排序

            // 顯示排序後的技能列表
            for skill_id in skill_ids {
                let selected = self.selected_skill.as_ref() == Some(skill_id);

                let button = Button::new(skill_id)
                    .fill(if selected {
                        ui.style().visuals.selection.bg_fill
                    } else {
                        ui.style().visuals.widgets.noninteractive.bg_fill
                    })
                    .min_size(egui::vec2(ui.available_width(), 0.0));

                if ui.add(button).clicked() {
                    // 點擊就直接切換技能
                    self.selected_skill = Some(skill_id.clone());
                }
            }
        });
    }

    fn show_skill_editor(&mut self, ui: &mut Ui) {
        // 首先添加標題和按鈕（這些保持在固定位置）
        let mut delete_clicked = false;
        let mut add_effect_clicked = false;
        let mut delete_effect_indices = None;

        if let Some(skill_id) = &self.selected_skill {
            ui.heading(format!("編輯技能: {}", skill_id));

            ui.horizontal(|ui| {
                delete_clicked = ui.button("刪除技能").clicked();
            });

            ui.add_space(8.0);
            ui.add(Separator::default());

            // 計算 ScrollArea 的最大高度，為底部留出空間
            let available_height = ui.available_height();
            let scroll_height = available_height.max(100.0) - 40.0; // 為底部狀態欄保留空間

            // 添加可捲動區域，設定最大高度
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .max_height(scroll_height)
                .show(ui, |ui| {
                    // 在可捲動區域內編輯技能，直接使用 skills_data 中的技能
                    if let Some(skill) = self.skills_data.skills.get_mut(skill_id) {
                        // 基本屬性編輯
                        ui.heading("基本屬性");

                        // 標籤編輯
                        ui.collapsing("標籤", |ui| {
                            if Self::show_tags_editor(ui, skill) {
                                self.has_unsaved_changes_flag = true;
                            }
                        });

                        // 範圍編輯
                        ui.horizontal(|ui| {
                            ui.label("範圍:");
                            if ui
                                .add(DragValue::new(&mut skill.range.0).prefix("最小: "))
                                .changed()
                            {
                                self.has_unsaved_changes_flag = true;
                            }
                            if ui
                                .add(DragValue::new(&mut skill.range.1).prefix("最大: "))
                                .changed()
                            {
                                self.has_unsaved_changes_flag = true;
                            }
                        });

                        // 消耗編輯
                        ui.horizontal(|ui| {
                            ui.label("消耗:");
                            if ui.add(DragValue::new(&mut skill.cost)).changed() {
                                self.has_unsaved_changes_flag = true;
                            }
                        });

                        // 命中率編輯
                        ui.horizontal(|ui| {
                            ui.label("命中率:");
                            let mut has_hit_rate = skill.hit_rate.is_some();
                            if ui.checkbox(&mut has_hit_rate, "").changed() {
                                skill.hit_rate = if has_hit_rate { Some(100) } else { None };
                                self.has_unsaved_changes_flag = true;
                            }

                            if let Some(hit_rate) = &mut skill.hit_rate {
                                if ui
                                    .add_enabled(
                                        has_hit_rate,
                                        DragValue::new(hit_rate).range(0..=100).suffix("%"),
                                    )
                                    .changed()
                                {
                                    self.has_unsaved_changes_flag = true;
                                }
                            }
                        });

                        // 爆擊率編輯
                        ui.horizontal(|ui| {
                            ui.label("爆擊率:");
                            let mut has_crit_rate = skill.crit_rate.is_some();
                            if ui.checkbox(&mut has_crit_rate, "").changed() {
                                skill.crit_rate = if has_crit_rate { Some(10) } else { None };
                                self.has_unsaved_changes_flag = true;
                            }

                            if let Some(crit_rate) = &mut skill.crit_rate {
                                if ui
                                    .add_enabled(
                                        has_crit_rate,
                                        DragValue::new(crit_rate).range(0..=100).suffix("%"),
                                    )
                                    .changed()
                                {
                                    self.has_unsaved_changes_flag = true;
                                }
                            }
                        });

                        ui.add_space(8.0);
                        ui.add(Separator::default());

                        // 效果編輯
                        ui.horizontal(|ui| {
                            ui.heading("效果");
                            add_effect_clicked = ui.button("新增效果").clicked();
                        });

                        // 處理效果編輯
                        for (index, effect) in skill.effects.iter_mut().enumerate() {
                            ui.push_id(index, |ui| {
                                let mut delete_effect_clicked = false;
                                ui.horizontal(|ui| {
                                    match effect {
                                        Effect::Hp { .. } => {
                                            ui.label("HP 效果");
                                        }
                                        Effect::Burn { .. } => {
                                            ui.label("燃燒效果");
                                        }
                                        Effect::MovePoints { .. } => {
                                            ui.label("移動點數效果");
                                        }
                                    }

                                    delete_effect_clicked = ui.button("🗑").clicked();
                                });

                                if delete_effect_clicked {
                                    delete_effect_indices = Some(index);
                                }

                                ui.indent(format!("effect_{}", index), |ui| {
                                    if Self::show_effect_editor(ui, effect, Self::show_shape_editor)
                                    {
                                        self.has_unsaved_changes_flag = true;
                                    }
                                });

                                ui.add_space(8.0);
                            });
                        }
                    }
                });
        } else {
            ui.heading("技能編輯器");
            ui.label("選擇或建立一個技能開始編輯");
        }

        // 處理刪除技能按鈕
        if delete_clicked && self.selected_skill.is_some() {
            let skill_id = self.selected_skill.clone().unwrap();
            self.confirmation_action = ConfirmationAction::DeleteSkill(skill_id);
            self.show_confirmation_dialog = true;
        }

        // 處理添加效果按鈕
        if add_effect_clicked {
            self.show_add_effect_popup = true;
        }

        // 處理刪除效果
        if delete_effect_indices.is_some() && self.selected_skill.is_some() {
            let skill_id = self
                .selected_skill
                .clone()
                .expect("selected skill in race condition");
            let index = delete_effect_indices
                .take()
                .expect("delete effect in race condition");
            self.confirmation_action = ConfirmationAction::DeleteEffect(skill_id, index);
            self.show_confirmation_dialog = true;
        }
    }

    fn show_add_effect_popup(&mut self, ctx: &egui::Context) {
        if !self.show_add_effect_popup {
            return;
        }
        if self.selected_skill.is_none() {
            return;
        }

        let mut open = self.show_add_effect_popup;
        let mut effects = Vec::new();

        egui::Window::new("新增效果")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                for effect in Effect::iter() {
                    let flag = match effect {
                        Effect::Hp { .. } => ui.button("新增 HP 效果").clicked(),
                        Effect::Burn { .. } => ui.button("新增燃燒效果").clicked(),
                        Effect::MovePoints { .. } => ui.button("新增移動點數效果").clicked(),
                    };
                    effects.push((flag, effect));
                }
            });

        // 在閉包外處理按鈕事件
        for (flag, effect) in effects {
            if !flag {
                continue;
            }
            let skill_id = self
                .selected_skill
                .as_ref()
                .expect("selected skill in race condition");
            if let Some(skill) = self.skills_data.skills.get_mut(skill_id) {
                skill.effects.push(effect);
                self.has_unsaved_changes_flag = true; // 標記為已修改
                open = false;
            }
        }

        self.show_add_effect_popup = open;
    }

    fn show_confirmation_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_confirmation_dialog {
            return;
        }

        let mut open = self.show_confirmation_dialog;
        let title = "確認";
        let message = match &self.confirmation_action {
            ConfirmationAction::None => "確定要執行此操作嗎？",
            ConfirmationAction::DeleteSkill(_) => "確定要刪除此技能嗎？",
            ConfirmationAction::DeleteEffect(_, _) => "確定要刪除此效果嗎？",
        };

        let mut confirm_clicked = false;
        let mut cancel_clicked = false;
        let action_clone = self.confirmation_action.clone();

        egui::Window::new(title)
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(message);
                ui.horizontal(|ui| {
                    confirm_clicked = ui.button("確定").clicked();
                    cancel_clicked = ui.button("取消").clicked();
                });
            });

        // 在閉包外處理按鈕事件
        if confirm_clicked {
            match action_clone {
                ConfirmationAction::DeleteSkill(skill_id) => {
                    if let Err(err) = self.skills_data.delete_skill(&skill_id) {
                        self.set_status(err, true);
                    } else {
                        self.has_unsaved_changes_flag = true; // 標記為已修改
                        self.set_status("成功刪除技能".to_string(), false);
                        self.selected_skill = None;
                    }
                }
                ConfirmationAction::DeleteEffect(skill_id, index) => {
                    if let Some(skill) = self.skills_data.skills.get_mut(&skill_id) {
                        if index < skill.effects.len() {
                            skill.effects.remove(index);
                            self.has_unsaved_changes_flag = true; // 標記為已修改
                            self.set_status("成功刪除效果".to_string(), false);
                        }
                    }
                }
                _ => {}
            }
            open = false;
        }

        if cancel_clicked {
            open = false;
        }

        self.show_confirmation_dialog = open;
    }

    fn create_skill(&mut self) {
        if self.new_skill_id.is_empty() {
            self.set_status("技能 ID 不能為空".to_string(), true);
            return;
        }

        match self.skills_data.create_skill(&self.new_skill_id) {
            Ok(_) => {
                // 建立後直接選中這個技能
                self.selected_skill = Some(self.new_skill_id.clone());
                self.new_skill_id.clear();
                self.has_unsaved_changes_flag = true; // 標記為已修改
                self.set_status(format!("成功建立技能"), false);
            }
            Err(err) => {
                self.set_status(err, true);
            }
        }
    }

    fn show_tags_editor(ui: &mut Ui, skill: &mut Skill) -> bool {
        let mut changed = false;
        let active = vec![Tag::Passive, Tag::Active];
        let area = vec![Tag::Single, Tag::Area];
        let range = vec![Tag::Caster, Tag::Melee, Tag::Ranged];

        ui.group(|ui| {
            let mut selected = if skill.tags.contains(&Tag::Passive) {
                0
            } else {
                1 // 默認為 Active
            };

            tag_button_group(ui, &active, skill, &mut selected);
        });

        ui.group(|ui| {
            let mut selected = if skill.tags.contains(&Tag::Area) {
                1
            } else {
                0 // 默認為 Single
            };

            tag_button_group(ui, &area, skill, &mut selected);
        });

        ui.group(|ui| {
            let mut selected = if skill.tags.contains(&Tag::Caster) {
                0
            } else if skill.tags.contains(&Tag::Ranged) {
                2
            } else {
                1 // 默認為 Melee
            };

            tag_button_group(ui, &range, skill, &mut selected);
        });

        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                for tag in Tag::iter() {
                    if active.contains(&tag) || area.contains(&tag) || range.contains(&tag) {
                        continue;
                    }
                    let tag_str = format!("{:?}", tag).to_lowercase();
                    let has_tag = skill.tags.contains(&tag);
                    let mut checked = has_tag;

                    if ui.checkbox(&mut checked, tag_str).changed() {
                        if checked && !has_tag {
                            skill.tags.insert(tag.clone());
                        } else if !checked && has_tag {
                            skill.tags.remove(&tag);
                        }
                        changed = true;
                    }
                }
            });
        });

        changed
    }

    fn show_effect_editor(
        ui: &mut Ui,
        effect: &mut Effect,
        shape_editor: impl Fn(&mut Ui, &mut Shape) -> bool,
    ) -> bool {
        let mut changed = false;
        match effect {
            Effect::Hp {
                target_type,
                shape,
                value,
            } => {
                // 目標類型
                ui.horizontal(|ui| {
                    ui.label("目標類型:");
                    let response = egui::ComboBox::new("target_type", "")
                        .selected_text(format!("{:?}", target_type.clone()).to_lowercase())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(target_type, TargetType::Caster, "施法者");
                            ui.selectable_value(target_type, TargetType::Ally, "盟友");
                            ui.selectable_value(
                                target_type,
                                TargetType::AllyExcludeCaster,
                                "盟友（排除施法者）",
                            );
                            ui.selectable_value(target_type, TargetType::Enemy, "敵人");
                            ui.selectable_value(target_type, TargetType::Any, "任何");
                            ui.selectable_value(
                                target_type,
                                TargetType::AnyExcludeCaster,
                                "任何（排除施法者）",
                            );
                        });
                    if response.response.changed() {
                        changed = true;
                    }
                });

                // 形狀
                ui.horizontal(|ui| {
                    ui.label("形狀:");
                    if shape_editor(ui, shape) {
                        changed = true;
                    }
                });

                // 數值
                ui.horizontal(|ui| {
                    ui.label("HP 變化值:");
                    if ui.add(DragValue::new(value)).changed() {
                        changed = true;
                    }
                });
            }
            Effect::Burn {
                target_type,
                shape,
                duration,
            } => {
                // 目標類型
                ui.horizontal(|ui| {
                    ui.label("目標類型:");
                    let response = egui::ComboBox::new("target_type", "")
                        .selected_text(format!("{:?}", target_type.clone()).to_lowercase())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(target_type, TargetType::Caster, "施法者");
                            ui.selectable_value(target_type, TargetType::Ally, "盟友");
                            ui.selectable_value(
                                target_type,
                                TargetType::AllyExcludeCaster,
                                "盟友（排除施法者）",
                            );
                            ui.selectable_value(target_type, TargetType::Enemy, "敵人");
                            ui.selectable_value(target_type, TargetType::Any, "任何");
                            ui.selectable_value(
                                target_type,
                                TargetType::AnyExcludeCaster,
                                "任何（排除施法者）",
                            );
                        });

                    if response.response.changed() {
                        changed = true;
                    }
                });

                // 形狀
                ui.horizontal(|ui| {
                    ui.label("形狀:");
                    if shape_editor(ui, shape) {
                        changed = true;
                    }
                });

                // 持續回合
                ui.horizontal(|ui| {
                    ui.label("持續回合:");
                    if ui.add(DragValue::new(duration)).changed() {
                        changed = true;
                    }
                });
            }
            Effect::MovePoints {
                target_type,
                shape,
                value,
                duration,
            } => {
                // 目標類型
                ui.horizontal(|ui| {
                    ui.label("目標類型:");
                    let response = egui::ComboBox::new("target_type", "")
                        .selected_text(format!("{:?}", target_type.clone()).to_lowercase())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(target_type, TargetType::Caster, "施法者");
                            ui.selectable_value(target_type, TargetType::Ally, "盟友");
                            ui.selectable_value(
                                target_type,
                                TargetType::AllyExcludeCaster,
                                "盟友（排除施法者）",
                            );
                            ui.selectable_value(target_type, TargetType::Enemy, "敵人");
                            ui.selectable_value(target_type, TargetType::Any, "任何");
                            ui.selectable_value(
                                target_type,
                                TargetType::AnyExcludeCaster,
                                "任何（排除施法者）",
                            );
                        });
                    if response.response.changed() {
                        changed = true;
                    }
                });

                // 形狀
                ui.horizontal(|ui| {
                    ui.label("形狀:");
                    if shape_editor(ui, shape) {
                        changed = true;
                    }
                });

                // 數值
                ui.horizontal(|ui| {
                    ui.label("移動點數變化值:");
                    if ui.add(DragValue::new(value)).changed() {
                        changed = true;
                    }
                });

                // 持續回合
                ui.horizontal(|ui| {
                    ui.label("持續回合 (-1=永久, 0=立即):");
                    if ui
                        .add(DragValue::new(duration).range(-1..=i32::MAX))
                        .changed()
                    {
                        changed = true;
                    }
                });
            }
        }

        changed
    }

    fn show_shape_editor(ui: &mut Ui, shape: &mut Shape) -> bool {
        let mut changed = false;
        let shape_type = match shape {
            Shape::Point => "點".to_string(),
            Shape::Circle(_) => "圓形".to_string(),
            Shape::Line(_) => "直線".to_string(),
            Shape::Cone(_, _) => "錐形".to_string(),
        };

        // 切換
        egui::ComboBox::new("shape_type", "")
            .selected_text(shape_type)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(matches!(shape, Shape::Point), "點")
                    .clicked()
                {
                    *shape = Shape::Point;
                    changed = true;
                }
                if ui
                    .selectable_label(matches!(shape, Shape::Circle(_)), "圓形")
                    .clicked()
                {
                    if !matches!(shape, Shape::Circle(_)) {
                        *shape = Shape::Circle(1);
                        changed = true;
                    }
                }
                if ui
                    .selectable_label(matches!(shape, Shape::Line(_)), "直線")
                    .clicked()
                {
                    if !matches!(shape, Shape::Line(_)) {
                        *shape = Shape::Line(3);
                        changed = true;
                    }
                }
                if ui
                    .selectable_label(matches!(shape, Shape::Cone(_, _)), "錐形")
                    .clicked()
                {
                    if !matches!(shape, Shape::Cone(_, _)) {
                        *shape = Shape::Cone(3, 45);
                        changed = true;
                    }
                }
            });

        // 各個形狀細節
        ui.horizontal(|ui| match shape {
            Shape::Point => {}
            Shape::Circle(radius) => {
                ui.add_space(20.0);
                ui.label("半徑:");
                if ui.add(DragValue::new(radius).range(1..=10)).changed() {
                    changed = true;
                }
            }
            Shape::Line(length) => {
                ui.add_space(20.0);
                ui.label("長度:");
                if ui.add(DragValue::new(length).range(1..=10)).changed() {
                    changed = true;
                }
            }
            Shape::Cone(length, angle) => {
                ui.add_space(20.0);
                ui.label("長度:");
                if ui.add(DragValue::new(length).range(1..=10)).changed() {
                    changed = true;
                }
                ui.label("角度:");
                if ui
                    .add(DragValue::new(angle).range(10.0..=120.0).suffix("°"))
                    .changed()
                {
                    changed = true;
                }
            }
        });

        // Return whether anything changed
        changed
    }

    fn save_file(&mut self, path: PathBuf) {
        for (skill_id, skill) in self.skills_data.skills.iter() {
            if let Err(err) = SkillsData::validate(skill) {
                self.set_status(format!("技能 {} 驗證失敗: {}", skill_id, err), true);
                return;
            }
        }
        match self.skills_data.save_to_file(&path) {
            Ok(_) => {
                self.current_file_path = Some(path);
                self.has_unsaved_changes_flag = false;
                self.set_status(format!("成功儲存檔案"), false);
            }
            Err(err) => {
                self.set_status(format!("儲存檔案失敗: {}", err), true);
            }
        }
    }

    fn load_file(&mut self, path: PathBuf) {
        match SkillsData::from_file(&path) {
            Ok(skills_data) => {
                let current_file_path = Some(path);
                *self = Self {
                    skills_data,
                    current_file_path,
                    ..Default::default()
                };
                self.set_status(format!("成功載入檔案"), false);
            }
            Err(err) => {
                self.set_status(format!("載入檔案失敗: {}", err), true);
            }
        }
    }

    fn set_status(&mut self, message: String, is_error: bool) {
        self.status_message = Some((message, is_error));
    }

    fn show_status_message(&mut self, ctx: &egui::Context) {
        if let Some((message, is_error)) = &self.status_message {
            show_status_message(ctx, message, *is_error);
        }
    }

    /// 檢查目前編輯中的技能是否有未保存的變動
    pub fn has_unsaved_changes(&self) -> bool {
        self.has_unsaved_changes_flag
    }
}

fn tag_button_group(ui: &mut Ui, tags: &[Tag], skill: &mut Skill, selected: &mut usize) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        for (i, tag) in tags.iter().enumerate() {
            if ui
                .radio_value(selected, i, format!("{:?}", tag).to_lowercase())
                .clicked()
            {
                for t in tags {
                    skill.tags.remove(t);
                }
                skill.tags.insert(tag.clone());
                changed = true;
            }
        }
    });
    changed
}

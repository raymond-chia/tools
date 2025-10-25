use crate::common::*;
use eframe::{Frame, egui};
use egui::{Button, DragValue, ScrollArea, Separator, Ui};
use serde::{Deserialize, Serialize};
use skills_lib::*;
use std::collections::{BTreeMap, HashMap};
use std::io;
use std::path::{Path, PathBuf};
use strum::IntoEnumIterator;

const BASIC_PASSIVE_TARGET_TYPE: TargetType = TargetType::Caster;
const BASIC_PASSIVE_SHAPE: Shape = Shape::Point;
const BASIC_PASSIVE_DURATION: i32 = -1;

/// 判斷是否為種族技能五大效果
fn is_basic_passive_effect(effect: &Effect) -> bool {
    matches!(
        effect,
        Effect::MaxHp { .. }
            | Effect::Initiative { .. }
            | Effect::Evasion { .. }
            | Effect::Block { .. }
            | Effect::MovePoints { .. }
    )
}

/// 技能資料集
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SkillsData {
    #[serde(flatten)]
    pub skills: HashMap<SkillID, Skill>,
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
        // 條件1: active, passive, basic passive 只能擇一
        let mut count = 0;
        for tag in [&Tag::BasicPassive, &Tag::Passive, &Tag::Active] {
            if skill.tags.contains(tag) {
                count += 1;
            }
        }
        if count != 1 {
            return Err(
                "技能不能同時是基礎被動 (Basic Passive)、被動 (Passive)、主動 (Active) 標籤"
                    .to_string(),
            );
        }
        if skill.tags.contains(&Tag::BasicPassive) {
            if let Err(msg) = validate_basic_passive_skill(skill) {
                return Err(format!("種族說明技能格式錯誤: {}", msg));
            }
        }

        // 條件2: single, area 只能擇一
        let mut count = 0;
        for tag in [&Tag::Single, &Tag::Area] {
            if skill.tags.contains(tag) {
                count += 1;
            }
        }
        if count != 1 {
            return Err("技能不能同時是單體 (Single) 和範圍 (Area)".to_string());
        }

        // 條件3: caster, melee, ranged 只能擇一
        let mut count = 0;
        for tag in [&Tag::Caster, &Tag::Melee, &Tag::Ranged] {
            if skill.tags.contains(tag) {
                count += 1;
            }
        }
        if count != 1 {
            return Err("技能的作用範圍 (Caster/Melee/Ranged) 只能擇一".to_string());
        }

        // 檢查單體技能
        if skill.tags.contains(&Tag::Single) {
            if skill.effects[0].shape() != &Shape::Point {
                return Err("單體技能的效果形狀必須是點".to_string());
            }
        }

        // 檢查範圍技能
        if skill.tags.contains(&Tag::Area) {
            match skill.effects[0].shape() {
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
            }
        }

        // 檢查施法者技能
        if skill.tags.contains(&Tag::Caster) {
            if skill.range.0 != 0 || skill.range.1 != 0 {
                return Err("施法者技能的範圍必須是 (0, 0)".to_string());
            }

            if skill.effects[0].target_type() != &TargetType::Caster {
                return Err("施法者技能的目標類型必須是施法者".to_string());
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
    new_skill_id: SkillID,
    selected_skill: Option<SkillID>,
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
        let mut result = Self::default();
        result.reload();
        return result;
    }

    /// 重新載入固定技能檔案（SKILLS_FILE），失敗時保留原資料並回傳錯誤
    pub fn reload(&mut self) {
        self.load_file(skills_file());
    }

    /// 儲存技能資料到固定檔案（SKILLS_FILE），失敗時回傳錯誤
    pub fn save(&mut self) {
        self.save_file(skills_file());
    }
}

#[derive(Debug, Clone)]
enum ConfirmationAction {
    None,
    DeleteSkill(SkillID),
    DeleteEffect(SkillID, usize),
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
                ui.separator();
                if ui.button("重新載入").clicked() {
                    self.reload();
                }
                if ui.button("儲存").clicked() {
                    self.save();
                }
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
            /// 三層 tag 分組 helper
            fn group_by_tags<'a>(
                skills: &HashMap<SkillID, Skill>,
                primary: &[Tag],
                secondary: &[Tag],
                tertiary: &[Tag],
            ) -> (BTreeMap<(Tag, Tag, Tag), Vec<SkillID>>, Vec<SkillID>) {
                let mut map = BTreeMap::new();
                let mut unmatched = Vec::new();
                for (id, skill) in skills {
                    let p = primary.iter().find(|t| skill.tags.contains(t)).cloned();
                    let s = secondary.iter().find(|t| skill.tags.contains(t)).cloned();
                    let t = tertiary.iter().find(|t| skill.tags.contains(t)).cloned();
                    if let (Some(p), Some(s), Some(t)) = (p, s, t) {
                        map.entry((p, s, t))
                            .or_insert_with(Vec::new)
                            .push(id.clone());
                    } else {
                        unmatched.push(id.clone());
                    }
                }
                (map, unmatched)
            }

            let primary = [Tag::Active, Tag::Passive];
            let secondary = [Tag::Physical, Tag::Magical];
            let tertiary = [Tag::Caster, Tag::Melee, Tag::Ranged];
            let (grouped, unmatched) =
                group_by_tags(&self.skills_data.skills, &primary, &secondary, &tertiary);
            // 種族技能
            let mut basic_passive_skill_ids = Vec::new();
            for (id, skill) in &self.skills_data.skills {
                if skill.tags.contains(&Tag::BasicPassive) {
                    basic_passive_skill_ids.push(id.clone());
                }
            }

            for ((p, s, t), skill_ids) in grouped {
                let title = format!("─── {:?}-{:?}-{:?}技能 ───", p, s, t);
                self.show_skill_category(ui, &title, &skill_ids);
            }
            self.show_skill_category(ui, "─── 種族技能 ───", &basic_passive_skill_ids);
            // 顯示未完全分組的技能
            let unmatched: Vec<SkillID> = unmatched
                .into_iter()
                .filter(|id| !basic_passive_skill_ids.contains(id))
                .collect();
            if !unmatched.is_empty() {
                self.show_skill_category(ui, "─── 未分類技能 ───", &unmatched);
            }
        });
    }

    /// 類別技能顯示（可直接操作 self.selected_skill）
    fn show_skill_category(&mut self, ui: &mut Ui, title: &str, skill_ids: &[SkillID]) {
        ui.label(title);
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
                self.selected_skill = Some(skill_id.clone());
            }
        }
        ui.add(Separator::default());
    }

    fn show_skill_editor(&mut self, ui: &mut Ui) {
        // 首先添加標題和按鈕（這些保持在固定位置）
        let mut delete_clicked = false;
        let mut copy_clicked = false;
        let mut init_basic_passive = false;
        let mut add_effect_clicked = false;
        let mut move_up_effect_index: Option<usize> = None;
        let mut move_down_effect_index: Option<usize> = None;
        let mut delete_effect_index = None;

        let skill_id = match &self.selected_skill {
            None => {
                ui.heading("技能編輯器");
                ui.label("選擇或建立一個技能開始編輯");
                return;
            }
            Some(skill_id) => skill_id,
        };
        ui.heading("技能編輯: ");
        let skill = match self.skills_data.skills.get_mut(skill_id) {
            None => {
                self.set_status("技能不存在".to_string(), true);
                return;
            }
            Some(skill) => skill,
        };
        let mut new_skill_id = skill_id.clone();
        ui.text_edit_singleline(&mut new_skill_id);

        ui.horizontal(|ui| {
            delete_clicked = ui.button("刪除技能").clicked();
            copy_clicked = ui.button("複製技能").clicked();

            // 新增「初始化種族說明技能」按鈕
            if skill.tags.contains(&Tag::BasicPassive) {
                init_basic_passive = ui.button("初始化種族說明技能").clicked();
            }
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
                // 基本屬性編輯
                ui.heading("基本屬性");

                // 標籤編輯
                ui.collapsing("標籤", |ui| {
                    if Self::show_tags_editor(ui, skill) {
                        self.has_unsaved_changes_flag = true;
                    }
                });

                // 範圍編輯
                // 若為種族技能，隱藏範圍、消耗、命中、爆擊
                self.has_unsaved_changes_flag |= show_basic_skill_editor(ui, skill);

                ui.add_space(8.0);
                ui.add(Separator::default());

                // 效果編輯
                ui.horizontal(|ui| {
                    ui.heading("效果");
                    add_effect_clicked = ui.button("新增效果").clicked();
                });

                // 處理效果編輯
                let effects_len = skill.effects.len();
                for (index, effect) in skill.effects.iter_mut().enumerate() {
                    ui.push_id(index, |ui| {
                        self.has_unsaved_changes_flag |= show_skill_effect_editor(
                            ui,
                            index,
                            effect,
                            skill.tags.contains(&Tag::BasicPassive),
                            effects_len,
                            &mut move_up_effect_index,
                            &mut move_down_effect_index,
                            &mut delete_effect_index,
                        );

                        ui.add_space(8.0);
                    });
                }
            });

        // for 迴圈外統一處理 move
        if let Some(idx) = move_up_effect_index.take() {
            skill.effects.swap(idx, idx - 1);
            self.has_unsaved_changes_flag = true;
        }
        if let Some(idx) = move_down_effect_index.take() {
            skill.effects.swap(idx, idx + 1);
            self.has_unsaved_changes_flag = true;
        }

        if &new_skill_id != skill_id && !self.skills_data.skills.contains_key(&new_skill_id) {
            let skill = match self.skills_data.skills.remove(skill_id) {
                None => {
                    self.set_status(format!("舊技能名稱應該存在: {skill_id}"), true);
                    return;
                }
                Some(skill) => skill,
            };
            self.skills_data.skills.insert(new_skill_id.clone(), skill);
            self.selected_skill = Some(new_skill_id.clone());
        }

        // 處理刪除技能按鈕
        if delete_clicked && self.selected_skill.is_some() {
            let skill_id = self.selected_skill.clone().unwrap();
            self.confirmation_action = ConfirmationAction::DeleteSkill(skill_id);
            self.show_confirmation_dialog = true;
        }
        // 處理複製技能按鈕
        if copy_clicked {
            self.copy_skill();
        }
        if init_basic_passive {
            self.init_basic_passive_skill_effects(&new_skill_id);
        }

        // 處理添加效果按鈕
        if add_effect_clicked {
            self.show_add_effect_popup = true;
        }

        // 處理刪除效果
        if delete_effect_index.is_some() && self.selected_skill.is_some() {
            let skill_id = self
                .selected_skill
                .clone()
                .expect("selected skill in race condition");
            let index = delete_effect_index
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
                        Effect::MaxHp { .. } => ui.button("新增最大 HP 效果").clicked(),
                        Effect::Initiative { .. } => ui.button("新增先攻值效果").clicked(),
                        Effect::Evasion { .. } => ui.button("新增閃避效果").clicked(),
                        Effect::Block { .. } => ui.button("新增格擋效果").clicked(),
                        Effect::MovePoints { .. } => ui.button("新增移動點數效果").clicked(),
                        Effect::Burn { .. } => ui.button("新增燃燒效果").clicked(),
                        Effect::HitAndRun { .. } => ui.button("新增打帶跑效果").clicked(),
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
        let active = [Tag::BasicPassive, Tag::Passive, Tag::Active];
        let area = [Tag::Single, Tag::Area];
        let range = [Tag::Caster, Tag::Melee, Tag::Ranged];
        let pm_group = [Tag::Physical, Tag::Magical];

        ui.group(|ui| {
            // 0: Basic Passive, 1: Passive, 2: Active
            let mut selected = if skill.tags.contains(&Tag::BasicPassive) {
                active.iter().position(|e| e == &Tag::BasicPassive).unwrap()
            } else if skill.tags.contains(&Tag::Passive) {
                active.iter().position(|e| e == &Tag::Passive).unwrap()
            } else {
                // 默認為 Active
                active.iter().position(|e| e == &Tag::Active).unwrap()
            };
            changed |= tag_button_group(ui, &active, skill, &mut selected);
        });

        ui.group(|ui| {
            let mut selected = if skill.tags.contains(&Tag::Area) {
                area.iter().position(|e| e == &Tag::Area).unwrap()
            } else {
                // 默認為 Single
                area.iter().position(|e| e == &Tag::Single).unwrap()
            };
            changed |= tag_button_group(ui, &area, skill, &mut selected);
        });

        ui.group(|ui| {
            let mut selected = if skill.tags.contains(&Tag::Caster) {
                range.iter().position(|e| e == &Tag::Caster).unwrap()
            } else if skill.tags.contains(&Tag::Ranged) {
                range.iter().position(|e| e == &Tag::Ranged).unwrap()
            } else {
                // 默認為 Melee
                range.iter().position(|e| e == &Tag::Melee).unwrap()
            };
            changed |= tag_button_group(ui, &range, skill, &mut selected);
        });

        ui.group(|ui| {
            for tag in pm_group.iter() {
                let tag_str = format!("{:?}", tag).to_lowercase();
                let mut checked = skill.tags.contains(tag);
                if ui.checkbox(&mut checked, tag_str).changed() {
                    if checked {
                        skill.tags.insert(tag.clone());
                    } else {
                        skill.tags.remove(tag);
                    }
                    changed = true;
                }
            }
        });

        // 其他 tag 多選
        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                for tag in Tag::iter() {
                    if active.contains(&tag)
                        || area.contains(&tag)
                        || range.contains(&tag)
                        || pm_group.contains(&tag)
                    {
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

    /// 複製目前選取的技能，產生新 ID（自動加 "_copy" 並避免重複），並選取新技能
    fn copy_skill(&mut self) {
        let skill_id = match &self.selected_skill {
            Some(id) => id.clone(),
            None => {
                self.set_status("請先選擇要複製的技能".to_string(), true);
                return;
            }
        };
        let orig_skill = match self.skills_data.skills.get(&skill_id) {
            Some(skill) => skill.clone(),
            None => {
                self.set_status("技能不存在".to_string(), true);
                return;
            }
        };
        // 自動產生新 ID
        let mut new_id = format!("{}_copy", skill_id);
        let mut idx = 2;
        while self.skills_data.skills.contains_key(&new_id) {
            new_id = format!("{}_copy{}", skill_id, idx);
            idx += 1;
        }
        self.skills_data.skills.insert(new_id.clone(), orig_skill);
        self.selected_skill = Some(new_id.clone());
        self.has_unsaved_changes_flag = true;
        self.set_status(format!("已複製技能為 {}", new_id), false);
    }

    /// 初始化種族說明技能效果，符合 validate_basic_passive_skill 規則
    fn init_basic_passive_skill_effects(&mut self, skill: &str) {
        // 定義五種效果的順序與型別
        let basic_passive_types: [fn(i32) -> Effect; 5] = [
            |value| Effect::MaxHp {
                target_type: BASIC_PASSIVE_TARGET_TYPE,
                shape: BASIC_PASSIVE_SHAPE,
                value,
                duration: BASIC_PASSIVE_DURATION,
            },
            |value| Effect::Initiative {
                target_type: BASIC_PASSIVE_TARGET_TYPE,
                shape: BASIC_PASSIVE_SHAPE,
                value,
                duration: BASIC_PASSIVE_DURATION,
            },
            |value| Effect::Evasion {
                target_type: BASIC_PASSIVE_TARGET_TYPE,
                shape: BASIC_PASSIVE_SHAPE,
                value,
                duration: BASIC_PASSIVE_DURATION,
            },
            |value| Effect::Block {
                target_type: BASIC_PASSIVE_TARGET_TYPE,
                shape: BASIC_PASSIVE_SHAPE,
                value,
                duration: BASIC_PASSIVE_DURATION,
            },
            |value| Effect::MovePoints {
                target_type: BASIC_PASSIVE_TARGET_TYPE,
                shape: BASIC_PASSIVE_SHAPE,
                value,
                duration: BASIC_PASSIVE_DURATION,
            },
        ];

        // 先收集現有五種效果的 value
        let mut found: [Option<i32>; 5] = [None, None, None, None, None];
        let mut others: Vec<Effect> = Vec::new();

        let skill = match self.skills_data.skills.get_mut(skill) {
            None => {
                self.set_status("技能不存在".to_string(), true);
                return;
            }
            Some(skill) => skill,
        };
        for eff in &skill.effects {
            match eff {
                Effect::MaxHp { value, .. } => found[0] = Some(*value),
                Effect::Initiative { value, .. } => found[1] = Some(*value),
                Effect::Evasion { value, .. } => found[2] = Some(*value),
                Effect::Block { value, .. } => found[3] = Some(*value),
                Effect::MovePoints { value, .. } => found[4] = Some(*value),
                _ => others.push(eff.clone()),
            }
        }

        // 產生五種效果（保留 value，覆蓋其他欄位，缺少則補 0）
        let mut new_basic_passive_effects = Vec::with_capacity(5);
        for i in 0..5 {
            let value = found[i].unwrap_or(0);
            new_basic_passive_effects.push(basic_passive_types[i](value));
        }

        // 依 validate_basic_passive_skill 順序排列於最前面，其他效果保留順序在末尾
        skill.tags = [Tag::BasicPassive, Tag::Single, Tag::Caster]
            .into_iter()
            .collect();
        skill.effects.clear();
        skill.effects.extend(new_basic_passive_effects);
        skill.effects.extend(others);

        self.has_unsaved_changes_flag = true;
        self.set_status("已初始化種族說明技能效果".to_string(), false);
    }

    fn save_file(&mut self, path: PathBuf) {
        for (skill_id, skill) in &self.skills_data.skills {
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

fn validate_basic_passive_skill(skill: &Skill) -> Result<(), String> {
    if skill.tags.len() != 3
        || !skill.tags.contains(&Tag::BasicPassive)
        || !skill.tags.contains(&Tag::Single)
        || !skill.tags.contains(&Tag::Caster)
    {
        return Err("基礎被動標籤不對".to_string());
    }
    if skill.range != (0, 0) {
        return Err("基礎被動不該有範圍".to_string());
    }
    if skill.cost != 0 {
        return Err("基礎被動不該有消耗".to_string());
    }
    if skill.accuracy != None {
        return Err("基礎被動不該有命中率".to_string());
    }
    if skill.crit_rate != None {
        return Err("基礎被動不該有暴擊率".to_string());
    }
    let mut effects = skill.effects.iter();
    let check = |effect: &Effect| {
        effect.target_type() == &BASIC_PASSIVE_TARGET_TYPE
            && effect.shape() == &BASIC_PASSIVE_SHAPE
            && effect.duration() == BASIC_PASSIVE_DURATION
    };
    let max_hp = effects
        .next()
        .ok_or_else(|| "max_hp not found".to_string())?;
    match max_hp {
        Effect::MaxHp { .. } => {
            if !check(max_hp) {
                return Err("hp max 設定有問題".to_string());
            }
        }
        _ => return Err("first effect must be max_hp".to_string()),
    }
    let initiative = effects
        .next()
        .ok_or_else(|| "initiative not found".to_string())?;
    match initiative {
        Effect::Initiative { .. } => {
            if !check(initiative) {
                return Err("initiative 設定有問題".to_string());
            }
        }
        _ => return Err("second effect must be initiative".to_string()),
    }
    let evasion = effects
        .next()
        .ok_or_else(|| "evasion not found".to_string())?;
    match evasion {
        Effect::Evasion { .. } => {
            if !check(evasion) {
                return Err("evasion 設定有問題".to_string());
            }
        }
        _ => return Err("third effect must be evasion".to_string()),
    }
    let block = effects
        .next()
        .ok_or_else(|| "block not found".to_string())?;
    match block {
        Effect::Block { .. } => {
            if !check(block) {
                return Err("block 設定有問題".to_string());
            }
        }
        _ => return Err("fourth effect must be block".to_string()),
    }
    let move_points = effects
        .next()
        .ok_or_else(|| "move_points not found".to_string())?;
    match move_points {
        Effect::MovePoints { .. } => {
            if !check(move_points) {
                return Err("move_points 設定有問題".to_string());
            }
        }
        _ => return Err("fifth effect must be move_points".to_string()),
    }
    Ok(())
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

fn show_basic_skill_editor(ui: &mut Ui, skill: &mut Skill) -> bool {
    let mut changed = false;
    if skill.tags.contains(&Tag::BasicPassive) {
        return changed;
    }
    ui.horizontal(|ui| {
        ui.label("範圍:");
        if ui
            .add(DragValue::new(&mut skill.range.0).prefix("最小: "))
            .changed()
        {
            changed = true;
        }
        if ui
            .add(DragValue::new(&mut skill.range.1).prefix("最大: "))
            .changed()
        {
            changed = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label("消耗:");
        if ui.add(DragValue::new(&mut skill.cost)).changed() {
            changed = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label("命中數值:");
        let mut has_accuracy = skill.accuracy.is_some();
        if ui.checkbox(&mut has_accuracy, "").changed() {
            skill.accuracy = if has_accuracy { Some(100) } else { None };
            changed = true;
        }

        if let Some(accuracy) = &mut skill.accuracy {
            if ui
                .add_enabled(has_accuracy, DragValue::new(accuracy).range(0..=i32::MAX))
                .changed()
            {
                changed = true;
            }
        }
    });

    ui.horizontal(|ui| {
        ui.label("爆擊率:");
        let mut has_crit_rate = skill.crit_rate.is_some();
        if ui.checkbox(&mut has_crit_rate, "").changed() {
            skill.crit_rate = if has_crit_rate { Some(10) } else { None };
            changed = true;
        }

        if let Some(crit_rate) = &mut skill.crit_rate {
            if ui
                .add_enabled(
                    has_crit_rate,
                    DragValue::new(crit_rate).range(0..=100).suffix("%"),
                )
                .changed()
            {
                changed = true;
            }
        }
    });
    changed
}

fn show_skill_effect_editor(
    ui: &mut Ui,
    index: usize,
    effect: &mut Effect,
    is_basic_passive_skill: bool,
    effects_len: usize,
    move_up_effect_index: &mut Option<usize>,
    move_down_effect_index: &mut Option<usize>,
    delete_effect_index: &mut Option<usize>,
) -> bool {
    let mut move_up_clicked = false;
    let mut move_down_clicked = false;
    let mut delete_effect_clicked = false;
    let is_basic_passive_effect = is_basic_passive_skill && is_basic_passive_effect(effect);

    ui.horizontal(|ui| {
        match effect {
            Effect::Hp { .. } => ui.label("HP"),
            Effect::MaxHp { .. } => ui.label("最大 HP"),
            Effect::Initiative { .. } => ui.label("先攻值"),
            Effect::Evasion { .. } => ui.label("閃避"),
            Effect::Block { .. } => ui.label("格擋"),
            Effect::MovePoints { .. } => ui.label("移動點數"),
            Effect::Burn { .. } => ui.label("燃燒"),
            Effect::HitAndRun { .. } => ui.label("打帶跑效果"),
        };
        // 種族效果不顯示刪除、上下移動
        if !is_basic_passive_effect {
            move_up_clicked = ui.add_enabled(index > 0, Button::new("↑")).clicked();
            move_down_clicked = ui
                .add_enabled(index + 1 < effects_len, Button::new("↓"))
                .clicked();
            delete_effect_clicked = ui.button("🗑").clicked();
        }
    });

    if !is_basic_passive_effect {
        if move_up_clicked {
            *move_up_effect_index = Some(index);
        }
        if move_down_clicked {
            *move_down_effect_index = Some(index);
        }
        if delete_effect_clicked {
            *delete_effect_index = Some(index);
        }
    }

    // 效果編輯器：種族效果不顯示目標、形狀、持續回合
    let mut changed = false;
    ui.indent(format!("effect_{}", index), |ui| {
        if !is_basic_passive_effect {
            if show_effect_editor(ui, effect) {
                changed = true;
            }
        } else {
            // 只顯示數值編輯器
            match effect {
                Effect::MaxHp { value, .. } => {
                    changed = show_value_editor(ui, value, "");
                }
                Effect::Initiative { value, .. } => {
                    changed = show_value_editor(ui, value, "");
                }
                Effect::Evasion { value, .. } => {
                    changed = show_value_editor(ui, value, "");
                }
                Effect::Block { value, .. } => {
                    changed = show_value_editor(ui, value, "");
                }
                Effect::MovePoints { value, .. } => {
                    changed = show_value_editor(ui, value, "");
                }
                _ => {}
            }
        }
    });
    changed
}

fn show_effect_editor(ui: &mut Ui, effect: &mut Effect) -> bool {
    let mut changed = false;
    match effect {
        Effect::Hp {
            target_type,
            shape,
            value,
        } => {
            changed |= show_target_type_editor(ui, target_type);
            ui.horizontal(|ui| {
                ui.label("形狀:");
                changed |= show_shape_editor(ui, shape);
            });
            changed |= show_value_editor(ui, value, "HP 變化值:");
        }
        Effect::MaxHp {
            target_type,
            shape,
            value,
            duration,
        } => {
            changed |= show_target_type_editor(ui, target_type);
            ui.horizontal(|ui| {
                ui.label("形狀:");
                changed |= show_shape_editor(ui, shape);
            });
            changed |= show_value_editor(ui, value, "最大 HP 變化值:");
            changed |= show_duration_editor(ui, duration);
        }
        Effect::Initiative {
            target_type,
            shape,
            value,
            duration,
        } => {
            changed |= show_target_type_editor(ui, target_type);
            ui.horizontal(|ui| {
                ui.label("形狀:");
                changed |= show_shape_editor(ui, shape);
            });
            changed |= show_value_editor(ui, value, "先攻變化值:");
            changed |= show_duration_editor(ui, duration);
        }
        Effect::Evasion {
            target_type,
            shape,
            value,
            duration,
        } => {
            changed |= show_target_type_editor(ui, target_type);
            ui.horizontal(|ui| {
                ui.label("形狀:");
                changed |= show_shape_editor(ui, shape);
            });
            changed |= show_value_editor(ui, value, "閃避數值變化：");
            changed |= show_duration_editor(ui, duration);
        }
        Effect::Block {
            target_type,
            shape,
            value,
            duration,
        } => {
            changed |= show_target_type_editor(ui, target_type);
            ui.horizontal(|ui| {
                ui.label("形狀:");
                changed |= show_shape_editor(ui, shape);
            });
            changed |= show_value_editor(ui, value, "格擋數值變化：");
            changed |= show_duration_editor(ui, duration);
        }
        Effect::MovePoints {
            target_type,
            shape,
            value,
            duration,
        } => {
            changed |= show_target_type_editor(ui, target_type);
            ui.horizontal(|ui| {
                ui.label("形狀:");
                changed |= show_shape_editor(ui, shape);
            });
            changed |= show_value_editor(ui, value, "移動點數變化值:");
            changed |= show_duration_editor(ui, duration);
        }
        Effect::Burn {
            target_type,
            shape,
            duration,
        } => {
            changed |= show_target_type_editor(ui, target_type);
            ui.horizontal(|ui| {
                ui.label("形狀:");
                changed |= show_shape_editor(ui, shape);
            });
            changed |= show_duration_editor(ui, duration);
        }
        Effect::HitAndRun {
            target_type,
            shape,
            duration,
        } => {
            changed |= show_target_type_editor(ui, target_type);
            ui.horizontal(|ui| {
                ui.label("形狀:");
                changed |= show_shape_editor(ui, shape);
            });
            changed |= show_duration_editor(ui, duration);
        }
    }
    changed
}

// 共用目標類型編輯器
fn show_target_type_editor(ui: &mut Ui, target_type: &mut TargetType) -> bool {
    let mut changed = false;
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
                ui.selectable_value(target_type, TargetType::AnyUnit, "任何單位");
                ui.selectable_value(target_type, TargetType::Any, "任何");
            });
        if response.response.changed() {
            changed = true;
        }
    });
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

// 共用持續回合編輯器
fn show_duration_editor(ui: &mut Ui, duration: &mut i32) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("持續回合 (-1=永久):");
        if ui
            .add(DragValue::new(duration).range(-1..=i32::MAX))
            .changed()
        {
            changed = true;
        }
    });
    changed
}

// 共用數值編輯器（label 可參數化）
fn show_value_editor(ui: &mut Ui, value: &mut i32, label: &str) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        if ui.add(DragValue::new(value)).changed() {
            changed = true;
        }
    });
    changed
}

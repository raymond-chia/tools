use eframe::{Frame, egui};
use egui::{Button, DragValue, RichText, ScrollArea, Separator, Ui};
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use skills_lib::{Effect, Shape, Skill, Tag, TargetType};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{self, Error, ErrorKind};
use std::path::{Path, PathBuf};
use strum::IntoEnumIterator;

/// 技能資料集
#[derive(Debug, Clone, Deserialize, Serialize)]
struct SkillsData {
    #[serde(flatten)]
    skills: HashMap<String, Skill>,
}

impl SkillsData {
    /// 從指定路徑載入 TOML 檔案
    fn from_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let content = fs::read_to_string(path)?;
        Self::from_toml_str(&content)
    }

    /// 從 TOML 字串解析
    fn from_toml_str(content: &str) -> io::Result<Self> {
        let skills_map: HashMap<String, Skill> = toml::from_str(content).map_err(|err| {
            Error::new(ErrorKind::InvalidData, format!("解析 TOML 失敗: {}", err))
        })?;

        Ok(Self { skills: skills_map })
    }

    /// 轉換為 TOML 格式
    fn to_toml(&self) -> io::Result<String> {
        let sorted_skills: BTreeMap<_, _> = self.skills.clone().into_iter().collect();

        toml::to_string_pretty(&sorted_skills)
            .map_err(|err| Error::new(ErrorKind::InvalidData, format!("序列化 TOML 失敗: {}", err)))
    }

    /// 寫入到檔案
    fn save_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let toml_content = self.to_toml()?;
        fs::write(path, toml_content)
    }

    /// 新增技能
    fn create_skill(&mut self, skill_id: &str) -> Result<(), String> {
        if self.skills.contains_key(skill_id) {
            return Err("技能 ID 已存在".to_string());
        }
        self.skills.insert(skill_id.to_string(), Skill::default());
        Ok(())
    }

    /// 更新技能屬性
    fn update_skill(&mut self, skill_id: &str, updated_skill: Skill) -> Result<(), String> {
        let Some(skill) = self.skills.get_mut(skill_id) else {
            return Err(format!("找不到技能 ID: {}", skill_id));
        };
        if let Err(e) = Self::validate(&updated_skill) {
            return Err(format!("不合法的技能設定: {}", e));
        }
        *skill = updated_skill;
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
        let has_active = skill.tags.contains(&Tag::Active);
        let has_passive = skill.tags.contains(&Tag::Passive);
        if has_active && has_passive {
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

        // 單體技能檢查
        if skill.tags.contains(&Tag::Single) {
            match &skill.effects[0] {
                Effect::Hp { shape, .. } | Effect::Burn { shape, .. } => {
                    if shape != &Shape::Point {
                        return Err("單體技能的效果形狀必須是點".to_string());
                    }
                }
            }
        }

        // 範圍技能檢查
        if skill.tags.contains(&Tag::Area) {
            match &skill.effects[0] {
                Effect::Hp { shape, .. } | Effect::Burn { shape, .. } => match shape {
                    Shape::Point => {
                        return Err("範圍技能的效果形狀不能是點".to_string());
                    }
                    Shape::Circle(radius) => {
                        if *radius < 2 {
                            return Err("範圍技能的效果形狀半徑不能小於 2".to_string());
                        }
                    }
                    Shape::Rectangle(width, height) => {
                        if *width < 2 && *height < 2 {
                            return Err("範圍技能的效果形狀寬度和高度不能小於 2".to_string());
                        }
                    }
                    Shape::Line(length) => {
                        if *length < 2 {
                            return Err("範圍技能的效果形狀長度不能小於 2".to_string());
                        }
                    }
                    Shape::Cone(radius, angle) => {
                        if *radius < 2 && *angle < 90.0 {
                            return Err(
                                "範圍技能的效果形狀半徑不能小於 2 同時角度又小於 90".to_string()
                            );
                        }
                    }
                },
            }
        }

        // 施法者技能檢查
        if skill.tags.contains(&Tag::Caster) {
            match &skill.effects[0] {
                Effect::Hp { target_type, .. } | Effect::Burn { target_type, .. } => {
                    if target_type != &skills_lib::TargetType::Caster {
                        return Err("施法者技能的目標類型必須是施法者".to_string());
                    }
                }
            }
        }

        Ok(())
    }
}

pub struct SkillsEditor {
    skills_data: SkillsData,
    current_file_path: Option<PathBuf>,
    new_skill_id: String,
    temp_skill: Option<(String, Skill)>,
    status_message: Option<(String, bool)>, // message, is_error
    show_add_effect_popup: bool,
    show_confirmation_dialog: bool,
    confirmation_action: ConfirmationAction,
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
            current_file_path: None,
            new_skill_id: String::new(),
            temp_skill: None,
            status_message: None,
            show_add_effect_popup: false,
            show_confirmation_dialog: false,
            confirmation_action: ConfirmationAction::None,
        }
    }
}

impl SkillsEditor {
    pub fn new(_: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    fn load_file(&mut self, path: PathBuf) {
        match SkillsData::from_file(&path) {
            Ok(data) => {
                self.skills_data = data;
                self.current_file_path = Some(path);
                self.temp_skill = None;
                self.set_status(format!("成功載入檔案"), false);
            }
            Err(err) => {
                self.set_status(format!("載入檔案失敗: {}", err), true);
            }
        }
    }

    fn save_file(&mut self, path: &Path) {
        match self.skills_data.save_to_file(path) {
            Ok(_) => {
                self.current_file_path = Some(path.to_path_buf());
                self.set_status(format!("成功儲存檔案"), false);
            }
            Err(err) => {
                self.set_status(format!("儲存檔案失敗: {}", err), true);
            }
        }
    }

    fn set_status(&mut self, message: String, is_error: bool) {
        self.status_message = Some((message, is_error));
    }

    fn create_skill(&mut self) {
        if self.new_skill_id.is_empty() {
            self.set_status("技能 ID 不能為空".to_string(), true);
            return;
        }

        match self.skills_data.create_skill(&self.new_skill_id) {
            Ok(_) => {
                self.temp_skill = Some((
                    self.new_skill_id.clone(),
                    self.skills_data
                        .skills
                        .get(&self.new_skill_id)
                        .unwrap()
                        .clone(),
                ));
                self.new_skill_id.clear();
                self.set_status(format!("成功建立技能"), false);
            }
            Err(err) => {
                self.set_status(err, true);
            }
        }
    }

    fn show_file_menu(&mut self, ui: &mut Ui) {
        egui::menu::bar(ui, |ui| {
            egui::menu::menu_button(ui, "檔案", |ui| {
                if ui.button("新增").clicked() {
                    self.skills_data = SkillsData {
                        skills: HashMap::new(),
                    };
                    self.current_file_path = None;
                    self.temp_skill = None;
                    self.set_status("已建立新檔案".to_string(), false);
                    ui.close_menu();
                }

                if ui.button("開啟...").clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("TOML", &["toml"])
                        .set_directory(".")
                        .pick_file()
                    {
                        self.load_file(path);
                    }
                    ui.close_menu();
                }

                if ui.button("儲存").clicked() {
                    let should_open_dialog = self.current_file_path.is_none();
                    if !should_open_dialog {
                        let path = self.current_file_path.as_ref().unwrap().clone();
                        self.save_file(&path);
                    } else {
                        if let Some(path) = FileDialog::new()
                            .add_filter("TOML", &["toml"])
                            .set_directory(".")
                            .save_file()
                        {
                            self.save_file(&path);
                        }
                    }
                    ui.close_menu();
                }

                if ui.button("另存為...").clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("TOML", &["toml"])
                        .set_directory(".")
                        .save_file()
                    {
                        self.save_file(&path);
                    }
                    ui.close_menu();
                }
            });
        });
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
                let selected = self.temp_skill.as_ref().map(|(id, _)| id) == Some(skill_id);

                let button = Button::new(skill_id)
                    .fill(if selected {
                        ui.style().visuals.selection.bg_fill
                    } else {
                        ui.style().visuals.widgets.noninteractive.bg_fill
                    })
                    .min_size(egui::vec2(ui.available_width(), 0.0));

                if ui.add(button).clicked() {
                    let skill = self.skills_data.skills.get(skill_id).unwrap().clone();
                    self.temp_skill = Some((skill_id.clone(), skill));
                }
            }
        });
    }

    fn show_skill_editor(&mut self, ui: &mut Ui) {
        // 首先添加標題和按鈕（這些保持在固定位置）
        let mut save_clicked = false;
        let mut delete_clicked = false;
        let mut add_effect_clicked = false;
        let mut delete_effect_indices: Vec<usize> = Vec::new();

        if let Some((skill_id, _)) = &self.temp_skill {
            ui.heading(format!("編輯技能: {}", skill_id));

            ui.horizontal(|ui| {
                save_clicked = ui.button("儲存變更").clicked();
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
                    // 在可捲動區域內編輯技能，直接使用 self.temp_skill
                    if let Some((_, skill)) = &mut self.temp_skill {
                        // 基本屬性編輯
                        ui.heading("基本屬性");

                        // 標籤編輯
                        ui.collapsing("標籤", |ui| {
                            Self::show_tags_editor(ui, skill);
                        });

                        // 範圍編輯
                        ui.horizontal(|ui| {
                            ui.label("範圍:");
                            ui.add(DragValue::new(&mut skill.range.0).prefix("最小: "));
                            ui.add(DragValue::new(&mut skill.range.1).prefix("最大: "));
                        });

                        // 消耗編輯
                        ui.horizontal(|ui| {
                            ui.label("消耗:");
                            ui.add(DragValue::new(&mut skill.cost));
                        });

                        // 命中率編輯
                        ui.horizontal(|ui| {
                            ui.label("命中率:");
                            let mut has_hit_rate = skill.hit_rate.is_some();
                            if ui.checkbox(&mut has_hit_rate, "").changed() {
                                skill.hit_rate = if has_hit_rate { Some(100) } else { None };
                            }

                            if let Some(hit_rate) = &mut skill.hit_rate {
                                ui.add_enabled(
                                    has_hit_rate,
                                    DragValue::new(hit_rate).range(0..=100).suffix("%"),
                                );
                            }
                        });

                        // 爆擊率編輯
                        ui.horizontal(|ui| {
                            ui.label("爆擊率:");
                            let mut has_crit_rate = skill.crit_rate.is_some();
                            if ui.checkbox(&mut has_crit_rate, "").changed() {
                                skill.crit_rate = if has_crit_rate { Some(10) } else { None };
                            }

                            if let Some(crit_rate) = &mut skill.crit_rate {
                                ui.add_enabled(
                                    has_crit_rate,
                                    DragValue::new(crit_rate).range(0..=100).suffix("%"),
                                );
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
                                    }

                                    delete_effect_clicked = ui.button("🗑").clicked();
                                });

                                if delete_effect_clicked {
                                    delete_effect_indices.push(index);
                                }

                                ui.indent(format!("effect_{}", index), |ui| {
                                    Self::show_effect_editor(ui, effect, Self::show_shape_editor);
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

        // 處理按鈕事件（在 ScrollArea 外部）
        // 克隆必要的數據以避免借用衝突
        let action = if save_clicked {
            if let Some((skill_id, skill)) = &self.temp_skill {
                let skill_id_clone = skill_id.clone();
                let skill_clone = skill.clone();

                match self.skills_data.update_skill(&skill_id_clone, skill_clone) {
                    Ok(_) => Some(("成功更新技能".to_string(), false)),
                    Err(err) => Some((err, true)),
                }
            } else {
                None
            }
        } else {
            None
        };

        // 應用 save 操作的結果
        if let Some((message, is_error)) = action {
            self.set_status(message, is_error);
        }

        // 處理刪除技能按鈕
        if delete_clicked && self.temp_skill.is_some() {
            let skill_id = self.temp_skill.as_ref().unwrap().0.clone();
            self.confirmation_action = ConfirmationAction::DeleteSkill(skill_id);
            self.show_confirmation_dialog = true;
        }

        // 處理添加效果按鈕
        if add_effect_clicked {
            self.show_add_effect_popup = true;
        }

        // 處理刪除效果
        if !delete_effect_indices.is_empty() && self.temp_skill.is_some() {
            let skill_id = self.temp_skill.as_ref().unwrap().0.clone();
            let index = *delete_effect_indices.first().unwrap(); // 僅處理第一個
            self.confirmation_action = ConfirmationAction::DeleteEffect(skill_id, index);
            self.show_confirmation_dialog = true;
        }
    }

    fn show_tags_editor(ui: &mut Ui, skill: &mut Skill) {
        for tag in Tag::iter() {
            let tag_str = format!("{:?}", tag).to_lowercase();
            let has_tag = skill.tags.contains(&tag);
            let mut checked = has_tag;

            if ui.checkbox(&mut checked, tag_str).changed() {
                if checked && !has_tag {
                    skill.tags.push(tag.clone());
                } else if !checked && has_tag {
                    skill.tags.retain(|t| t != &tag);
                }
            }
        }
    }

    fn show_effect_editor(
        ui: &mut Ui,
        effect: &mut Effect,
        shape_editor: impl Fn(&mut Ui, &mut Shape),
    ) {
        match effect {
            Effect::Hp {
                target_type,
                shape,
                value,
            } => {
                // 目標類型
                ui.horizontal(|ui| {
                    ui.label("目標類型:");
                    egui::ComboBox::new("target_type", "")
                        .selected_text(format!("{:?}", target_type).to_lowercase())
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
                });

                // 形狀
                ui.horizontal(|ui| {
                    ui.label("形狀:");
                    shape_editor(ui, shape);
                });

                // 數值
                ui.horizontal(|ui| {
                    ui.label("HP 變化值:");
                    ui.add(DragValue::new(value));
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
                    egui::ComboBox::new("target_type", "")
                        .selected_text(format!("{:?}", target_type).to_lowercase())
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
                });

                // 形狀
                ui.horizontal(|ui| {
                    ui.label("形狀:");
                    shape_editor(ui, shape);
                });

                // 持續回合
                ui.horizontal(|ui| {
                    ui.label("持續回合:");
                    ui.add(DragValue::new(duration));
                });
            }
        }
    }

    fn show_shape_editor(ui: &mut Ui, shape: &mut Shape) {
        let shape_type = match shape {
            Shape::Point => "點".to_string(),
            Shape::Circle(_) => "圓形".to_string(),
            Shape::Rectangle(_, _) => "矩形".to_string(),
            Shape::Line(_) => "直線".to_string(),
            Shape::Cone(_, _) => "錐形".to_string(),
        };

        egui::ComboBox::new("shape_type", "")
            .selected_text(shape_type)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(matches!(shape, Shape::Point), "點")
                    .clicked()
                {
                    *shape = Shape::Point;
                }
                if ui
                    .selectable_label(matches!(shape, Shape::Circle(_)), "圓形")
                    .clicked()
                {
                    if !matches!(shape, Shape::Circle(_)) {
                        *shape = Shape::Circle(1);
                    }
                }
                if ui
                    .selectable_label(matches!(shape, Shape::Rectangle(_, _)), "矩形")
                    .clicked()
                {
                    if !matches!(shape, Shape::Rectangle(_, _)) {
                        *shape = Shape::Rectangle(2, 2);
                    }
                }
                if ui
                    .selectable_label(matches!(shape, Shape::Line(_)), "直線")
                    .clicked()
                {
                    if !matches!(shape, Shape::Line(_)) {
                        *shape = Shape::Line(3);
                    }
                }
                if ui
                    .selectable_label(matches!(shape, Shape::Cone(_, _)), "錐形")
                    .clicked()
                {
                    if !matches!(shape, Shape::Cone(_, _)) {
                        *shape = Shape::Cone(3, 45.0);
                    }
                }
            });

        ui.horizontal(|ui| match shape {
            Shape::Point => {}
            Shape::Circle(radius) => {
                ui.add_space(20.0);
                ui.label("半徑:");
                ui.add(DragValue::new(radius).range(1..=10));
            }
            Shape::Rectangle(width, height) => {
                ui.add_space(20.0);
                ui.label("寬度:");
                ui.add(DragValue::new(width).range(1..=10));
                ui.label("高度:");
                ui.add(DragValue::new(height).range(1..=10));
            }
            Shape::Line(length) => {
                ui.add_space(20.0);
                ui.label("長度:");
                ui.add(DragValue::new(length).range(1..=10));
            }
            Shape::Cone(length, angle) => {
                ui.add_space(20.0);
                ui.label("長度:");
                ui.add(DragValue::new(length).range(1..=10));
                ui.label("角度:");
                ui.add(DragValue::new(angle).range(10.0..=120.0).suffix("°"));
            }
        });
    }

    fn show_add_effect_popup(&mut self, ctx: &egui::Context) {
        if !self.show_add_effect_popup {
            return;
        }

        let mut open = self.show_add_effect_popup;
        let mut add_hp_effect = false;
        let mut add_burn_effect = false;

        egui::Window::new("新增效果")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                add_hp_effect = ui.button("新增 HP 效果").clicked();
                add_burn_effect = ui.button("新增燃燒效果").clicked();
            });

        // 在閉包外處理按鈕事件
        if add_hp_effect {
            if let Some((_, skill)) = &mut self.temp_skill {
                skill.effects.push(Effect::Hp {
                    target_type: TargetType::Any,
                    shape: Shape::Point,
                    value: 0,
                });
                open = false; // 將會設置到 self.show_add_effect_popup
            }
        }

        if add_burn_effect {
            if let Some((_, skill)) = &mut self.temp_skill {
                skill.effects.push(Effect::Burn {
                    target_type: TargetType::Any,
                    shape: Shape::Point,
                    duration: 3,
                });
                open = false; // 將會設置到 self.show_add_effect_popup
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
                        self.set_status("成功刪除技能".to_string(), false);
                        self.temp_skill = None;
                    }
                }
                ConfirmationAction::DeleteEffect(skill_id, index) => {
                    if let Some(skill) = self.skills_data.skills.get_mut(&skill_id) {
                        if index < skill.effects.len() {
                            skill.effects.remove(index);
                            // 更新編輯中的技能
                            if let Some((id, _)) = &self.temp_skill {
                                if id == &skill_id {
                                    self.temp_skill = Some((skill_id, skill.clone()));
                                }
                            }
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

    fn show_status_message(&mut self, ctx: &egui::Context) {
        if let Some((message, is_error)) = &self.status_message {
            let color = if *is_error {
                egui::Color32::RED
            } else {
                egui::Color32::GREEN
            };

            egui::TopBottomPanel::bottom("status_panel").show(ctx, |ui| {
                ui.label(RichText::new(message).color(color));
            });
        }
    }
}

impl eframe::App for SkillsEditor {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            self.show_file_menu(ui);
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
}

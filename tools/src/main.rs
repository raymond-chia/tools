mod skill_data;

use eframe::{Frame, egui};
use egui::{
    Button, DragValue, FontData, FontDefinitions, FontFamily, RichText, ScrollArea, Separator, Ui,
};
use rfd::FileDialog;
use skill_data::SkillsData;
use skills_lib::{Effect, Shape, Skill, Tag, TargetType};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

struct SkillsEditor {
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
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // 設定字體以支援繁體中文
        let mut fonts = FontDefinitions::default();

        // 在 Windows 中，使用系統已安裝的中文字體
        // 微軟正黑體是 Windows 中常見的繁體中文字體
        match std::fs::read("C:\\Windows\\Fonts\\msjh.ttc") {
            Ok(font_data) => {
                fonts
                    .font_data
                    .insert("msyh".to_owned(), FontData::from_owned(font_data).into());

                // 將中文字體添加到 Proportional 字體族中的首位
                fonts
                    .families
                    .get_mut(&FontFamily::Proportional)
                    .unwrap()
                    .insert(0, "msyh".to_owned());
            }
            Err(err) => {
                println!("無法載入中文字體: {}", err);
                // 這裡可以加載備用字體或繼續使用預設字體
            }
        }

        // 設置字體
        cc.egui_ctx.set_fonts(fonts);

        // 設定初始字型大小和樣式
        let mut style = (*cc.egui_ctx.style()).clone();
        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::new(24.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Heading,
            egui::FontId::new(32.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::new(24.0, egui::FontFamily::Proportional),
        );
        cc.egui_ctx.set_style(style);

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
            for skill_id in self.skills_data.skills.keys().collect::<Vec<_>>() {
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

            // 添加可捲動區域
            ScrollArea::vertical()
                .auto_shrink([false; 2])
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
        let all_tags = [
            Tag::Active,
            Tag::Passive,
            Tag::Single,
            Tag::Area,
            Tag::Melee,
            Tag::Ranged,
            Tag::Attack,
            Tag::Beneficial,
            Tag::BodyControl,
            Tag::MindControl,
            Tag::Magic,
            Tag::Heal,
            Tag::Fire,
        ];

        for tag in all_tags.iter() {
            let tag_str = format!("{:?}", tag).to_lowercase();
            let has_tag = skill.tags.contains(tag);
            let mut checked = has_tag;

            if ui.checkbox(&mut checked, tag_str).changed() {
                if checked && !has_tag {
                    skill.tags.push(tag.clone());
                } else if !checked && has_tag {
                    skill.tags.retain(|t| t != tag);
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

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1024.0, 768.0]),
        ..Default::default()
    };

    eframe::run_native(
        "技能編輯器",
        options,
        Box::new(|cc| Ok(Box::new(SkillsEditor::new(cc)))),
    )
}

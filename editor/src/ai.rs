use crate::common::*;
use chess_lib::*;
use eframe::{Frame, egui};
use egui::*;
use std::io;

#[derive(Debug)]
pub struct AIEditor {
    ai_config: AIConfig,
    unit_templates: Vec<UnitTemplateType>,
    //
    selected_unit_type: Option<String>,
    new_tendency: Tendency,
    // status
    has_unsaved_changes: bool,
    status_message: Option<(String, bool)>,
}

impl Default for AIEditor {
    fn default() -> Self {
        Self {
            ai_config: AIConfig::default(),
            unit_templates: Vec::new(),
            selected_unit_type: None,
            new_tendency: Tendency::default(),
            has_unsaved_changes: false,
            status_message: None,
        }
    }
}

impl AIEditor {
    pub fn new() -> Self {
        let mut editor = Self::default();
        editor.reload();
        editor
    }

    fn reload(&mut self) {
        // 重新載入 unit_templates
        let unit_templates = match crate::units::load_unit_templates(unit_templates_file()) {
            Ok(unit_templates) => unit_templates,
            Err(err) => {
                self.set_status(format!("載入單位類型失敗: {}", err), true);
                return;
            }
        };
        self.unit_templates = unit_templates.into_iter().map(|u| u.name).collect();
        let is_selected_exist = self
            .selected_unit_type
            .as_ref()
            .map_or(false, |selected| self.unit_templates.contains(selected));
        if !is_selected_exist {
            // 如果選中的單位不存在，則清除選中狀態
            self.selected_unit_type = None;
        }
        // 重新載入 AI
        let ai_config = match from_file::<_, AIConfig>(ai_file()) {
            Ok(ai_config) => ai_config,
            Err(err) => {
                self.set_status(format!("載入 AI 設定失敗: {}", err), true);
                return;
            }
        };
        self.ai_config = ai_config;
        self.set_status("已重新載入 AI 設定".to_string(), false);
    }

    pub fn update(&mut self, ctx: &Context, _: &mut Frame) {
        SidePanel::left("ai_list_panel")
            .default_width(200.0)
            .show(ctx, |ui| {
                if ui.button("重新載入").clicked() {
                    self.reload();
                }
                if ui.button("儲存").clicked() {
                    if let Err(e) = save_ai(&self.ai_config) {
                        self.set_status(format!("儲存失敗: {e}"), true);
                    } else {
                        self.set_status("儲存成功".to_string(), false);
                        self.has_unsaved_changes = false;
                    }
                }
                self.show_unit_type_list(ui);
            });

        CentralPanel::default().show(ctx, |ui| {
            self.show_tendency_editor(ui);
        });

        self.show_status_message(ctx);
    }

    fn show_unit_type_list(&mut self, ui: &mut Ui) {
        ui.heading("單位類型列表");

        ScrollArea::vertical().show(ui, |ui| {
            for unit_type in &self.unit_templates {
                let selected = self.selected_unit_type.as_ref() == Some(unit_type);
                let button = Button::new(unit_type)
                    .fill(if selected {
                        ui.style().visuals.selection.bg_fill
                    } else {
                        ui.style().visuals.widgets.noninteractive.bg_fill
                    })
                    .min_size(vec2(ui.available_width(), 0.0));
                if ui.add(button).clicked() {
                    self.selected_unit_type = Some(unit_type.clone());
                    // 確保切換後是乾淨的
                    self.new_tendency = Tendency::default();
                }
            }
        });
    }

    fn show_tendency_editor(&mut self, ui: &mut Ui) {
        let unit_type = match &self.selected_unit_type {
            None => {
                ui.heading("AI 編輯器");
                ui.label("選擇一個單位開始編輯");
                return;
            }
            Some(unit_type) => unit_type,
        };

        ui.heading(format!("編輯單位類型: {}", unit_type));

        ui.add_space(8.0);
        ui.add(Separator::default());

        let available_height = ui.available_height();
        let scroll_height = available_height.max(100.0) - 40.0;

        let mut is_inserting_new_tendency = false;
        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .max_height(scroll_height)
            .show(ui, |ui| {
                if let Some(tendency) = self.ai_config.tendencies.get_mut(unit_type) {
                    ui.heading("權重設定");
                    Self::show_weights_editor(
                        ui,
                        &mut tendency.weights,
                        &mut self.has_unsaved_changes,
                    );

                    ui.add_space(8.0);
                    ui.heading("位置偏好");
                    Self::show_position_preference_editor(
                        ui,
                        &mut tendency.positioning_preference,
                        &mut self.has_unsaved_changes,
                    );
                } else {
                    let mut changed = false;
                    ui.heading("權重設定");
                    Self::show_weights_editor(ui, &mut self.new_tendency.weights, &mut changed);

                    ui.add_space(8.0);
                    ui.heading("位置偏好");
                    Self::show_position_preference_editor(
                        ui,
                        &mut self.new_tendency.positioning_preference,
                        &mut changed,
                    );

                    if changed {
                        self.has_unsaved_changes = true;
                    }

                    ui.add_space(8.0);
                    if ui.button("儲存新 tendency").clicked() && self.has_unsaved_changes {
                        is_inserting_new_tendency = true;
                    }
                }
            });
        if is_inserting_new_tendency {
            let new_tendency = std::mem::take(&mut self.new_tendency);
            self.ai_config
                .tendencies
                .insert(unit_type.clone(), new_tendency);
            self.set_status("已新增 tendency, 請記得儲存 AI 設定檔".to_string(), false);
        }
    }

    fn show_weights_editor(ui: &mut Ui, weights: &mut Weights, changed: &mut bool) {
        ui.horizontal(|ui| {
            ui.label("攻擊:");
            *changed |= ui.add(DragValue::new(&mut weights.attack)).changed();
        });
        ui.horizontal(|ui| {
            ui.label("支援:");
            *changed |= ui.add(DragValue::new(&mut weights.support)).changed();
        });
        ui.horizontal(|ui| {
            ui.label("逼近敵人:");
            *changed |= ui.add(DragValue::new(&mut weights.nearest_enemy)).changed();
        });
        ui.horizontal(|ui| {
            ui.label("距離基準分:");
            *changed |= ui.add(DragValue::new(&mut weights.distance_base)).changed();
        });
    }

    fn show_position_preference_editor(
        ui: &mut Ui,
        pos: &mut PositionPreference,
        changed: &mut bool,
    ) {
        ui.horizontal(|ui| {
            ui.label("位置偏好:");
            let mut selected = match pos {
                PositionPreference::Frontline => 0,
                PositionPreference::Flexible => 1,
                PositionPreference::Backline => 2,
            };
            let items = ["前線", "彈性", "後排"];
            for (i, label) in items.iter().enumerate() {
                if ui.radio_value(&mut selected, i, *label).clicked() {
                    *pos = match i {
                        0 => PositionPreference::Frontline,
                        1 => PositionPreference::Flexible,
                        2 => PositionPreference::Backline,
                        _ => PositionPreference::Flexible,
                    };
                    *changed = true;
                }
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

fn save_ai(ai_config: &AIConfig) -> io::Result<()> {
    to_file(ai_file(), ai_config)
}

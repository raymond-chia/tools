use crate::constants::{
    DATA_DIRECTORY_PATH, FILE_EXTENSION_TOML, LIST_PANEL_WIDTH, SPACING_MEDIUM, SPACING_SMALL,
};
use crate::editor_item::EditorItem;
use crate::generic_editor::{EditMode, GenericEditorState};
use crate::generic_io::{load_file, save_file};
use crate::state::{EditorApp, EditorTab};
use crate::tabs;
use board::alias::{SkillName, TypeName};
use std::path::PathBuf;
use strum::IntoEnumIterator;

/// 渲染項目列表（左側）
fn render_item_list<T: EditorItem>(
    ui: &mut egui::Ui,
    state: &mut GenericEditorState<T>,
    width: f32,
    height: f32,
) {
    ui.vertical(|ui| {
        ui.set_width(width);
        ui.set_height(height);

        ui.heading(format!("{}列表", T::type_name()));
        ui.add_space(SPACING_SMALL);

        // 操作按鈕
        let is_editing = state.edit_mode != EditMode::None;
        let has_selection = state.selected_index.is_some();

        ui.horizontal(|ui| {
            if ui.button("新增").clicked() {
                state.start_creating();
            }
            ui.add_enabled_ui(!is_editing && has_selection, |ui| {
                if ui.button("編輯").clicked() {
                    if let Some(index) = state.selected_index {
                        state.start_editing(index);
                    }
                }
                if ui.button("複製").clicked() {
                    if let Some(index) = state.selected_index {
                        state.start_copying(index);
                    }
                }
                if ui.button("刪除").clicked() {
                    if let Some(index) = state.selected_index {
                        state.delete_item(index);
                    }
                }
            });
        });

        ui.add_space(SPACING_SMALL);

        // 項目列表
        egui::ScrollArea::vertical()
            .id_salt("item_list_scroll")
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for (index, item) in state.items.iter().enumerate() {
                    let is_selected = Some(index) == state.selected_index;
                    if ui.selectable_label(is_selected, item.name()).clicked() && !is_editing {
                        state.selected_index = Some(index);
                    }
                }
            });
    });
}

/// 渲染編輯區域（右側）
fn render_edit_area<T: EditorItem, F>(
    ui: &mut egui::Ui,
    state: &mut GenericEditorState<T>,
    render_form: F,
) where
    F: Fn(&mut egui::Ui, &mut T),
{
    ui.vertical(|ui| {
        ui.heading("編輯區域");
        ui.add_space(SPACING_SMALL);

        let is_editable = state.edit_mode != EditMode::None;

        if is_editable && state.editing_item.is_some() {
            // 編輯模式：直接修改 editing_item
            // 先放置確認/取消按鈕，保持可見
            ui.horizontal(|ui| {
                if ui.button("確認").clicked() {
                    state.confirm_edit();
                }
                if ui.button("取消").clicked() {
                    state.cancel_edit();
                }
            });

            ui.add_space(SPACING_MEDIUM);

            // 表單放在下面，使用可用高度
            egui::ScrollArea::vertical()
                .id_salt("edit_area_scroll")
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    if let Some(item) = &mut state.editing_item {
                        ui.add_enabled_ui(true, |ui| {
                            render_form(ui, item);
                        });
                    }
                });
        } else {
            // 預覽模式：clone 一份來顯示
            if let Some(index) = state.selected_index {
                if let Some(item) = state.items.get(index) {
                    let mut item_copy = item.clone();
                    egui::ScrollArea::vertical()
                        .id_salt("edit_area_scroll")
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            ui.add_enabled_ui(false, |ui| {
                                render_form(ui, &mut item_copy);
                            });
                        });
                }
            } else {
                ui.label(format!(
                    "請選擇{}進行編輯，或點擊「新增」創建新{}",
                    T::type_name(),
                    T::type_name()
                ));
            }
        }
    });
}

/// 渲染泛型編輯器 UI
fn render_editor_ui<T: EditorItem, F>(
    ui: &mut egui::Ui,
    state: &mut GenericEditorState<T>,
    data_key: &str,
    render_form: F,
) where
    F: Fn(&mut egui::Ui, &mut T),
{
    ui.heading(format!("{}編輯器", T::type_name()));
    ui.add_space(SPACING_MEDIUM);

    let file_path =
        PathBuf::from(DATA_DIRECTORY_PATH).join(format!("{}{}", data_key, FILE_EXTENSION_TOML));

    // 頂部按鈕列
    ui.horizontal(|ui| {
        if ui.button("載入").clicked() {
            load_file(state, &file_path, data_key);
        }
        if ui.button("儲存").clicked() {
            save_file(state, &file_path, data_key);
        }

        ui.add_space(SPACING_MEDIUM);

        // 訊息區域
        if state.message_visible {
            if ui.button("隱藏").clicked() {
                state.message_visible = false;
            }
            let color = if state.is_error {
                egui::Color32::RED
            } else {
                egui::Color32::GREEN
            };
            ui.colored_label(color, &state.message);
        } else {
            if ui.button("顯示訊息").clicked() {
                state.message_visible = true;
            }
        }
    });

    ui.add_space(SPACING_MEDIUM);

    let left_width = LIST_PANEL_WIDTH;
    let height = ui.available_height();

    // 主內容區域
    ui.horizontal(|ui| {
        // 左側：項目列表
        render_item_list(ui, state, left_width, height);
        ui.separator();
        // 右側：編輯區域
        render_edit_area(ui, state, render_form);
    });
}

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                EditorTab::iter().for_each(|tab| {
                    ui.selectable_value(&mut self.current_tab, tab, tab.to_string());
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.current_tab {
            EditorTab::Object => render_editor_ui(
                ui,
                &mut self.object_editor,
                tabs::object_tab::file_name(),
                tabs::object_tab::render_form,
            ),
            EditorTab::Skill => render_editor_ui(
                ui,
                &mut self.skill_editor,
                tabs::skill_tab::file_name(),
                tabs::skill_tab::render_form,
            ),
            EditorTab::Unit => {
                let available_skills: Vec<SkillName> = self
                    .skill_editor
                    .items
                    .iter()
                    .map(|skill| skill.name.clone())
                    .collect();

                render_editor_ui(
                    ui,
                    &mut self.unit_editor,
                    tabs::unit_tab::file_name(),
                    |ui, unit| tabs::unit_tab::render_form(ui, unit, &available_skills),
                )
            }
            EditorTab::Level => {
                let available_units: Vec<TypeName> = self
                    .unit_editor
                    .items
                    .iter()
                    .map(|unit| unit.name.clone())
                    .collect();

                let available_objects: Vec<TypeName> = self
                    .object_editor
                    .items
                    .iter()
                    .map(|obj| obj.name.clone())
                    .collect();

                render_editor_ui(
                    ui,
                    &mut self.level_editor,
                    tabs::level_tab::file_name(),
                    |ui, level| {
                        tabs::level_tab::render_form(
                            ui,
                            level,
                            &available_units,
                            &available_objects,
                        )
                    },
                )
            }
        });
    }
}

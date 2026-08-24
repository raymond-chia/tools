use crate::constants::{
    DATA_DIRECTORY_PATH, FILE_EXTENSION_TOML, LIST_PANEL_WIDTH, SPACING_MEDIUM, SPACING_SMALL,
};
use crate::define_editors;
use crate::editor_item::EditorItem;
use crate::generic_editor::{EditMode, GenericEditorState, MessageState};
use crate::tabs;
use crate::utils::dnd::render_dnd_handle;
use crate::utils::search::{match_search_query, render_search_input};
use board::domain::core_types::{EquipmentType as EquipmentKind, SkillType};
use board::loader_schema::{EquipmentType, LevelType, ObjectType, UnitType};
use std::path::{Path, PathBuf};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};

define_editors! {
    default: Object,

    Object => {
        display: "物件",
        field: object_editor,
        type: ObjectType,
        file_fn: tabs::object_tab::file_name,
    },
    Skill => {
        display: "技能",
        field: skill_editor,
        type: SkillType,
        file_fn: tabs::skill_tab::file_name,
    },
    Equipment => {
        display: "裝備",
        field: equipment_editor,
        type: EquipmentType,
        file_fn: tabs::equipment_tab::file_name,
    },
    Unit => {
        display: "單位",
        field: unit_editor,
        type: UnitType,
        file_fn: tabs::unit_tab::file_name,
    },
    Level => {
        display: "關卡",
        field: level_editor,
        type: LevelType,
        file_fn: tabs::level_tab::file_name,
    },
}

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                EditorTab::iter().for_each(|tab: EditorTab| {
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
                None,
            ),
            EditorTab::Skill => {
                self.skill_editor.ui_state.available_objects = self
                    .object_editor
                    .items
                    .iter()
                    .map(|obj| obj.name().to_string())
                    .collect();

                render_editor_ui(
                    ui,
                    &mut self.skill_editor,
                    tabs::skill_tab::file_name(),
                    tabs::skill_tab::render_form,
                    None,
                )
            }
            EditorTab::Equipment => {
                self.equipment_editor.ui_state.available_skills = self
                    .skill_editor
                    .items
                    .iter()
                    .map(|skill| skill.name().clone())
                    .collect();

                render_editor_ui(
                    ui,
                    &mut self.equipment_editor,
                    tabs::equipment_tab::file_name(),
                    tabs::equipment_tab::render_form,
                    Some(ReferenceCleanup {
                        has_invalid: tabs::equipment_tab::has_invalid_references,
                        has_invalid_item: tabs::equipment_tab::has_invalid_reference,
                        clear_invalid: tabs::equipment_tab::clear_invalid_references,
                    }),
                )
            }
            EditorTab::Unit => {
                self.unit_editor.ui_state.available_skills = self
                    .skill_editor
                    .items
                    .iter()
                    .map(|skill| skill.name().clone())
                    .collect();
                self.unit_editor.ui_state.available_weapons =
                    equipment_names_by_kind(&self.equipment_editor.items, EquipmentKind::Weapon);
                self.unit_editor.ui_state.available_armors =
                    equipment_names_by_kind(&self.equipment_editor.items, EquipmentKind::Armor);
                self.unit_editor.ui_state.available_accessories =
                    equipment_names_by_kind(&self.equipment_editor.items, EquipmentKind::Accessory);

                render_editor_ui(
                    ui,
                    &mut self.unit_editor,
                    tabs::unit_tab::file_name(),
                    tabs::unit_tab::render_form,
                    Some(ReferenceCleanup {
                        has_invalid: tabs::unit_tab::has_invalid_references,
                        has_invalid_item: tabs::unit_tab::has_invalid_reference,
                        clear_invalid: tabs::unit_tab::clear_invalid_references,
                    }),
                )
            }
            EditorTab::Level => {
                self.level_editor.ui_state.available_objects = self.object_editor.items.clone();
                self.level_editor.ui_state.available_units = self.unit_editor.items.clone();
                self.level_editor.ui_state.available_skills = self.skill_editor.items.clone();
                self.level_editor.ui_state.available_equipments =
                    self.equipment_editor.items.clone();

                render_editor_ui(
                    ui,
                    &mut self.level_editor,
                    tabs::level_tab::file_name(),
                    tabs::level_tab::render_form,
                    Some(ReferenceCleanup {
                        has_invalid: tabs::level_tab::has_invalid_references,
                        has_invalid_item: tabs::level_tab::has_invalid_reference,
                        clear_invalid: tabs::level_tab::clear_invalid_references,
                    }),
                )
            }
        });
    }
}

/// 協調編輯器各區域的渲染
fn render_editor_ui<T: EditorItem>(
    ui: &mut egui::Ui,
    state: &mut GenericEditorState<T>,
    data_key: &str,
    render_form: fn(&mut egui::Ui, &mut T, &mut T::UIState, &mut MessageState),
    reference_cleanup: Option<ReferenceCleanup<T>>,
) {
    ui.heading(format!("{}編輯器", T::type_name()));
    ui.add_space(SPACING_MEDIUM);

    let file_path =
        PathBuf::from(DATA_DIRECTORY_PATH).join(format!("{}{}", data_key, FILE_EXTENSION_TOML));

    render_file_operations_bar(ui, state, &file_path, data_key, reference_cleanup.as_ref());
    ui.add_space(SPACING_MEDIUM);

    // 主內容區域
    let height = ui.available_height();
    ui.horizontal(|ui| {
        // 左側：項目列表
        render_item_list(ui, state, LIST_PANEL_WIDTH, height);
        ui.separator();
        // 右側：編輯區域
        render_edit_area(ui, state, render_form, reference_cleanup.as_ref());
    });
}

/// 渲染檔案操作列（載入、儲存、訊息）
fn render_file_operations_bar<T: EditorItem>(
    ui: &mut egui::Ui,
    state: &mut GenericEditorState<T>,
    file_path: &Path,
    data_key: &str,
    reference_cleanup: Option<&ReferenceCleanup<T>>,
) {
    ui.horizontal(|ui| {
        if ui.button("載入").clicked() {
            T::load(state, file_path, data_key);
        }
        if ui.button("儲存").clicked() {
            if reference_cleanup.is_some_and(|cleanup| (cleanup.has_invalid)(state)) {
                state
                    .message_state
                    .set_error("存在失效引用，請先清除後再儲存");
            } else {
                T::save(state, file_path, data_key);
            }
        }

        if let Some(cleanup) = reference_cleanup {
            render_clear_invalid_references_button(ui, state, cleanup);
        }

        ui.add_space(SPACING_MEDIUM);

        // 訊息區域
        if state.message_state.message_visible {
            if ui.button("隱藏").clicked() {
                state.message_state.message_visible = false;
            }
            let color = if state.message_state.is_error {
                egui::Color32::RED
            } else {
                egui::Color32::GREEN
            };
            ui.colored_label(color, &state.message_state.message);
        } else {
            if ui.button("顯示訊息").clicked() {
                state.message_state.message_visible = true;
            }
        }
    });
}

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

        render_action_buttons(ui, state);
        ui.add_space(SPACING_SMALL);

        render_search_input(ui, &mut state.search_query);
        ui.add_space(SPACING_SMALL);

        render_items_scroll_area(ui, state);
    });
}

/// 渲染操作按鈕（新增、編輯、複製、刪除）
fn render_action_buttons<T: EditorItem>(ui: &mut egui::Ui, state: &mut GenericEditorState<T>) {
    let is_editing = state.is_editing();
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
}

/// 渲染搜尋框

/// 渲染可捲動的項目列表
fn render_items_scroll_area<T: EditorItem>(ui: &mut egui::Ui, state: &mut GenericEditorState<T>) {
    egui::ScrollArea::vertical()
        .id_salt("item_list_scroll")
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            let is_editing = state.is_editing();
            let can_drag = state.search_query.is_empty() && !is_editing;
            let query_lower = state.search_query.to_lowercase();

            // 提前收集符合搜尋條件的項目（索引和名稱），避免借用衝突
            let visible_items: Vec<(usize, String)> = state
                .items
                .iter()
                .enumerate()
                .filter(|(_, item)| match_search_query(item.name(), &query_lower))
                .map(|(idx, item)| (idx, item.name().to_string()))
                .collect();

            for (original_index, item_name) in visible_items {
                let is_selected = Some(original_index) == state.selected_index;

                if let Some((from, to)) =
                    render_list_item(ui, state, original_index, &item_name, is_selected, can_drag)
                {
                    state.move_item(from, to);
                }
            }
        });
}

/// 渲染單個列表項目
fn render_list_item<T: EditorItem>(
    ui: &mut egui::Ui,
    state: &mut GenericEditorState<T>,
    original_index: usize,
    item_name: &str,
    is_selected: bool,
    can_drag: bool,
) -> Option<(usize, usize)> {
    let mut dnd_result = None;

    ui.horizontal(|ui| {
        if can_drag {
            let item_id = egui::Id::new("item_drag").with(original_index);
            dnd_result = render_dnd_handle(ui, item_id, original_index, "☰");
        } else {
            // 不可拖曳：只顯示 handle，不啟用拖曳功能
            ui.label("☰");
        }

        // 項目標籤：點擊選取
        if ui.selectable_label(is_selected, item_name).clicked() {
            state.selected_index = Some(original_index);
        }
    });

    dnd_result
}

/// 渲染編輯區域（右側）
fn render_edit_area<T: EditorItem>(
    ui: &mut egui::Ui,
    state: &mut GenericEditorState<T>,
    render_form: fn(&mut egui::Ui, &mut T, &mut T::UIState, &mut MessageState),
    reference_cleanup: Option<&ReferenceCleanup<T>>,
) {
    ui.vertical(|ui| {
        let is_editable = state.is_editing();

        if is_editable {
            // 編輯模式：直接修改 edit_mode 中的項目
            // 先放置確認/取消按鈕，保持可見
            ui.horizontal(|ui| {
                if ui.button("確認").clicked() {
                    let has_invalid_reference =
                        reference_cleanup.is_some_and(|cleanup| match &state.edit_mode {
                            EditMode::Creating(item) | EditMode::Editing(_, item) => {
                                (cleanup.has_invalid_item)(item, &state.ui_state)
                            }
                            EditMode::None => false,
                        });

                    if has_invalid_reference {
                        state
                            .message_state
                            .set_error("存在失效引用，請先清除後再確認");
                    } else {
                        state.confirm_edit();
                    }
                }
                if ui.button("取消").clicked() {
                    state.cancel_edit();
                }
            });

            ui.add_space(SPACING_MEDIUM);

            // 表單放在下面，使用可用高度
            match &mut state.edit_mode {
                EditMode::Creating(item) | EditMode::Editing(_, item) => {
                    egui::ScrollArea::vertical()
                        .id_salt("edit_area_scroll")
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            ui.add_enabled_ui(true, |ui| {
                                render_form(
                                    ui,
                                    item,
                                    &mut state.ui_state,
                                    &mut state.message_state,
                                );
                            });
                        });
                }
                EditMode::None => {
                    ui.label(format!("不應該發生，但為了編譯器完整性處理"));
                }
            }
        } else {
            // 預覽模式：clone 一份來顯示
            if let Some(item) = state
                .selected_index
                .and_then(|index| state.items.get_mut(index))
            {
                egui::ScrollArea::vertical()
                    .id_salt("edit_area_scroll")
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.add_enabled_ui(false, |ui| {
                            render_form(ui, item, &mut state.ui_state, &mut state.message_state);
                        });
                    });
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

// ==================== 本地輔助函數 ====================

fn equipment_names_by_kind(
    equipments: &[EquipmentType],
    kind: EquipmentKind,
) -> Vec<board::domain::alias::TypeName> {
    equipments
        .iter()
        .filter(|equipment| equipment.typ == kind)
        .map(|equipment| equipment.name.clone())
        .collect()
}

struct ReferenceCleanup<T: EditorItem> {
    has_invalid: fn(&GenericEditorState<T>) -> bool,
    has_invalid_item: fn(&T, &T::UIState) -> bool,
    clear_invalid: fn(&mut GenericEditorState<T>),
}

fn render_clear_invalid_references_button<T: EditorItem>(
    ui: &mut egui::Ui,
    state: &mut GenericEditorState<T>,
    cleanup: &ReferenceCleanup<T>,
) {
    let has_invalid_references = !state.is_editing() && (cleanup.has_invalid)(state);

    ui.add_enabled_ui(has_invalid_references, |ui| {
        if ui.button("清除失效引用").clicked() {
            (cleanup.clear_invalid)(state);
        }
    });
}

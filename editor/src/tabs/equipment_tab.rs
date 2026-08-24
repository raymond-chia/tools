//! 裝備編輯器 tab

use crate::constants::{SPACING_MEDIUM, SPACING_SMALL};
use crate::editor_item::EditorItem;
use crate::generic_editor::{GenericEditorState, MessageState};
use crate::tabs::reference;
use crate::utils::search::{filter_by_search, render_search_input};
use board::domain::alias::SkillName;
use board::domain::core_types::EquipmentType as EquipmentKind;
use board::loader_schema::EquipmentType;
use std::collections::HashSet;
use strum::IntoEnumIterator;

/// 裝備編輯器的 UI 狀態
#[derive(Debug, Default)]
pub struct EquipmentTabUIState {
    pub available_skills: Vec<SkillName>,
    pub skill_search_query: SkillName,
}

// ==================== EditorItem 實作 ====================

impl EditorItem for EquipmentType {
    type UIState = EquipmentTabUIState;

    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn type_name() -> &'static str {
        "裝備"
    }

    fn after_confirm(&mut self, ui_state: &Self::UIState) {
        reorder_skills(&mut self.granted_skills, &ui_state.available_skills);
    }
}

/// 取得裝備的檔案名稱
pub fn file_name() -> &'static str {
    "equipments"
}

// ==================== 表單渲染 ====================

/// 渲染裝備編輯表單
pub fn render_form(
    ui: &mut egui::Ui,
    equipment: &mut EquipmentType,
    ui_state: &mut EquipmentTabUIState,
    _message_state: &mut MessageState,
) {
    ui.horizontal(|ui| {
        ui.label("名稱：");
        ui.text_edit_singleline(&mut equipment.name);
    });

    ui.horizontal(|ui| {
        ui.label("類型：");
        for kind in EquipmentKind::iter() {
            ui.selectable_value(&mut equipment.typ, kind, kind.to_string());
        }
    });

    ui.add_space(SPACING_SMALL);
    ui.separator();
    ui.heading("授予技能");

    if ui_state.available_skills.is_empty() {
        ui.label("（尚未定義任何技能，請先到「技能」tab 創建技能）");
    } else {
        render_search_input(ui, &mut ui_state.skill_search_query);
        ui.add_space(SPACING_SMALL);

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let visible_skills =
                    filter_by_search(&ui_state.available_skills, &ui_state.skill_search_query);

                if visible_skills.is_empty() && !ui_state.skill_search_query.is_empty() {
                    ui.label("找不到符合的技能");
                } else {
                    for skill_name in visible_skills {
                        let mut selected = equipment.granted_skills.contains(skill_name);
                        if ui.checkbox(&mut selected, skill_name).changed() {
                            if selected {
                                equipment.granted_skills.push(skill_name.clone());
                            } else {
                                equipment.granted_skills.retain(|name| name != skill_name);
                            }
                        }
                    }
                }
            });
    }

    ui.separator();
    ui.label(format!("已選擇：{} 個技能", equipment.granted_skills.len()));
    ui.horizontal_wrapped(|ui| {
        for skill_name in &equipment.granted_skills {
            ui.label(skill_name);
            ui.add_space(SPACING_MEDIUM);
        }
    });
}

// ==================== 本地輔助函數 ====================

struct ValidEquipmentReferences {
    skills: HashSet<SkillName>,
}

impl ValidEquipmentReferences {
    fn from_ui_state(ui_state: &EquipmentTabUIState) -> Self {
        Self {
            skills: ui_state.available_skills.iter().cloned().collect(),
        }
    }
}

fn reorder_skills(skills: &mut Vec<SkillName>, available_skills: &[SkillName]) {
    let mut ordered_skills: Vec<SkillName> = available_skills
        .iter()
        .filter(|name| skills.contains(name))
        .cloned()
        .collect();
    ordered_skills.extend(
        skills
            .iter()
            .filter(|name| !available_skills.contains(name))
            .cloned(),
    );
    *skills = ordered_skills;
}

/// 是否存在已被刪除的技能引用。
pub fn has_invalid_references(state: &GenericEditorState<EquipmentType>) -> bool {
    state
        .items
        .iter()
        .any(|equipment| has_invalid_reference(equipment, &state.ui_state))
}

/// 是否存在已被刪除的技能引用。
pub fn has_invalid_reference(equipment: &EquipmentType, ui_state: &EquipmentTabUIState) -> bool {
    let valid_references = ValidEquipmentReferences::from_ui_state(ui_state);

    reference::has_invalid(equipment.granted_skills.iter(), &valid_references.skills)
}

/// 清除所有裝備中已失效的技能引用。
pub fn clear_invalid_references(state: &mut GenericEditorState<EquipmentType>) {
    let valid_references = ValidEquipmentReferences::from_ui_state(&state.ui_state);

    for equipment in &mut state.items {
        reference::retain_valid(&mut equipment.granted_skills, &valid_references.skills);
    }
}

//! 單位編輯器 tab

use crate::constants::{CLEAR_LABEL, SPACING_SMALL};
use crate::editor_item::EditorItem;
use crate::generic_editor::{GenericEditorState, MessageState};
use crate::tabs::reference;
use crate::tabs::skill_selection::{render_selected_skills_summary, render_skill_selector};
use crate::utils::search::{combobox_with_dynamic_height, filter_by_search, render_search_input};
use board::domain::alias::{SkillName, TypeName};
use board::ecs_types::components::EquippedItems;
use board::loader_schema::UnitType;
use std::collections::HashSet;

/// 單位表單目前顯示的子分頁。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum UnitFormSubtab {
    #[default]
    Skills,
    Equipments,
}

/// 單位編輯器的 UI 狀態
#[derive(Debug, Default)]
pub struct UnitTabUIState {
    pub available_skills: Vec<SkillName>,
    pub available_weapons: Vec<TypeName>,
    pub available_armors: Vec<TypeName>,
    pub available_accessories: Vec<TypeName>,

    pub skill_search_query: SkillName,
    pub weapon_search_query: TypeName,
    pub armor_search_query: TypeName,
    pub accessory_search_query: TypeName,

    active_subtab: UnitFormSubtab,
}

// ==================== EditorItem 實作 ====================

impl EditorItem for UnitType {
    type UIState = UnitTabUIState;

    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn type_name() -> &'static str {
        "單位"
    }

    fn after_confirm(&mut self, ui_state: &Self::UIState) {
        // 依技能列表的順序重建已選技能，使儲存順序與列表一致
        self.skills = ui_state
            .available_skills
            .iter()
            .filter(|name| self.skills.contains(name))
            .cloned()
            .collect();
    }
}

/// 取得單位的檔案名稱
pub fn file_name() -> &'static str {
    "units"
}

// ==================== 表單渲染 ====================

/// 渲染單位編輯表單
pub fn render_form(
    ui: &mut egui::Ui,
    unit: &mut UnitType,
    ui_state: &mut UnitTabUIState,
    _message_state: &mut MessageState,
) {
    ui.horizontal(|ui| {
        ui.label("名稱：");
        ui.text_edit_singleline(&mut unit.name);

        if ui_state.active_subtab == UnitFormSubtab::Skills {
            ui.separator();
            // 與裝備頁相同，讓已選技能摘要位於名稱欄右側。
            ui.vertical(|ui| render_selected_skills_summary(ui, &unit.skills));
        }
    });

    ui.add_space(SPACING_SMALL);
    ui.separator();

    ui.horizontal(|ui| {
        ui.selectable_value(&mut ui_state.active_subtab, UnitFormSubtab::Skills, "技能");
        ui.selectable_value(
            &mut ui_state.active_subtab,
            UnitFormSubtab::Equipments,
            "裝備",
        );
    });

    ui.add_space(SPACING_SMALL);
    ui.separator();

    match ui_state.active_subtab {
        UnitFormSubtab::Skills => render_skill_subtab(ui, unit, ui_state),
        UnitFormSubtab::Equipments => render_equipment_subtab(ui, unit, ui_state),
    }
}

/// 渲染技能子分頁。
fn render_skill_subtab(ui: &mut egui::Ui, unit: &mut UnitType, ui_state: &mut UnitTabUIState) {
    ui.heading("技能選擇");

    ui.add_space(SPACING_SMALL);
    ui.separator();

    render_skill_selector(
        ui,
        &ui_state.available_skills,
        &mut ui_state.skill_search_query,
        &mut unit.skills,
    );
}

/// 渲染裝備子分頁。
fn render_equipment_subtab(ui: &mut egui::Ui, unit: &mut UnitType, ui_state: &mut UnitTabUIState) {
    ui.heading("預設裝備");

    render_equipment_selector(
        ui,
        "武器：",
        "unit_weapon",
        &mut unit.equipment.weapon,
        &ui_state.available_weapons,
        &mut ui_state.weapon_search_query,
    );
    render_equipment_selector(
        ui,
        "防具：",
        "unit_armor",
        &mut unit.equipment.armor,
        &ui_state.available_armors,
        &mut ui_state.armor_search_query,
    );
    render_equipment_selector(
        ui,
        "第一飾品：",
        "unit_first_accessory",
        &mut unit.equipment.first_accessory,
        &ui_state.available_accessories,
        &mut ui_state.accessory_search_query,
    );
    render_equipment_selector(
        ui,
        "第二飾品：",
        "unit_second_accessory",
        &mut unit.equipment.second_accessory,
        &ui_state.available_accessories,
        &mut ui_state.accessory_search_query,
    );
}

fn render_equipment_selector(
    ui: &mut egui::Ui,
    label: &str,
    id: &str,
    selected_equipment: &mut Option<TypeName>,
    available_equipments: &[TypeName],
    search_query: &mut TypeName,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        let selected_text = selected_equipment.as_deref().unwrap_or("（未裝備）");
        combobox_with_dynamic_height(id, selected_text, available_equipments.len() + 1).show_ui(
            ui,
            |ui| {
                ui.selectable_value(selected_equipment, None, CLEAR_LABEL);
                let response = render_search_input(ui, search_query);
                ui.memory_mut(|memory| memory.request_focus(response.id));
                ui.separator();

                let visible_equipments = filter_by_search(available_equipments, search_query);
                if visible_equipments.is_empty() && !search_query.is_empty() {
                    ui.label("找不到符合的裝備");
                } else {
                    for equipment_name in visible_equipments {
                        ui.selectable_value(
                            selected_equipment,
                            Some(equipment_name.clone()),
                            equipment_name,
                        );
                    }
                }
            },
        );
    });
}

// ==================== 本地輔助函數 ====================

struct ValidUnitReferences {
    skills: HashSet<SkillName>,
    weapons: HashSet<TypeName>,
    armors: HashSet<TypeName>,
    accessories: HashSet<TypeName>,
}

impl ValidUnitReferences {
    fn from_ui_state(ui_state: &UnitTabUIState) -> Self {
        Self {
            skills: ui_state.available_skills.iter().cloned().collect(),
            weapons: ui_state.available_weapons.iter().cloned().collect(),
            armors: ui_state.available_armors.iter().cloned().collect(),
            accessories: ui_state.available_accessories.iter().cloned().collect(),
        }
    }

    fn has_invalid_equipment(&self, equipment: &EquippedItems) -> bool {
        reference::has_invalid(equipment.weapon.iter(), &self.weapons)
            || reference::has_invalid(equipment.armor.iter(), &self.armors)
            || reference::has_invalid(equipment.first_accessory.iter(), &self.accessories)
            || reference::has_invalid(equipment.second_accessory.iter(), &self.accessories)
    }

    fn clear_invalid_equipment(&self, equipment: &mut EquippedItems) {
        reference::clear_invalid_option(&mut equipment.weapon, &self.weapons);
        reference::clear_invalid_option(&mut equipment.armor, &self.armors);
        reference::clear_invalid_option(&mut equipment.first_accessory, &self.accessories);
        reference::clear_invalid_option(&mut equipment.second_accessory, &self.accessories);
    }
}

/// 是否存在已被刪除的技能或裝備引用。
pub fn has_invalid_references(state: &GenericEditorState<UnitType>) -> bool {
    state
        .items
        .iter()
        .any(|unit| has_invalid_reference(unit, &state.ui_state))
}

/// 是否存在已被刪除的技能或裝備引用。
pub fn has_invalid_reference(unit: &UnitType, ui_state: &UnitTabUIState) -> bool {
    let valid_references = ValidUnitReferences::from_ui_state(ui_state);

    reference::has_invalid(unit.skills.iter(), &valid_references.skills)
        || valid_references.has_invalid_equipment(&unit.equipment)
}

/// 清除所有單位中已失效的技能與裝備引用。
pub fn clear_invalid_references(state: &mut GenericEditorState<UnitType>) {
    let valid_references = ValidUnitReferences::from_ui_state(&state.ui_state);

    for unit in &mut state.items {
        reference::retain_valid(&mut unit.skills, &valid_references.skills);
        valid_references.clear_invalid_equipment(&mut unit.equipment);
    }
}

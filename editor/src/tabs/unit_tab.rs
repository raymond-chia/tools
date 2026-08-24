//! 單位編輯器 tab

use crate::constants::{CLEAR_LABEL, SPACING_MEDIUM, SPACING_SMALL};
use crate::editor_item::EditorItem;
use crate::generic_editor::{GenericEditorState, MessageState};
use crate::tabs::reference;
use crate::utils::search::{combobox_with_dynamic_height, filter_by_search, render_search_input};
use board::domain::alias::{SkillName, TypeName};
use board::ecs_types::components::EquippedItems;
use board::loader_schema::UnitType;
use std::collections::HashSet;

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
    });

    ui.add_space(SPACING_SMALL);
    ui.separator();
    ui.heading("技能選擇");

    if ui_state.available_skills.is_empty() {
        ui.label("（尚未定義任何技能，請先到「技能」tab 創建技能）");
    } else {
        // 搜尋框
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
                        let mut selected = unit.skills.contains(skill_name);
                        if ui.checkbox(&mut selected, skill_name).changed() {
                            if selected {
                                unit.skills.push(skill_name.clone());
                            } else {
                                unit.skills.retain(|s| s != skill_name);
                            }
                        }
                    }
                }
            });
    }

    ui.separator();

    // 依儲存順序列出技能總數與已選技能名稱，方便快速檢視
    ui.label(format!("已選擇：{} 個技能", unit.skills.len()));
    ui.horizontal_wrapped(|ui| {
        for skill_name in &unit.skills {
            ui.label(skill_name);
            ui.add_space(SPACING_MEDIUM);
        }
    });

    ui.add_space(SPACING_SMALL);
    ui.separator();
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

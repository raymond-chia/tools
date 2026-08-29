//! 單位編輯器 tab

use crate::constants::{CLEAR_LABEL, SPACING_SMALL};
use crate::editor_item::EditorItem;
use crate::generic_editor::{GenericEditorState, MessageState};
use crate::tabs::reference;
use crate::tabs::skill_selection::{render_selected_skills_summary, render_skill_selector};
use crate::utils::search::{combobox_with_dynamic_height, filter_by_search, render_search_input};
use board::domain::alias::{SkillName, TypeName};
use board::domain::core_types::OffHandPermission;
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
    pub available_main_hand_weapons: Vec<TypeName>,
    pub available_two_handed_weapons: Vec<TypeName>,
    pub available_off_hand_weapons: Vec<TypeName>,
    pub available_shields: Vec<TypeName>,
    pub available_armors: Vec<TypeName>,
    pub available_accessories: Vec<TypeName>,

    pub skill_search_query: SkillName,
    pub main_hand_search_query: TypeName,
    pub off_hand_search_query: TypeName,
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

    let mut main_hand_options = ui_state.available_main_hand_weapons.clone();
    main_hand_options.extend(ui_state.available_two_handed_weapons.iter().cloned());
    let main_hand_changed = render_equipment_selector(
        ui,
        "主手：",
        "unit_main_hand",
        &mut unit.equipment.main_hand,
        &main_hand_options,
        &mut ui_state.main_hand_search_query,
    );

    let mut off_hand_permission_changed = false;
    ui.horizontal(|ui| {
        ui.label("副手權限：");
        for permission in [
            OffHandPermission::None,
            OffHandPermission::Weapon,
            OffHandPermission::Shield,
        ] {
            off_hand_permission_changed |= ui
                .selectable_value(
                    &mut unit.off_hand_permission,
                    permission,
                    permission.to_string(),
                )
                .changed();
        }
    });

    let main_hand_is_two_handed = main_hand_is_two_handed(unit, ui_state);
    if (main_hand_changed || off_hand_permission_changed)
        && (main_hand_is_two_handed || !off_hand_matches_permission(unit, ui_state))
    {
        unit.equipment.off_hand = None;
    }

    if main_hand_is_two_handed {
        ui.add_enabled_ui(false, |ui| {
            render_equipment_selector(
                ui,
                "副手：",
                "unit_off_hand",
                &mut unit.equipment.off_hand,
                &[],
                &mut ui_state.off_hand_search_query,
            );
        });
        ui.label("主手裝備雙手武器時，不能裝備副手。");
    } else {
        render_off_hand_selector(ui, unit, ui_state);
    }

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

fn render_off_hand_selector(ui: &mut egui::Ui, unit: &mut UnitType, ui_state: &mut UnitTabUIState) {
    let available_equipments = match unit.off_hand_permission {
        OffHandPermission::None => &[] as &[TypeName],
        OffHandPermission::Weapon => &ui_state.available_off_hand_weapons,
        OffHandPermission::Shield => &ui_state.available_shields,
    };

    ui.add_enabled_ui(unit.off_hand_permission != OffHandPermission::None, |ui| {
        render_equipment_selector(
            ui,
            "副手：",
            "unit_off_hand",
            &mut unit.equipment.off_hand,
            available_equipments,
            &mut ui_state.off_hand_search_query,
        );
    });
}

fn render_equipment_selector(
    ui: &mut egui::Ui,
    label: &str,
    id: &str,
    selected_equipment: &mut Option<TypeName>,
    available_equipments: &[TypeName],
    search_query: &mut TypeName,
) -> bool {
    let mut selection_changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        let selected_text = selected_equipment.as_deref().unwrap_or("（未裝備）");
        combobox_with_dynamic_height(id, selected_text, available_equipments.len() + 1).show_ui(
            ui,
            |ui| {
                selection_changed |= ui
                    .selectable_value(selected_equipment, None, CLEAR_LABEL)
                    .changed();
                let response = render_search_input(ui, search_query);
                ui.memory_mut(|memory| memory.request_focus(response.id));
                ui.separator();

                let visible_equipments = filter_by_search(available_equipments, search_query);
                if visible_equipments.is_empty() && !search_query.is_empty() {
                    ui.label("找不到符合的裝備");
                } else {
                    for equipment_name in visible_equipments {
                        selection_changed |= ui
                            .selectable_value(
                                selected_equipment,
                                Some(equipment_name.clone()),
                                equipment_name,
                            )
                            .changed();
                    }
                }
            },
        );
    });
    selection_changed
}

// ==================== 本地輔助函數 ====================

struct ValidUnitReferences {
    skills: HashSet<SkillName>,
    main_hand_weapons: HashSet<TypeName>,
    two_handed_weapons: HashSet<TypeName>,
    off_hand_weapons: HashSet<TypeName>,
    shields: HashSet<TypeName>,
    armors: HashSet<TypeName>,
    accessories: HashSet<TypeName>,
}

impl ValidUnitReferences {
    fn from_ui_state(ui_state: &UnitTabUIState) -> Self {
        Self {
            skills: ui_state.available_skills.iter().cloned().collect(),
            main_hand_weapons: ui_state
                .available_main_hand_weapons
                .iter()
                .cloned()
                .collect(),
            two_handed_weapons: ui_state
                .available_two_handed_weapons
                .iter()
                .cloned()
                .collect(),
            off_hand_weapons: ui_state
                .available_off_hand_weapons
                .iter()
                .cloned()
                .collect(),
            shields: ui_state.available_shields.iter().cloned().collect(),
            armors: ui_state.available_armors.iter().cloned().collect(),
            accessories: ui_state.available_accessories.iter().cloned().collect(),
        }
    }

    fn has_invalid_equipment(&self, unit: &UnitType) -> bool {
        let equipment = &unit.equipment;
        let main_hand_is_valid = equipment.main_hand.as_ref().is_none_or(|name| {
            self.main_hand_weapons.contains(name) || self.two_handed_weapons.contains(name)
        });

        !main_hand_is_valid
            || !self.off_hand_is_valid(unit)
            || reference::has_invalid(equipment.armor.iter(), &self.armors)
            || reference::has_invalid(equipment.first_accessory.iter(), &self.accessories)
            || reference::has_invalid(equipment.second_accessory.iter(), &self.accessories)
    }

    fn off_hand_is_valid(&self, unit: &UnitType) -> bool {
        let equipment = &unit.equipment;
        let matches_permission = match (&equipment.off_hand, unit.off_hand_permission) {
            (None, _) | (_, OffHandPermission::None) => equipment.off_hand.is_none(),
            (Some(name), OffHandPermission::Weapon) => self.off_hand_weapons.contains(name),
            (Some(name), OffHandPermission::Shield) => self.shields.contains(name),
        };
        let hand_combination_is_valid = equipment.off_hand.is_none()
            || !equipment
                .main_hand
                .as_ref()
                .is_some_and(|name| self.two_handed_weapons.contains(name));

        matches_permission && hand_combination_is_valid
    }

    fn clear_invalid_equipment(&self, unit: &mut UnitType) {
        let mut valid_main_hands = self.main_hand_weapons.clone();
        valid_main_hands.extend(self.two_handed_weapons.iter().cloned());
        reference::clear_invalid_option(&mut unit.equipment.main_hand, &valid_main_hands);
        if !self.off_hand_is_valid(unit) {
            unit.equipment.off_hand = None;
        }
        let equipment = &mut unit.equipment;
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
        || valid_references.has_invalid_equipment(unit)
}

/// 清除所有單位中已失效的技能與裝備引用。
pub fn clear_invalid_references(state: &mut GenericEditorState<UnitType>) {
    let valid_references = ValidUnitReferences::from_ui_state(&state.ui_state);

    for unit in &mut state.items {
        reference::retain_valid(&mut unit.skills, &valid_references.skills);
        valid_references.clear_invalid_equipment(unit);
    }
}

fn main_hand_is_two_handed(unit: &UnitType, ui_state: &UnitTabUIState) -> bool {
    unit.equipment
        .main_hand
        .as_ref()
        .is_some_and(|name| ui_state.available_two_handed_weapons.contains(name))
}

fn off_hand_matches_permission(unit: &UnitType, ui_state: &UnitTabUIState) -> bool {
    match (&unit.equipment.off_hand, unit.off_hand_permission) {
        (None, _) => true,
        (Some(_), OffHandPermission::None) => false,
        (Some(name), OffHandPermission::Weapon) => {
            ui_state.available_off_hand_weapons.contains(name)
        }
        (Some(name), OffHandPermission::Shield) => ui_state.available_shields.contains(name),
    }
}

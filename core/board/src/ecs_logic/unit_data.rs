use crate::domain::alias::TypeName;
use crate::ecs_types::components::{AttributeBundle, EquippedItems, Skills};
use crate::ecs_types::resources::GameData;
use crate::error::{DataError, Result};
use crate::loader_schema::UnitType;
use crate::logic::skill::unit_attributes;
use std::collections::HashSet;

pub(crate) struct InitialUnitData {
    pub(crate) equipped_items: EquippedItems,
    pub(crate) skills: Skills,
    pub(crate) attributes: AttributeBundle,
}

struct CalculatedUnitData {
    skills: Skills,
    attributes: AttributeBundle,
}

/// 建立單位的初始裝備、技能與屬性。
pub(crate) fn initial_unit_data(
    unit_type: &UnitType,
    game_data: &GameData,
) -> Result<InitialUnitData> {
    let equipped_items = unit_type.equipment.clone();
    let CalculatedUnitData { skills, attributes } =
        calculate_unit_data(unit_type, &equipped_items, game_data)?;
    Ok(InitialUnitData {
        equipped_items,
        skills,
        attributes,
    })
}

fn calculate_unit_data(
    unit_type: &UnitType,
    equipped_items: &EquippedItems,
    game_data: &GameData,
) -> Result<CalculatedUnitData> {
    let mut skill_names = Vec::new();
    let mut skill_names_seen = HashSet::new();
    for skill_name in &unit_type.skills {
        if skill_names_seen.insert(skill_name.clone()) {
            skill_names.push(skill_name.clone());
        }
    }
    for equipment_name in equipped_item_names(equipped_items) {
        let equipment = game_data
            .equipment_type_map
            .get(equipment_name)
            .ok_or_else(|| DataError::EquipmentTypeNotFound {
                equipment_name: equipment_name.clone(),
            })?;
        for skill_name in &equipment.granted_skills {
            if skill_names_seen.insert(skill_name.clone()) {
                skill_names.push(skill_name.clone());
            }
        }
    }
    let attributes = unit_attributes::calculate_attributes(
        unit_attributes::filter_continuous_effect(&skill_names, &[], &game_data.skill_map)?,
    );
    Ok(CalculatedUnitData {
        skills: Skills(skill_names),
        attributes,
    })
}

fn equipped_item_names(equipped_items: &EquippedItems) -> impl Iterator<Item = &TypeName> {
    equipped_items
        .main_hand
        .iter()
        .chain(equipped_items.off_hand.iter())
        .chain(equipped_items.armor.iter())
        .chain(equipped_items.first_accessory.iter())
        .chain(equipped_items.second_accessory.iter())
}

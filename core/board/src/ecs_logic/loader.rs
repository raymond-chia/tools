use crate::domain::core_types::{EquipmentType, OffHandPermission};
use crate::ecs_types::resources::GameData;
use crate::error::{DataError, LoadError, Result};
use crate::loader_schema::{
    EquipmentTomlType, EquipmentsToml, ObjectsToml, SkillsToml, UnitType, UnitsToml,
};
use bevy_ecs::prelude::World;
use std::collections::HashMap;

/// 遊戲資料的 TOML 來源字串集合
pub struct GameDataToml<'a> {
    pub units: &'a str,
    pub skills: &'a str,
    pub equipments: &'a str,
    pub objects: &'a str,
}

/// 反序列化 TOML 並將遊戲資料存入 World Resource
pub fn parse_and_insert_game_data(world: &mut World, source: GameDataToml<'_>) -> Result<()> {
    let parsed_skills: SkillsToml =
        toml::from_str(source.skills).map_err(|e| LoadError::DeserializeError {
            format: "skills.toml".to_string(),
            reason: e.to_string(),
        })?;

    let parsed_units: UnitsToml =
        toml::from_str(source.units).map_err(|e| LoadError::DeserializeError {
            format: "units.toml".to_string(),
            reason: e.to_string(),
        })?;

    let parsed_equipments: EquipmentsToml =
        toml::from_str(source.equipments).map_err(|e| LoadError::DeserializeError {
            format: "equipments.toml".to_string(),
            reason: e.to_string(),
        })?;

    let parsed_objects: ObjectsToml =
        toml::from_str(source.objects).map_err(|e| LoadError::DeserializeError {
            format: "objects.toml".to_string(),
            reason: e.to_string(),
        })?;

    let skill_map = parsed_skills
        .skills
        .into_iter()
        .map(|skill| (skill.name().clone(), skill))
        .collect::<HashMap<_, _>>();

    let equipment_type_map = parsed_equipments
        .equipments
        .into_iter()
        .map(|equipment| (equipment.name.clone(), equipment))
        .collect::<HashMap<_, _>>();

    for unit in &parsed_units.units {
        validate_equipment(unit, &equipment_type_map)?;
    }

    let unit_type_map = parsed_units
        .units
        .into_iter()
        .map(|unit| (unit.name.clone(), unit))
        .collect::<HashMap<_, _>>();

    let object_type_map = parsed_objects
        .objects
        .into_iter()
        .map(|object| (object.name.clone(), object))
        .collect::<HashMap<_, _>>();

    world.insert_resource(GameData {
        skill_map,
        unit_type_map,
        equipment_type_map,
        object_type_map,
    });

    Ok(())
}

fn validate_equipment(
    unit: &UnitType,
    equipment_type_map: &HashMap<String, EquipmentTomlType>,
) -> Result<()> {
    if let Some(equipment_name) = &unit.equipment.main_hand {
        let equipment = equipment_type_map.get(equipment_name).ok_or_else(|| {
            DataError::EquipmentTypeNotFound {
                equipment_name: equipment_name.clone(),
            }
        })?;
        if !matches!(
            equipment.typ,
            EquipmentType::Weapon | EquipmentType::TwoHandedWeapon
        ) {
            return Err(LoadError::ParseError(format!(
                "單位類型 {} 的主手不允許裝備: {}",
                unit.name, equipment_name
            ))
            .into());
        }
    }

    let equipment_name = match &unit.equipment.off_hand {
        Some(equipment_name) => equipment_name,
        None => return Ok(()),
    };
    let equipment =
        equipment_type_map
            .get(equipment_name)
            .ok_or_else(|| DataError::EquipmentTypeNotFound {
                equipment_name: equipment_name.clone(),
            })?;
    let main_hand_is_two_handed = unit
        .equipment
        .main_hand
        .as_ref()
        .and_then(|name| equipment_type_map.get(name))
        .is_some_and(|equipment| equipment.typ == EquipmentType::TwoHandedWeapon);
    let is_allowed = !main_hand_is_two_handed
        && matches!(
            (unit.off_hand_permission, equipment.typ),
            (OffHandPermission::Weapon, EquipmentType::Weapon)
                | (OffHandPermission::Shield, EquipmentType::Shield)
        );
    if !is_allowed {
        return Err(LoadError::ParseError(format!(
            "單位類型 {} 的副手不允許裝備: {}",
            unit.name, equipment_name
        ))
        .into());
    }
    Ok(())
}

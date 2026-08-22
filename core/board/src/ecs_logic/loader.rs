use crate::ecs_types::resources::GameData;
use crate::error::{LoadError, Result};
use crate::loader_schema::{EquipmentsToml, ObjectsToml, SkillsToml, UnitsToml};
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

    let unit_type_map = parsed_units
        .units
        .into_iter()
        .map(|unit| (unit.name.clone(), unit))
        .collect::<HashMap<_, _>>();

    let equipment_type_map = parsed_equipments
        .equipments
        .into_iter()
        .map(|equipment| (equipment.name.clone(), equipment))
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

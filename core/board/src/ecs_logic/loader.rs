use crate::ecs_types::resources::GameData;
use crate::error::{LoadError, Result};
use crate::loader_schema::{ObjectsToml, SkillsToml, UnitsToml};
use bevy_ecs::prelude::World;
use std::collections::HashMap;

/// 反序列化 TOML 並將遊戲資料存入 World Resource
pub fn parse_and_insert_game_data(
    world: &mut World,
    units_toml: &str,
    skills_toml: &str,
    objects_toml: &str,
) -> Result<()> {
    let parsed_skills: SkillsToml =
        toml::from_str(skills_toml).map_err(|e| LoadError::DeserializeError {
            format: "skills.toml".to_string(),
            reason: e.to_string(),
        })?;

    let parsed_units: UnitsToml =
        toml::from_str(units_toml).map_err(|e| LoadError::DeserializeError {
            format: "units.toml".to_string(),
            reason: e.to_string(),
        })?;

    let parsed_objects: ObjectsToml =
        toml::from_str(objects_toml).map_err(|e| LoadError::DeserializeError {
            format: "objects.toml".to_string(),
            reason: e.to_string(),
        })?;

    let skill_map = parsed_skills
        .skills
        .into_iter()
        .map(|skill| (skill.name.clone(), skill))
        .collect::<HashMap<_, _>>();

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
        object_type_map,
    });

    Ok(())
}

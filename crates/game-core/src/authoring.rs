//! Editor 與戰鬥共用的作者資料格式。
use super::{Definition, MapDef, SkillDef, Team, TerrainPlacement, TerrainTypeDef, UnitDef};
use crate::error::{self, GameError};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Deserialize, Serialize)]
pub struct Definitions {
    pub terrain_types: HashMap<String, TerrainTypeDef>,
    pub skills: Vec<SkillDef>,
    pub unit_types: Vec<UnitType>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct UnitType {
    pub id: String,
    pub visual: String,
    #[serde(default = "one")]
    pub width: i32,
    #[serde(default = "one")]
    pub height: i32,
    pub hp: i32,
    pub movement: u32,
    pub initiative: i32,
    pub dodge: i32,
    pub block: i32,
    pub attack: i32,
    pub power: i32,
    #[serde(default)]
    pub skills: Vec<String>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Map {
    pub name: String,
    pub width: i32,
    pub height: i32,
    #[serde(default)]
    pub terrains: Vec<TerrainPlacement>,
    #[serde(default)]
    pub units: Vec<UnitPlacement>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct UnitPlacement {
    pub id: String,
    pub unit_type: String,
    pub team: Team,
    pub x: i32,
    pub y: i32,
}

fn one() -> i32 {
    1
}

pub(super) fn into_definition(definitions: Definitions, map: Map) -> Result<Definition, GameError> {
    if map.name.trim().is_empty() {
        return Err(error::empty_map_name());
    }
    if map
        .units
        .iter()
        .any(|unit| matches!(&unit.team, Team::Enemy(name) if name.trim().is_empty()))
    {
        return Err(error::empty_enemy_faction());
    }
    let skill_ids: HashSet<_> = definitions
        .skills
        .iter()
        .map(|skill| skill.id.as_str())
        .collect();
    let mut types = HashMap::new();
    for kind in definitions.unit_types {
        if kind.id.trim().is_empty() || kind.hp <= 0 || kind.width <= 0 || kind.height <= 0 {
            return Err(error::invalid_unit_type(&kind.id));
        }
        if let Some(unknown) = kind
            .skills
            .iter()
            .find(|skill| !skill_ids.contains(skill.as_str()))
        {
            return Err(error::unknown_unit_skill(&kind.id, unknown));
        }
        if types.insert(kind.id.clone(), kind).is_some() {
            return Err(error::duplicate_unit_type_id());
        }
    }
    let mut used_ids = HashSet::new();
    let mut units = Vec::new();
    for placement in map.units {
        if placement.id.trim().is_empty() {
            return Err(error::empty_unit_placement_id());
        }
        if !used_ids.insert(placement.id.clone()) {
            return Err(error::duplicate_unit_placement_id(&placement.id));
        }
        let kind = types
            .get(&placement.unit_type)
            .ok_or_else(|| error::unknown_unit_type(&placement.unit_type))?;
        units.push(UnitDef {
            id: placement.id,
            unit_type: kind.id.clone(),
            visual: kind.visual.clone(),
            team: placement.team,
            x: placement.x,
            y: placement.y,
            width: kind.width,
            height: kind.height,
            hp: kind.hp,
            movement: kind.movement,
            initiative: kind.initiative,
            dodge: kind.dodge,
            block: kind.block,
            attack: kind.attack,
            power: kind.power,
            skills: kind.skills.clone(),
        });
    }
    Ok(Definition {
        map: MapDef {
            width: map.width,
            height: map.height,
            terrains: map.terrains,
        },
        terrain_types: definitions.terrain_types,
        skills: definitions.skills,
        units,
    })
}

pub fn definitions_to_json(text: &str) -> Result<String, GameError> {
    let value: Definitions =
        toml::from_str(text).map_err(|e| error::definitions_toml_parse(e.to_string()))?;
    serde_json::to_string(&value).map_err(|e| error::json_serialize(e.to_string()))
}

pub fn map_to_json(text: &str) -> Result<String, GameError> {
    let value: Map = toml::from_str(text).map_err(|e| error::map_toml_parse(e.to_string()))?;
    serde_json::to_string(&value).map_err(|e| error::json_serialize(e.to_string()))
}

/// 將 Godot 編輯器傳來的 JSON 定義與地圖資料驗證後，轉成儲存或試玩用的 TOML 文件。
/// JSON 僅用於編輯器與 Rust 之間傳遞資料；實際保存的檔案仍是 TOML。
pub fn documents_from_json(definitions: &str, map: &str) -> Result<(String, String), GameError> {
    let definitions: Definitions = serde_json::from_str(definitions)
        .map_err(|e| error::definitions_json_parse(e.to_string()))?;
    let map: Map = serde_json::from_str(map).map_err(|e| error::map_json_parse(e.to_string()))?;
    super::Game::from_authoring(definitions.clone(), map.clone())?;
    Ok((
        toml::to_string_pretty(&definitions).map_err(|e| error::toml_serialize(e.to_string()))?,
        toml::to_string_pretty(&map).map_err(|e| error::toml_serialize(e.to_string()))?,
    ))
}

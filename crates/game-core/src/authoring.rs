//! Editor 與戰鬥共用的作者資料格式。
use super::{Definition, MapDef, SkillDef, Team, TerrainTypeDef, TriggerDef, UnitDef};
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
    pub name: String,
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
    pub damage: i32,
    #[serde(default)]
    pub skills: Vec<String>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Map {
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub costs: Vec<u32>,
    #[serde(default)]
    pub triggers: Vec<TriggerDef>,
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

pub(super) fn into_definition(definitions: Definitions, map: Map) -> Result<Definition, String> {
    if map.name.trim().is_empty() {
        return Err("地圖名稱不可為空".into());
    }
    if map
        .units
        .iter()
        .any(|unit| matches!(&unit.team, Team::Enemy(name) if name.trim().is_empty()))
    {
        return Err("敵方派系名稱不可為空".into());
    }
    let skill_ids: HashSet<_> = definitions
        .skills
        .iter()
        .map(|skill| skill.id.as_str())
        .collect();
    let mut types = HashMap::new();
    for kind in definitions.unit_types {
        if kind.id.trim().is_empty() || kind.hp <= 0 || kind.width <= 0 || kind.height <= 0 {
            return Err(format!("{} 的 id、HP 與佔用尺寸無效", kind.id));
        }
        if let Some(unknown) = kind
            .skills
            .iter()
            .find(|skill| !skill_ids.contains(skill.as_str()))
        {
            return Err(format!("{} 使用未知技能 {}", kind.id, unknown));
        }
        if types.insert(kind.id.clone(), kind).is_some() {
            return Err("單位定義 id 重複".into());
        }
    }
    let mut used_ids = HashSet::new();
    let mut units = Vec::new();
    for placement in map.units {
        if placement.id.trim().is_empty() {
            return Err("單位配置 id 不可為空".into());
        }
        if !used_ids.insert(placement.id.clone()) {
            return Err(format!("單位配置 id 重複：{}", placement.id));
        }
        let kind = types
            .get(&placement.unit_type)
            .ok_or_else(|| format!("找不到單位定義 {}", placement.unit_type))?;
        units.push(UnitDef {
            id: placement.id,
            name: kind.name.clone(),
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
            damage: kind.damage,
            skills: kind.skills.clone(),
        });
    }
    Ok(Definition {
        map: MapDef {
            width: map.width,
            height: map.height,
            costs: map.costs,
            triggers: map.triggers,
        },
        terrain_types: definitions.terrain_types,
        skills: definitions.skills,
        units,
    })
}

pub fn definitions_to_json(text: &str) -> Result<String, String> {
    let value: Definitions = toml::from_str(text).map_err(|e| e.to_string())?;
    serde_json::to_string(&value).map_err(|e| e.to_string())
}

pub fn map_to_json(text: &str) -> Result<String, String> {
    let value: Map = toml::from_str(text).map_err(|e| e.to_string())?;
    serde_json::to_string(&value).map_err(|e| e.to_string())
}

/// 將 Godot 編輯器傳來的 JSON 定義與地圖資料驗證後，轉成儲存或試玩用的 TOML 文件。
/// JSON 僅用於編輯器與 Rust 之間傳遞資料；實際保存的檔案仍是 TOML。
pub fn documents_from_json(definitions: &str, map: &str) -> Result<(String, String), String> {
    let definitions: Definitions = serde_json::from_str(definitions).map_err(|e| e.to_string())?;
    let map: Map = serde_json::from_str(map).map_err(|e| e.to_string())?;
    super::Game::from_authoring(definitions.clone(), map.clone())?;
    Ok((
        toml::to_string_pretty(&definitions).map_err(|e| e.to_string())?,
        toml::to_string_pretty(&map).map_err(|e| e.to_string())?,
    ))
}

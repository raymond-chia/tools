//! 原始資料格式，供編輯器與正式遊戲載入文件時使用。
//! 由 Game::from_authoring 驗證並建立戰鬥；本檔宣告的格式不作為戰鬥運行時資料。
use super::{SkillDef, Team, TerrainPlacement, TerrainTypeDef};
use crate::error::{self, GameError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub id: i64,
    pub unit_type: String,
    pub team: Team,
    pub x: i32,
    pub y: i32,
}

fn one() -> i32 {
    1
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

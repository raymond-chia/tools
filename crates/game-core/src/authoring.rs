//! 原始資料格式，供編輯器與正式遊戲載入文件時使用。
//! 由 Game::from_authoring 驗證並建立戰鬥；本檔宣告的格式不作為戰鬥運行時資料。
use super::{SkillDef, Team, TerrainPlacement, TerrainTypeDef};
use crate::error::{self, GameError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub struct Definitions {
    pub ai_profiles: Vec<AiProfile>,
    pub terrain_types: Vec<TerrainTypeDef>,
    pub skills: Vec<SkillDef>,
    pub unit_types: Vec<UnitType>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct UnitType {
    pub id: String,
    pub ai_profile: String,
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

/// 共用戰術偏好；權重為非負整數，0 代表不評估該項收益。
#[derive(Clone, Deserialize, Serialize)]
pub struct AiProfile {
    pub id: String,
    pub distance_preference: DistancePreference,
    pub damage_weight: u32,
    pub healing_weight: u32,
    pub positioning_weight: u32,
    /// 額外偏好命中機率；0 維持只依傷害收益評估。
    #[serde(default)]
    pub hit_weight: u32,
    /// 攻擊上回合攻擊過的對象時，增加固定收益。
    #[serde(default)]
    pub pursuit_weight: u32,
}

/// 跨可用技能比較合法射程決定近遠站位，不另存固定格數。
#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DistancePreference {
    Near,
    Far,
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

// 共用文件讀取與格式轉換不依賴 editor feature；遊戲仍可直接讀取 TOML。
pub fn definitions_to_json(text: &str) -> Result<String, GameError> {
    let value: Definitions =
        toml::from_str(text).map_err(|e| error::definitions_toml_parse(e.to_string()))?;
    serde_json::to_string(&value).map_err(|e| error::json_serialize(e.to_string()))
}

pub fn map_to_json(text: &str) -> Result<String, GameError> {
    let value: Map = toml::from_str(text).map_err(|e| error::map_toml_parse(e.to_string()))?;
    serde_json::to_string(&value).map_err(|e| error::json_serialize(e.to_string()))
}

//! 原始資料格式，供編輯器與正式遊戲載入文件時使用。
//! 驗證與建局準備集中於 validation；單位與地形配置會轉成執行期資料，勝敗條件組則由核心直接使用。
pub(crate) mod validation;
use super::{GridPos, SkillDef, Team, TerrainPlacement, TerrainTypeDef};
use crate::error::{self, GameError};
use crate::model::default_one;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub struct Definitions {
    #[serde(default)]
    pub equipment: Vec<EquipmentDef>,
    /// 所有單位預設取得的被動技能；同效果由單位版本覆蓋。
    #[serde(default)]
    pub default_passive_skills: Vec<String>,
    #[serde(default)]
    pub ai_profiles: Vec<AiProfile>,
    #[serde(default)]
    pub terrain_types: Vec<TerrainTypeDef>,
    #[serde(default)]
    pub skills: Vec<SkillDef>,
    #[serde(default)]
    pub unit_types: Vec<UnitType>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct UnitType {
    pub id: String,
    #[serde(default)]
    pub ai_profile: String,
    #[serde(default)]
    pub visual: String,
    #[serde(default = "default_one")]
    pub width: i32,
    #[serde(default = "default_one")]
    pub height: i32,
    #[serde(default = "default_one")]
    pub hp: i32,
    #[serde(default)]
    pub movement: u32,
    #[serde(default)]
    pub initiative: i32,
    #[serde(default)]
    pub dodge: i32,
    #[serde(default)]
    pub hit: i32,
    #[serde(default)]
    pub physical_power: i32,
    #[serde(default)]
    pub magical_power: i32,
    #[serde(default)]
    pub main_hand: String,
    #[serde(default)]
    pub off_hand: String,
    #[serde(default)]
    pub armor: String,
    #[serde(default)]
    pub accessory: String,
    #[serde(default)]
    pub skills: Vec<String>,
    /// 與全體預設合併，同效果由此清單覆蓋；空清單保留預設。
    #[serde(default)]
    pub passive_skills: Vec<String>,
}

/// 共用戰術偏好；權重為非負整數，0 代表不評估該項收益。
#[derive(Clone, Deserialize, Serialize)]
pub struct AiProfile {
    pub id: String,
    #[serde(default)]
    pub distance_preference: DistancePreference,
    #[serde(default = "default_ai_weight")]
    pub damage_weight: u32,
    #[serde(default)]
    pub healing_weight: u32,
    #[serde(default = "default_ai_weight")]
    pub positioning_weight: u32,
    /// 額外偏好命中機率；0 維持只依傷害收益評估。
    #[serde(default)]
    pub hit_weight: u32,
    /// 攻擊上回合攻擊過的對象時，增加固定收益。
    #[serde(default)]
    pub pursuit_weight: u32,
}

/// 跨可用技能比較合法射程決定近遠站位，不另存固定格數。
#[derive(Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DistancePreference {
    #[default]
    Near,
    Far,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Map {
    pub victory: OutcomeGroup,
    pub defeat: OutcomeGroup,
    #[serde(default)]
    pub cargo: Option<CargoObjective>,
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_one")]
    pub width: i32,
    #[serde(default = "default_one")]
    pub height: i32,
    #[serde(default)]
    pub terrains: Vec<TerrainPlacement>,
    #[serde(default)]
    pub units: Vec<UnitPlacement>,
}

/// 勝敗各自指定條件組，條件本身不決定結果分類。
#[derive(Clone, Deserialize, Serialize)]
pub struct OutcomeGroup {
    pub mode: ConditionMode,
    pub conditions: Vec<OutcomeCondition>,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionMode {
    All,
    Any,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutcomeCondition {
    AllEnemiesDefeated,
    AllPlayersDefeated,
    CargoEscaped,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct CargoObjective {
    pub positions: Vec<GridPos>,
    pub exits: Vec<GridPos>,
}

/// 出現門檻依簡單、普通、困難排序；沒有隱含預設難度。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Difficulty {
    Easy,
    Normal,
    Hard,
}

/// 單位在畫面上的水平方向，不影響戰鬥規則。
#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Facing {
    Left,
    Right,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct UnitPlacement {
    pub facing: Facing,
    pub id: i64,
    pub min_difficulty: Option<Difficulty>,
    #[serde(default)]
    pub unit_type: String,
    #[serde(default)]
    pub team: Team,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
}

// 共用文件讀取與格式轉換不依賴 editor feature；遊戲仍可直接讀取 TOML。
pub fn definitions_to_json(text: &str) -> Result<String, GameError> {
    let value: Definitions =
        toml::from_str(text).map_err(|e| error::definitions_toml_parse(e.to_string()))?;
    serde_json::to_string(&value).map_err(|e| error::json_serialize(e.to_string()))
}

pub fn map_to_json(text: &str) -> Result<String, GameError> {
    let value: Map = toml::from_str(text).map_err(|e| error::map_toml_parse(e.to_string()))?;
    validation::validate_map(&value)?;
    serde_json::to_string(&value).map_err(|e| error::json_serialize(e.to_string()))
}

/// 裝備種類決定占用手數；護具與飾品不占用手。
#[derive(Clone, Deserialize, Serialize)]
pub struct EquipmentDef {
    pub id: String,
    #[serde(default)]
    pub slot: EquipmentSlot,
    #[serde(default)]
    pub hp: i32,
    #[serde(default)]
    pub physical_power: i32,
    #[serde(default)]
    pub magical_power: i32,
    #[serde(default)]
    pub block: i32,
    #[serde(default)]
    pub block_reduction: i32,
}

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EquipmentSlot {
    #[default]
    OneHand,
    TwoHand,
    Armor,
    Accessory,
}

fn default_ai_weight() -> u32 {
    10
}

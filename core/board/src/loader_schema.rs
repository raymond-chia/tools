//! Loader 相關的資料結構定義

use crate::domain::alias::{Coord, ID, MovementCost, SkillName, TypeName};
use crate::domain::core_types::SkillType;
use crate::ecs_types::components::Position;
use serde::{Deserialize, Serialize};

// ============================================================================
// 單位系統 (Unit System)
// ============================================================================

/// 單位類型定義
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UnitType {
    pub name: String,
    pub skills: Vec<SkillName>,
}

// ============================================================================
// 物件系統 (Object System)
// ============================================================================

/// 物件類型定義
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObjectType {
    pub name: String,
    pub movement_cost: MovementCost,
    pub blocks_sight: bool,
    pub blocks_sound: bool,
}

// ============================================================================
// 關卡系統 (Level System)
// ============================================================================

/// 陣營定義（關卡中的陣營設定）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Faction {
    pub id: ID,
    pub name: String,
    pub alliance: ID,
    /// 陣營顏色（RGB）
    pub color: [u8; 3],
}

/// 單位配置（關卡中的單位放置）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UnitPlacement {
    pub unit_type_name: TypeName,
    pub faction_id: ID,
    pub position: Position,
}

/// 物件配置（關卡中的物件放置）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObjectPlacement {
    pub object_type_name: TypeName,
    pub position: Position,
}

/// 關卡類型定義
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LevelType {
    pub name: String,
    pub board_width: Coord,
    pub board_height: Coord,
    pub factions: Vec<Faction>,
    pub max_player_units: usize,
    pub deployment_positions: Vec<Position>,
    pub unit_placements: Vec<UnitPlacement>,
    pub object_placements: Vec<ObjectPlacement>,
}

// ============================================================================
// 頂層 TOML 反序列化結構
// ============================================================================

/// 技能 TOML 頂層結構
#[derive(Debug, Serialize, Deserialize)]
pub struct SkillsToml {
    pub skills: Vec<SkillType>,
}

/// 單位 TOML 頂層結構
#[derive(Debug, Serialize, Deserialize)]
pub struct UnitsToml {
    pub units: Vec<UnitType>,
}

/// 物件 TOML 頂層結構
#[derive(Debug, Serialize, Deserialize)]
pub struct ObjectsToml {
    pub objects: Vec<ObjectType>,
}

//! ECS World Resource 定義

use crate::domain::alias::{Coord, ID, SkillName, TypeName};
use crate::domain::core_types::TurnEntry;
use crate::ecs_types::components::Position;
use crate::loader_schema::{Faction, ObjectType, SkillType, UnitType};
use bevy_ecs::prelude::Resource;
use std::collections::{HashMap, HashSet};

/// 解析後的靜態遊戲資料，作為 World Resource
#[derive(Debug, Resource)]
pub struct GameData {
    pub skill_map: HashMap<SkillName, SkillType>,
    pub unit_type_map: HashMap<TypeName, UnitType>,
    pub object_type_map: HashMap<TypeName, ObjectType>,
}

/// 棋盤尺寸（全局唯一，作為 Resource）
#[derive(Debug, Clone, Copy, Resource)]
pub struct Board {
    pub width: Coord,
    pub height: Coord,
}

/// 關卡靜態元數據（關卡初始化後存入 World）
#[derive(Debug, Clone, Resource)]
pub struct LevelConfig {
    pub name: String,
    pub factions: HashMap<ID, Faction>,
}

/// 部署設定（關卡初始化後存入 World，供部署階段查詢）
#[derive(Debug, Clone, Resource)]
pub struct DeploymentConfig {
    pub max_player_units: usize,
    pub deployment_positions: HashSet<Position>,
}

/// 回合順序 Resource
//      目前的 Occupant 作為穩定身份標識是正確的設計——它讓領域邏輯和回合計算完全不依賴 ECS。
//      如果查詢效率是問題，
//      可以在 ecs_logic 層維護一個 HashMap<Occupant, Entity> 的快取 resource，
//      而不是把 Entity 塞進 TurnEntry。
#[derive(Debug, Clone, Resource)]
pub struct TurnOrder {
    pub round: u32,
    pub entries: Vec<TurnEntry>,
    pub current_index: usize,
}

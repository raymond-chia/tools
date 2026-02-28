//! ECS World Resource 定義

use crate::domain::alias::{Coord, SkillName, TypeName};
use crate::ecs_types::components::Position;
use crate::loader_schema::{Faction, ObjectType, SkillType, UnitType};
use bevy_ecs::prelude::Resource;
use std::collections::HashMap;

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
    pub factions: Vec<Faction>,
}

/// 部署設定（關卡初始化後存入 World，供部署階段查詢）
#[derive(Debug, Clone, Resource)]
pub struct DeploymentConfig {
    pub max_player_units: usize,
    pub deployment_positions: Vec<Position>,
}

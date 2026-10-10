//! 集中管理遊戲規則中可供企劃調整的數值。

pub const ATTACK_DIE_SIDES: u32 = 20;
pub const INITIATIVE_DIE_SIDES: u32 = 20;
pub const BASE_DEFENSE: i32 = 10;
pub const COLLISION_DAMAGE: i32 = 2;
pub const PUSH_DISTANCE: i32 = 1;
pub const DEFAULT_MANA: i32 = 1;

pub const ENCOUNTER_RANGE: i32 = 10;

/// 未放置 ground 地形的格子視為此地形。
pub const DEFAULT_GROUND_TERRAIN: &str = "plain";

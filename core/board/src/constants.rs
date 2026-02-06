//! 遊戲常數定義

use crate::alias::MovementCost;

/// 基礎移動成本
pub const BASIC_MOVEMENT_COST: MovementCost = 10;

/// 無法通過的移動成本
pub const IMPASSABLE_MOVEMENT_COST: MovementCost = BASIC_MOVEMENT_COST * 1000;

/// 即死傷害（負數表示傷害）
pub const CONTACT_HEALTH_DAMAGE: i32 = -10000;

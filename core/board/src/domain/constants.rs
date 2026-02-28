//! 遊戲常數定義

use crate::domain::alias::{ID, MovementCost};

/// 玩家所屬同盟 ID（寫死，未來擴展時移除）
pub const PLAYER_ALLIANCE_ID: ID = 0;

/// 玩家陣營 ID（寫死，未來擴展時移除）
pub const PLAYER_FACTION_ID: ID = 0;

/// 基礎移動成本
pub const BASIC_MOVEMENT_COST: MovementCost = 10;

/// 無法通過的移動成本
pub const IMPASSABLE_MOVEMENT_COST: MovementCost = BASIC_MOVEMENT_COST * 1000;

/// 即死傷害（負數表示傷害）
pub const HP_MODIFY_DAMAGE: i32 = -10000;

//! 核心業務邏輯（不是 ECS System）

pub mod board;
pub mod movement;

use crate::alias::MovementCost;

pub const BASIC_MOVEMENT_COST: MovementCost = 10;

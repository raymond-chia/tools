//! lib.rs：
//! - 負責模組匯入、型別定義與全域常數。
//! - 僅作為 crate 入口與 re-export，不放具體邏輯或資料結構實作。
//! - 不負責戰鬥、AI、棋盤、單位等細節邏輯。
use serde::{Deserialize, Serialize};

mod action;
mod ai;
mod battle;
mod board;
mod error;
mod statistics;
mod unit;

pub use action::*;
pub use ai::*;
pub use battle::*;
pub use board::*;
pub use error::*;
pub use statistics::*;
pub use unit::*;

pub type BoardID = String;
pub type UnitID = u32;
pub type ObjectID = u32;
pub type UnitTemplateType = String;
pub type TeamID = String;
pub type MovementCost = usize;
pub type ReactionCount = usize;
pub type RGB = (u8, u8, u8);
pub type RGBA = (u8, u8, u8, u8);
pub type AIScore = f32;

pub const TEAM_PLAYER: &str = "player";
pub const TEAM_NONE: &str = "none"; // 中立物件的 TeamID
pub const MAX_MOVEMENT_COST: MovementCost = 999;

// 光源範圍（曼哈頓距離）
pub const TORCH_BRIGHT_RANGE: usize = 1;
pub const TORCH_DIM_RANGE: usize = 3;
pub const CAMPFIRE_BRIGHT_RANGE: usize = 6;
pub const CAMPFIRE_DIM_RANGE: usize = 12;

/// Burn 效果每回合造成的固定傷害
const BURN_DAMAGE_PER_TURN: i32 = 5;

#[derive(
    Debug, Deserialize, Serialize, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct Pos {
    pub x: usize,
    pub y: usize,
}

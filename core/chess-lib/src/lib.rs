use serde::{Deserialize, Serialize};

mod action;
mod ai;
mod battle;
mod board;
mod error;
mod unit;

pub use action::*;
pub use ai::*;
pub use battle::*;
pub use board::*;
pub use error::*;
pub use unit::*;

pub type BoardID = String;
pub type UnitID = u64;
pub type UnitTemplateType = String;
pub type TeamID = String;
pub type MovementCost = usize;
pub type RGB = (u8, u8, u8);
pub type RGBA = (u8, u8, u8, u8);
pub type AIScore = f32;

pub const PLAYER_TEAM: &str = "player";
pub const MAX_MOVEMENT_COST: MovementCost = 999;

#[derive(
    Debug, Deserialize, Serialize, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct Pos {
    pub x: usize,
    pub y: usize,
}

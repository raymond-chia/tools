use serde::{Deserialize, Serialize};

mod algo;

pub use algo::*;

pub type UnitID = u64;
pub type TeamID = String;
pub type MovementCost = usize;

pub const PLAYER_TEAM: &str = "player";
pub const MAX_MOVEMENT_COST: MovementCost = 99;

#[derive(
    Debug, Deserialize, Serialize, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct Pos {
    pub x: usize,
    pub y: usize,
}

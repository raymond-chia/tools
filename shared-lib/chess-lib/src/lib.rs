use serde::{Deserialize, Serialize};

mod action;
mod battle;
mod board;
mod unit;

pub use action::*;
pub use battle::*;
pub use board::*;
pub use unit::*;

pub type BoardID = String;
pub type UnitID = u64;
pub type UnitTemplateType = String;
pub type TeamID = String;
pub type MovementCost = usize;

pub const PLAYER_TEAM: &str = "player";
pub const MAX_MOVEMENT_COST: MovementCost = 999;

#[derive(Debug)]
pub enum Error {
    InvalidParameter,

    NotEnoughPoints,
    NotReachable(Pos),

    NoUnitOnPos(Pos),
    AlliedUnitOnPos(Pos),
    HostileUnitOnPos(Pos),

    NoTileOnPos(Pos),
}

#[derive(
    Debug, Deserialize, Serialize, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct Pos {
    pub x: usize,
    pub y: usize,
}

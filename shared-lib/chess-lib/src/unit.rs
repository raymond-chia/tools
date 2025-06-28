use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Deserialize, Serialize)]
pub struct Team {
    pub id: TeamID,
    pub color: (u8, u8, u8), // RGB color
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UnitConfig {
    pub id: UnitID,
    pub unit_type: UnitType,
    pub team: TeamID,
    pub pos: Pos,
}

#[derive(Debug)]
pub struct Unit {
    pub id: UnitID,
    pub team: TeamID,
    pub moved: MovementCost,
    pub move_points: MovementCost,
    pub skills: BTreeSet<String>,
}

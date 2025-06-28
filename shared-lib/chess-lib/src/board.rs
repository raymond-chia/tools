use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Debug, Deserialize, Serialize)]
pub enum Terrain {
    Plain,
    Hill,
    Mountain,
    Forest,
    ShallowWater,
    DeepWater,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Object {
    Wall,
    Tent2 { rel: Pos, duration: u32 },
    Tent15 { rel: Pos, duration: u32 },
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Tile {
    pub terrain: Terrain,
    pub object: Option<Object>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BoardConfig {
    pub tiles: Vec<Vec<Tile>>,
    pub teams: BTreeMap<TeamID, Team>,
    // 以上會同步到 Board
    pub deployable: BTreeSet<Pos>,
    pub units: BTreeMap<UnitID, UnitConfig>,
}

#[derive(Debug)]
pub struct Board {
    pub tiles: Vec<Vec<Tile>>,
    pub teams: HashMap<TeamID, Team>,
    pub units: HashMap<UnitID, Unit>,
    pub pos_to_unit: HashMap<Pos, UnitID>,
}

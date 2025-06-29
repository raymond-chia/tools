use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use strum_macros::{Display, EnumIter};

#[derive(Debug, Deserialize, Serialize, Clone, Copy, Default, Display, EnumIter, PartialEq)]
pub enum Terrain {
    #[default]
    Plain,
    Hill,
    Mountain,
    Forest,
    ShallowWater,
    DeepWater,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Object {
    Wall,
    Tent2 { rel: Pos, duration: u32 },
    Tent15 { rel: Pos, duration: u32 },
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Tile {
    pub terrain: Terrain,
    pub object: Option<Object>,
}

// config 欄位需要排序
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct BoardConfig {
    pub tiles: Vec<Vec<Tile>>,
    pub teams: BTreeMap<TeamID, Team>,
    // 以上會同步到 Board
    pub deployable: BTreeSet<Pos>,
    pub units: BTreeMap<UnitID, UnitMarker>,
}

#[derive(Debug)]
pub struct Board {
    pub tiles: Vec<Vec<Tile>>,
    pub teams: HashMap<TeamID, Team>,
    pub units: HashMap<UnitID, Unit>,
    pub pos_to_unit: HashMap<Pos, UnitID>,
}

impl Board {
    pub fn from_config(
        config: BoardConfig,
        unit_templates: &BTreeMap<UnitTemplateType, UnitTemplate>,
    ) -> Result<Self, String> {
        let teams = HashMap::from_iter(config.teams.into_iter().map(|(id, team)| (id, team)));

        let mut units = HashMap::new();
        let mut pos_to_unit = HashMap::new();
        for (unit_id, unit_config) in config.units {
            let unit = Unit::from_template(
                &unit_config,
                unit_templates
                    .get(&unit_config.unit_template_type)
                    .ok_or_else(|| {
                        format!("missing unit template: {}", &unit_config.unit_template_type)
                    })?,
            );
            pos_to_unit.insert(unit_config.pos, unit_id);
            units.insert(unit_id, unit);
        }

        Ok(Board {
            tiles: config.tiles,
            teams,
            units,
            pos_to_unit,
        })
    }
}

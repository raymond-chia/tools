use crate::*;
use serde::{Deserialize, Serialize};
use skills_lib::*;
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

#[derive(Debug, Deserialize, Serialize, Clone, Copy, Default, Display, EnumIter, PartialEq)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Debug, Deserialize, Serialize, Clone, Display, EnumIter, PartialEq)]
pub enum Object {
    Wall,
    Tent2 {
        orientation: Orientation,
        rel: Pos,
        duration: u32,
    },
    Tent15 {
        orientation: Orientation,
        rel: Pos,
        duration: u32,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Tile {
    pub terrain: Terrain,
    pub object: Option<Object>,
}

// config 欄位需要排序
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct BoardConfig {
    pub tiles: Vec<Vec<Tile>>,
    pub teams: BTreeMap<TeamID, Team>,
    // 以上會同步到 Board
    pub deployable: BTreeSet<Pos>,
    #[serde(with = "unitid_key_map")]
    pub units: BTreeMap<UnitID, UnitMarker>,
}

#[derive(Debug, Default)]
pub struct Board {
    pub tiles: Vec<Vec<Tile>>,
    pub teams: HashMap<TeamID, Team>,
    pub units: HashMap<UnitID, Unit>,
    pub unit_map: UnitMap,
}

pub trait UnitTemplateGetter {
    fn get(&self, typ: &UnitTemplateType) -> Option<&UnitTemplate>;
}

impl Board {
    pub fn from_config(
        config: BoardConfig,
        unit_templates: &impl UnitTemplateGetter,
        skills: &BTreeMap<SkillID, Skill>,
    ) -> Result<Self, String> {
        let teams = HashMap::from_iter(config.teams.into_iter());

        let mut units = HashMap::new();
        let mut unit_map = UnitMap::default();
        for (unit_id, unit_config) in config.units {
            let template = unit_templates
                .get(&unit_config.unit_template_type)
                .ok_or_else(|| {
                    format!("missing unit template: {}", &unit_config.unit_template_type)
                })?;
            let unit = Unit::from_template(&unit_config, template, skills)?;
            unit_map.insert(unit_id, unit_config.pos);
            units.insert(unit_id, unit);
        }

        Ok(Board {
            tiles: config.tiles,
            teams,
            units,
            unit_map,
        })
    }

    pub fn pos_to_unit(&self, pos: Pos) -> Option<UnitID> {
        self.unit_map.get_unit(pos)
    }

    pub fn unit_to_pos(&self, unit_id: &UnitID) -> Option<Pos> {
        self.unit_map.get_pos(*unit_id)
    }
}

// $x:expr: 匹配一個運算式
// $t:ty: 匹配型別
// $id:ident: 匹配識別字
macro_rules! impl_board {
    ($t:ty) => {
        impl $t {
            pub fn width(&self) -> usize {
                self.tiles.first().map_or(0, |row| row.len())
            }

            pub fn height(&self) -> usize {
                self.tiles.len()
            }

            pub fn get_tile(&self, pos: Pos) -> Option<&Tile> {
                let Pos { x, y } = pos;
                self.tiles.get(y)?.get(x)
            }

            pub fn get_tile_mut(&mut self, pos: Pos) -> Option<&mut Tile> {
                let Pos { x, y } = pos;
                self.tiles.get_mut(y)?.get_mut(x)
            }
        }
    };
}

impl_board!(BoardConfig);
impl_board!(Board);

pub fn movement_cost(t: Terrain) -> MovementCost {
    match t {
        Terrain::Plain => 10,
        Terrain::Hill => 13,
        Terrain::Mountain => 20,
        Terrain::Forest => 13,
        Terrain::ShallowWater => 17,
        Terrain::DeepWater => MAX_MOVEMENT_COST,
    }
}

#[derive(Debug, Default)]
pub struct UnitMap {
    pos_to_unit: HashMap<Pos, UnitID>,
    unit_to_pos: HashMap<UnitID, Pos>,
}

impl UnitMap {
    pub fn insert(&mut self, unit_id: UnitID, pos: Pos) {
        self.pos_to_unit.insert(pos, unit_id);
        self.unit_to_pos.insert(unit_id, pos);
    }

    pub fn move_unit(&mut self, unit_id: UnitID, from: Pos, to: Pos) -> Result<(), String> {
        if self.unit_to_pos.get(&unit_id) != Some(&from) {
            return Err(format!("unit {} is not at {:?}", unit_id, from));
        }
        if self.pos_to_unit.get(&to).is_some() {
            return Err(format!("position {:?} is already occupied", to));
        }
        self.pos_to_unit.remove(&from);
        self.pos_to_unit.insert(to, unit_id);
        self.unit_to_pos.insert(unit_id, to);
        Ok(())
    }

    pub fn get_unit(&self, pos: Pos) -> Option<UnitID> {
        self.pos_to_unit.get(&pos).copied()
    }

    pub fn get_pos(&self, unit_id: UnitID) -> Option<Pos> {
        self.unit_to_pos.get(&unit_id).copied()
    }
}

// 讓 BTreeMap<UnitID, UnitMarker> 可以用 string key 序列化
mod unitid_key_map {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(
        map: &BTreeMap<UnitID, UnitMarker>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let string_map: BTreeMap<String, &UnitMarker> =
            map.iter().map(|(k, v)| (k.to_string(), v)).collect();
        string_map.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<BTreeMap<UnitID, UnitMarker>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string_map: BTreeMap<String, UnitMarker> = BTreeMap::deserialize(deserializer)?;
        string_map
            .into_iter()
            .map(|(k, v)| {
                k.parse()
                    .map(|num| (num, v))
                    .map_err(serde::de::Error::custom)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_board_from_config() {
        let (skills, template) = {
            let data = include_str!("../tests/unit.json");
            let v: serde_json::Value = serde_json::from_str(data).unwrap();
            let sprint_data = include_str!("../tests/skill_sprint.json");
            let sprint_skill: Skill = serde_json::from_str(sprint_data).unwrap();
            let slash_data = include_str!("../tests/skill_slash.json");
            let slash_skill: Skill = serde_json::from_str(slash_data).unwrap();
            let skills = BTreeMap::from([
                ("sprint".to_string(), sprint_skill),
                ("slash".to_string(), slash_skill),
            ]);
            let template: UnitTemplate = serde_json::from_value(v["UnitTemplate"].clone()).unwrap();
            (skills, template)
        };

        // 準備 BoardConfig
        let data = include_str!("../tests/board.json");
        let config: BoardConfig = serde_json::from_str(data).unwrap();
        assert_eq!(config.deployable, BTreeSet::from([Pos { x: 1, y: 1 }]));

        // 準備 UnitTemplateGetter stub
        struct StubGetter {
            template: UnitTemplate,
        }
        impl UnitTemplateGetter for StubGetter {
            fn get(&self, typ: &UnitTemplateType) -> Option<&UnitTemplate> {
                if typ == &self.template.name {
                    Some(&self.template)
                } else {
                    None
                }
            }
        }
        let stub_getter = StubGetter { template };

        // 執行 from_config
        let board = Board::from_config(config, &stub_getter, &skills).unwrap();

        // 驗證 tiles
        assert_eq!(board.width(), 2);
        assert_eq!(board.height(), 2);
        assert_eq!(
            board.get_tile(Pos { x: 0, y: 0 }).unwrap().terrain,
            Terrain::Plain
        );
        assert_eq!(
            board.get_tile(Pos { x: 0, y: 1 }).unwrap().terrain,
            Terrain::Hill
        );

        // 驗證 team
        assert_eq!(board.teams.len(), 1);
        assert!(board.teams.contains_key("t1"));

        // 驗證 unit
        assert_eq!(board.units.len(), 1);
        assert!(board.units.contains_key(&42));
        assert_eq!(board.unit_to_pos(&42), Some(Pos { x: 0, y: 0 }));
    }

    #[test]
    fn test_movement_cost() {
        assert_eq!(movement_cost(Terrain::Plain), 10);
        assert_eq!(movement_cost(Terrain::Hill), 13);
        assert_eq!(movement_cost(Terrain::Mountain), 20);
        assert_eq!(movement_cost(Terrain::Forest), 13);
        assert_eq!(movement_cost(Terrain::ShallowWater), 17);
        assert_eq!(movement_cost(Terrain::DeepWater), MAX_MOVEMENT_COST);
    }
}

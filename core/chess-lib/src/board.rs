//! board.rs：
//! - 定義棋盤（Board）、地形（Terrain）、物件（Object）、單位配置等資料結構。
//! - 負責棋盤初始化、單位與位置對應、地形查詢等邏輯。
//! - 不負責單位屬性計算、AI 決策或戰鬥流程。
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
    Tree,
    Cliff {
        orientation: Orientation,
    },
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

impl Object {
    /// 是否允許通行：物件自身負責描述能否通行的規則
    /// - Wall / Tree -> 阻擋（不可通行）
    /// - 多格帳篷 / 其他 -> 允許通行（可視需求調整）
    pub fn is_passable(&self) -> bool {
        match self {
            Object::Wall
            | Object::Tree
            | Object::Cliff { .. }
            | Object::Tent2 { .. }
            | Object::Tent15 { .. } => false,
        }
    }
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
    ) -> Result<Self, Error> {
        let func = "Board::from_config";

        let teams = HashMap::from_iter(config.teams.into_iter());

        let mut units = HashMap::new();
        let mut unit_map = UnitMap::default();
        for (unit_id, unit_config) in config.units {
            let template = unit_templates
                .get(&unit_config.unit_template_type)
                .ok_or_else(|| Error::MissingUnitTemplate {
                    func,
                    template_type: unit_config.unit_template_type.clone(),
                })?;
            let unit =
                Unit::from_template(&unit_config, template, skills).map_err(|e| Error::Wrap {
                    func,
                    source: Box::new(e),
                })?;
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

    pub fn unit_to_pos(&self, unit_id: UnitID) -> Option<Pos> {
        self.unit_map.get_pos(unit_id)
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
    // 初始化專用
    pub fn insert(&mut self, unit_id: UnitID, pos: Pos) {
        self.pos_to_unit.insert(pos, unit_id);
        self.unit_to_pos.insert(unit_id, pos);
    }

    pub fn move_unit(&mut self, unit_id: UnitID, from: Pos, to: Pos) -> Result<(), Error> {
        let func = "UnitMap::move_unit";

        if self.get_pos(unit_id) != Some(from) {
            return Err(Error::UnitNotAtPos {
                func,
                unit_id,
                pos: from,
            });
        }
        if self.get_unit(to).is_some() {
            return Err(Error::PosOccupied { func, pos: to });
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
    // 用於測試自訂序列化
    use serde_json;

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
        assert_eq!(board.pos_to_unit(Pos { x: 0, y: 0 }), Some(42));
        assert_eq!(board.unit_to_pos(42), Some(Pos { x: 0, y: 0 }));
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

    #[test]
    fn test_move_unit() {
        let mut unit_map = UnitMap::default();
        let unit_id = 1;
        let from = Pos { x: 0, y: 0 };
        let to = Pos { x: 1, y: 0 };
        let other = Pos { x: 2, y: 0 };

        // 先插入單位
        unit_map.insert(unit_id, from);

        // 正常移動
        assert!(unit_map.move_unit(unit_id, from, to).is_ok());
        assert_eq!(unit_map.get_unit(to), Some(unit_id));
        assert_eq!(unit_map.get_pos(unit_id), Some(to));
        assert_eq!(unit_map.get_unit(from), None);

        // from 位置錯誤
        let err = unit_map.move_unit(unit_id, from, other).unwrap_err();
        match err {
            Error::UnitNotAtPos {
                unit_id: e_id, pos, ..
            } => {
                assert_eq!(e_id, unit_id);
                assert_eq!(pos, from);
            }
            _ => panic!("應該是 UnitNotAtPos"),
        }

        // to 位置已被佔用
        let unit_id2 = 2;
        unit_map.insert(unit_id2, from);
        let err = unit_map.move_unit(unit_id2, from, to).unwrap_err();
        match err {
            Error::PosOccupied { pos, .. } => {
                assert_eq!(pos, to);
            }
            _ => panic!("應該是 PosOccupied"),
        }
    }

    #[test]
    fn test_unitid_key_map_serialize() {
        // 準備資料
        let mut map: BTreeMap<UnitID, UnitMarker> = BTreeMap::new();
        map.insert(
            42,
            UnitMarker {
                id: 42,
                unit_template_type: "TestTemplate".to_string(),
                team: "t1".to_string(),
                pos: Pos { x: 1, y: 2 },
            },
        );
        map.insert(
            7,
            UnitMarker {
                id: 7,
                unit_template_type: "Another".to_string(),
                team: "t2".to_string(),
                pos: Pos { x: 0, y: 0 },
            },
        );

        // 序列化
        // 直接呼叫自訂 serialize
        let json = {
            let mut buf = Vec::new();
            {
                let mut ser = serde_json::Serializer::new(&mut buf);
                super::unitid_key_map::serialize(&map, &mut ser).unwrap();
            }
            String::from_utf8(buf).unwrap()
        };

        // 反序列化
        let deser: BTreeMap<UnitID, UnitMarker> = {
            let mut de = serde_json::Deserializer::from_str(&json);
            super::unitid_key_map::deserialize(&mut de).unwrap()
        };

        // 驗證
        assert_eq!(deser.len(), 2);
        assert_eq!(deser[&42].unit_template_type, "TestTemplate");
        assert_eq!(deser[&42].pos, Pos { x: 1, y: 2 });
        assert_eq!(deser[&42].id, 42);
        assert_eq!(deser[&42].team, "t1");
        assert_eq!(deser[&7].unit_template_type, "Another");
        assert_eq!(deser[&7].pos, Pos { x: 0, y: 0 });
        assert_eq!(deser[&7].id, 7);
        assert_eq!(deser[&7].team, "t2");
    }
}

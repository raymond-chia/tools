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
    Up,
    Down,
    Left,
    Right,
}

/// 光照等級
#[derive(
    Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Display, EnumIter,
)]
pub enum LightLevel {
    Darkness = 0, // 黑暗
    Dim = 1,      // 微光
    Bright = 2,   // 明亮
}

impl Default for LightLevel {
    fn default() -> Self {
        LightLevel::Bright
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Display, EnumIter, PartialEq)]
pub enum Object {
    Tree,
    Wall,
    Cliff { orientation: Orientation },
    Pit,
    Tent2 { orientation: Orientation, rel: Pos },
    Tent15 { orientation: Orientation, rel: Pos },
    Torch { lit: bool },
    Campfire { lit: bool },
}

impl Object {
    /// 是否允許通行：物件自身負責描述能否通行的規則
    pub fn is_passable(&self) -> bool {
        match self {
            Object::Tree
            | Object::Wall
            | Object::Cliff { .. }
            | Object::Pit
            | Object::Tent2 { .. }
            | Object::Tent15 { .. }
            | Object::Campfire { .. } => false,
            Object::Torch { .. } => true,
        }
    }

    /// 根據距離計算光照等級
    /// 返回 Darkness 表示無光或熄滅
    pub fn light_level_at(&self, distance: usize) -> LightLevel {
        match self {
            Object::Torch { lit: true } => {
                if TORCH_BRIGHT_RANGE > 0 && distance <= TORCH_BRIGHT_RANGE {
                    LightLevel::Bright
                } else if TORCH_DIM_RANGE > 0 && distance <= TORCH_DIM_RANGE {
                    LightLevel::Dim
                } else {
                    LightLevel::Darkness
                }
            }
            Object::Campfire { lit: true } => {
                if CAMPFIRE_BRIGHT_RANGE > 0 && distance <= CAMPFIRE_BRIGHT_RANGE {
                    LightLevel::Bright
                } else if CAMPFIRE_DIM_RANGE > 0 && distance <= CAMPFIRE_DIM_RANGE {
                    LightLevel::Dim
                } else {
                    LightLevel::Darkness
                }
            }
            Object::Tree
            | Object::Wall
            | Object::Cliff { .. }
            | Object::Pit
            | Object::Tent2 { .. }
            | Object::Tent15 { .. }
            | Object::Torch { lit: false }
            | Object::Campfire { lit: false } => LightLevel::Darkness,
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
    #[serde(default)]
    pub ambient_light: LightLevel, // 環境光（向後兼容）
    // 以上會同步到 Board
    pub deployable: BTreeSet<Pos>,
    #[serde(with = "unitid_key_map")]
    pub units: BTreeMap<UnitID, UnitMarker>,
}

#[derive(Debug, Default)]
pub struct Board {
    pub tiles: Vec<Vec<Tile>>,
    pub teams: HashMap<TeamID, Team>,
    pub ambient_light: LightLevel,
    pub light_sources: Vec<Pos>,
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

        let teams = HashMap::from_iter(config.teams);

        let mut units = HashMap::new();
        let mut unit_map = UnitMap::default();
        for (unit_id, unit_config) in config.units {
            let template = unit_templates
                .get(&unit_config.unit_template_type)
                .ok_or_else(|| Error::MissingUnitTemplate {
                    func,
                    template_type: unit_config.unit_template_type.clone(),
                })?;
            let unit = Unit::from_template(&unit_config, template, skills).wrap_context(func)?;
            unit_map.insert(unit_id, unit_config.pos);
            units.insert(unit_id, unit);
        }

        let mut board = Board {
            tiles: config.tiles,
            teams,
            ambient_light: config.ambient_light,
            light_sources: Vec::new(),
            units,
            unit_map,
        };

        // 初始化光源快取
        board.rebuild_light_sources_cache();

        Ok(board)
    }

    pub fn pos_to_unit(&self, pos: Pos) -> Option<UnitID> {
        self.unit_map.get_unit(pos)
    }

    pub fn unit_to_pos(&self, unit_id: UnitID) -> Option<Pos> {
        self.unit_map.get_pos(unit_id)
    }

    /// 掃描棋盤建立光源快取
    pub fn rebuild_light_sources_cache(&mut self) {
        self.light_sources.clear();
        for y in 0..self.height() {
            for x in 0..self.width() {
                let pos = Pos { x, y };
                if let Some(tile) = self.get_tile(pos) {
                    if let Some(obj) = &tile.object {
                        match obj {
                            Object::Torch { .. } | Object::Campfire { .. } => {
                                self.light_sources.push(pos);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    pub fn get_light_level(
        &self,
        pos: Pos,
        _skills: &BTreeMap<SkillID, Skill>,
    ) -> Result<LightLevel, Error> {
        let func = "get_light_level";
        let mut max_light = self.ambient_light;

        // 早期退出：如果環境光已經是最亮，直接返回
        if max_light == LightLevel::Bright {
            return Ok(LightLevel::Bright);
        }

        // 檢查所有 Object 光源
        for &source_pos in &self.light_sources {
            let tile = self
                .get_tile(source_pos)
                .ok_or_else(|| Error::InvalidImplementation {
                    func,
                    detail: format!(
                        "light_sources cache contains invalid position {:?}",
                        source_pos
                    ),
                })?;
            let obj = tile
                .object
                .as_ref()
                .ok_or_else(|| Error::InvalidImplementation {
                    func,
                    detail: format!(
                        "light_sources cache contains position {:?} without light source",
                        source_pos
                    ),
                })?;

            let distance = manhattan_distance(pos, source_pos);
            let light_level = obj.light_level_at(distance);

            max_light = max_light.max(light_level);
            if max_light == LightLevel::Bright {
                return Ok(LightLevel::Bright);
            }
        }

        // 檢查所有單位的 CarriesLight 效果
        for (unit_id, unit) in &self.units {
            let Some(unit_pos) = self.unit_to_pos(*unit_id) else {
                continue;
            };

            let distance = manhattan_distance(pos, unit_pos);
            let light_level = unit::effects_to_light_level(unit.status_effects.iter(), distance);

            max_light = max_light.max(light_level);
            if max_light == LightLevel::Bright {
                return Ok(LightLevel::Bright);
            }
        }

        Ok(max_light)
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

/// 計算曼哈頓距離
pub fn manhattan_distance(a: Pos, b: Pos) -> usize {
    let dx = if a.x > b.x { a.x - b.x } else { b.x - a.x };
    let dy = if a.y > b.y { a.y - b.y } else { b.y - a.y };
    dx + dy
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
        assert_eq!(deser[&42].id, 42);
        assert_eq!(deser[&42].unit_template_type, "TestTemplate");
        assert_eq!(deser[&42].team, "t1");
        assert_eq!(deser[&42].pos, Pos { x: 1, y: 2 });
        assert_eq!(deser[&7].id, 7);
        assert_eq!(deser[&7].unit_template_type, "Another");
        assert_eq!(deser[&7].team, "t2");
        assert_eq!(deser[&7].pos, Pos { x: 0, y: 0 });
    }

    #[test]
    fn test_object_light_level_at_torch() {
        let torch_lit = Object::Torch { lit: true };

        // 距離 0-3：Bright
        assert_eq!(torch_lit.light_level_at(0), LightLevel::Bright);
        assert_eq!(torch_lit.light_level_at(3), LightLevel::Bright);

        // 距離 4-5：Dim
        assert_eq!(torch_lit.light_level_at(4), LightLevel::Dim);
        assert_eq!(torch_lit.light_level_at(5), LightLevel::Dim);

        // 距離 > 5：Darkness
        assert_eq!(torch_lit.light_level_at(6), LightLevel::Darkness);
        assert_eq!(torch_lit.light_level_at(10), LightLevel::Darkness);

        // 熄滅的火把：所有距離都是 Darkness
        let torch_unlit = Object::Torch { lit: false };
        assert_eq!(torch_unlit.light_level_at(0), LightLevel::Darkness);
        assert_eq!(torch_unlit.light_level_at(3), LightLevel::Darkness);
    }

    #[test]
    fn test_object_light_level_at_campfire() {
        let campfire_lit = Object::Campfire { lit: true };

        // 距離 0-5：Bright
        assert_eq!(campfire_lit.light_level_at(0), LightLevel::Bright);
        assert_eq!(campfire_lit.light_level_at(5), LightLevel::Bright);

        // 距離 6-8：Dim
        assert_eq!(campfire_lit.light_level_at(6), LightLevel::Dim);
        assert_eq!(campfire_lit.light_level_at(8), LightLevel::Dim);

        // 距離 > 8：Darkness
        assert_eq!(campfire_lit.light_level_at(9), LightLevel::Darkness);

        // 熄滅的營火：所有距離都是 Darkness
        let campfire_unlit = Object::Campfire { lit: false };
        assert_eq!(campfire_unlit.light_level_at(0), LightLevel::Darkness);
        assert_eq!(campfire_unlit.light_level_at(5), LightLevel::Darkness);
    }

    #[test]
    fn test_object_light_level_at_non_light_objects() {
        // 其他物件不發光：所有距離都是 Darkness
        assert_eq!(Object::Tree.light_level_at(0), LightLevel::Darkness);
        assert_eq!(Object::Wall.light_level_at(0), LightLevel::Darkness);
        assert_eq!(Object::Pit.light_level_at(0), LightLevel::Darkness);
        assert_eq!(
            Object::Cliff {
                orientation: Orientation::Up
            }
            .light_level_at(0),
            LightLevel::Darkness
        );
    }

    #[test]
    fn test_object_is_passable_light_sources() {
        // Torch（火把）可通行，Campfire（營火）不可通行
        assert!(Object::Torch { lit: true }.is_passable());
        assert!(Object::Torch { lit: false }.is_passable());
        assert!(!Object::Campfire { lit: true }.is_passable());
        assert!(!Object::Campfire { lit: false }.is_passable());
    }

    #[test]
    fn test_board_config_backward_compatibility() {
        // 測試舊存檔（沒有 ambient_light）的兼容性
        let json = r#"{
            "tiles": [[{"terrain": "Plain", "object": null}]],
            "teams": {},
            "deployable": [],
            "units": {}
        }"#;
        let config: BoardConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.ambient_light, LightLevel::Bright); // default
    }

    #[test]
    fn test_get_light_level_ambient() {
        // 只有環境光時返回環境光
        let mut tiles = vec![vec![Tile::default()]];
        tiles[0][0].terrain = Terrain::Plain;

        let config = BoardConfig {
            tiles,
            teams: BTreeMap::new(),
            ambient_light: LightLevel::Darkness,
            deployable: BTreeSet::new(),
            units: BTreeMap::new(),
        };

        let skills = BTreeMap::new();
        struct EmptyGetter;
        impl UnitTemplateGetter for EmptyGetter {
            fn get(&self, _typ: &UnitTemplateType) -> Option<&UnitTemplate> {
                None
            }
        }

        let board = Board::from_config(config, &EmptyGetter, &skills).unwrap();
        assert_eq!(
            board.get_light_level(Pos { x: 0, y: 0 }, &skills),
            Ok(LightLevel::Darkness)
        );
    }

    #[test]
    fn test_get_light_level_torch_bright() {
        // Torch 3 格內為 Bright
        let mut tiles = vec![vec![Tile::default(); 5]; 5];
        tiles[2][2].object = Some(Object::Torch { lit: true });

        let config = BoardConfig {
            tiles,
            teams: BTreeMap::new(),
            ambient_light: LightLevel::Darkness,
            deployable: BTreeSet::new(),
            units: BTreeMap::new(),
        };

        let skills = BTreeMap::new();
        struct EmptyGetter;
        impl UnitTemplateGetter for EmptyGetter {
            fn get(&self, _typ: &UnitTemplateType) -> Option<&UnitTemplate> {
                None
            }
        }

        let board = Board::from_config(config, &EmptyGetter, &skills).unwrap();

        // Torch 在 (2, 2)，TORCH_BRIGHT_RADIUS = 3
        // 距離 0：同位置
        assert_eq!(
            board.get_light_level(Pos { x: 2, y: 2 }, &skills),
            Ok(LightLevel::Bright)
        );

        // 距離 1
        assert_eq!(
            board.get_light_level(Pos { x: 3, y: 2 }, &skills),
            Ok(LightLevel::Bright)
        );

        // 距離 3（邊界）
        assert_eq!(
            board.get_light_level(Pos { x: 2, y: 5 }, &skills),
            Ok(LightLevel::Bright)
        );
        assert_eq!(
            board.get_light_level(Pos { x: 5, y: 2 }, &skills),
            Ok(LightLevel::Bright)
        );
    }

    #[test]
    fn test_get_light_level_torch_dim() {
        // Torch 4-5 格為 Dim
        let mut tiles = vec![vec![Tile::default(); 10]; 10];
        tiles[5][5].object = Some(Object::Torch { lit: true });

        let config = BoardConfig {
            tiles,
            teams: BTreeMap::new(),
            ambient_light: LightLevel::Darkness,
            deployable: BTreeSet::new(),
            units: BTreeMap::new(),
        };

        let skills = BTreeMap::new();
        struct EmptyGetter;
        impl UnitTemplateGetter for EmptyGetter {
            fn get(&self, _typ: &UnitTemplateType) -> Option<&UnitTemplate> {
                None
            }
        }

        let board = Board::from_config(config, &EmptyGetter, &skills).unwrap();

        // Torch 在 (5, 5)，TORCH_DIM_RADIUS = 5
        // 距離 4（Dim）
        assert_eq!(
            board.get_light_level(Pos { x: 5, y: 9 }, &skills),
            Ok(LightLevel::Dim)
        );

        // 距離 5（邊界，Dim）
        assert_eq!(
            board.get_light_level(Pos { x: 5, y: 0 }, &skills),
            Ok(LightLevel::Dim)
        );

        // 距離 6（超出範圍，回到環境光 Darkness）
        assert_eq!(
            board.get_light_level(Pos { x: 0, y: 0 }, &skills),
            Ok(LightLevel::Darkness)
        );
    }

    #[test]
    fn test_get_light_level_multiple_sources() {
        // 多光源重疊取最亮
        let mut tiles = vec![vec![Tile::default(); 10]; 10];
        tiles[2][2].object = Some(Object::Torch { lit: true });
        tiles[5][5].object = Some(Object::Campfire { lit: true });

        let config = BoardConfig {
            tiles,
            teams: BTreeMap::new(),
            ambient_light: LightLevel::Darkness,
            deployable: BTreeSet::new(),
            units: BTreeMap::new(),
        };

        let skills = BTreeMap::new();
        struct EmptyGetter;
        impl UnitTemplateGetter for EmptyGetter {
            fn get(&self, _typ: &UnitTemplateType) -> Option<&UnitTemplate> {
                None
            }
        }

        let board = Board::from_config(config, &EmptyGetter, &skills).unwrap();

        // 位置 (4, 4)：
        // - 距離 Torch(2,2) = |4-2| + |4-2| = 4 (Dim)
        // - 距離 Campfire(5,5) = |4-5| + |4-5| = 2 (Bright)
        // 取最亮 = Bright
        assert_eq!(
            board.get_light_level(Pos { x: 4, y: 4 }, &skills),
            Ok(LightLevel::Bright)
        );
    }

    #[test]
    fn test_rebuild_light_sources_cache() {
        // 測試光源快取建立（快取所有 Torch/Campfire，不論是否點燃）
        let mut tiles = vec![vec![Tile::default(); 5]; 5];
        tiles[1][1].object = Some(Object::Torch { lit: true });
        tiles[2][2].object = Some(Object::Campfire { lit: true });
        tiles[3][3].object = Some(Object::Torch { lit: false }); // 熄滅的也計入快取

        let config = BoardConfig {
            tiles,
            teams: BTreeMap::new(),
            ambient_light: LightLevel::Bright,
            deployable: BTreeSet::new(),
            units: BTreeMap::new(),
        };

        let skills = BTreeMap::new();
        struct EmptyGetter;
        impl UnitTemplateGetter for EmptyGetter {
            fn get(&self, _typ: &UnitTemplateType) -> Option<&UnitTemplate> {
                None
            }
        }

        let board = Board::from_config(config, &EmptyGetter, &skills).unwrap();

        // 驗證快取（包含點燃和熄滅的）
        assert_eq!(board.light_sources.len(), 3);
        assert!(board.light_sources.contains(&Pos { x: 1, y: 1 }));
        assert!(board.light_sources.contains(&Pos { x: 2, y: 2 }));
        assert!(board.light_sources.contains(&Pos { x: 3, y: 3 })); // 熄滅的也在快取
    }

    #[test]
    fn test_get_light_level_carries_light() {
        use std::collections::BTreeSet;
        // 測試單位攜帶光源（CarriesLight 效果）
        let tiles = vec![vec![Tile::default(); 10]; 10];

        let config = BoardConfig {
            tiles,
            teams: BTreeMap::new(),
            ambient_light: LightLevel::Darkness,
            deployable: BTreeSet::new(),
            units: BTreeMap::new(),
        };

        let skills = BTreeMap::new();
        struct EmptyGetter;
        impl UnitTemplateGetter for EmptyGetter {
            fn get(&self, _typ: &UnitTemplateType) -> Option<&UnitTemplate> {
                None
            }
        }

        let mut board = Board::from_config(config, &EmptyGetter, &skills).unwrap();

        // 創建一個帶有 CarriesLight 效果的單位
        let unit_id = 1;
        let mut unit = Unit {
            id: unit_id,
            unit_template_type: "test".to_string(),
            team: "team1".to_string(),
            hp: 100,
            max_hp: 100,
            mp: 50,
            max_mp: 50,
            move_points: 5,
            moved: 0,
            has_cast_skill_this_turn: false,
            reactions_used_this_turn: 0,
            max_reactions_per_turn: 0,
            skills: BTreeSet::new(),
            status_effects: vec![],
        };

        // 添加 CarriesLight 效果（bright_range: 3, dim_range: 5）
        unit.status_effects.push(Effect::CarriesLight {
            target_type: TargetType::Caster,
            shape: Shape::Point,
            bright_range: 3,
            dim_range: 5,
            duration: -1,
        });

        // 將單位放置在 (5, 5)
        board.unit_map.insert(unit_id, Pos { x: 5, y: 5 });
        board.units.insert(unit_id, unit);

        // 測試不同距離的光照等級
        // 距離 0：同位置（應為 Bright）
        assert_eq!(
            board.get_light_level(Pos { x: 5, y: 5 }, &skills),
            Ok(LightLevel::Bright)
        );

        // 距離 1（應為 Bright）
        assert_eq!(
            board.get_light_level(Pos { x: 6, y: 5 }, &skills),
            Ok(LightLevel::Bright)
        );

        // 距離 3：邊界（應為 Bright）
        assert_eq!(
            board.get_light_level(Pos { x: 5, y: 8 }, &skills),
            Ok(LightLevel::Bright)
        );

        // 距離 4（應為 Dim）
        assert_eq!(
            board.get_light_level(Pos { x: 5, y: 9 }, &skills),
            Ok(LightLevel::Dim)
        );

        // 距離 5：邊界（應為 Dim）
        assert_eq!(
            board.get_light_level(Pos { x: 8, y: 7 }, &skills),
            Ok(LightLevel::Dim)
        );

        // 距離 6：超出範圍（應為 Darkness）
        assert_eq!(
            board.get_light_level(Pos { x: 0, y: 0 }, &skills),
            Ok(LightLevel::Darkness)
        );
    }

    #[test]
    fn test_manhattan_distance() {
        // 同一點距離為 0
        let p1 = Pos { x: 5, y: 5 };
        assert_eq!(manhattan_distance(p1, p1), 0);

        // 水平距離
        let p2 = Pos { x: 10, y: 5 };
        assert_eq!(manhattan_distance(p1, p2), 5);
        assert_eq!(manhattan_distance(p2, p1), 5); // 對稱性

        // 垂直距離
        let p3 = Pos { x: 5, y: 10 };
        assert_eq!(manhattan_distance(p1, p3), 5);

        // 對角線距離
        let p4 = Pos { x: 8, y: 9 };
        assert_eq!(manhattan_distance(p1, p4), 7); // |8-5| + |9-5| = 3 + 4 = 7

        // 經典範例：(0,0) 到 (3,4)
        let origin = Pos { x: 0, y: 0 };
        let p5 = Pos { x: 3, y: 4 };
        assert_eq!(manhattan_distance(origin, p5), 7);
    }

    #[test]
    fn test_light_level_ordering() {
        // 驗證光照等級排序：Bright > Dim > Darkness
        assert!(LightLevel::Bright > LightLevel::Dim);
        assert!(LightLevel::Dim > LightLevel::Darkness);
        assert!(LightLevel::Bright > LightLevel::Darkness);

        // 驗證 max() 操作（多光源取最亮）
        assert_eq!(LightLevel::Bright.max(LightLevel::Dim), LightLevel::Bright);
        assert_eq!(LightLevel::Dim.max(LightLevel::Darkness), LightLevel::Dim);
        assert_eq!(
            LightLevel::Darkness.max(LightLevel::Bright),
            LightLevel::Bright
        );
    }

    #[test]
    fn test_light_level_default() {
        // 驗證預設值為 Bright
        assert_eq!(LightLevel::default(), LightLevel::Bright);
    }
}

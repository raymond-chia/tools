//! board.rs：
//! - 定義棋盤（Board）、地形（Terrain）、物件（Object）、單位配置等資料結構。
//! - 負責棋盤初始化、單位與位置對應、地形查詢等邏輯。
//! - 不負責單位屬性計算、AI 決策或戰鬥流程。
use crate::*;
use object_lib::{ObjectType, Orientation};
use serde::{Deserialize, Serialize};
use skills_lib::*;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use strum_macros::{Display, EnumIter};

#[derive(Debug, Deserialize, Serialize, Clone, Copy, Default, Display, EnumIter, PartialEq)]
pub enum Terrain {
    #[default]
    Plain,
    ShallowWater,
    DeepWater,
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

/// 物體：棋盤上的物件實例
/// 支持多格物體、臨時/永久、所屬隊伍
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Object {
    pub id: ObjectID,
    pub affected_positions: Vec<Pos>,
    pub object_type: ObjectType,
    pub duration: i32,        // -1 = 永久，> 0 = 臨時
    pub creator_team: TeamID, // "none" = 地圖預設的中立物件
}

impl Object {
    /// 是否允許通行：物件自身負責描述能否通行的規則
    pub fn is_passable(&self) -> bool {
        match &self.object_type {
            ObjectType::Tree
            | ObjectType::Wall
            | ObjectType::Cliff { .. }
            | ObjectType::Pit
            | ObjectType::Tent2 { .. }
            | ObjectType::Tent15 { .. }
            | ObjectType::Campfire { .. } => false,
            ObjectType::Torch { .. } => true,
        }
    }

    /// 從特定方向查看時是否阻擋視線
    /// from_pos: 觀察者位置
    /// obj_pos: 物件位置
    ///
    /// Cliff 高地機制：
    /// - Cliff orientation 指向「懸崖下方」
    /// - 從上方（與 orientation 同向或正交）往下看：不阻擋
    /// - 從下方（與 orientation 反向）往上看：阻擋視線
    pub fn blocks_sight_from(&self, from_pos: Pos, obj_pos: Pos) -> bool {
        match &self.object_type {
            // 完全阻擋視線的物件（無方向性）
            ObjectType::Wall
            | ObjectType::Tree
            | ObjectType::Tent2 { .. }
            | ObjectType::Tent15 { .. } => true,
            // 不阻擋視線的物件
            ObjectType::Pit | ObjectType::Torch { .. } | ObjectType::Campfire { .. } => false,
            // Cliff 有方向性：只有從下方往上看才阻擋
            // TODO: 峽谷兩端互相看不見的限制
            ObjectType::Cliff { orientation } => {
                // 計算觀察者相對於懸崖的方向
                let dx = from_pos.x as isize - obj_pos.x as isize;
                let dy = from_pos.y as isize - obj_pos.y as isize;

                // 判斷觀察者是否在懸崖下方
                // 只有從懸崖下方往上看才會被阻擋
                let is_from_below = match orientation {
                    Orientation::Up => dy < 0,
                    Orientation::Down => dy > 0,
                    Orientation::Left => dx < 0,
                    Orientation::Right => dx > 0,
                };
                is_from_below
            }
        }
    }

    /// 根據距離計算光照等級
    /// 返回 Darkness 表示無光或熄滅
    pub fn light_level_at(&self, distance: usize) -> LightLevel {
        match &self.object_type {
            ObjectType::Torch { lit: true } => {
                if TORCH_BRIGHT_RANGE > 0 && distance <= TORCH_BRIGHT_RANGE {
                    LightLevel::Bright
                } else if TORCH_DIM_RANGE > 0 && distance <= TORCH_DIM_RANGE {
                    LightLevel::Dim
                } else {
                    LightLevel::Darkness
                }
            }
            ObjectType::Campfire { lit: true } => {
                if CAMPFIRE_BRIGHT_RANGE > 0 && distance <= CAMPFIRE_BRIGHT_RANGE {
                    LightLevel::Bright
                } else if CAMPFIRE_DIM_RANGE > 0 && distance <= CAMPFIRE_DIM_RANGE {
                    LightLevel::Dim
                } else {
                    LightLevel::Darkness
                }
            }
            ObjectType::Tree
            | ObjectType::Wall
            | ObjectType::Cliff { .. }
            | ObjectType::Pit
            | ObjectType::Tent2 { .. }
            | ObjectType::Tent15 { .. }
            | ObjectType::Torch { lit: false }
            | ObjectType::Campfire { lit: false } => LightLevel::Darkness,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Tile {
    pub terrain: Terrain,
}

// config 欄位需要排序
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct BoardConfig {
    pub tiles: Vec<Vec<Tile>>,
    pub teams: BTreeMap<TeamID, Team>,
    #[serde(default)]
    pub ambient_light: LightLevel,
    // 以上會同步到 Board
    pub deployable: BTreeSet<Pos>,
    #[serde(with = "unitid_key_map")]
    pub units: BTreeMap<UnitID, UnitMarker>,
    #[serde(default)]
    pub objects: BTreeMap<ObjectID, Object>,
}

#[derive(Debug, Default)]
pub struct Board {
    pub tiles: Vec<Vec<Tile>>,
    pub teams: HashMap<TeamID, Team>,
    pub ambient_light: LightLevel,
    pub units: HashMap<UnitID, Unit>,
    pub unit_map: UnitMap,
    pub object_map: ObjectMap,
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

        // 物體系統初始化
        let mut object_map = ObjectMap::default();
        for (_, obj) in config.objects {
            object_map.insert(obj);
        }

        Ok(Board {
            tiles: config.tiles,
            teams,
            ambient_light: config.ambient_light,
            units,
            unit_map,
            object_map,
        })
    }

    pub fn pos_to_unit(&self, pos: Pos) -> Option<UnitID> {
        self.unit_map.get_unit(pos)
    }

    pub fn unit_to_pos(&self, unit_id: UnitID) -> Option<Pos> {
        self.unit_map.get_pos(unit_id)
    }

    /// 檢查位置的地形和物件是否可通行
    pub fn is_tile_passable(&self, pos: Pos) -> bool {
        self.get_tile(pos).is_some()
            && self
                .object_map
                .get_objects_at(pos)
                .iter()
                .all(|obj| obj.is_passable())
    }

    /// 檢查觀察者是否能看到目標位置
    ///
    /// 結合物理視線、光照和 Sense 能力的完整可見性判斷
    ///
    /// 參數：
    /// - from: 觀察者位置
    /// - to: 目標位置
    /// - observer_unit_id: 觀察者單位 ID
    /// - skills: 技能定義表
    ///
    /// 返回：是否可見
    ///
    /// 檢查流程：
    /// 1. 檢查物理視線（透過 has_line_of_sight）
    /// 2. 檢查目標位置光照等級
    /// 3. 若目標處於黑暗，檢查觀察者是否有足夠距離的 Sense 能力
    pub fn can_see_target(
        &self,
        (observer_unit_id, from): (UnitID, Pos),
        to: Pos,
        skills: &BTreeMap<SkillID, Skill>,
    ) -> Result<bool, Error> {
        let func = "can_see_target";

        // 1. 檢查物理視線
        if !has_line_of_sight(self, from, to)? {
            return Ok(false);
        }

        // 2. 檢查目標位置光照
        let light_level = self.get_light_level(to, skills)?;

        // 3. 如果是黑暗，檢查觀察者是否有 Sense 能力
        if light_level == LightLevel::Darkness {
            let observer =
                self.units
                    .get(&observer_unit_id)
                    .ok_or_else(|| Error::NoActingUnit {
                        func,
                        unit_id: observer_unit_id,
                    })?;

            let distance = manhattan_distance(from, to);
            let has_sense = skills_to_sense(observer.skills.iter(), skills, distance)?;

            if !has_sense {
                return Ok(false); // 黑暗中沒有 Sense 能力
            }
        }

        Ok(true)
    }

    pub fn get_light_level(
        &self,
        pos: Pos,
        _skills: &BTreeMap<SkillID, Skill>,
    ) -> Result<LightLevel, Error> {
        let mut max_light = self.ambient_light;

        // 早期退出：如果環境光已經是最亮，直接返回
        if max_light == LightLevel::Bright {
            return Ok(LightLevel::Bright);
        }

        // 檢查所有 Object 光源
        for obj in self.object_map.values() {
            // 先檢查是否是光源類型
            if !matches!(
                obj.object_type,
                ObjectType::Torch { .. } | ObjectType::Campfire { .. }
            ) {
                continue;
            }

            // 對於光源物體的每個位置
            for &source_pos in &obj.affected_positions {
                let distance = manhattan_distance(pos, source_pos);
                let light_level = obj.light_level_at(distance);

                max_light = max_light.max(light_level);
                if max_light == LightLevel::Bright {
                    return Ok(LightLevel::Bright);
                }
            }
        }

        // 檢查所有單位的 CarriesLight 效果
        for (unit_id, unit) in &self.units {
            let unit_pos = match self.unit_to_pos(*unit_id) {
                None => continue,
                Some(p) => p,
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

#[derive(Debug, Default)]
pub struct ObjectMap {
    objects: HashMap<ObjectID, Object>,
    pos_to_object: HashMap<Pos, Vec<ObjectID>>,
}

impl ObjectMap {
    /// 插入物件並同步位置索引
    pub fn insert(&mut self, object: Object) {
        let object_id = object.id;
        for &pos in &object.affected_positions {
            self.pos_to_object.entry(pos).or_default().push(object_id);
        }
        self.objects.insert(object_id, object);
    }

    /// 移除物件並清理位置索引
    /// 返回被移除的物件，如果物件不存在則返回 None
    pub fn remove(&mut self, object_id: ObjectID) -> Option<Object> {
        let object = self.objects.remove(&object_id)?;

        for pos in &object.affected_positions {
            if let Some(ids) = self.pos_to_object.get_mut(pos) {
                ids.retain(|&id| id != object_id);
                if ids.is_empty() {
                    self.pos_to_object.remove(pos);
                }
            }
        }

        Some(object)
    }

    /// 取得物件參照
    pub fn get(&self, object_id: ObjectID) -> Option<&Object> {
        self.objects.get(&object_id)
    }

    /// 減少物件的 duration（永久物件 -1 不減少）
    pub fn decrease_object_duration(&mut self, object_id: ObjectID) {
        if let Some(object) = self.objects.get_mut(&object_id) {
            if object.duration > 0 {
                object.duration -= 1;
            }
        }
    }

    /// 取得位置上的所有物件
    /// 因為封裝保證同步，pos_to_object 中的 ID 一定存在於 objects
    pub fn get_objects_at(&self, pos: Pos) -> Vec<&Object> {
        match self.pos_to_object.get(&pos) {
            Some(ids) => ids.iter().map(|id| &self.objects[id]).collect(),
            None => Vec::new(),
        }
    }

    /// 遍歷所有物件
    pub fn values(&self) -> impl Iterator<Item = &Object> {
        self.objects.values()
    }

    /// 遍歷所有物件（ID, Object）
    pub fn iter(&self) -> impl Iterator<Item = (&ObjectID, &Object)> {
        self.objects.iter()
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

use inner::*;
mod inner {
    use super::*;

    /// 檢查從 from 到 to 是否有視線（line of sight）
    ///
    /// 純粹檢查物理視線，考慮因素：
    /// 1. 障礙物阻擋視線（使用 Bresenham 追蹤路徑）
    /// 2. Cliff 的方向性視線（高地優勢）
    /// 3. 一律使用 blocks_sight_from 進行方向性判斷
    ///
    /// 注意：
    /// - 此函數只檢查「物理視線」，不考慮單位阻擋
    /// - 不考慮光照影響（黑暗狀態由上層邏輯處理）
    pub fn has_line_of_sight(board: &Board, from: Pos, to: Pos) -> Result<bool, Error> {
        let func = "has_line_of_sight";

        // 同一位置永遠可見
        if from == to {
            return Ok(true);
        }

        // 檢查起點和終點是否在棋盤範圍內
        if board.get_tile(from).is_none() {
            return Err(Error::NoTileAtPos { func, pos: from });
        }
        if board.get_tile(to).is_none() {
            return Err(Error::NoTileAtPos { func, pos: to });
        }

        // 計算距離用於 Bresenham 路徑長度
        let distance = manhattan_distance(from, to);

        // 使用 Bresenham 算法追蹤視線路徑
        let path = bresenham_line(from, to, distance + 1, |pos| {
            pos.x < board.width() && pos.y < board.height()
        });

        // 檢查路徑上的每個格子（不包括起點，包括終點）
        for (i, &pos) in path.iter().enumerate() {
            // 跳過起點
            if i == 0 {
                continue;
            }

            // 到達目標位置，停止檢查（目標本身總是可見）
            if pos == to {
                break;
            }

            // 檢查中間路徑是否有物件阻擋視線（一律使用 blocks_sight_from）
            for obj in board.object_map.get_objects_at(pos) {
                if obj.blocks_sight_from(from, pos) {
                    return Ok(false);
                }
            }
        }

        // 視線暢通或已到達目標
        Ok(true)
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
            Terrain::ShallowWater
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
        let torch_lit = Object {
            id: 1,
            affected_positions: vec![],
            object_type: ObjectType::Torch { lit: true },
            duration: -1,
            creator_team: TEAM_NONE.to_string(),
        };

        // 距離 <= 1 (TORCH_BRIGHT_RANGE)：Bright
        assert_eq!(torch_lit.light_level_at(0), LightLevel::Bright);
        assert_eq!(torch_lit.light_level_at(1), LightLevel::Bright);

        // 距離 2-3 (TORCH_DIM_RANGE)：Dim
        assert_eq!(torch_lit.light_level_at(2), LightLevel::Dim);
        assert_eq!(torch_lit.light_level_at(3), LightLevel::Dim);

        // 距離 > 3：Darkness
        assert_eq!(torch_lit.light_level_at(4), LightLevel::Darkness);
        assert_eq!(torch_lit.light_level_at(10), LightLevel::Darkness);

        // 熄滅的火把：所有距離都是 Darkness
        let torch_unlit = Object {
            id: 2,
            affected_positions: vec![],
            object_type: ObjectType::Torch { lit: false },
            duration: -1,
            creator_team: TEAM_NONE.to_string(),
        };
        assert_eq!(torch_unlit.light_level_at(0), LightLevel::Darkness);
        assert_eq!(torch_unlit.light_level_at(3), LightLevel::Darkness);
    }

    #[test]
    fn test_object_light_level_at_campfire() {
        let campfire_lit = Object {
            id: 1,
            affected_positions: vec![],
            object_type: ObjectType::Campfire { lit: true },
            duration: -1,
            creator_team: TEAM_NONE.to_string(),
        };

        // 距離 <= 6 (CAMPFIRE_BRIGHT_RANGE)：Bright
        assert_eq!(campfire_lit.light_level_at(0), LightLevel::Bright);
        assert_eq!(campfire_lit.light_level_at(6), LightLevel::Bright);

        // 距離 7-12 (CAMPFIRE_DIM_RANGE)：Dim
        assert_eq!(campfire_lit.light_level_at(7), LightLevel::Dim);
        assert_eq!(campfire_lit.light_level_at(12), LightLevel::Dim);

        // 距離 > 12：Darkness
        assert_eq!(campfire_lit.light_level_at(13), LightLevel::Darkness);

        // 熄滅的營火：所有距離都是 Darkness
        let campfire_unlit = Object {
            id: 2,
            affected_positions: vec![],
            object_type: ObjectType::Campfire { lit: false },
            duration: -1,
            creator_team: TEAM_NONE.to_string(),
        };
        assert_eq!(campfire_unlit.light_level_at(0), LightLevel::Darkness);
        assert_eq!(campfire_unlit.light_level_at(6), LightLevel::Darkness);
    }

    #[test]
    fn test_object_light_level_at_non_light_objects() {
        // 其他物件不發光：所有距離都是 Darkness
        let tree = Object {
            id: 1,
            affected_positions: vec![],
            object_type: ObjectType::Tree,
            duration: -1,
            creator_team: TEAM_NONE.to_string(),
        };
        assert_eq!(tree.light_level_at(0), LightLevel::Darkness);

        let wall = Object {
            id: 2,
            affected_positions: vec![],
            object_type: ObjectType::Wall,
            duration: -1,
            creator_team: TEAM_NONE.to_string(),
        };
        assert_eq!(wall.light_level_at(0), LightLevel::Darkness);

        let pit = Object {
            id: 3,
            affected_positions: vec![],
            object_type: ObjectType::Pit,
            duration: -1,
            creator_team: TEAM_NONE.to_string(),
        };
        assert_eq!(pit.light_level_at(0), LightLevel::Darkness);

        let cliff = Object {
            id: 4,
            affected_positions: vec![],
            object_type: ObjectType::Cliff {
                orientation: Orientation::Up,
            },
            duration: -1,
            creator_team: TEAM_NONE.to_string(),
        };
        assert_eq!(cliff.light_level_at(0), LightLevel::Darkness);
    }

    #[test]
    fn test_object_is_passable_light_sources() {
        // Torch（火把）可通行，Campfire（營火）不可通行
        let torch_lit = Object {
            id: 1,
            affected_positions: vec![],
            object_type: ObjectType::Torch { lit: true },
            duration: -1,
            creator_team: TEAM_NONE.to_string(),
        };
        assert!(torch_lit.is_passable());

        let torch_unlit = Object {
            id: 2,
            affected_positions: vec![],
            object_type: ObjectType::Torch { lit: false },
            duration: -1,
            creator_team: TEAM_NONE.to_string(),
        };
        assert!(torch_unlit.is_passable());

        let campfire_lit = Object {
            id: 3,
            affected_positions: vec![],
            object_type: ObjectType::Campfire { lit: true },
            duration: -1,
            creator_team: TEAM_NONE.to_string(),
        };
        assert!(!campfire_lit.is_passable());

        let campfire_unlit = Object {
            id: 4,
            affected_positions: vec![],
            object_type: ObjectType::Campfire { lit: false },
            duration: -1,
            creator_team: TEAM_NONE.to_string(),
        };
        assert!(!campfire_unlit.is_passable());
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
            objects: BTreeMap::new(),
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
        let tiles = vec![vec![Tile::default(); 5]; 5];

        let mut objects = BTreeMap::new();
        objects.insert(
            1,
            Object {
                id: 1,
                affected_positions: vec![Pos { x: 2, y: 2 }],
                object_type: ObjectType::Torch { lit: true },
                duration: -1,
                creator_team: TEAM_NONE.to_string(),
            },
        );

        let config = BoardConfig {
            tiles,
            teams: BTreeMap::new(),
            ambient_light: LightLevel::Darkness,
            deployable: BTreeSet::new(),
            units: BTreeMap::new(),
            objects,
        };

        let skills = BTreeMap::new();
        struct EmptyGetter;
        impl UnitTemplateGetter for EmptyGetter {
            fn get(&self, _typ: &UnitTemplateType) -> Option<&UnitTemplate> {
                None
            }
        }

        let board = Board::from_config(config, &EmptyGetter, &skills).unwrap();

        // Torch 在 (2, 2)，TORCH_BRIGHT_RANGE = 1
        // 距離 0：同位置
        assert_eq!(
            board.get_light_level(Pos { x: 2, y: 2 }, &skills),
            Ok(LightLevel::Bright)
        );

        // 距離 1（邊界）
        assert_eq!(
            board.get_light_level(Pos { x: 3, y: 2 }, &skills),
            Ok(LightLevel::Bright)
        );
        assert_eq!(
            board.get_light_level(Pos { x: 2, y: 3 }, &skills),
            Ok(LightLevel::Bright)
        );
    }

    #[test]
    fn test_get_light_level_torch_dim() {
        // Torch 2-3 格為 Dim
        let tiles = vec![vec![Tile::default(); 10]; 10];

        let mut objects = BTreeMap::new();
        objects.insert(
            1,
            Object {
                id: 1,
                affected_positions: vec![Pos { x: 5, y: 5 }],
                object_type: ObjectType::Torch { lit: true },
                duration: -1,
                creator_team: TEAM_NONE.to_string(),
            },
        );

        let config = BoardConfig {
            tiles,
            teams: BTreeMap::new(),
            ambient_light: LightLevel::Darkness,
            deployable: BTreeSet::new(),
            units: BTreeMap::new(),
            objects,
        };

        let skills = BTreeMap::new();
        struct EmptyGetter;
        impl UnitTemplateGetter for EmptyGetter {
            fn get(&self, _typ: &UnitTemplateType) -> Option<&UnitTemplate> {
                None
            }
        }

        let board = Board::from_config(config, &EmptyGetter, &skills).unwrap();

        // Torch 在 (5, 5)，TORCH_DIM_RANGE = 3
        // 距離 2（Dim）
        assert_eq!(
            board.get_light_level(Pos { x: 5, y: 7 }, &skills),
            Ok(LightLevel::Dim)
        );

        // 距離 3（邊界，Dim）
        assert_eq!(
            board.get_light_level(Pos { x: 5, y: 8 }, &skills),
            Ok(LightLevel::Dim)
        );

        // 距離 4（超出範圍，回到環境光 Darkness）
        assert_eq!(
            board.get_light_level(Pos { x: 0, y: 0 }, &skills),
            Ok(LightLevel::Darkness)
        );
    }

    #[test]
    fn test_get_light_level_multiple_sources() {
        // 多光源重疊取最亮
        let tiles = vec![vec![Tile::default(); 10]; 10];

        let mut objects = BTreeMap::new();
        objects.insert(
            1,
            Object {
                id: 1,
                affected_positions: vec![Pos { x: 2, y: 2 }],
                object_type: ObjectType::Torch { lit: true },
                duration: -1,
                creator_team: TEAM_NONE.to_string(),
            },
        );
        objects.insert(
            2,
            Object {
                id: 2,
                affected_positions: vec![Pos { x: 5, y: 5 }],
                object_type: ObjectType::Campfire { lit: true },
                duration: -1,
                creator_team: TEAM_NONE.to_string(),
            },
        );

        let config = BoardConfig {
            tiles,
            teams: BTreeMap::new(),
            ambient_light: LightLevel::Darkness,
            deployable: BTreeSet::new(),
            units: BTreeMap::new(),
            objects,
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
            objects: BTreeMap::new(),
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

    #[test]
    fn test_blocks_sight_from_non_directional() {
        // 測試無方向性的物件（任何方向都阻擋或不阻擋）
        let observer = Pos { x: 3, y: 3 };
        let obj_pos = Pos { x: 5, y: 5 };

        // 完全阻擋視線的物件
        let wall = Object {
            id: 1,
            affected_positions: vec![],
            object_type: ObjectType::Wall,
            duration: -1,
            creator_team: TEAM_NONE.to_string(),
        };
        assert!(wall.blocks_sight_from(observer, obj_pos));

        let tree = Object {
            id: 2,
            affected_positions: vec![],
            object_type: ObjectType::Tree,
            duration: -1,
            creator_team: TEAM_NONE.to_string(),
        };
        assert!(tree.blocks_sight_from(observer, obj_pos));

        let tent2 = Object {
            id: 3,
            affected_positions: vec![],
            object_type: ObjectType::Tent2 {
                orientation: Orientation::Up,
            },
            duration: -1,
            creator_team: TEAM_NONE.to_string(),
        };
        assert!(tent2.blocks_sight_from(observer, obj_pos));

        let tent15 = Object {
            id: 4,
            affected_positions: vec![],
            object_type: ObjectType::Tent15 {
                orientation: Orientation::Up,
            },
            duration: -1,
            creator_team: TEAM_NONE.to_string(),
        };
        assert!(tent15.blocks_sight_from(observer, obj_pos));

        // 不阻擋視線的物件
        let pit = Object {
            id: 5,
            affected_positions: vec![],
            object_type: ObjectType::Pit,
            duration: -1,
            creator_team: TEAM_NONE.to_string(),
        };
        assert!(!pit.blocks_sight_from(observer, obj_pos));

        let torch = Object {
            id: 6,
            affected_positions: vec![],
            object_type: ObjectType::Torch { lit: true },
            duration: -1,
            creator_team: TEAM_NONE.to_string(),
        };
        assert!(!torch.blocks_sight_from(observer, obj_pos));

        let campfire = Object {
            id: 7,
            affected_positions: vec![],
            object_type: ObjectType::Campfire { lit: true },
            duration: -1,
            creator_team: TEAM_NONE.to_string(),
        };
        assert!(!campfire.blocks_sight_from(observer, obj_pos));
    }

    #[test]
    fn test_cliff_blocks_sight_from_direction() {
        // Cliff 朝上（orientation: Up），表示懸崖下方在上方（北側），高地在下方（南側）
        let cliff_up = Object {
            id: 1,
            affected_positions: vec![],
            object_type: ObjectType::Cliff {
                orientation: Orientation::Up,
            },
            duration: -1,
            creator_team: TEAM_NONE.to_string(),
        };
        let cliff_pos = Pos { x: 5, y: 5 };

        // 從下方（y > cliff_pos.y，南側，高地）往上看：不被阻擋（從高地往低地看）
        assert!(!cliff_up.blocks_sight_from(Pos { x: 5, y: 6 }, cliff_pos));
        assert!(!cliff_up.blocks_sight_from(Pos { x: 5, y: 10 }, cliff_pos));

        // 從上方（y < cliff_pos.y，北側，低地）往下看：被阻擋（從低地往高地看）
        assert!(cliff_up.blocks_sight_from(Pos { x: 5, y: 4 }, cliff_pos));
        assert!(cliff_up.blocks_sight_from(Pos { x: 5, y: 0 }, cliff_pos));

        // 從側面（x 不同，y 相同）：不阻擋
        assert!(!cliff_up.blocks_sight_from(Pos { x: 3, y: 5 }, cliff_pos));
        assert!(!cliff_up.blocks_sight_from(Pos { x: 7, y: 5 }, cliff_pos));

        // Cliff 朝下（orientation: Down），表示懸崖下方在下方（南側），高地在上方（北側）
        let cliff_down = Object {
            id: 2,
            affected_positions: vec![],
            object_type: ObjectType::Cliff {
                orientation: Orientation::Down,
            },
            duration: -1,
            creator_team: TEAM_NONE.to_string(),
        };

        // 從上方（北側，高地）往下看：不被阻擋（從高地往低地看）
        assert!(!cliff_down.blocks_sight_from(Pos { x: 5, y: 3 }, cliff_pos));

        // 從下方（南側，低地）往上看：被阻擋（從低地往高地看）
        assert!(cliff_down.blocks_sight_from(Pos { x: 5, y: 7 }, cliff_pos));

        // Cliff 朝左（orientation: Left），表示懸崖下方在左方（西側），高地在右方（東側）
        let cliff_left = Object {
            id: 3,
            affected_positions: vec![],
            object_type: ObjectType::Cliff {
                orientation: Orientation::Left,
            },
            duration: -1,
            creator_team: TEAM_NONE.to_string(),
        };

        // 從右方（東側，高地）往左看：不被阻擋（從高地往低地看）
        assert!(!cliff_left.blocks_sight_from(Pos { x: 6, y: 5 }, cliff_pos));

        // 從左方（西側，低地）往右看：被阻擋（從低地往高地看）
        assert!(cliff_left.blocks_sight_from(Pos { x: 4, y: 5 }, cliff_pos));

        // Cliff 朝右（orientation: Right），表示懸崖下方在右方（東側），高地在左方（西側）
        let cliff_right = Object {
            id: 4,
            affected_positions: vec![],
            object_type: ObjectType::Cliff {
                orientation: Orientation::Right,
            },
            duration: -1,
            creator_team: TEAM_NONE.to_string(),
        };

        // 從左方（西側，高地）往右看：不被阻擋（從高地往低地看）
        assert!(!cliff_right.blocks_sight_from(Pos { x: 3, y: 5 }, cliff_pos));

        // 從右方（東側，低地）往左看：被阻擋（從低地往高地看）
        assert!(cliff_right.blocks_sight_from(Pos { x: 7, y: 5 }, cliff_pos));
    }

    #[test]
    fn test_has_line_of_sight_basic() {
        // 建立簡單棋盤
        let tiles = vec![vec![Tile::default(); 10]; 10];

        // 添加牆壁障礙物
        let mut objects = BTreeMap::new();
        objects.insert(
            1,
            Object {
                id: 1,
                affected_positions: vec![Pos { x: 5, y: 5 }],
                object_type: ObjectType::Wall,
                duration: -1,
                creator_team: TEAM_NONE.to_string(),
            },
        );

        let config = BoardConfig {
            tiles,
            teams: BTreeMap::new(),
            ambient_light: LightLevel::Bright,
            deployable: BTreeSet::new(),
            units: BTreeMap::new(),
            objects,
        };

        let skills = BTreeMap::new();
        struct EmptyGetter;
        impl UnitTemplateGetter for EmptyGetter {
            fn get(&self, _typ: &UnitTemplateType) -> Option<&UnitTemplate> {
                None
            }
        }

        let board = Board::from_config(config, &EmptyGetter, &skills).unwrap();

        // 測試同一位置
        assert!(has_line_of_sight(&board, Pos { x: 0, y: 0 }, Pos { x: 0, y: 0 }).unwrap());

        // 測試無障礙物的視線
        assert!(has_line_of_sight(&board, Pos { x: 0, y: 0 }, Pos { x: 3, y: 3 }).unwrap());

        // 測試可以看到障礙物本身
        assert!(has_line_of_sight(&board, Pos { x: 3, y: 5 }, Pos { x: 5, y: 5 }).unwrap());

        // 測試被牆阻擋的視線
        assert!(!has_line_of_sight(&board, Pos { x: 3, y: 5 }, Pos { x: 7, y: 5 }).unwrap());
    }

    #[test]
    fn test_has_line_of_sight_cliff_high_ground() {
        // 建立棋盤，測試 Cliff 高地視線
        let tiles = vec![vec![Tile::default(); 10]; 10];

        // 在 (5, 5) 放置 Cliff，朝上（懸崖下方在北側，高地在南側）
        let mut objects = BTreeMap::new();
        objects.insert(
            1,
            Object {
                id: 1,
                affected_positions: vec![Pos { x: 5, y: 5 }],
                object_type: ObjectType::Cliff {
                    orientation: Orientation::Up,
                },
                duration: -1,
                creator_team: TEAM_NONE.to_string(),
            },
        );

        let config = BoardConfig {
            tiles,
            teams: BTreeMap::new(),
            ambient_light: LightLevel::Bright,
            deployable: BTreeSet::new(),
            units: BTreeMap::new(),
            objects,
        };

        let skills = BTreeMap::new();
        struct EmptyGetter;
        impl UnitTemplateGetter for EmptyGetter {
            fn get(&self, _typ: &UnitTemplateType) -> Option<&UnitTemplate> {
                None
            }
        }

        let board = Board::from_config(config, &EmptyGetter, &skills).unwrap();

        // 從南方（y=7，高地）往北看：可以看到懸崖後方（從高地往低地看）
        assert!(has_line_of_sight(&board, Pos { x: 5, y: 7 }, Pos { x: 5, y: 3 }).unwrap());

        // 從北方（y=3，低地）往南看：被懸崖阻擋（從低地往高地看）
        assert!(!has_line_of_sight(&board, Pos { x: 5, y: 3 }, Pos { x: 5, y: 7 }).unwrap());

        // 從北方（低地）可以看到懸崖本身
        assert!(has_line_of_sight(&board, Pos { x: 5, y: 3 }, Pos { x: 5, y: 5 }).unwrap());
    }

    #[test]
    fn test_can_see_target_physical_blocked() {
        // 測試物理視線被阻擋
        let tiles = vec![vec![Tile::default(); 10]; 10];

        let mut objects = BTreeMap::new();
        objects.insert(
            1,
            Object {
                id: 1,
                affected_positions: vec![Pos { x: 5, y: 5 }],
                object_type: ObjectType::Wall, // 中間有牆
                duration: -1,
                creator_team: TEAM_NONE.to_string(),
            },
        );

        let mut units = BTreeMap::new();
        units.insert(
            1,
            UnitMarker {
                id: 1,
                unit_template_type: "observer".to_string(),
                team: "player".to_string(),
                pos: Pos { x: 3, y: 5 },
            },
        );

        let config = BoardConfig {
            tiles,
            teams: BTreeMap::new(),
            ambient_light: LightLevel::Bright,
            deployable: BTreeSet::new(),
            units,
            objects,
        };

        let skills = BTreeMap::new();
        let template = UnitTemplate {
            name: "observer".to_string(),
            skills: BTreeSet::new(),
        };
        struct TestGetter {
            template: UnitTemplate,
        }
        impl UnitTemplateGetter for TestGetter {
            fn get(&self, typ: &UnitTemplateType) -> Option<&UnitTemplate> {
                if typ == &self.template.name {
                    Some(&self.template)
                } else {
                    None
                }
            }
        }

        let board = Board::from_config(config, &TestGetter { template }, &skills).unwrap();

        // 物理視線被阻擋，應該看不到牆後方
        assert!(
            !board
                .can_see_target((1, Pos { x: 3, y: 5 }), Pos { x: 7, y: 5 }, &skills)
                .unwrap()
        );
    }

    #[test]
    fn test_can_see_target_bright_light() {
        // 測試明亮環境下可直接看到
        let tiles = vec![vec![Tile::default(); 10]; 10];

        let mut units = BTreeMap::new();
        units.insert(
            1,
            UnitMarker {
                id: 1,
                unit_template_type: "observer".to_string(),
                team: "player".to_string(),
                pos: Pos { x: 0, y: 0 },
            },
        );

        let config = BoardConfig {
            tiles,
            teams: BTreeMap::new(),
            ambient_light: LightLevel::Bright,
            deployable: BTreeSet::new(),
            units,
            objects: BTreeMap::new(),
        };

        let skills = BTreeMap::new();
        let template = UnitTemplate {
            name: "observer".to_string(),
            skills: BTreeSet::new(),
        };
        struct TestGetter {
            template: UnitTemplate,
        }
        impl UnitTemplateGetter for TestGetter {
            fn get(&self, typ: &UnitTemplateType) -> Option<&UnitTemplate> {
                if typ == &self.template.name {
                    Some(&self.template)
                } else {
                    None
                }
            }
        }

        let board = Board::from_config(config, &TestGetter { template }, &skills).unwrap();

        // 明亮環境下，物理視線暢通即可見
        assert!(
            board
                .can_see_target((1, Pos { x: 0, y: 0 }), Pos { x: 5, y: 5 }, &skills)
                .unwrap()
        );
    }

    #[test]
    fn test_can_see_target_darkness_no_sense() {
        // 測試黑暗中無 Sense 能力
        let tiles = vec![vec![Tile::default(); 10]; 10];

        let mut units = BTreeMap::new();
        units.insert(
            1,
            UnitMarker {
                id: 1,
                unit_template_type: "observer".to_string(),
                team: "player".to_string(),
                pos: Pos { x: 0, y: 0 },
            },
        );

        let config = BoardConfig {
            tiles,
            teams: BTreeMap::new(),
            ambient_light: LightLevel::Darkness, // 黑暗環境
            deployable: BTreeSet::new(),
            units,
            objects: BTreeMap::new(),
        };

        let skills = BTreeMap::new();
        let template = UnitTemplate {
            name: "observer".to_string(),
            skills: BTreeSet::new(), // 沒有 Sense 技能
        };
        struct TestGetter {
            template: UnitTemplate,
        }
        impl UnitTemplateGetter for TestGetter {
            fn get(&self, typ: &UnitTemplateType) -> Option<&UnitTemplate> {
                if typ == &self.template.name {
                    Some(&self.template)
                } else {
                    None
                }
            }
        }

        let board = Board::from_config(config, &TestGetter { template }, &skills).unwrap();

        // 黑暗中沒有 Sense 能力，看不到
        assert!(
            !board
                .can_see_target((1, Pos { x: 0, y: 0 }), Pos { x: 5, y: 5 }, &skills)
                .unwrap()
        );
    }

    #[test]
    fn test_can_see_target_darkness_with_sense() {
        // 測試黑暗中有 Sense 能力
        use skills_lib::{Effect, Shape, Skill, TargetType};

        let tiles = vec![vec![Tile::default(); 10]; 10];

        let mut units = BTreeMap::new();
        units.insert(
            1,
            UnitMarker {
                id: 1,
                unit_template_type: "observer".to_string(),
                team: "player".to_string(),
                pos: Pos { x: 0, y: 0 },
            },
        );

        let config = BoardConfig {
            tiles,
            teams: BTreeMap::new(),
            ambient_light: LightLevel::Darkness, // 黑暗環境
            deployable: BTreeSet::new(),
            units,
            objects: BTreeMap::new(),
        };

        let mut skills = BTreeMap::new();
        skills.insert(
            "sense_skill".to_string(),
            Skill {
                tags: BTreeSet::new(),
                range: (0, 0),
                cost: 0,
                accuracy: None,
                crit_rate: None,
                effects: vec![Effect::Sense {
                    target_type: TargetType::Caster,
                    shape: Shape::Point,
                    range: 10,
                    duration: -1, // 永久效果
                }],
            },
        );

        let mut observer_skills = BTreeSet::new();
        observer_skills.insert("sense_skill".to_string());

        let template = UnitTemplate {
            name: "observer".to_string(),
            skills: observer_skills,
        };

        struct TestGetter {
            template: UnitTemplate,
        }
        impl UnitTemplateGetter for TestGetter {
            fn get(&self, typ: &UnitTemplateType) -> Option<&UnitTemplate> {
                if typ == &self.template.name {
                    Some(&self.template)
                } else {
                    None
                }
            }
        }

        let board = Board::from_config(config, &TestGetter { template }, &skills).unwrap();

        // 黑暗中有 Sense 能力（range=10，距離 < 10），可以看到
        assert!(
            board
                .can_see_target((1, Pos { x: 0, y: 0 }), Pos { x: 5, y: 5 }, &skills)
                .unwrap()
        );

        // 超出 Sense 範圍（距離 > 10），看不到
        assert!(
            !board
                .can_see_target((1, Pos { x: 0, y: 0 }), Pos { x: 9, y: 9 }, &skills)
                .unwrap()
        );
    }
}

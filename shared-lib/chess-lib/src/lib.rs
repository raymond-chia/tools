use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use strum_macros::EnumIter;

/// 戰場
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Battlefield {
    #[serde(default)]
    pub id: String, // 戰場ID
    #[serde(default)]
    pub grid: Vec<Vec<Cell>>, // 二維網格
    #[serde(default)]
    pub teams: BTreeMap<String, Team>, // 隊伍列表（id為key）
    #[serde(default)]
    pub objectives: BattleObjectiveType, // 戰鬥目標
    #[serde(default)]
    pub deployable_positions: BTreeSet<Pos>, // 可部署的位置集合
    #[serde(default)]
    pub unit_id_to_team: BTreeMap<String, Unit>, // 單位ID到單位資訊的映射
}

/// 位置
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pos {
    pub x: usize,
    pub y: usize,
}

impl PartialOrd for Pos {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Pos {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.x, self.y).cmp(&(other.x, other.y))
    }
}

/// 地形類型
#[derive(Debug, Deserialize, Serialize, EnumIter, Clone, Copy, PartialEq, Eq)]
pub enum Terrain {
    Plain,        // 平原
    Hill,         // 丘陵
    Mountain,     // 山地
    Forest,       // 森林
    ShallowWater, // 淺水
    DeepWater,    // 深水
}

/// 戰場物件
#[derive(Debug, Deserialize, Serialize, EnumIter, Clone, Copy, PartialEq, Eq)]
pub enum BattlefieldObject {
    Wall, // 牆壁 (不可通行)
}

/// 單位資訊
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Unit {
    pub id: String,
    pub unit_type: String,
    pub team_id: String,
}

/// 格子
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Cell {
    pub terrain: Terrain,                  // 地形
    pub object: Option<BattlefieldObject>, // 格子上的物件
    pub unit_id: Option<String>,           // 格子上的單位ID（如果有）
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            terrain: Terrain::Plain,
            object: None,
            unit_id: None,
        }
    }
}

/// 隊伍
/// 顏色結構
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Eq, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct Team {
    pub id: String,   // 隊伍ID，如 "player", "enemy1" 等
    pub color: Color, // 隊伍顏色
}

// 以 id 排序
impl Ord for Team {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}
impl PartialOrd for Team {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// 戰鬥目標類型
#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum BattleObjectiveType {
    // 組合目標（需要完成所有子目標）
    Composite {
        objectives: BTreeMap<String, BattleObjectiveType>,
    },

    // 選擇性目標（完成其中一個即可）
    Alternative {
        objectives: BTreeMap<String, BattleObjectiveType>,
    },

    // 順序目標（按順序完成）
    Sequential {
        objectives: BTreeMap<String, BattleObjectiveType>,
    },

    // 消滅特定隊伍所有單位
    EliminateTeam {
        team_id: String,
    },

    // 殲滅指定目標單位
    EliminateUnit {
        unit_id: String,
    },

    // 存活特定回合數
    Survive {
        turns: usize,
    },

    // 佔領特定位置
    CapturePosition {
        positions: Vec<Pos>,
    },

    // 捕捉特定單位
    CaptureUnit {
        unit_id: String,
    },
}

impl Default for BattleObjectiveType {
    fn default() -> Self {
        BattleObjectiveType::Alternative {
            objectives: BTreeMap::new(),
        }
    }
}

impl Battlefield {
    /// 創建指定大小的戰場
    pub fn new(id: &str, width: usize, height: usize) -> Self {
        let grid: Vec<Vec<Cell>> = (0..height)
            .map(|_| (0..width).map(|_| Cell::default()).collect())
            .collect();

        Self {
            id: id.to_string(),
            grid,
            teams: {
                let mut map = BTreeMap::new();
                map.insert(
                    "player".to_string(),
                    Team {
                        id: "player".to_string(),
                        color: Color {
                            r: 0,
                            g: 100,
                            b: 255,
                        }, // 預設藍色
                    },
                );
                map
            },
            objectives: BattleObjectiveType::default(),
            deployable_positions: BTreeSet::new(),
            unit_id_to_team: BTreeMap::new(),
        }
    }

    /// 獲取戰場寬度
    pub fn width(&self) -> usize {
        self.grid.get(0).map_or(0, |row| row.len())
    }

    /// 獲取戰場高度
    pub fn height(&self) -> usize {
        self.grid.len()
    }

    /// 檢查位置是否在戰場範圍內
    pub fn is_valid_position(&self, pos: &Pos) -> bool {
        let Pos { x, y } = *pos;
        y < self.height() && x < self.width()
    }

    /// 檢查指定位置是否可部署
    pub fn is_deployable(&self, pos: &Pos) -> bool {
        self.deployable_positions.contains(pos)
    }

    /// 設置指定位置的地形
    pub fn set_terrain(&mut self, pos: &Pos, terrain: Terrain) -> bool {
        if !self.is_valid_position(pos) {
            return false;
        }
        self.grid[pos.y][pos.x].terrain = terrain;
        true
    }

    /// 設置指定位置的物件
    pub fn set_object(&mut self, pos: &Pos, object: Option<BattlefieldObject>) -> bool {
        if !self.is_valid_position(pos) {
            return false;
        }
        self.grid[pos.y][pos.x].object = object;
        true
    }

    /// 設置指定位置的單位與隊伍
    pub fn set_unit_and_team(&mut self, pos: &Pos, unit: Option<Unit>) -> bool {
        if !self.is_valid_position(pos) {
            return false;
        }
        self.grid[pos.y][pos.x].unit_id = unit.as_ref().map(|u| u.id.clone());
        match unit {
            Some(unit_info) => {
                self.unit_id_to_team.insert(unit_info.id.clone(), unit_info);
            }
            None => {
                if let Some(prev_uid) = &self.grid[pos.y][pos.x].unit_id {
                    self.unit_id_to_team.remove(prev_uid);
                }
            }
        }
        true
    }

    /// 獲取單位所屬隊伍
    pub fn get_unit_team(&self, unit_id: &str) -> Option<&str> {
        self.unit_id_to_team
            .get(unit_id)
            .map(|u| u.team_id.as_str())
    }
}

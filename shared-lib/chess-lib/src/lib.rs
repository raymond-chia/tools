use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap, HashSet};
use strum_macros::EnumIter;

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

/// 位置
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

/// 格子
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Cell {
    pub terrain: Terrain,                  // 地形
    pub object: Option<BattlefieldObject>, // 格子上的物件
    pub unit_type: Option<String>,         // 格子上的單位種類（如果有）
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            terrain: Terrain::Plain,
            object: None,
            unit_type: None,
        }
    }
}

/// 隊伍
#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct Team {
    pub id: String,          // 隊伍ID，如 "player", "enemy1" 等
    pub color: (u8, u8, u8), // 隊伍顏色 (RGB)
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
        objectives: HashMap<String, BattleObjectiveType>,
    },

    // 選擇性目標（完成其中一個即可）
    Alternative {
        objectives: HashMap<String, BattleObjectiveType>,
    },

    // 順序目標（按順序完成）
    Sequential {
        objectives: Vec<BattleObjectiveType>,
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
        positions: Vec<Position>,
    },

    // 捕捉特定單位
    CaptureUnit {
        unit_id: String,
    },
}

/// 戰場
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Battlefield {
    pub id: String,                              // 戰場ID
    pub grid: Vec<Vec<Cell>>,                    // 二維網格
    pub teams: BTreeSet<Team>,                   // 隊伍列表（排序集合）
    pub objectives: BattleObjectiveType,         // 戰鬥目標
    pub deployable_positions: HashSet<Position>, // 可部署的位置集合
    pub unit_team_map: HashMap<String, String>,  // 單位類型到隊伍ID的映射
}

impl Battlefield {
    /// 創建指定大小的戰場
    pub fn new(id: &str, width: usize, height: usize) -> Self {
        let mut grid = Vec::with_capacity(height);
        for _ in 0..height {
            let mut row = Vec::with_capacity(width);
            for _ in 0..width {
                row.push(Cell::default());
            }
            grid.push(row);
        }

        Self {
            id: id.to_string(),
            grid,
            teams: {
                let mut set = BTreeSet::new();
                set.insert(Team {
                    id: "player".to_string(),
                    color: (0, 100, 255), // 預設藍色
                });
                set
            },
            objectives: BattleObjectiveType::Alternative {
                objectives: HashMap::new(),
            },
            deployable_positions: HashSet::new(),
            unit_team_map: HashMap::new(),
        }
    }

    /// 獲取戰場寬度
    pub fn width(&self) -> usize {
        if self.grid.is_empty() {
            0
        } else {
            self.grid[0].len()
        }
    }

    /// 獲取戰場高度
    pub fn height(&self) -> usize {
        self.grid.len()
    }

    /// 檢查位置是否在戰場範圍內
    pub fn is_valid_position(&self, pos: &Position) -> bool {
        pos.y < self.height() && pos.x < self.width()
    }

    /// 獲取單位所屬隊伍
    pub fn get_unit_team(&self, unit_type: &str) -> Option<String> {
        self.unit_team_map.get(unit_type).cloned()
    }

    /// 設置單位所屬隊伍
    pub fn set_unit_team(&mut self, unit_type: &str, team_id: &str) {
        self.unit_team_map
            .insert(unit_type.to_string(), team_id.to_string());
    }

    /// 移除單位所屬隊伍關聯
    pub fn remove_unit_team(&mut self, unit_type: &str) {
        self.unit_team_map.remove(unit_type);
    }
}

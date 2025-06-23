use crate::PLAYER_TEAM;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};
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
    pub unit_id_to_unit: BTreeMap<String, UnitConfig>, // 單位ID到單位資訊的映射
}

/// 位置
#[derive(Debug, Deserialize, Serialize, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Pos {
    pub x: usize,
    pub y: usize,
}

impl Ord for Pos {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.x, self.y).cmp(&(other.x, other.y))
    }
}

impl PartialOrd for Pos {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
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
    Wall,                                     // 牆壁 (不可通行)
    Tent2 { durability: i32, rel_pos: Pos },  // 帳篷（2格，rel_pos 為 (0,0) 或 (1,0)/(0,1)）
    Tent15 { durability: i32, rel_pos: Pos }, // 帳篷
}

/// 單位資訊
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct UnitConfig {
    pub id: String,
    pub unit_type: String,
    pub team_id: String,
}

/// 戰場上的單位
#[derive(Debug, Clone)]
pub struct Unit {
    pub config: UnitConfig, // 單位設定資料
    pub move_points: u32,   // 單位移動力
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
                let mut teams = BTreeMap::new();
                teams.insert(
                    PLAYER_TEAM.to_string(),
                    Team {
                        id: PLAYER_TEAM.to_string(),
                        color: Color {
                            r: 0,
                            g: 100,
                            b: 255,
                        }, // 預設藍色
                    },
                );
                teams
            },
            objectives: BattleObjectiveType::default(),
            deployable_positions: BTreeSet::new(),
            unit_id_to_unit: BTreeMap::new(),
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
    pub fn is_valid_position(&self, pos: Pos) -> bool {
        let Pos { x, y } = pos;
        x < self.width() && y < self.height()
    }

    /// 檢查指定位置是否可部署
    pub fn is_deployable(&self, pos: Pos) -> bool {
        self.deployable_positions.contains(&pos)
    }

    pub fn is_passable(&self, active_team: &str, pos: Pos) -> bool {
        if !self.is_valid_position(pos) {
            return false;
        }
        // 不可通行的地形
        if self.movement_cost(pos) >= 99 {
            return false;
        }
        let cell = self.get_cell(pos);
        // 友方單位不阻擋
        // 其他單位阻擋
        if let Some(unit_id) = &cell.unit_id {
            let unit = self.unit_id_to_unit.get(unit_id).expect("unit not found");
            if unit.team_id != active_team {
                return false;
            }
        }
        // 不可通行的物件
        if let Some(obj) = &cell.object {
            match obj {
                BattlefieldObject::Wall
                | BattlefieldObject::Tent2 { .. }
                | BattlefieldObject::Tent15 { .. } => {
                    return false;
                }
            }
        }
        return true;
    }

    pub fn movement_cost(&self, pos: Pos) -> usize {
        match self.get_cell(pos).terrain {
            Terrain::Plain => 10,
            Terrain::Hill => 15,
            Terrain::Mountain => 20,
            Terrain::Forest => 15,
            Terrain::ShallowWater => 15,
            Terrain::DeepWater => 99,
        }
    }

    pub fn get_cell(&self, pos: Pos) -> &Cell {
        &self.grid[pos.y][pos.x]
    }

    pub fn get_cell_mut(&mut self, pos: Pos) -> &mut Cell {
        &mut self.grid[pos.y][pos.x]
    }

    /// 設置指定位置的地形
    pub fn set_terrain(&mut self, pos: Pos, terrain: Terrain) -> bool {
        if !self.is_valid_position(pos) {
            return false;
        }
        self.get_cell_mut(pos).terrain = terrain;
        true
    }

    /// 設置指定位置的物件
    pub fn set_object(&mut self, pos: Pos, object: Option<BattlefieldObject>) -> bool {
        if !self.is_valid_position(pos) {
            return false;
        }
        self.get_cell_mut(pos).object = object;
        true
    }

    /// 設置指定位置的單位與隊伍
    pub fn set_unit(&mut self, pos: Pos, unit: Option<UnitConfig>) -> bool {
        if !self.is_valid_position(pos) {
            return false;
        }
        if let Some(prev_uid) = self.get_cell(pos).unit_id.clone() {
            self.unit_id_to_unit.remove(&prev_uid);
        }
        match unit {
            Some(unit) => {
                self.get_cell_mut(pos).unit_id = Some(unit.id.clone());
                self.unit_id_to_unit.insert(unit.id.clone(), unit);
            }
            None => {
                self.get_cell_mut(pos).unit_id = None;
            }
        }
        true
    }
}

impl Battlefield {
    /// https://github.com/TheAlgorithms/Rust/blob/master/src/graph/breadth_first_search.rs
    ///
    /// 計算一階段與二階段移動範圍
    /// - start: 起點
    /// - move_range: 單次移動力
    /// - active_team: 行動單位隊伍
    /// 回傳 (一階段, 二階段)
    pub fn bfs_move_ranges(
        &self,
        start: Pos,
        active_team: &str,
        move_points: usize,
        moved_distance: usize,
    ) -> (HashSet<Pos>, HashSet<Pos>) {
        let mut visited = HashSet::new();
        let mut first = HashSet::new();
        let mut second = HashSet::new();
        let mut queue = VecDeque::new();

        visited.insert(start);
        queue.push_back((start, moved_distance));
        while let Some((pos, used_points)) = queue.pop_front() {
            let cell = self.get_cell(pos);
            let cost = self.movement_cost(pos);
            if start == pos {
                // 起點不算在內
            } else if used_points <= move_points {
                // 不能停在友方單位
                if cell.unit_id.is_none() {
                    first.insert(pos);
                }
            } else if used_points <= move_points * 2 {
                // 不能停在友方單位
                if cell.unit_id.is_none() {
                    second.insert(pos);
                }
            } else {
                continue;
            }
            // 四方向
            for (dx, dy) in &[(0, 1), (1, 0), (-1, 0), (0, -1)] {
                let nx = pos.x as isize + dx;
                let ny = pos.y as isize + dy;
                if nx < 0 || ny < 0 {
                    continue; // 越界檢查
                }
                let next_pos = Pos {
                    x: nx as usize,
                    y: ny as usize,
                };
                if !self.is_passable(active_team, next_pos) {
                    continue;
                }
                if visited.insert(next_pos) {
                    queue.push_back((next_pos, used_points + cost));
                }
            }
        }
        (first, second)
    }
}

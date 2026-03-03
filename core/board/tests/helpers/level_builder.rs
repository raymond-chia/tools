//! 測試輔助：LevelBuilder 與 load_from_ascii
//!
//! 提供用 ASCII art 視覺化定義關卡的工具，取代手寫 TOML 字串。

use board::ecs_types::components::Position;
use board::ecs_types::resources::Board;
use board::error::{LoadError, Result};
use board::loader_schema::{Faction, LevelType, ObjectPlacement, UnitPlacement};
use std::collections::HashMap;

// ============================================================================
// load_from_ascii
// ============================================================================

/// 從 ASCII 格式載入棋盤
///
/// ASCII 格式：每行用空格分隔的符號
/// - `.` = 有效位置
/// - 其他字符串（`S`、`E` 等）= 標記位置（也作為有效位置）
/// - 相同的標記會全部收集成 Vec
///
/// 返回：(棋盤, 所有位置, 標記映射)
///
/// 例如：
/// ```text
/// S . .
/// . . E
/// . . .
/// ```
pub fn load_from_ascii(ascii: &str) -> Result<(Board, HashMap<String, Vec<Position>>)> {
    let lines: Vec<&str> = ascii
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    if lines.is_empty() {
        return Err(LoadError::ParseError("棋盤為空".to_string()).into());
    }

    // 推導寬度（第一行的符號數）
    let width = lines[0]
        .split_whitespace()
        .count()
        .try_into()
        .map_err(|_| LoadError::ParseError("棋盤寬度過大".to_string()))?;

    let height = lines
        .len()
        .try_into()
        .map_err(|_| LoadError::ParseError("棋盤高度過大".to_string()))?;

    let board = Board { width, height };

    let mut markers: HashMap<String, Vec<Position>> = HashMap::new();

    for (y, line) in lines.iter().enumerate() {
        for (x, cell) in line.split_whitespace().enumerate() {
            let pos = Position { x, y };

            // 非 `.` 的符號記為標記
            if cell != "." {
                markers
                    .entry(cell.to_string())
                    .or_insert_with(Vec::new)
                    .push(pos);
            }
        }
    }

    Ok((board, markers))
}

// ============================================================================
// LevelBuilder 輔助型別
// ============================================================================

struct UnitMarkerDef {
    marker: String,
    type_name: String,
    faction_id: u32,
}

struct ObjectMarkerDef {
    marker: String,
    type_name: String,
}

// ============================================================================
// LevelBuilder
// ============================================================================

/// 用 ASCII art 建立關卡 TOML 字串
///
/// # 使用範例
///
/// ```
/// let level_toml = LevelBuilder::from_ascii("
///   D . . . .
///   . . . . .
///   . . . . .
///   . . . . .
///   . . . . W
/// ")
/// .unit("W", "warrior", 1)
/// .deploy("D")
/// .to_toml();
/// ```
pub struct LevelBuilder {
    ascii: String,
    name: String,
    max_player_units: Option<usize>,
    factions: Vec<Faction>,
    unit_markers: Vec<UnitMarkerDef>,
    object_markers: Vec<ObjectMarkerDef>,
    deploy_marker: Option<String>,
}

impl LevelBuilder {
    /// 以 ASCII art 初始化 builder
    pub fn from_ascii(ascii: &str) -> Self {
        LevelBuilder {
            ascii: ascii.to_string(),
            name: "test-level".to_string(),
            max_player_units: None,
            factions: vec![
                Faction {
                    id: 0,
                    name: "player".to_string(),
                    alliance: 0,
                    color: [0, 0, 255],
                },
                Faction {
                    id: 1,
                    name: "enemy".to_string(),
                    alliance: 1,
                    color: [255, 0, 0],
                },
            ],
            unit_markers: Vec::new(),
            object_markers: Vec::new(),
            deploy_marker: None,
        }
    }

    /// 手動設定玩家單位上限
    pub fn max_player_units(mut self, n: usize) -> Self {
        self.max_player_units = Some(n);
        self
    }

    /// 設定標記為部署點
    pub fn deploy(mut self, marker: &str) -> Self {
        self.deploy_marker = Some(marker.to_string());
        self
    }

    /// 設定標記對應的單位類型與陣營
    pub fn unit(mut self, marker: &str, type_name: &str, faction_id: u32) -> Self {
        self.unit_markers.push(UnitMarkerDef {
            marker: marker.to_string(),
            type_name: type_name.to_string(),
            faction_id,
        });
        self
    }

    /// 設定標記對應的物件類型
    pub fn object(mut self, marker: &str, type_name: &str) -> Self {
        self.object_markers.push(ObjectMarkerDef {
            marker: marker.to_string(),
            type_name: type_name.to_string(),
        });
        self
    }

    /// 組裝完整 TOML 字串
    pub fn to_toml(self) -> Result<String> {
        let (board, markers) = load_from_ascii(&self.ascii)?;

        // 計算 max_player_units：若未設定，等於部署標記的位置數
        let deploy_count: usize = self
            .deploy_marker
            .as_ref()
            .and_then(|marker| markers.get(marker))
            .map(|positions| positions.len())
            .unwrap_or(0);
        let max_player_units = self.max_player_units.unwrap_or(deploy_count);

        // 構建部署位置
        let deployment_positions: Vec<Position> = self
            .deploy_marker
            .as_ref()
            .and_then(|marker| markers.get(marker))
            .map(|positions| positions.clone())
            .unwrap_or_default();

        // 構建單位配置
        let unit_placements: Vec<UnitPlacement> = self
            .unit_markers
            .iter()
            .flat_map(|unit_def| {
                markers
                    .get(&unit_def.marker)
                    .map(|positions| {
                        positions
                            .iter()
                            .map(|pos| UnitPlacement {
                                unit_type_name: unit_def.type_name.clone(),
                                faction_id: unit_def.faction_id,
                                position: *pos,
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            })
            .collect();

        // 構建物件配置
        let object_placements: Vec<ObjectPlacement> = self
            .object_markers
            .iter()
            .flat_map(|object_def| {
                markers
                    .get(&object_def.marker)
                    .map(|positions| {
                        positions
                            .iter()
                            .map(|pos| ObjectPlacement {
                                object_type_name: object_def.type_name.clone(),
                                position: *pos,
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            })
            .collect();

        let level = LevelType {
            name: self.name,
            board_width: board.width,
            board_height: board.height,
            max_player_units,
            factions: self.factions,
            deployment_positions,
            unit_placements,
            object_placements,
        };

        toml::to_string_pretty(&level).map_err(|e| {
            LoadError::SerializeError {
                format: "level".to_string(),
                reason: e.to_string(),
            }
            .into()
        })
    }
}

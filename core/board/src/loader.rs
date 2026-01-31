//! 棋盤載入器

use crate::alias::Coord;
use crate::component::{Board, Position};
use crate::error::{LoadError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
pub fn load_from_ascii(
    ascii: &str,
) -> Result<(Board, Vec<Position>, HashMap<String, Vec<Position>>)> {
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

    let mut positions = Vec::new();
    let mut markers: HashMap<String, Vec<Position>> = HashMap::new();

    for (y, line) in lines.iter().enumerate() {
        for (x, cell) in line.split_whitespace().enumerate() {
            let pos = Position { x, y };
            positions.push(pos);

            // 非 `.` 的符號記為標記
            if cell != "." {
                markers
                    .entry(cell.to_string())
                    .or_insert_with(Vec::new)
                    .push(pos);
            }
        }
    }

    Ok((board, positions, markers))
}

/// 勝利條件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum VictoryCondition {
    EliminateAllEnemies,
    SurviveRounds { rounds: u32 },
}

/// 失敗條件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DefeatCondition {
    AllPlayerUnitsDead,
}

/// 關卡資訊
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelInfo {
    pub name: String,
    pub width: Coord,
    pub height: Coord,
}

/// 地圖物件資料
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectData {
    pub x: Coord,
    pub y: Coord,
    pub object_type: String,
    pub blocks_movement: bool,
    pub blocks_sight: bool,
    pub blocks_sound: bool,
    pub instant_death: bool,
}

/// 單位資料
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitData {
    pub x: Coord,
    pub y: Coord,
    pub faction_id: u32,
}

/// 完整關卡資料
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelData {
    pub level: LevelInfo,
    pub victory_conditions: Vec<VictoryCondition>,
    pub defeat_conditions: Vec<DefeatCondition>,
    pub objects: Vec<ObjectData>,
    pub units: Vec<UnitData>,
}

/// 從 TOML 字符串載入關卡
pub fn load_level(toml_content: &str) -> Result<LevelData> {
    toml::from_str(toml_content).map_err(|e| {
        LoadError::DeserializeError {
            format: "toml".to_string(),
            reason: e.to_string(),
        }
        .into()
    })
}

/// 將關卡序列化為 TOML 字符串
pub fn save_level(level: &LevelData) -> Result<String> {
    toml::to_string_pretty(level).map_err(|e| {
        LoadError::SerializeError {
            format: "toml".to_string(),
            reason: e.to_string(),
        }
        .into()
    })
}

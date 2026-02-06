//! 棋盤載入器

use crate::component::{Board, Position};
use crate::error::{LoadError, Result};
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

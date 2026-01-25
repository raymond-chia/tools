//! 棋盤載入器

use crate::component::{Board, Position};
use crate::error::{BoardError, Result};

/// 從 ASCII 格式載入棋盤
///
/// ASCII 格式：每行用空格分隔的符號，`.` 代表有效位置
/// 例如：
/// ```text
/// . . .
/// . . .
/// . . .
/// ```
pub fn load_from_ascii(ascii: &str) -> Result<(Board, Vec<Position>)> {
    let lines: Vec<&str> = ascii
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    if lines.is_empty() {
        return Err(BoardError::ParseError("棋盤為空".to_string()).into());
    }

    // 推導寬度（第一行的符號數）
    let width = lines[0]
        .split_whitespace()
        .count()
        .try_into()
        .map_err(|_| BoardError::ParseError("棋盤寬度過大".to_string()))?;

    let height = lines
        .len()
        .try_into()
        .map_err(|_| BoardError::ParseError("棋盤高度過大".to_string()))?;

    let board = Board { width, height };

    let mut positions = Vec::new();
    for (y, line) in lines.iter().enumerate() {
        for (x, _cell) in line.split_whitespace().enumerate() {
                positions.push(Position { x, y });
        }
    }

    Ok((board, positions))
}

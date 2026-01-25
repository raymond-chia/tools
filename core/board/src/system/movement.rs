//! 移動邏輯

use super::board::is_valid_position;
use crate::component::{Board, Position};
use crate::error::{BoardError, Result};
use crate::typ::Direction;

/// 計算從當前位置往指定方向移動一格後的位置，檢查棋盤邊界
///
/// 返回 `None` 當新位置超出棋盤邊界
pub fn step_in_direction(board: Board, pos: Position, direction: Direction) -> Option<Position> {
    let new_pos = match direction {
        Direction::Up => {
            if pos.y == 0 {
                return None;
            }
            Position {
                x: pos.x,
                y: pos.y - 1,
            }
        }
        Direction::Down => Position {
            x: pos.x,
            y: pos.y + 1,
        },
        Direction::Left => {
            if pos.x == 0 {
                return None;
            }
            Position {
                x: pos.x - 1,
                y: pos.y,
            }
        }
        Direction::Right => Position {
            x: pos.x + 1,
            y: pos.y,
        },
    };

    if is_valid_position(board, new_pos) {
        Some(new_pos)
    } else {
        None
    }
}

/// 計算從起點到終點的移動路徑（簡單直線路徑）
///
/// 演算法：簡單的水平+垂直移動
/// 1. 先水平移動到目標 x 座標
/// 2. 再垂直移動到目標 y 座標
/// 3. 返回完整路徑（不包含起點）
///
/// # Fail fast 驗證：
/// - 起點必須在棋盤內
/// - 終點必須在棋盤內
pub fn manhattan_path(board: Board, from: Position, to: Position) -> Result<Vec<Position>> {
    // Fail fast：驗證起點和終點都在棋盤內
    if !is_valid_position(board, from) {
        return Err(BoardError::OutOfBounds {
            x: from.x,
            y: from.y,
            width: board.width,
            height: board.height,
        }
        .into());
    }

    if !is_valid_position(board, to) {
        return Err(BoardError::OutOfBounds {
            x: to.x,
            y: to.y,
            width: board.width,
            height: board.height,
        }
        .into());
    }

    let mut path = Vec::new();
    let mut current = from;

    // 水平移動到目標 x 座標
    while current.x != to.x {
        let direction = if current.x < to.x {
            Direction::Right
        } else {
            Direction::Left
        };

        match step_in_direction(board, current, direction) {
            Some(next_pos) => {
                current = next_pos;
                path.push(current);
            }
            None => {
                // 這不應該發生，因為我們已經驗證了邊界
                return Err(BoardError::InternalError("無效的水平移動".to_string()).into());
            }
        }
    }

    // 垂直移動到目標 y 座標
    while current.y != to.y {
        let direction = match current.y < to.y {
            true => Direction::Down,
            false => Direction::Up,
        };

        match step_in_direction(board, current, direction) {
            Some(next_pos) => {
                current = next_pos;
                path.push(current);
            }
            None => {
                // 這不應該發生，因為我們已經驗證了邊界
                return Err(BoardError::InternalError("無效的垂直移動".to_string()).into());
            }
        }
    }

    Ok(path)
}

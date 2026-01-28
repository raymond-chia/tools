//! 移動邏輯

use crate::component::{Board, Faction, Position};
use crate::error::{BoardError, Result};
use crate::logic::BASIC_MOVEMENT_COST;
use crate::logic::board::is_valid_position;
use pathfinding::prelude::astar;

/// 移動方向（四方向）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

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

/// 移動者（位置 + 陣營）
#[derive(Debug, Clone, Copy)]
pub struct Mover {
    pub pos: Position,
    pub faction: Faction,
}

/// 計算從起點到終點的移動路徑（含碰撞檢測，使用 A* 尋路）
///
/// 演算法：A* 尋路，優先考慮曼哈頓距離
/// 1. 使用 A* 找出最短路徑
/// 2. 檢查每一步是否可通行（碰撞檢測）
/// 3. 如果有敵軍阻擋，自動迂迴
/// 4. 返回完整路徑（不包含起點）
///
/// # Fail fast 驗證：
/// - 起點必須在棋盤內
/// - 終點必須在棋盤內
/// - 終點必須可通行（無敵軍）
///
/// # 碰撞規則：
/// - 友軍（相同 Faction）可穿越
/// - 敵軍（不同 Faction）不可穿越
/// - 無單位的位置可通行
pub fn manhattan_path<F>(
    board: Board,
    mover: Mover,
    to: Position,
    get_occupant_faction: F,
) -> Result<Vec<Position>>
where
    F: Fn(Position) -> Option<Faction> + Copy,
{
    let from = mover.pos;
    let mover_faction = mover.faction;

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

    // Fail fast：驗證終點必須是空的（無任何單位）
    let to_occupant_faction = get_occupant_faction(to);
    if to_occupant_faction.is_some() {
        return Err(BoardError::PathBlocked { x: to.x, y: to.y }.into());
    }

    // 起點等於終點的特殊情況
    if from == to {
        return Ok(vec![]);
    }

    // 使用 A* 尋路
    let path = astar(
        &from,
        |&pos| {
            let mut successors = Vec::new();
            for direction in &[
                Direction::Up,
                Direction::Down,
                Direction::Left,
                Direction::Right,
            ] {
                if let Some(next_pos) = step_in_direction(board, pos, *direction) {
                    let occupant_faction = get_occupant_faction(next_pos);
                    if is_passable(mover_faction, occupant_faction) {
                        successors.push((next_pos, BASIC_MOVEMENT_COST));
                    }
                }
            }
            successors
        },
        |&pos| {
            // 啟發式函數：曼哈頓距離 + 方向偏好
            // 優先走左右（水平），後走上下（垂直）
            let dx = if pos.x > to.x {
                pos.x - to.x
            } else {
                to.x - pos.x
            };
            let dy = if pos.y > to.y {
                pos.y - to.y
            } else {
                to.y - pos.y
            };
            dx * 3 + dy * 2
        },
        |&pos| pos == to,
    );

    match path {
        Some((path, _cost)) => {
            // path 是從 from 到 to 的完整路徑（包含起點）
            // 去掉起點並返回
            let mut result = path;
            result.remove(0);
            Ok(result)
        }
        None => {
            // 找不到路徑
            Err(BoardError::PathBlocked { x: to.x, y: to.y }.into())
        }
    }
}

/// 碰撞檢測：檢查位置是否可通行
///
/// 規則：
/// - 友軍（相同 Faction）可穿越
/// - 敵軍（不同 Faction）不可穿越
/// - 無單位的位置可通行
fn is_passable(mover_faction: Faction, occupant: Option<Faction>) -> bool {
    match occupant {
        None => true,
        Some(faction) => faction == mover_faction,
    }
}

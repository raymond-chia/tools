//! 移動邏輯

use crate::alias::MovementCost;
use crate::component::{Board, Faction, Position};
use crate::error::{BoardError, Result};
use crate::logic::board::is_valid_position;
use std::collections::{HashSet, VecDeque};

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

/// 計算給定移動力預算內可到達的所有位置
///
/// 使用 BFS 探索所有可到達位置
/// 1. 從起點開始擴展
/// 2. 檢查每一步是否可通行（碰撞檢測）
/// 3. 只包含消耗成本 <= 預算的位置
/// 4. 返回所有可到達位置（不包含起點）
///
/// # Fail fast 驗證：
/// - 起點必須在棋盤內
///
/// # 碰撞規則：
/// - 友軍（相同 Faction）可穿越
/// - 敵軍（不同 Faction）不可穿越
/// - 無單位的位置可通行
///
/// # 地形消耗：
/// - `get_terrain_cost` 返回該位置的移動成本
/// - `usize::MAX` 表示不可通行（例如牆壁）
pub fn reachable_positions<F, G>(
    board: Board,
    mover: Mover,
    budget: MovementCost,
    get_occupant_faction: F,
    get_terrain_cost: G,
) -> Result<Vec<Position>>
where
    F: Fn(Position) -> Option<Faction> + Copy,
    G: Fn(Position) -> MovementCost + Copy,
{
    let from = mover.pos;
    let mover_faction = mover.faction;

    // Fail fast：驗證起點在棋盤內
    if !is_valid_position(board, from) {
        return Err(BoardError::OutOfBounds {
            x: from.x,
            y: from.y,
            width: board.width,
            height: board.height,
        }
        .into());
    }

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut reachable = Vec::new();

    visited.insert(from);
    queue.push_back((from, 0));

    while let Some((pos, cost)) = queue.pop_front() {
        // 不包含起點，且只包含無單位佔據的位置
        if pos != from && get_occupant_faction(pos).is_none() {
            reachable.push(pos);
        }

        // 探索相鄰位置
        for direction in &[
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ] {
            if let Some(next_pos) = step_in_direction(board, pos, *direction) {
                if visited.contains(&next_pos) {
                    continue;
                }

                let terrain_cost = get_terrain_cost(next_pos);
                if terrain_cost == MovementCost::MAX {
                    continue;
                }

                let new_cost = cost + terrain_cost;
                if new_cost > budget {
                    continue;
                }

                let occupant_faction = get_occupant_faction(next_pos);
                if !is_passable(mover_faction, occupant_faction) {
                    continue;
                }

                visited.insert(next_pos);
                queue.push_back((next_pos, new_cost));
            }
        }
    }

    Ok(reachable)
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

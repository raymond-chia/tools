//! 移動邏輯

use crate::alias::MovementCost;
use crate::component::{Board, Faction, Position};
use crate::error::{BoardError, Result};
use crate::logic::board::is_valid_position;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

/// 移動方向（四方向）
#[derive(Debug, Clone, Copy)]
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
#[derive(Debug)]
pub struct Mover {
    pub pos: Position,
    pub faction: Faction,
}

/// 可到達位置的資訊（含成本與前驅節點）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReachableInfo {
    pub cost: MovementCost,
    pub prev: Position, // 上一個位置（可能是起點）
}

/// 計算給定移動力預算內可到達的所有位置
///
/// 使用 Dijkstra 算法探索所有可到達位置
/// 1. 從起點開始擴展
/// 2. 檢查每一步是否可通行（碰撞檢測）
/// 3. 只包含消耗成本 <= 預算的位置
/// 4. 返回所有可到達位置及其成本與前驅節點（不包含起點）
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
) -> Result<HashMap<Position, ReachableInfo>>
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

    let mut dist: HashMap<Position, MovementCost> = HashMap::new();
    let mut prev: HashMap<Position, Position> = HashMap::new();
    let mut queue: BinaryHeap<Reverse<(MovementCost, Position)>> = BinaryHeap::new();

    dist.insert(from, 0);
    queue.push(Reverse((0, from)));

    // Dijkstra 探索
    while let Some(Reverse((cost, pos))) = queue.pop() {
        // 跳過過時的隊列項（已有更優路徑）
        if cost > dist.get(&pos).copied().unwrap_or(MovementCost::MAX) {
            continue;
        }

        // 探索相鄰位置
        const DIRECTIONS: [Direction; 4] = [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ];
        for direction in DIRECTIONS {
            if let Some(next_pos) = step_in_direction(board, pos, direction) {
                let terrain_cost = get_terrain_cost(next_pos);
                if terrain_cost == MovementCost::MAX {
                    continue;
                }

                let new_cost = dist[&pos] + terrain_cost;
                if new_cost > budget {
                    continue;
                }

                let occupant_faction = get_occupant_faction(next_pos);
                if !is_passable(mover_faction, occupant_faction) {
                    continue;
                }

                // 如果是更優路徑，更新距離和前驅
                let best_cost = dist.get(&next_pos).copied().unwrap_or(MovementCost::MAX);
                if new_cost < best_cost {
                    dist.insert(next_pos, new_cost);
                    prev.insert(next_pos, pos);
                    queue.push(Reverse((new_cost, next_pos)));
                }
            }
        }
    }

    // 分離結果收集邏輯
    let reachable = dist
        .into_iter()
        .filter_map(|(pos, cost)| {
            if pos == from || get_occupant_faction(pos).is_some() {
                return None;
            }
            Some((
                pos,
                ReachableInfo {
                    cost,
                    prev: prev[&pos],
                },
            ))
        })
        .collect();

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

use crate::action::algo::{self, Board as AlgoBoard};
use crate::*;
use std::collections::HashMap;

struct MovableBoardView<'a> {
    board: &'a Board,
    move_points: MovementCost,
    moved_distance: MovementCost,
}

impl<'a> AlgoBoard for MovableBoardView<'a> {
    fn is_valid(&self, pos: Pos) -> bool {
        self.board.get_tile(pos).is_some()
    }

    fn is_passable(&self, active_unit_pos: Pos, pos: Pos, total: MovementCost) -> bool {
        // 不能超越兩倍移動力
        if total > self.move_points * 2 - self.moved_distance {
            return false;
        }
        // 不能穿越敵軍
        let Some(unit_id) = self.board.pos_to_unit.get(&active_unit_pos) else {
            // 不合理
            return false;
        };
        let Some(active_team) = self.board.units.get(unit_id).map(|unit| &unit.team) else {
            // 不合理
            return false;
        };
        let Some(unit_id) = self.board.pos_to_unit.get(&pos) else {
            // 沒有單位在目標位置
            return true;
        };
        let target_team = self.board.units.get(unit_id).map_or("", |unit| &unit.team);
        active_team == target_team
    }

    fn get_cost(&self, pos: Pos) -> MovementCost {
        self.board
            .get_tile(pos)
            .map(|t| movement_cost(t.terrain))
            .unwrap_or(MAX_MOVEMENT_COST)
    }

    fn get_neighbors(&self, pos: Pos) -> Vec<Pos> {
        let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        dirs.into_iter()
            .map(|(dx, dy)| (dx + pos.x as isize, dy + pos.y as isize))
            .filter_map(|(x, y)| {
                if x >= 0 && y >= 0 {
                    Some((x as usize, y as usize))
                } else {
                    None
                }
            })
            .map(|(x, y)| Pos { x, y })
            .collect()
    }
}

/// 計算指定單位的可移動範圍
pub fn movable_area(board: &Board, from: Pos) -> HashMap<Pos, (MovementCost, Pos)> {
    let unit_id = match board.pos_to_unit.get(&from) {
        Some(id) => *id,
        None => return HashMap::new(),
    };
    let unit = match board.units.get(&unit_id) {
        Some(u) => u,
        None => return HashMap::new(),
    };
    let view = MovableBoardView {
        board,
        move_points: unit.move_points,
        moved_distance: unit.moved,
    };
    let result = algo::dijkstra(&view, from);
    result
}

pub fn reconstruct_path(
    map: &HashMap<Pos, (MovementCost, Pos)>,
    from: Pos,
    to: Pos,
) -> Result<Vec<Pos>, Error> {
    let mut path = Vec::new();
    let mut current = to;
    while current != from {
        let Some((_, prev)) = map.get(&current) else {
            return Err(Error::NotReachable(to));
        };
        path.push(current);
        current = *prev;
    }
    path.push(from);
    path.reverse();
    Ok(path)
}

pub fn move_unit_with_path(board: &mut Board, path: Vec<Pos>) -> Result<(), Error> {
    let actor = path.get(0).ok_or(Error::InvalidParameter)?;
    let mut actor = *actor;
    for next in path {
        let result = move_unit(board, actor, next);
        match result {
            Ok(_) => {
                actor = next;
            }
            Err(Error::AlliedUnitOnPos(_)) => {}
            _ => return result,
        }
    }
    Ok(())
}

/// 將 actor 位置的單位移動到 to 位置
pub fn move_unit(board: &mut Board, actor: Pos, to: Pos) -> Result<(), Error> {
    if actor == to {
        return Ok(()); // 不需要移動
    }
    // to 應該在棋盤上
    let Some(tile) = board.get_tile(to) else {
        return Err(Error::NoTileOnPos(to));
    };
    let terrain = tile.terrain;
    // 檢查 from 位置有無單位
    let unit_id = match board.pos_to_unit.get(&actor) {
        Some(id) => *id,
        None => return Err(Error::NoUnitOnPos(actor)),
    };
    let Some(active_unit) = board.units.get(&unit_id) else {
        return Err(Error::NoUnitOnPos(actor));
    };
    // 檢查 to 位置是否已有單位
    let result = if let Some(unit_id) = board.pos_to_unit.get(&to) {
        let Some(target_unit) = board.units.get(unit_id) else {
            return Err(Error::NoUnitOnPos(to));
        };
        if active_unit.team != target_unit.team {
            return Err(Error::HostileUnitOnPos(to));
        }
        Err(Error::AlliedUnitOnPos(to))
    } else {
        Ok(())
    };
    let Some(active_unit) = board.units.get_mut(&unit_id) else {
        return Err(Error::NoUnitOnPos(actor));
    };
    let cost = movement_cost(terrain);
    if active_unit.moved + cost > active_unit.move_points * 2 {
        return Err(Error::NotEnoughPoints);
    }
    active_unit.moved += cost;
    if result.is_ok() {
        board.pos_to_unit.remove(&actor);
        board.pos_to_unit.insert(to, unit_id);
    }
    result
}

pub fn movement_preview_color(
    board: &Board,
    movable: &HashMap<Pos, (MovementCost, Pos)>,
    active_unit_id: &UnitID,
    path: &[Pos],
    pos: Pos,
) -> Result<RGBA, Error> {
    if board.pos_to_unit.contains_key(&pos) {
        return Err(Error::NotReachable(pos));
    }
    let Some((cost, _)) = movable.get(&pos) else {
        return Err(Error::NotReachable(pos));
    };
    let cost = *cost;

    let (move_points, moved) = board
        .units
        .get(active_unit_id)
        .map(|u| (u.move_points, u.moved))
        .unwrap_or((0, 0));
    let is_first = moved + cost <= move_points;
    // 顯示可移動範圍
    let color = if is_first {
        (50, 100, 255, 150) // 淺藍
    } else {
        (50, 50, 255, 150) // 深藍
    };
    // 顯示移動路徑
    if !path.contains(&pos) {
        // 不在移動路徑
        return Ok(color);
    }
    if is_first {
        Ok((125, 0, 125, 150)) // 淺紫
    } else {
        Ok((100, 0, 100, 150)) // 深紫
    }
}

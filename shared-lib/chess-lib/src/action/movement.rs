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

    fn is_passable(&self, _active_unit_pos: Pos, pos: Pos, total: MovementCost) -> bool {
        if total > self.move_points * 2 - self.moved_distance {
            return false;
        }
        // 起點可重疊，其餘不可有單位
        !self.board.pos_to_unit.contains_key(&pos)
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

/// 將 from 位置的單位移動到 to 位置
pub fn move_unit(board: &mut Board, from: Pos, to: Pos) -> Result<(), String> {
    // 檢查 from 位置有無單位
    let unit_id = match board.pos_to_unit.get(&from) {
        Some(id) => *id,
        None => return Err(format!("from 位置 {:?} 沒有單位", from)),
    };
    // 檢查 to 位置是否已有單位
    if board.pos_to_unit.contains_key(&to) {
        return Err(format!("to 位置 {:?} 已有單位", to));
    }
    // 更新 pos_to_unit 映射
    board.pos_to_unit.remove(&from);
    board.pos_to_unit.insert(to, unit_id);
    Ok(())
}

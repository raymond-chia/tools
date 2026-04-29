use crate::ecs_types::components::Position;
use std::collections::HashSet;

/// 判斷從 `from` 到 `to` 是否有視線（Bresenham 直線算法）
///
/// 規則：
/// - `from == to` → 永遠有視線
/// - `from` 或 `to` 格本身有阻擋物 → 無視線
/// - 中間格有阻擋物 → 無視線
pub(crate) fn has_line_of_sight(
    from: Position,
    to: Position,
    blocks_sight: &HashSet<Position>,
) -> bool {
    if from == to {
        return true;
    }
    if blocks_sight.contains(&from) || blocks_sight.contains(&to) {
        return false;
    }

    let mut x = from.x as i32;
    let mut y = from.y as i32;
    let to_x = to.x as i32;
    let to_y = to.y as i32;

    let dx = (to_x - x).abs();
    let dy = (to_y - y).abs();
    let step_x = if to_x > x { 1 } else { -1 };
    let step_y = if to_y > y { 1 } else { -1 };
    let mut error = dx - dy;

    loop {
        if x == to_x && y == to_y {
            break;
        }

        let double_error = error * 2;
        if double_error > -dy {
            error -= dy;
            x += step_x;
        }
        if double_error < dx {
            error += dx;
            y += step_y;
        }

        if x == to_x && y == to_y {
            break;
        }

        let mid = Position {
            x: x as usize,
            y: y as usize,
        };
        if blocks_sight.contains(&mid) {
            return false;
        }
    }

    true
}

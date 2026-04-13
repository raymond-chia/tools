use super::normalize_direction;
use crate::domain::alias::Coord;
use crate::domain::core_types::Area;
use crate::ecs_types::components::Position;
use crate::ecs_types::resources::Board;
use crate::error::Result;
use crate::logic::board;

/// 計算射程內所有格子（曼哈頓距離在 [min_range, max_range] 內）
pub(crate) fn compute_range_positions(
    caster: Position,
    range: (Coord, Coord),
    board: Board,
) -> Vec<Position> {
    let (min_range, max_range) = range;
    let mut positions = Vec::new();

    let caster_x = caster.x as i32;
    let caster_y = caster.y as i32;
    let max_range = max_range as i32;
    let min_range = min_range as i32;

    for dy in -max_range..=max_range {
        for dx in -max_range..=max_range {
            let distance = dx.abs() + dy.abs();
            if distance < min_range || distance > max_range {
                continue;
            }

            let x = caster_x + dx;
            let y = caster_y + dy;

            if let Some(pos) = board::try_position(board, x, y) {
                positions.push(pos);
            }
        }
    }

    positions
}

/// 計算 AOE 影響的所有位置
/// - Single: 回傳該格
/// - Diamond/Cross: 以 target 為中心，忽略 caster
/// - Line: 以 caster→target 方向延伸
pub(crate) fn compute_affected_positions(
    area: &Area,
    caster: Position,
    target: Position,
    board: Board,
) -> Result<Vec<Position>> {
    match area {
        Area::Single => Ok(vec![target]),
        Area::Diamond { radius } => Ok(compute_diamond(target, *radius, board)),
        Area::Cross { length } => Ok(compute_cross(target, *length, board)),
        Area::Line { length } => compute_line(caster, target, *length, board),
    }
}

// ============================================================================
// AOE 計算
// ============================================================================

/// 計算菱形 AOE（曼哈頓距離）
fn compute_diamond(target: Position, radius: Coord, board: Board) -> Vec<Position> {
    let mut positions = Vec::new();
    let target_x = target.x as i32;
    let target_y = target.y as i32;
    let radius = radius as i32;

    for dy in -(radius)..=radius {
        for dx in -(radius)..=radius {
            if (dx.abs() + dy.abs()) > radius {
                continue;
            }
            let x = target_x + dx;
            let y = target_y + dy;

            if let Some(pos) = board::try_position(board, x, y) {
                positions.push(pos);
            }
        }
    }

    positions
}

/// 計算十字形 AOE
fn compute_cross(target: Position, length: Coord, board: Board) -> Vec<Position> {
    let mut positions = vec![target];
    let target_x = target.x as i32;
    let target_y = target.y as i32;
    let length = length as i32;

    const DIRECTIONS: [(i32, i32); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];

    for (step_x, step_y) in DIRECTIONS {
        for i in 1..=length {
            let x = target_x + step_x * i;
            let y = target_y + step_y * i;
            match board::try_position(board, x, y) {
                Some(pos) => positions.push(pos),
                // 邊界裁切：遇到無效位置就停止該方向的延伸
                None => break,
            }
        }
    }

    positions
}

/// 計算直線 AOE（從施放者到目標方向，長度為 length 格）
fn compute_line(
    caster: Position,
    target: Position,
    length: Coord,
    board: Board,
) -> Result<Vec<Position>> {
    let (step_x, step_y) = normalize_direction(caster, target)?;

    let mut positions = Vec::new();
    let caster_x = caster.x as i32;
    let caster_y = caster.y as i32;
    let length = length as i32;

    for i in 0..=length {
        let x = caster_x + step_x * i;
        let y = caster_y + step_y * i;
        match board::try_position(board, x, y) {
            Some(pos) => positions.push(pos),
            // 邊界裁切：遇到無效位置就停止延伸
            None => break,
        }
    }

    Ok(positions)
}

//! 瞄準格的視線：所有技能共用，效果範圍不再逐格檢查。
use crate::error::{self, GameError};
use crate::model::{Board, Footprint, GridPos};
use crate::movement::{distance, footprint_cells};
use crate::terrain::{terrain_type, terrains_at};
use bevy_ecs::prelude::World;

/// 任一佔用格同時滿足射程與視線即可；無合法射程時區分過近、過遠，否則回報視線遮擋。
pub(crate) fn check_sight_in_range(
    world: &World,
    origin: GridPos,
    footprint: Footprint,
    target: GridPos,
    min_range: i32,
    max_range: i32,
) -> Result<(), GameError> {
    let mut all_too_close = true;
    let mut has_cell_in_range = false;
    for cell in footprint_cells(origin, footprint) {
        let range = distance(cell, target);
        all_too_close &= range < min_range;
        if (min_range..=max_range).contains(&range) {
            has_cell_in_range = true;
            if clear_line(world, cell, target) {
                return Ok(());
            }
        }
    }
    if has_cell_in_range {
        Err(error::target_not_visible())
    } else if all_too_close {
        Err(error::target_too_close())
    } else {
        Err(error::target_too_far())
    }
}

#[cfg(test)]
pub(crate) fn has_sight(
    world: &World,
    origin: GridPos,
    footprint: Footprint,
    target: GridPos,
) -> bool {
    footprint_cells(origin, footprint)
        .into_iter()
        .any(|cell| clear_line(world, cell, target))
}

/// 固定端點順序，避免 Bresenham 在半格取捨時產生方向不對稱。
/// 起點與終點不遮擋自己的視線，只檢查兩者之間的格子。
fn clear_line(world: &World, origin: GridPos, target: GridPos) -> bool {
    let (start, end) = if (origin.x, origin.y) <= (target.x, target.y) {
        (origin, target)
    } else {
        (target, origin)
    };
    let mut x = i64::from(start.x);
    let mut y = i64::from(start.y);
    let dx = (i64::from(end.x) - x).abs();
    let dy = -(i64::from(end.y) - y).abs();
    let sx = if start.x < end.x { 1 } else { -1 };
    let sy = if start.y < end.y { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        let cell = GridPos {
            x: x as i32,
            y: y as i32,
        };
        if cell == end {
            return true;
        }
        if cell != start
            && terrains_at(world, cell)
                .iter()
                .any(|kind| terrain_type(world.resource::<Board>(), kind).blocks_sight)
        {
            return false;
        }
        let twice = 2 * error;
        if twice >= dy {
            error += dy;
            x += sx;
        }
        if twice <= dx {
            error += dx;
            y += sy;
        }
    }
}

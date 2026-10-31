//! 上層撒放與搬動；可通行摘要沿用執行期，Godot 只傳遞操作。
use super::{
    EditorError,
    map::{BrushArea, brush_cells, unit_occupies},
};
use crate::authoring::{Definitions, Map};
use crate::model::TerrainGrid;
use crate::{Game, GridPos, TerrainEntryRule, TerrainLayer, TerrainPlacement};
use serde::Deserialize;
use std::collections::{HashSet, VecDeque};

#[derive(Deserialize)]
struct Scatter {
    #[serde(flatten)]
    area: BrushArea,
    kind: String,
    percent: u32,
    preserve_connectivity: bool,
    seed: u64,
}

#[derive(Deserialize)]
struct Move {
    from: GridPos,
    to: GridPos,
}

fn load(definitions: &str, map: &str) -> Result<(Definitions, Map), EditorError> {
    let definitions = super::json::from_str(definitions)
        .map_err(|e| EditorError::conversion("definitions_json_parse", e.to_string()))?;
    let map = super::json::from_str(map)
        .map_err(|e| EditorError::conversion("map_json_parse", e.to_string()))?;
    Ok((definitions, map))
}

fn response(definitions: Definitions, map: Map, changed: bool) -> Result<String, EditorError> {
    let mut game =
        Game::from_authoring(definitions, map.clone(), crate::authoring::Difficulty::Hard)?;
    Ok(
        serde_json::json!({"changed": changed, "map": map, "snapshot": game.take_snapshot(None)})
            .to_string(),
    )
}

fn overlay_at(definitions: &Definitions, map: &Map, cell: GridPos) -> Option<usize> {
    map.terrains.iter().position(|terrain| {
        terrain.x == cell.x
            && terrain.y == cell.y
            && definitions
                .terrain_types
                .iter()
                .any(|entry| entry.id == terrain.kind && entry.layer == TerrainLayer::Overlay)
    })
}

fn index(grid: &TerrainGrid, cell: GridPos) -> usize {
    (cell.y * grid.width + cell.x) as usize
}

fn neighbors(grid: &TerrainGrid, bounds: (i32, i32, i32, i32), cell: GridPos) -> Vec<GridPos> {
    [(1, 0), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .map(|(dx, dy)| GridPos {
            x: cell.x + dx,
            y: cell.y + dy,
        })
        .filter(|p| {
            p.x >= bounds.0
                && p.y >= bounds.1
                && p.x <= bounds.2
                && p.y <= bounds.3
                && grid.cells[index(grid, *p)].passable
        })
        .collect()
}

// 封住一格後，原本相鄰的可走格仍須能繞路抵達彼此；不要求接通既有孤島。
fn can_block(grid: &TerrainGrid, bounds: (i32, i32, i32, i32), cell: GridPos) -> bool {
    let adjacent = neighbors(grid, bounds, cell);
    let start = match adjacent.first() {
        Some(start) => *start,
        None => return true,
    };
    let mut remaining: HashSet<_> = adjacent.into_iter().collect();
    let mut seen = HashSet::from([start]);
    let mut queue = VecDeque::from([start]);
    while let Some(current) = queue.pop_front() {
        remaining.remove(&current);
        if remaining.is_empty() {
            return true;
        }
        for next in neighbors(grid, bounds, current) {
            if seen.insert(next) {
                queue.push_back(next);
            }
        }
    }
    false
}

pub fn scatter_overlay_from_json(
    definitions: &str,
    map: &str,
    command: &str,
) -> Result<String, EditorError> {
    let (definitions, mut map) = load(definitions, map)?;
    let Scatter {
        area,
        kind,
        percent,
        preserve_connectivity,
        seed,
    } = super::json::from_str(command)
        .map_err(|e| EditorError::operation("invalid_command", &e.to_string(), Vec::new()))?;
    if percent > 100 {
        return Err(EditorError::operation("invalid_percent", "", Vec::new()));
    }
    let terrain = definitions
        .terrain_types
        .iter()
        .find(|entry| entry.id == kind && entry.layer == TerrainLayer::Overlay)
        .ok_or_else(|| EditorError::operation("not_found", &kind, Vec::new()))?;
    let blocks = terrain.entry_rule != TerrainEntryRule::Walkable;
    let cells = brush_cells(&map, area)?;
    let bounds = match (cells.first(), cells.last()) {
        (Some(first), Some(last)) => (
            (first.x - 1).max(0),
            (first.y - 1).max(0),
            (last.x + 1).min(map.width - 1),
            (last.y + 1).min(map.height - 1),
        ),
        _ => return response(definitions, map, false),
    };
    let existing = cells
        .iter()
        .filter(|cell| {
            map.terrains
                .iter()
                .any(|t| t.x == cell.x && t.y == cell.y && t.kind == kind)
        })
        .count();
    let mut remaining = (cells.len() * percent as usize / 100).saturating_sub(existing);
    let mut game = Game::from_authoring(
        definitions.clone(),
        map.clone(),
        crate::authoring::Difficulty::Hard,
    )?;
    game.set_random_seed(seed);
    let mut candidates = Vec::new();
    for cell in cells {
        if overlay_at(&definitions, &map, cell).is_none()
            && (!blocks || !unit_occupies(&definitions, &map, cell)?)
        {
            candidates.push(cell);
        }
    }
    // Fisher–Yates 洗牌；亂數沿用核心來源，重做由歷史保存結果而非重新抽籤。
    for n in (2..=candidates.len()).rev() {
        let chosen = crate::game::die(&mut game.world, n as u32) as usize - 1;
        candidates.swap(n - 1, chosen);
    }
    let mut grid = game.world.resource_mut::<TerrainGrid>();
    let mut changed = false;
    for cell in candidates {
        if remaining == 0 {
            break;
        }
        let i = index(&grid, cell);
        if !grid.cells[i].passable {
            continue;
        }
        if blocks {
            grid.cells[i].passable = false;
            if preserve_connectivity && !can_block(&grid, bounds, cell) {
                grid.cells[i].passable = true;
                continue;
            }
        }
        map.terrains.push(TerrainPlacement {
            x: cell.x,
            y: cell.y,
            kind: kind.clone(),
        });
        remaining -= 1;
        changed = true;
    }
    response(definitions, map, changed)
}

pub fn move_overlay_from_json(
    definitions: &str,
    map: &str,
    command: &str,
) -> Result<String, EditorError> {
    let (definitions, mut map) = load(definitions, map)?;
    let Move { from, to } = super::json::from_str(command)
        .map_err(|e| EditorError::operation("invalid_command", &e.to_string(), Vec::new()))?;
    let game = Game::from_authoring(
        definitions.clone(),
        map.clone(),
        crate::authoring::Difficulty::Hard,
    )?;
    let source = overlay_at(&definitions, &map, from);
    if from == to
        || to.x < 0
        || to.y < 0
        || to.x >= map.width
        || to.y >= map.height
        || overlay_at(&definitions, &map, to).is_some()
        || source.is_none()
    {
        return response(definitions, map, false);
    }
    let source = source.expect("上方已確認來源 overlay 存在");
    let terrain = definitions
        .terrain_types
        .iter()
        .find(|t| t.id == map.terrains[source].kind)
        .expect("地圖驗證已確認地形種類存在");
    let grid = game.world.resource::<TerrainGrid>();
    if !grid.cells[index(grid, to)].passable
        || (terrain.entry_rule != TerrainEntryRule::Walkable
            && unit_occupies(&definitions, &map, to)?)
    {
        return response(definitions, map, false);
    }
    map.terrains[source].x = to.x;
    map.terrains[source].y = to.y;
    response(definitions, map, true)
}

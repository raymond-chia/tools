//! 地形筆刷的替換、清除與合法性判斷集中於核心編輯器。
use super::EditorError;
use crate::authoring::{Definitions, Map};
use crate::{Game, GridPos, TerrainEntryRule, TerrainLayer, TerrainPlacement};
use serde::Deserialize;

#[derive(Deserialize)]
pub(super) struct BrushArea {
    x: i32,
    y: i32,
    size: i32,
}

#[derive(Deserialize)]
struct TerrainBrush {
    #[serde(flatten)]
    area: BrushArea,
    layer: TerrainLayer,
    kind: String,
}

// 預覽與實際塗抹共用定位與裁切，偶數邊長向座標正方向多延伸一格。
pub(super) fn brush_cells(map: &Map, area: BrushArea) -> Result<Vec<GridPos>, EditorError> {
    let BrushArea { x, y, size } = area;
    if size < 1 {
        return Err(EditorError::operation("invalid_brush_size", "", Vec::new()));
    }
    if x < 0 || y < 0 || x >= map.width || y >= map.height {
        return Ok(Vec::new());
    }
    let before = (size - 1) / 2;
    let after = size / 2;
    let mut cells = Vec::new();
    // 先裁切延伸量，再與起點相加，避免過大的作者輸入造成溢位。
    for y in (y - before).max(0)..=y + after.min(map.height - 1 - y) {
        for x in (x - before).max(0)..=x + after.min(map.width - 1 - x) {
            cells.push(GridPos { x, y });
        }
    }
    Ok(cells)
}

/// 只查詢筆刷覆蓋格子，不修改地圖，也不建立遊戲預覽。
pub fn terrain_brush_cells_from_json(map: &str, command: &str) -> Result<String, EditorError> {
    let map: Map = super::json::from_str(map)
        .map_err(|e| EditorError::conversion("map_json_parse", e.to_string()))?;
    let area: BrushArea = super::json::from_str(command)
        .map_err(|e| EditorError::operation("invalid_command", &e.to_string(), Vec::new()))?;
    let cells = brush_cells(&map, area)?;
    Ok(serde_json::json!({"cells": cells}).to_string())
}

pub fn paint_terrain_from_json(
    definitions: &str,
    map: &str,
    command: &str,
) -> Result<String, EditorError> {
    let definitions: Definitions = super::json::from_str(definitions)
        .map_err(|e| EditorError::conversion("definitions_json_parse", e.to_string()))?;
    let mut map: Map = super::json::from_str(map)
        .map_err(|e| EditorError::conversion("map_json_parse", e.to_string()))?;
    let TerrainBrush { area, layer, kind } = super::json::from_str(command)
        .map_err(|e| EditorError::operation("invalid_command", &e.to_string(), Vec::new()))?;
    let cells = brush_cells(&map, area)?;
    if cells.is_empty() {
        return Err(EditorError::operation("invalid_cell", "", Vec::new()));
    }
    let blocks_units = if kind.is_empty() {
        if layer != TerrainLayer::Overlay {
            return Err(EditorError::operation(
                "cannot_clear_ground",
                "",
                Vec::new(),
            ));
        }
        false
    } else {
        let terrain = definitions
            .terrain_types
            .iter()
            .find(|entry| entry.id == kind)
            .ok_or_else(|| EditorError::operation("not_found", &kind, Vec::new()))?;
        if terrain.layer != layer {
            return Err(EditorError::operation("wrong_layer", &kind, Vec::new()));
        }
        terrain.entry_rule != TerrainEntryRule::Walkable
    };
    let mut changed = false;
    for GridPos { x, y } in cells {
        if blocks_units && unit_occupies(&definitions, &map, GridPos { x, y })? {
            continue;
        }
        let replaces = |terrain: &TerrainPlacement| {
            terrain.x == x
                && terrain.y == y
                && definitions
                    .terrain_types
                    .iter()
                    .find(|entry| entry.id == terrain.kind)
                    .is_some_and(|definition| definition.layer == layer)
        };
        let current: Vec<_> = map
            .terrains
            .iter()
            .filter(|terrain| replaces(terrain))
            .collect();
        let cell_changed = if kind.is_empty() {
            !current.is_empty()
        } else {
            current.len() != 1 || current[0].kind != kind
        };
        if cell_changed {
            changed = true;
            map.terrains.retain(|terrain| !replaces(terrain));
            if !kind.is_empty() {
                map.terrains.push(TerrainPlacement {
                    x,
                    y,
                    kind: kind.clone(),
                });
            }
        }
    }
    // 同時產生預覽，Godot 不再重做已通過的驗證。
    let mut game =
        Game::from_authoring(definitions, map.clone(), crate::authoring::Difficulty::Hard)?;
    Ok(
        serde_json::json!({"changed": changed, "map": map, "snapshot": game.take_snapshot(None)})
            .to_string(),
    )
}

// 一般筆刷、撒放與搬動共用單位佔用規則。
pub(super) fn unit_occupies(
    definitions: &Definitions,
    map: &Map,
    cell: GridPos,
) -> Result<bool, EditorError> {
    for unit in &map.units {
        let unit_type = definitions
            .unit_types
            .iter()
            .find(|entry| entry.id == unit.unit_type)
            .ok_or_else(|| crate::error::unknown_unit_type(&unit.unit_type))?;
        let dx = i64::from(cell.x) - i64::from(unit.x);
        let dy = i64::from(cell.y) - i64::from(unit.y);
        if dx >= 0 && dx < i64::from(unit_type.width) && dy >= 0 && dy < i64::from(unit_type.height)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

//! 地形筆刷的替換、清除與合法性判斷集中於核心編輯器。
use super::EditorError;
use crate::authoring::{Definitions, Map};
use crate::{Game, TerrainLayer, TerrainPlacement};
use serde::Deserialize;

#[derive(Deserialize)]
struct TerrainBrush {
    x: i32,
    y: i32,
    layer: TerrainLayer,
    kind: String,
}

pub fn paint_terrain_from_json(
    definitions: &str,
    map: &str,
    command: &str,
) -> Result<String, EditorError> {
    let definitions: Definitions = super::json::from_str(definitions)
        .map_err(|e| EditorError::input("definitions_json_parse", e.to_string()))?;
    let mut map: Map = super::json::from_str(map)
        .map_err(|e| EditorError::input("map_json_parse", e.to_string()))?;
    let TerrainBrush { x, y, layer, kind } = super::json::from_str(command)
        .map_err(|e| EditorError::operation("invalid_command", &e.to_string(), Vec::new()))?;
    if x < 0 || y < 0 || x >= map.width || y >= map.height {
        return Err(EditorError::operation("invalid_cell", "", Vec::new()));
    }
    if kind.is_empty() {
        if layer != TerrainLayer::Overlay {
            return Err(EditorError::operation(
                "cannot_clear_ground",
                "",
                Vec::new(),
            ));
        }
    } else {
        let terrain = definitions
            .terrain_types
            .iter()
            .find(|entry| entry.id == kind)
            .ok_or_else(|| EditorError::operation("not_found", &kind, Vec::new()))?;
        if terrain.layer != layer {
            return Err(EditorError::operation("wrong_layer", &kind, Vec::new()));
        }
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
    let changed = if kind.is_empty() {
        !current.is_empty()
    } else {
        current.len() != 1 || current[0].kind != kind
    };
    if changed {
        map.terrains.retain(|terrain| !replaces(terrain));
        if !kind.is_empty() {
            map.terrains.push(TerrainPlacement { x, y, kind });
        }
    }
    // 同時產生預覽，Godot 不再重做已通過的驗證。
    let game = Game::from_authoring(definitions, map.clone())?;
    Ok(
        serde_json::json!({"changed": changed, "map": map, "snapshot": game.snapshot(None)})
            .to_string(),
    )
}

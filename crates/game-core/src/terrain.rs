//! 地形種類查詢與地形對移動、通行、防禦的影響；固定與暫時地形一律經由 terrains_at 合併。
use crate::model::{
    Board, Footprint, GridPos, TemporaryTerrains, TerrainEntryRule, TerrainTypeDef,
};
use crate::movement::footprint_cells;
use bevy_ecs::prelude::World;

pub(crate) fn terrain_type<'a>(board: &'a Board, kind: &str) -> &'a TerrainTypeDef {
    board
        .terrain_types
        .get(kind)
        .expect("載入時已驗證所有地形種類")
}

pub(crate) fn terrain_damage(board: &Board, kind: &str) -> i32 {
    terrain_type(board, kind).damage
}

pub(crate) fn terrains_at(w: &World, position: GridPos) -> Vec<String> {
    let mut kinds = w
        .resource::<Board>()
        .terrains
        .get(&position)
        .cloned()
        .unwrap_or_default();
    if let Some(temporary) = w.resource::<TemporaryTerrains>().0.get(&position) {
        for kind in temporary.keys() {
            if !kinds.contains(kind) {
                kinds.push(kind.clone());
            }
        }
    }
    kinds.sort();
    kinds
}

pub(crate) fn movement_cost(w: &World, position: GridPos) -> u32 {
    let board = w.resource::<Board>();
    1 + terrains_at(w, position)
        .iter()
        .map(|kind| terrain_type(board, kind).extra_movement_cost)
        .sum::<u32>()
}

pub(crate) fn terrain_ends_movement(w: &World, position: GridPos) -> bool {
    terrains_at(w, position)
        .iter()
        .any(|kind| terrain_type(w.resource::<Board>(), kind).damage > 0)
}

/// 佔用範圍內任一格（含暫時地形）的進入規則符合條件。
fn footprint_has_entry_rule(
    w: &World,
    position: GridPos,
    footprint: Footprint,
    matches_rule: impl Fn(TerrainEntryRule) -> bool,
) -> bool {
    let board = w.resource::<Board>();
    footprint_cells(position, footprint)
        .into_iter()
        .any(|cell| {
            terrains_at(w, cell)
                .iter()
                .any(|kind| matches_rule(terrain_type(board, kind).entry_rule))
        })
}

pub(crate) fn footprint_on_impassable(w: &World, position: GridPos, footprint: Footprint) -> bool {
    footprint_has_entry_rule(w, position, footprint, |rule| {
        rule != TerrainEntryRule::Walkable
    })
}

pub(crate) fn footprint_blocks_push(w: &World, position: GridPos, footprint: Footprint) -> bool {
    footprint_has_entry_rule(w, position, footprint, |rule| {
        rule == TerrainEntryRule::Blocked
    })
}

/// 佔用範圍內各格地形懲罰總和的最大值。
pub(crate) fn footprint_terrain_penalty(
    w: &World,
    position: GridPos,
    footprint: Footprint,
    penalty: impl Fn(&TerrainTypeDef) -> i32,
) -> i32 {
    let board = w.resource::<Board>();
    footprint_cells(position, footprint)
        .into_iter()
        .map(|cell| {
            terrains_at(w, cell)
                .iter()
                .map(|kind| penalty(terrain_type(board, kind)))
                .sum::<i32>()
        })
        .max()
        .unwrap_or(0)
}

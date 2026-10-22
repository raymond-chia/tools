//! 地形種類查詢與地形對移動、通行、防禦的影響；固定與暫時地形一律經由 terrains_at 合併。
use crate::game::{DamageResult, Game};
use crate::gameplay_config::DEFAULT_GROUND_TERRAIN;
use crate::model::{
    Board, CombatLogEvent, Encounter, Footprint, GridPos, Hp, Log, Pos, TemporaryTerrain,
    TemporaryTerrains, TerrainEntryRule, TerrainLayer, TerrainTypeDef, Unit,
};
use crate::movement::footprint_cells;
use bevy_ecs::prelude::{Entity, World};
use std::collections::HashMap;

pub(crate) fn terrain_type<'a>(board: &'a Board, kind: &str) -> &'a TerrainTypeDef {
    let Board {
        width: _,
        height: _,
        terrains: _,
        terrain_types,
    } = board;
    terrain_types.get(kind).expect("載入時已驗證所有地形種類")
}

pub(crate) fn terrain_damage(board: &Board, kind: &str) -> i32 {
    let TerrainTypeDef {
        blocks_sight: _,
        id: _,
        layer: _,
        entry_rule: _,
        damage,
        extra_movement_cost: _,
        dodge_penalty: _,
        block_penalty: _,
    } = terrain_type(board, kind);
    *damage
}

/// 作者驗證與執行期共用固定地形查詢；未放置 ground 時補上預設地面。
pub(crate) fn fixed_terrains_at<'a>(
    terrain_types: &HashMap<String, TerrainTypeDef>,
    terrains: &'a HashMap<GridPos, Vec<String>>,
    position: GridPos,
) -> Vec<&'a str> {
    let mut kinds: Vec<_> = terrains
        .get(&position)
        .into_iter()
        .flatten()
        .map(String::as_str)
        .collect();
    if !kinds
        .iter()
        .any(|kind| terrain_types[*kind].layer == TerrainLayer::Ground)
    {
        kinds.push(DEFAULT_GROUND_TERRAIN);
    }
    kinds
}

/// 回傳此格所有地形；未放置 ground 時補上預設地面。
pub(crate) fn terrains_at(w: &World, position: GridPos) -> Vec<String> {
    let Board {
        width: _,
        height: _,
        terrains,
        terrain_types,
    } = w.resource::<Board>();
    let mut kinds: Vec<_> = fixed_terrains_at(terrain_types, terrains, position)
        .into_iter()
        .map(str::to_owned)
        .collect();
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

pub(crate) fn ground_at<'a>(board: &Board, kinds: &'a [String]) -> Option<&'a String> {
    kinds.iter().find(|kind| {
        let TerrainTypeDef {
            blocks_sight: _,
            id: _,
            layer,
            entry_rule: _,
            damage: _,
            extra_movement_cost: _,
            dodge_penalty: _,
            block_penalty: _,
        } = terrain_type(board, kind);
        *layer == TerrainLayer::Ground
    })
}

pub(crate) fn movement_cost(w: &World, position: GridPos) -> u32 {
    let board = w.resource::<Board>();
    1 + terrains_at(w, position)
        .iter()
        .map(|kind| {
            let TerrainTypeDef {
                blocks_sight: _,
                id: _,
                layer: _,
                entry_rule: _,
                damage: _,
                extra_movement_cost,
                dodge_penalty: _,
                block_penalty: _,
            } = terrain_type(board, kind);
            *extra_movement_cost
        })
        .sum::<u32>()
}

/// 暫時地形的剩餘回合數（含本回合）；同格已有同種類固定地形時以固定地形為準，回傳 None。
pub(crate) fn temporary_remaining_rounds(w: &World, position: GridPos, kind: &str) -> Option<u32> {
    let Board {
        width: _,
        height: _,
        terrains,
        terrain_types: _,
    } = w.resource::<Board>();
    if terrains
        .get(&position)
        .is_some_and(|kinds| kinds.iter().any(|fixed| fixed == kind))
    {
        return None;
    }
    let TemporaryTerrain {
        expires_after_round,
    } = w
        .resource::<TemporaryTerrains>()
        .0
        .get(&position)?
        .get(kind)?;
    let Encounter {
        participants: _,
        order: _,
        cursor: _,
        round,
    } = w.resource::<Encounter>();
    Some(expires_after_round.saturating_sub(*round) + 1)
}

/// 佔用範圍內的地形種類；同種類跨越多格時只結算一次。
fn footprint_terrains(w: &World, position: GridPos, footprint: Footprint) -> Vec<String> {
    let mut kinds: Vec<_> = footprint_cells(position, footprint)
        .into_iter()
        .flat_map(|cell| terrains_at(w, cell))
        .collect();
    kinds.sort();
    kinds.dedup();
    kinds
}

/// 與防禦懲罰一致，移動成本取佔用範圍內各格成本的最大值。
pub(crate) fn footprint_movement_cost(w: &World, position: GridPos, footprint: Footprint) -> u32 {
    footprint_cells(position, footprint)
        .into_iter()
        .map(|cell| movement_cost(w, cell))
        .max()
        .expect("載入時已驗證單位佔用尺寸為正值，應至少佔用一格")
}

pub(crate) fn terrain_ends_movement(w: &World, position: GridPos, footprint: Footprint) -> bool {
    footprint_terrains(w, position, footprint)
        .iter()
        .any(|kind| {
            let TerrainTypeDef {
                blocks_sight: _,
                id: _,
                layer: _,
                entry_rule: _,
                damage,
                extra_movement_cost: _,
                dodge_penalty: _,
                block_penalty: _,
            } = terrain_type(w.resource::<Board>(), kind);
            *damage > 0
        })
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
            terrains_at(w, cell).iter().any(|kind| {
                let TerrainTypeDef {
                    blocks_sight: _,
                    id: _,
                    layer: _,
                    entry_rule,
                    damage: _,
                    extra_movement_cost: _,
                    dodge_penalty: _,
                    block_penalty: _,
                } = terrain_type(board, kind);
                matches_rule(*entry_rule)
            })
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

/// 進入地形的方式；只有被推動時會觸發瞬間倒地。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TerrainEntry {
    Movement,
    Push,
}

impl Game {
    /// 共用地形傷害與事件結算；回傳是否倒地，倒地後立即停止處理剩餘地形。
    pub(crate) fn apply_terrain_entry(
        &mut self,
        entity: Entity,
        id: i64,
        entry: TerrainEntry,
    ) -> bool {
        let Pos(position) = *self
            .world
            .get::<Pos>(entity)
            .expect("進入地形的單位應具有 Pos 元件");
        let footprint = *self
            .world
            .get::<Footprint>(entity)
            .expect("進入地形的單位應具有 Footprint 元件");
        for terrain in footprint_terrains(&self.world, position, footprint) {
            let TerrainTypeDef {
                blocks_sight: _,
                id: _,
                layer: _,
                entry_rule,
                damage,
                extra_movement_cost: _,
                dodge_penalty: _,
                block_penalty: _,
            } = terrain_type(self.world.resource::<Board>(), &terrain);
            let instant_down = entry == TerrainEntry::Push
                && *entry_rule == TerrainEntryRule::InstantDownWhenPushed;
            let damage = if instant_down {
                let Hp {
                    current,
                    maximum: _,
                } = self
                    .world
                    .get::<Hp>(entity)
                    .expect("進入地形的單位應具有 Hp 元件");
                *current
            } else {
                *damage
            };
            if damage == 0 {
                continue;
            }
            let Unit {
                unit_type,
                visual: _,
                team,
                movement: _,
                initiative: _,
                dodge: _,
                block: _,
                hit: _,
                physical_power: _,
                magical_power: _,
                block_reduction: _,
                equipment: _,
                skills: _,
            } = self
                .world
                .get::<Unit>(entity)
                .expect("進入地形的單位應具有 Unit 元件");
            let target_type = unit_type.clone();
            let target_team = team.clone();
            let DamageResult {
                remaining_hp,
                max_hp,
                downed,
            } = self.apply_damage(entity, id, damage);
            self.world
                .resource_mut::<Log>()
                .0
                .push(CombatLogEvent::TerrainDamage {
                    target: id,
                    target_type,
                    target_team,
                    terrain,
                    instant_down,
                    damage,
                    remaining_hp,
                    max_hp,
                    downed,
                });
            if downed {
                return true;
            }
        }
        false
    }
}

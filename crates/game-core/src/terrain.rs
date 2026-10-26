//! 地形種類查詢與地形對移動、通行、防禦的影響；固定與暫時地形一律經由 merged_terrains_at 合併。
//! 找路與視線讀取預先整理的 TerrainGrid，暫時地形只在本模組修改並同步重建。
use crate::game::{DamageResult, Game};
use crate::gameplay_config::DEFAULT_GROUND_TERRAIN;
use crate::model::{
    Board, CellTerrain, CombatLogEvent, Encounter, Footprint, GridPos, Hp, Log, TemporaryTerrain,
    TemporaryTerrains, TerrainEntryRule, TerrainGrid, TerrainLayer, TerrainTypeDef, Unit,
};
use crate::movement::{footprint_cells, placement};
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

/// 合併固定與暫時地形並排序；temporary 為 None 時只看永久地形。
fn merged_terrains_at<'a>(
    board: &'a Board,
    temporary: Option<&'a TemporaryTerrains>,
    position: GridPos,
) -> Vec<&'a str> {
    let Board {
        width: _,
        height: _,
        terrains,
        terrain_types,
    } = board;
    let mut kinds = fixed_terrains_at(terrain_types, terrains, position);
    if let Some(cell) = temporary.and_then(|TemporaryTerrains(cells)| cells.get(&position)) {
        for kind in cell.keys() {
            if !kinds.contains(&kind.as_str()) {
                kinds.push(kind);
            }
        }
    }
    kinds.sort();
    kinds
}

/// 回傳此格所有地形；未放置 ground 時補上預設地面。
pub(crate) fn terrains_at(w: &World, position: GridPos) -> Vec<String> {
    merged_terrains_at(
        w.resource::<Board>(),
        Some(w.resource::<TemporaryTerrains>()),
        position,
    )
    .into_iter()
    .map(str::to_owned)
    .collect()
}

/// 整理全地圖的地形摘要；temporary 為 None 時只看永久地形。
pub(crate) fn terrain_grid(board: &Board, temporary: Option<&TemporaryTerrains>) -> TerrainGrid {
    let Board {
        width,
        height,
        terrains: _,
        terrain_types: _,
    } = board;
    let cells = (0..*height)
        .flat_map(|y| (0..*width).map(move |x| GridPos { x, y }))
        .map(|position| {
            let types: Vec<_> = merged_terrains_at(board, temporary, position)
                .into_iter()
                .map(|kind| terrain_type(board, kind))
                .collect();
            CellTerrain {
                passable: types
                    .iter()
                    .all(|terrain| terrain.entry_rule == TerrainEntryRule::Walkable),
                hazard: types.iter().any(|terrain| terrain.damage > 0),
                blocks_sight: types.iter().any(|terrain| terrain.blocks_sight),
                movement_cost: 1 + types
                    .iter()
                    .map(|terrain| terrain.extra_movement_cost)
                    .sum::<u32>(),
                dodge_penalty: types.iter().map(|terrain| terrain.dodge_penalty).sum(),
                block_penalty: types.iter().map(|terrain| terrain.block_penalty).sum(),
            }
        })
        .collect();
    TerrainGrid {
        width: *width,
        height: *height,
        cells,
    }
}

/// 依目前固定與暫時地形重建 TerrainGrid；載入與暫時地形變動後呼叫。
pub(crate) fn refresh_terrain_grid(world: &mut World) {
    let grid = terrain_grid(
        world.resource::<Board>(),
        Some(world.resource::<TemporaryTerrains>()),
    );
    world.insert_resource(grid);
}

pub(crate) fn add_temporary_terrain(
    world: &mut World,
    position: GridPos,
    kind: String,
    expires_after_round: u32,
) {
    world
        .resource_mut::<TemporaryTerrains>()
        .0
        .entry(position)
        .or_default()
        .insert(
            kind,
            TemporaryTerrain {
                expires_after_round,
            },
        );
    refresh_terrain_grid(world);
}

/// 移除在指定回合前已到期的暫時地形。
pub(crate) fn expire_temporary_terrains(world: &mut World, round: u32) {
    world
        .resource_mut::<TemporaryTerrains>()
        .0
        .retain(|_, terrains| {
            terrains.retain(|_, terrain| terrain.expires_after_round >= round);
            !terrains.is_empty()
        });
    refresh_terrain_grid(world);
}

/// 呼叫端須確認位置在棋盤內。
pub(crate) fn grid_cell(grid: &TerrainGrid, position: GridPos) -> CellTerrain {
    let TerrainGrid {
        width,
        height: _,
        cells,
    } = grid;
    cells[(position.y * width + position.x) as usize]
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
pub(crate) fn footprint_movement_cost(
    grid: &TerrainGrid,
    position: GridPos,
    footprint: Footprint,
) -> u32 {
    footprint_cells(position, footprint)
        .into_iter()
        .map(|cell| grid_cell(grid, cell).movement_cost)
        .max()
        .expect("載入時已驗證單位佔用尺寸為正值，應至少佔用一格")
}

pub(crate) fn terrain_ends_movement(
    grid: &TerrainGrid,
    position: GridPos,
    footprint: Footprint,
) -> bool {
    footprint_cells(position, footprint)
        .into_iter()
        .any(|cell| grid_cell(grid, cell).hazard)
}

pub(crate) fn footprint_on_impassable(
    grid: &TerrainGrid,
    position: GridPos,
    footprint: Footprint,
) -> bool {
    footprint_cells(position, footprint)
        .into_iter()
        .any(|cell| !grid_cell(grid, cell).passable)
}

/// 佔用範圍內任一格（含暫時地形）為阻擋推擊的地形。
pub(crate) fn footprint_blocks_push(w: &World, position: GridPos, footprint: Footprint) -> bool {
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
                *entry_rule == TerrainEntryRule::Blocked
            })
        })
}

/// 佔用範圍內各格地形懲罰總和的最大值。
pub(crate) fn footprint_terrain_penalty(
    grid: &TerrainGrid,
    position: GridPos,
    footprint: Footprint,
    penalty: impl Fn(CellTerrain) -> i32,
) -> i32 {
    footprint_cells(position, footprint)
        .into_iter()
        .map(|cell| penalty(grid_cell(grid, cell)))
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
        let (position, footprint) = placement(&self.world, entity);
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
                active_skills: _,
                passive_skills: _,
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

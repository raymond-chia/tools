//! ECS 移動操作函數

use crate::domain::alias::{ID, MovementCost};
use crate::domain::constants::BASIC_MOVEMENT_COST;
use crate::ecs_logic::query::{get_all_objects, get_all_units, get_board, get_level_config};
use crate::ecs_types::components::{ActionState, MovementPoint, Occupant, Position, UnitFaction};
use crate::ecs_types::resources::TurnOrder;
use crate::error::{BoardError, DataError, Result};
use crate::logic::debug::short_type_name;
use crate::logic::movement::{Mover, ReachableInfo, reachable_positions, reconstruct_path};
use crate::logic::turn_order::get_active_unit;
use bevy_ecs::prelude::{Entity, World};
use std::collections::HashMap;

/// 移動結果，包含路徑與消耗
#[derive(Debug, Clone)]
pub struct MoveResult {
    pub path: Vec<Position>,
    pub cost: MovementCost,
}

/// 計算單位可到達的所有位置
///
/// 返回 position -> ReachableInfo 的對應表
pub fn get_reachable_positions(
    world: &mut World,
    occupant: Occupant,
) -> Result<HashMap<Position, ReachableInfo>> {
    // 查詢單位的位置、陣營與移動資訊
    let (unit_pos, faction, movement_point, movement_used) = world
        .query::<(
            &Occupant,
            &Position,
            &UnitFaction,
            &MovementPoint,
            &ActionState,
        )>()
        .iter(world)
        .find(|(occ, _, _, _, _)| **occ == occupant)
        .map(|(_, pos, unit_faction, movement_point, action_state)| {
            let used = match action_state {
                ActionState::Moved { cost } => *cost,
                ActionState::Done => movement_point.0 as MovementCost * 2, // 已結束，無剩餘預算
            };
            (*pos, unit_faction.0, movement_point.0 as MovementCost, used)
        })
        .ok_or_else(|| BoardError::OccupantNotFound { occupant })?;

    let board = get_board(world)?;
    let units = get_all_units(world)?;
    let objects = get_all_objects(world)?;
    let level_config = get_level_config(world)?;

    // 計算可用預算（2 倍移動力 - 已使用的）
    let budget = movement_point * 2 - movement_used;

    // 構建陣營 ID -> alliance ID 的對應表
    let faction_to_alliance: HashMap<ID, ID> = level_config
        .factions
        .iter()
        .map(|(id, f)| (*id, f.alliance))
        .collect();

    // 構建 occupant closure（查詢位置上的佔據者所屬的同盟）
    let get_occupant_alliance = |pos: Position| -> Option<ID> {
        // 先查詢單位
        if let Some(unit) = units.get(&pos) {
            let faction_id = unit.unit_faction.0;
            return Some(
                faction_to_alliance
                    .get(&faction_id)
                    .copied()
                    .unwrap_or_else(|| {
                        unreachable!("faction_id {} 不存在於 faction_to_alliance", faction_id)
                    }),
            );
        }
        // 無單位，返回 None
        None
    };

    // 構建 terrain_cost closure（物件消耗疊加在基礎消耗上）
    let get_terrain_cost = |pos: Position| -> MovementCost {
        let base_cost = BASIC_MOVEMENT_COST;
        if let Some(obj) = objects.get(&pos) {
            base_cost + obj.bundle.terrain_movement_cost.0
        } else {
            base_cost
        }
    };

    let mover_alliance =
        faction_to_alliance
            .get(&faction)
            .copied()
            .ok_or_else(|| DataError::InvalidComponent {
                name: short_type_name::<UnitFaction>(),
                note: format!("faction_id {} 在 faction_to_alliance 中找不到對應", faction),
            })?;

    let mover = Mover {
        pos: unit_pos,
        faction_alliance: mover_alliance,
    };

    reachable_positions(
        board,
        mover,
        budget,
        get_occupant_alliance,
        get_terrain_cost,
    )
}

/// 執行單位移動到指定位置
///
/// 執行步驟：
/// 1. 驗證目標位置可到達
/// 2. 計算移動路徑
/// 3. 更新單位位置
/// 4. 累加 MovementUsed
///
/// # 驗證：
/// - 目標位置不能被佔據（友軍或敵軍）
/// - 目標位置必須在可到達集合內
/// - 移動消耗不能超過可用預算
pub fn execute_move(world: &mut World, target: Position) -> Result<MoveResult> {
    // 從 TurnOrder 取得當前行動單位
    let turn_order =
        world
            .get_resource::<TurnOrder>()
            .ok_or_else(|| DataError::MissingResource {
                name: short_type_name::<TurnOrder>(),
                note: "請先呼叫 start_new_round".to_string(),
            })?;
    let occupant = get_active_unit(&turn_order.entries).ok_or(BoardError::NoActiveUnit)?;

    // 驗證佔據者存在並取得起點位置與 Entity
    let (entity, start_pos) = world
        .query::<(Entity, &Occupant, &Position)>()
        .iter(world)
        .find(|(_, occ, _)| **occ == occupant)
        .map(|(entity, _, pos)| (entity, *pos))
        .ok_or_else(|| BoardError::OccupantNotFound { occupant })?;

    // 計算可到達位置
    let reachable = get_reachable_positions(world, occupant)?;

    // 驗證目標位置是否可到達且不是僅穿越位置
    let reach_info = reachable
        .get(&target)
        .ok_or_else(|| BoardError::Unreachable {
            x: target.x,
            y: target.y,
        })?;

    // 目標位置不能是友軍佔據的位置（僅穿越）
    if reach_info.passthrough_only {
        return Err(BoardError::Unreachable {
            x: target.x,
            y: target.y,
        }
        .into());
    }

    let cost_to_target = reach_info.cost;

    // 計算路徑
    let path = reconstruct_path(&reachable, start_pos, target);

    // 更新位置和消耗
    let mut entity_mut = world.entity_mut(entity);
    {
        let mut pos =
            entity_mut
                .get_mut::<Position>()
                .ok_or_else(|| DataError::MissingComponent {
                    name: short_type_name::<Position>(),
                })?;
        *pos = target;
    }
    {
        let mut action_state =
            entity_mut
                .get_mut::<ActionState>()
                .ok_or_else(|| DataError::MissingComponent {
                    name: short_type_name::<ActionState>(),
                })?;
        match action_state.as_ref() {
            ActionState::Moved { cost } => {
                *action_state = ActionState::Moved {
                    cost: cost + cost_to_target,
                };
            }
            ActionState::Done => {
                // 理論上不會到這裡，因為 budget 為 0 時 reachable 為空
                unreachable!("ActionState::Done 時不應有可到達位置");
            }
        }
    }

    Ok(MoveResult {
        path,
        cost: cost_to_target,
    })
}

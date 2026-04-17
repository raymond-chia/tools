//! ECS 移動操作函數

use super::{get_component, get_component_mut};
use crate::domain::alias::{ID, MovementCost};
use crate::domain::constants::BASIC_MOVEMENT_COST;
use crate::ecs_logic::query::{
    build_faction_alliance_map, find_entity_by_occupant, get_resource, resolve_alliance,
};
use crate::ecs_types::components::{
    ActionState, MovementPoint, Object, ObjectMovementCost, Occupant, Position, Unit, UnitFaction,
};
use crate::ecs_types::resources::{Board, TurnOrder};
use crate::error::{BoardError, Result};
use crate::logic::movement::{Mover, ReachableInfo, reachable_positions, reconstruct_path};
use crate::logic::turn_order::get_active_unit;
use bevy_ecs::prelude::{With, World};
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
    let entity = find_entity_by_occupant(world, occupant)?;
    let entity_ref = world.entity(entity);
    let unit_pos = *get_component!(entity_ref, Position)?;
    let faction = get_component!(entity_ref, UnitFaction)?.0;
    let movement_point = get_component!(entity_ref, MovementPoint)?.0 as MovementCost;
    let movement_used = match get_component!(entity_ref, ActionState)? {
        ActionState::Moved { cost } => *cost,
        ActionState::Done => movement_point * 2,
    };

    let board = *get_resource::<Board>(world, "請先呼叫 spawn_level")?;
    let units_faction = get_units_faction_map(world)?;
    let objects_movement_cost = get_objects_movement_cost_map(world)?;
    let faction_to_alliance = build_faction_alliance_map(world)?;

    // 計算可用預算（2 倍移動力 - 已使用的）
    let budget = movement_point * 2 - movement_used;

    // 構建 occupant closure（查詢位置上的佔據者所屬的同盟）
    let get_occupant_alliance = |pos: Position| -> Option<ID> {
        units_faction.get(&pos).and_then(|faction_id| {
            faction_to_alliance
                .get(faction_id)
                .copied()
                .or_else(|| unreachable!("faction_id {} 不存在於 faction_to_alliance", faction_id))
        })
    };

    // 構建 terrain_cost closure（物件消耗疊加在基礎消耗上）
    let get_terrain_cost = |pos: Position| -> MovementCost {
        let base_cost = BASIC_MOVEMENT_COST;
        base_cost + objects_movement_cost.get(&pos).copied().unwrap_or(0)
    };

    let mover_alliance = resolve_alliance(&faction_to_alliance, faction)?;

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
    let turn_order = get_resource::<TurnOrder>(world, "請先呼叫 start_new_round")?;
    let occupant = get_active_unit(&turn_order.entries).ok_or(BoardError::NoActiveUnit)?;

    // 驗證佔據者存在並取得起點位置與 Entity
    let entity = find_entity_by_occupant(world, occupant)?;
    let start_pos = *get_component!(world.entity(entity), Position)?;

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
        let mut pos = get_component_mut!(entity_mut, Position)?;
        *pos = target;
    }
    {
        let mut action_state = get_component_mut!(entity_mut, ActionState)?;
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

// ============================================================================
// 移動系統專用查詢函式（精簡快照，避免複製整份 bundle）
// ============================================================================

/// 取得單位 faction 對應，只複製必要的 faction_id 欄位
fn get_units_faction_map(world: &mut World) -> Result<HashMap<Position, ID>> {
    let mut result = HashMap::new();
    let mut query = world.query_filtered::<(&Position, &UnitFaction), With<Unit>>();

    for (position, unit_faction) in query.iter(world) {
        result.insert(*position, unit_faction.0);
    }
    Ok(result)
}

/// 取得物件地形消耗對應，只複製必要的 terrain_movement_cost 欄位
fn get_objects_movement_cost_map(world: &mut World) -> Result<HashMap<Position, MovementCost>> {
    let mut result = HashMap::new();
    let mut query = world.query_filtered::<(&Position, &ObjectMovementCost), With<Object>>();

    for (position, movement_cost) in query.iter(world) {
        result.insert(*position, movement_cost.0);
    }
    Ok(result)
}

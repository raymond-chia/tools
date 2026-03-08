//! 回合順序 ECS 操作函數

use crate::domain::constants::PLAYER_FACTION_ID;
use crate::ecs_types::components::{Initiative, MovementUsed, Occupant, Unit, UnitFaction};
use crate::ecs_types::resources::TurnOrder;
use crate::error::{BoardError, DataError, Result};
use crate::logic::debug::short_type_name;
use crate::logic::turn_order::{self, TurnOrderInput};
use bevy_ecs::prelude::{Entity, With, World};
use rand::RngExt;

/// 查詢單位、擲骰、計算順序、插入 TurnOrder
fn insert_turn_order(world: &mut World, round: u32) {
    let inputs: Vec<TurnOrderInput> = world
        .query_filtered::<(&Occupant, &Initiative, &UnitFaction), With<Unit>>()
        .iter(world)
        .map(|(occupant, initiative, unit_faction)| TurnOrderInput {
            occupant: *occupant,
            initiative: initiative.0,
            is_player: unit_faction.0 == PLAYER_FACTION_ID,
        })
        .collect();

    // 純邏輯：計算回合順序
    let mut rng_int = rand::rng();
    let mut rng_float = rand::rng();
    let entries =
        turn_order::calculate_turn_order(&inputs, &mut || rng_int.random_range(1..=6), &mut || {
            rng_float.random_range(0.001..0.999)
        });

    world.insert_resource(TurnOrder {
        round,
        entries,
        current_index: 0,
    });
}

/// 從 World 取得 TurnOrder 的內部 helper
fn require_turn_order(world: &World) -> Result<&TurnOrder> {
    world.get_resource::<TurnOrder>().ok_or_else(|| {
        DataError::MissingResource {
            name: short_type_name::<TurnOrder>(),
            note: "請先呼叫 start_new_round".to_string(),
        }
        .into()
    })
}

/// 開始新的一輪（擲骰、排序、存入 TurnOrder Resource）並回傳
pub fn start_new_round(world: &mut World) -> Result<&TurnOrder> {
    // 讀取：檢查是否已存在 TurnOrder
    let existing = world.get_resource::<TurnOrder>();
    if existing.is_some() {
        return Err(DataError::ResourceAlreadyExists {
            name: short_type_name::<TurnOrder>(),
            note: "請先呼叫 end_battle 結束上一場戰鬥".to_string(),
        }
        .into());
    }

    insert_turn_order(world, 1);

    require_turn_order(world)
}

/// 結束當前單位的回合，推進到下一個；若全部結束則自動開始下一輪
pub fn end_current_turn(world: &mut World) -> Result<&TurnOrder> {
    // 讀寫：標記當前單位已行動，取得其 Occupant，檢查是否還有未行動的單位
    let turn_order =
        world
            .get_resource_mut::<TurnOrder>()
            .ok_or_else(|| DataError::MissingResource {
                name: short_type_name::<TurnOrder>(),
                note: "請先呼叫 start_new_round".to_string(),
            })?;
    let inner = turn_order.into_inner();
    inner.entries[inner.current_index].has_acted = true;
    let current_occupant = inner.entries[inner.current_index].occupant;
    let next_idx = inner.entries.iter().position(|e| !e.has_acted);

    match next_idx {
        Some(idx) => {
            // 還有未行動的單位，推進 current_index
            inner.current_index = idx;
        }
        None => {
            // 所有單位都已行動，開始新一輪
            let prev_round = inner.round;
            insert_turn_order(world, prev_round + 1);
        }
    }

    // 重置當前單位的 MovementUsed
    let (_, mut movement_used) = world
        .query::<(&Occupant, &mut MovementUsed)>()
        .iter_mut(world)
        .find(|(occ, _)| **occ == current_occupant)
        .ok_or_else(|| BoardError::OccupantNotFound {
            occupant: current_occupant,
        })?;
    movement_used.0 = 0;

    require_turn_order(world)
}

/// 查詢當前單位是否可延遲（未移動才可延遲）
pub fn can_delay_current_unit(world: &mut World) -> Result<bool> {
    let turn_order = require_turn_order(world)?;
    let current_occupant = turn_order.entries[turn_order.current_index].occupant;

    let (_, movement_used) = world
        .query::<(&Occupant, &MovementUsed)>()
        .iter(world)
        .find(|(occ, _)| **occ == current_occupant)
        .ok_or_else(|| BoardError::OccupantNotFound {
            occupant: current_occupant,
        })?;

    Ok(movement_used.0 == 0)
}

/// 延後當前單位到指定位置
pub fn delay_current_unit(world: &mut World, target_index: usize) -> Result<&TurnOrder> {
    // 檢查當前單位是否已行動（移動過就不能延遲）
    if !can_delay_current_unit(world)? {
        let turn_order = require_turn_order(world)?;
        let current_occupant = turn_order.entries[turn_order.current_index].occupant;
        return Err(BoardError::InvalidDelay {
            occupant: current_occupant,
            reason: "已移動的單位無法延遲".to_string(),
        }
        .into());
    }

    // 讀寫：延後單位並更新 current_index
    let inner = world
        .get_resource_mut::<TurnOrder>()
        .ok_or_else(|| DataError::MissingResource {
            name: short_type_name::<TurnOrder>(),
            note: "請先呼叫 start_new_round".to_string(),
        })?
        .into_inner();
    turn_order::delay_unit(&mut inner.entries, target_index)?;
    inner.current_index = inner
        .entries
        .iter()
        .position(|e| !e.has_acted)
        .unwrap_or_else(|| unreachable!("delay 後必定存在未行動的單位"));

    require_turn_order(world)
}

/// 移除死亡單位
pub fn remove_dead_unit(world: &mut World, occupant: Occupant) -> Result<&TurnOrder> {
    // 讀取：找到對應的 Entity
    let entity = world
        .query::<(Entity, &Occupant)>()
        .iter(world)
        .find(|(_, occ)| **occ == occupant)
        .map(|(entity, _)| entity)
        .ok_or_else(|| BoardError::OccupantNotFound { occupant })?;

    // 讀寫：從回合表移除並更新 current_index
    let inner = world
        .get_resource_mut::<TurnOrder>()
        .ok_or_else(|| DataError::MissingResource {
            name: short_type_name::<TurnOrder>(),
            note: "請先呼叫 start_new_round".to_string(),
        })?
        .into_inner();
    turn_order::remove_unit(&mut inner.entries, occupant)?;
    let next_idx = inner.entries.iter().position(|e| !e.has_acted);
    let prev_round = inner.round;

    match next_idx {
        Some(idx) => inner.current_index = idx,
        None => { /* 全部行動完畢，despawn 後開始新一輪 */ }
    }

    // 寫入：despawn Entity
    world.despawn(entity);

    // 若所有單位都已行動，開始新一輪
    if next_idx.is_none() {
        insert_turn_order(world, prev_round + 1);
    }

    require_turn_order(world)
}

/// 查詢當前回合狀態
// TODO: 未來檢查是否真的被 editor 用到，若沒有則刪除
pub fn get_turn_order(world: &World) -> Result<&TurnOrder> {
    require_turn_order(world)
}

/// 結束戰鬥，清理 TurnOrder Resource
pub fn end_battle(world: &mut World) -> Result<()> {
    world.remove_resource::<TurnOrder>();
    Ok(())
}

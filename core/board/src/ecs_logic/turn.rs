//! 回合順序 ECS 操作函數

use super::{get_component, get_component_mut};
use crate::domain::alias::TypeName;
use crate::domain::battle_log::LogEvent;
use crate::domain::constants::PLAYER_FACTION_ID;
use crate::ecs_logic::query::{find_entity_by_occupant, get_resource, get_resource_mut};
use crate::ecs_types::components::{
    ActionState, AppliedBuff, CurrentHp, Initiative, MaxReactionPoint, Occupant, OccupantTypeName,
    ReactionPoint, Unit, UnitFaction,
};
use crate::ecs_types::resources::{BattleLog, ReactionState, TurnOrder};
use crate::error::{BoardError, DataError, Result};
use crate::logic::debug::short_type_name;
use crate::logic::turn_order::{self, TurnOrderInput};
use bevy_ecs::prelude::{With, World};
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
    get_resource::<TurnOrder>(world, "請先呼叫 start_new_round")
}

/// 整輪輪替時呼叫:所有 buff 的 remaining_duration 減 1(None 視為無限期,不動)
fn tick_buff_durations(world: &mut World) {
    let mut query = world.query::<&mut AppliedBuff>();
    for mut buff in query.iter_mut(world) {
        if let Some(remaining) = buff.remaining_duration {
            buff.remaining_duration = Some(remaining.saturating_sub(1));
        }
    }
}

/// 開新一輪的單一入口：所有 buff 剩餘回合 -1，再重新擲骰排序為下一輪。
///
/// 由 `end_current_turn`（全員行動完畢換輪）與 `resolve_deaths`
/// （批次死光剩餘單位換輪）共同呼叫，確保兩條換輪路徑的副作用一致——
/// 避免某一條漏 tick buff 造成存活單位 buff 剩餘回合不遞減。
fn advance_to_new_round(world: &mut World, prev_round: u32) {
    tick_buff_durations(world);
    insert_turn_order(world, prev_round + 1);
}

/// 單位回合開始時呼叫:移除該單位身上已過期(remaining_duration == Some(0))的 buff
fn remove_expired_buffs_for(world: &mut World, occupant: Occupant) {
    let expired: Vec<bevy_ecs::entity::Entity> = world
        .query::<(bevy_ecs::entity::Entity, &AppliedBuff)>()
        .iter(world)
        .filter(|(_, buff)| buff.target == occupant && buff.remaining_duration == Some(0))
        .map(|(entity, _)| entity)
        .collect();

    for entity in expired {
        world.despawn(entity);
    }
}

/// 清除掛在指定 occupants 身上（target 在集合內）的所有 buff entity。
///
/// 由 `resolve_deaths` 在死者 despawn 後呼叫：buff 是獨立 entity，不會隨死者
/// entity 一併移除，須主動清除避免留下孤兒 buff。
fn remove_buffs_targeting(world: &mut World, targets: &[Occupant]) {
    let orphaned: Vec<bevy_ecs::entity::Entity> = world
        .query::<(bevy_ecs::entity::Entity, &AppliedBuff)>()
        .iter(world)
        .filter(|(_, buff)| targets.contains(&buff.target))
        .map(|(entity, _)| entity)
        .collect();

    for entity in orphaned {
        world.despawn(entity);
    }
}

/// 單位回合開始流程：移除該單位身上已過期的 buff。
///
/// 由 `end_current_turn`（推進到下一個單位）與 `resolve_deaths`
/// （死當前單位使下一個單位遞補為當前）共同呼叫，作為「回合開始」的單一入口。
fn begin_unit_turn(world: &mut World, occupant: Occupant) {
    remove_expired_buffs_for(world, occupant);
}

/// 單位回合結束流程：重置該單位的行動狀態與反應點數，為其下一輪預備。
fn end_unit_turn(world: &mut World, occupant: Occupant) -> Result<()> {
    let entity = find_entity_by_occupant(world, occupant)?;
    let mut entity_mut = world.entity_mut(entity);
    {
        let mut action_state = get_component_mut!(entity_mut, ActionState)?;
        *action_state = ActionState::Moved { cost: 0 };
    }
    {
        let max_reaction_point = get_component_mut!(entity_mut, MaxReactionPoint)?.0;
        let mut reaction_point = get_component_mut!(entity_mut, ReactionPoint)?;
        reaction_point.0 = max_reaction_point;
    }
    Ok(())
}

/// 取得 TurnOrder 當前行動單位
///
/// `current_index` 是行動單位的單一真相，由 `start_new_round`、
/// `end_current_turn`、`delay_current_unit`、`resolve_deaths` 維護
/// 永遠指向有效的未行動者。
pub fn get_current_unit(turn_order: &TurnOrder) -> Result<Occupant> {
    turn_order
        .entries
        .get(turn_order.current_index)
        .map(|entry| entry.occupant)
        .ok_or_else(|| BoardError::NoActiveUnit.into())
}

/// 開始新的一輪（擲骰、排序、存入 TurnOrder Resource）並回傳
pub fn start_new_round(world: &mut World) -> Result<&TurnOrder> {
    // 讀取：檢查是否已存在 TurnOrder
    if world.contains_resource::<TurnOrder>() {
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
pub fn end_current_turn(world: &mut World) -> Result<()> {
    // 讀寫：標記當前單位已行動，取得其 Occupant，檢查是否還有未行動的單位
    let turn_order = get_resource_mut::<TurnOrder>(world, "請先呼叫 start_new_round")?;
    let inner = turn_order.into_inner();
    let current_index = inner.current_index;
    let current_occupant = inner.entries[current_index].occupant;

    inner.entries[current_index].has_acted = true;
    let next_idx = turn_order::get_active_index(&inner.entries);
    match next_idx {
        Some(idx) => {
            // 還有未行動的單位，推進 current_index
            inner.current_index = idx;
        }
        None => {
            // 所有單位都已行動，開始新一輪
            let prev_round = inner.round;
            advance_to_new_round(world, prev_round);
        }
    }

    // 新一輪時 current_index 歸 0,此處統一取推進後的當前單位
    let next_occupant = {
        let turn_order = require_turn_order(world)?;
        get_current_unit(turn_order)?
    };

    // 剛結束回合的單位：重置行動狀態與反應點數
    end_unit_turn(world, current_occupant)?;
    // 下一個單位的回合開始
    begin_unit_turn(world, next_occupant);

    Ok(())
}

/// 查詢當前單位是否可延遲（未移動才可延遲）
pub fn can_delay_current_unit(world: &mut World) -> Result<bool> {
    let turn_order = require_turn_order(world)?;
    let current_occupant = get_current_unit(turn_order)?;

    let entity = find_entity_by_occupant(world, current_occupant)?;
    let action_state = get_component!(world.entity(entity), ActionState)?;

    Ok(matches!(action_state, ActionState::Moved { cost: 0 }))
}

/// 延後當前單位到指定位置
pub fn delay_current_unit(world: &mut World, target_index: usize) -> Result<()> {
    // 檢查當前單位是否已行動（移動過就不能延遲）
    if !can_delay_current_unit(world)? {
        let turn_order = require_turn_order(world)?;
        let current_occupant = get_current_unit(turn_order)?;
        return Err(BoardError::InvalidDelay {
            occupant: current_occupant,
            reason: "已移動的單位無法延遲".to_string(),
        }
        .into());
    }

    // 讀寫：延後單位並更新 current_index
    let inner = get_resource_mut::<TurnOrder>(world, "請先呼叫 start_new_round")?.into_inner();
    turn_order::delay_unit(&mut inner.entries, target_index)?;
    inner.current_index = turn_order::get_active_index(&inner.entries)
        .unwrap_or_else(|| unreachable!("delay 後必定存在未行動的單位"));

    Ok(())
}

/// 掃描全場 HP≤0 的單位、批次移出 TurnOrder 並 despawn，產生死亡 log
///
/// 批次語意：先收集所有死者，全部移除後**只判一次**是否全員行動完畢、
/// 要不要開新一輪。逐個移除各判一次會在 AOE 多死時提前誤開新一輪。
///
/// 對「無 `ReactionState`」（如 `execute_skill` 後）安全處理：沒有 pending
/// 可剔除就只做移除；有則一併把死者剔出 pending，避免死者出現在反應面板。
pub fn resolve_deaths(world: &mut World) -> Result<()> {
    // === 讀取階段：收集死者（Entity、Occupant、名稱快照）===
    let dead_units: Vec<(bevy_ecs::entity::Entity, Occupant, TypeName)> = world
        .query_filtered::<(
            bevy_ecs::entity::Entity,
            &Occupant,
            &CurrentHp,
            &OccupantTypeName,
        ), With<Unit>>()
        .iter(world)
        .filter(|(_, _, hp, _)| hp.0 <= 0)
        .map(|(entity, occupant, _, type_name)| (entity, *occupant, type_name.0.clone()))
        .collect();

    if dead_units.is_empty() {
        return Ok(());
    }

    // 移除前的當前單位，用於判斷遞補後當前單位是否改變（改變才跑回合開始）
    let prev_current = get_current_unit(require_turn_order(world)?)?;

    // === 純邏輯階段：產生死亡 log 事件（只記身分名稱快照）===
    let death_events: Vec<LogEvent> = dead_units
        .iter()
        .map(|(_, _, type_name)| LogEvent::Death {
            unit: type_name.clone(),
        })
        .collect();

    let dead_occupants: Vec<Occupant> = dead_units
        .iter()
        .map(|(_, occupant, _)| *occupant)
        .collect();

    // === 寫入階段 ===
    // append 死亡 log
    get_resource_mut::<BattleLog>(world, "請先呼叫 spawn_level")?
        .into_inner()
        .0
        .extend(death_events);

    // 從 pending 剔除死者（無 ReactionState 則跳過）
    if let Some(mut state) = world.get_resource_mut::<ReactionState>() {
        state
            .pending
            .retain(|reaction| !dead_occupants.contains(&reaction.reactor));
    }

    // 批次從回合表移除所有死者，最後只判一次是否開新一輪。
    // 全員行動完畢（無未行動者）回傳開新一輪所需的下一輪輪數，否則回傳 None。
    for (entity, _, _) in &dead_units {
        world.despawn(*entity);
    }
    // 清除掛在死者身上的孤兒 buff entity（buff 是獨立 entity，不隨死者一併移除）
    remove_buffs_targeting(world, &dead_occupants);
    let is_new_round = {
        let inner = get_resource_mut::<TurnOrder>(world, "請先呼叫 start_new_round")?.into_inner();
        for (_, occupant, _) in &dead_units {
            turn_order::remove_unit(&mut inner.entries, *occupant)?;
        }
        match turn_order::get_active_index(&inner.entries) {
            Some(idx) => {
                inner.current_index = idx;
                false
            }
            None => {
                let prev_round = inner.round;
                advance_to_new_round(world, prev_round);
                true
            }
        }
    };

    // 死當前單位使下一個單位遞補為當前、或批次死光後開新一輪 → 新當前單位跑回合開始。
    // 死非當前單位（且未換輪）時當前單位不變，不重跑回合開始。
    // 換輪時即使新當前與原當前是同一 occupant，仍屬新一輪的回合開始，須跑。
    let new_current = get_current_unit(require_turn_order(world)?)?;
    if is_new_round || new_current != prev_current {
        begin_unit_turn(world, new_current);
    }

    Ok(())
}

/// 查詢當前回合狀態
pub fn get_turn_order(world: &World) -> Result<&TurnOrder> {
    require_turn_order(world)
}

// TODO 有用到嗎 ?
/// 結束戰鬥，清理 TurnOrder Resource
pub fn end_battle(world: &mut World) -> Result<()> {
    world.remove_resource::<TurnOrder>();
    Ok(())
}

//! 戰鬥 log 產生 ECS 操作函數
//!
//! 把技能/反應執行產出的 `EffectEntry`（動畫用低階結算資料）轉成
//! 人類可讀、自帶名稱快照的 `LogEvent`，append 到 `BattleLog` Resource。
//!
//! 轉換必須在此層（ecs_logic）做：`EffectEntry` 只帶 caster ID 與 CheckTarget，
//! 名稱快照需查 World 取得 `OccupantTypeName`。所有名稱快照（caster/reactor/
//! trigger/target）都在此查 World，呼叫端不需為 log 預先查任何 `OccupantTypeName`，
//! 避免把 log 的需求洩漏成 `execute_skill`／`process_reactions` 的額外副作用。
//! 反應特有的 trigger 不在 `EffectEntry` 內，由呼叫端以 `Occupant` 傳入身分。

use crate::domain::alias::{ID, TypeName};
use crate::domain::battle_log::{LogCheck, LogCheckDetail, LogEffect, LogEvent, LogTarget};
use crate::ecs_logic::get_component;
use crate::ecs_logic::query::{find_entity_by_occupant, get_resource_mut};
use crate::ecs_types::components::{Object, Occupant, OccupantTypeName, Position};
use crate::ecs_types::resources::BattleLog;
use crate::error::Result;
use crate::logic::skill::skill_execution::{
    CheckDetail, CheckResult, CheckTarget, EffectEntry, ResolvedEffect,
};
use bevy_ecs::prelude::{With, World};

/// 將技能執行的 `EffectEntry` 序列轉成技能 log 事件並 append 到 BattleLog
///
/// 施放者名稱快照由本函數從 entry 的 caster ID 查 World 取得，呼叫端無需預先查。
///
/// 由呼叫端（editor）在 `execute_skill` 之後明確呼叫，core 不自動 append。
pub fn append_skill_log(world: &mut World, entries: &[EffectEntry]) -> Result<()> {
    let events = entries
        .iter()
        .map(|entry| {
            let caster = resolve_unit_name(world, entry.caster)?;
            let (target, check, check_detail, effect) = build_log_parts(world, entry)?;
            Ok(LogEvent::Skill {
                caster,
                skill_name: entry.skill_name.clone(),
                target,
                check,
                check_detail,
                effect,
            })
        })
        .collect::<Result<Vec<LogEvent>>>()?;

    append_events(world, events)
}

/// 將反應執行的 `EffectEntry` 序列轉成反應 log 事件並 append 到 BattleLog
///
/// reactor 名稱快照由本函數從 entry 的 caster ID 查 World 取得（反應者即 caster）；
/// trigger 不在 `EffectEntry` 內，由呼叫端以 `Occupant` 傳入身分、本函數查其名稱。
///
/// 由呼叫端（editor）在 `process_reactions` 回傳 `Executed` 之後明確呼叫，
/// trigger 取自 `Executed { trigger }`。core 不自動 append。
pub fn append_reaction_log(
    world: &mut World,
    trigger: Occupant,
    entries: &[EffectEntry],
) -> Result<()> {
    let trigger_name = resolve_occupant_name(world, trigger)?;

    let events = entries
        .iter()
        .map(|entry| {
            let reactor = resolve_unit_name(world, entry.caster)?;
            let (target, check, check_detail, effect) = build_log_parts(world, entry)?;
            Ok(LogEvent::Reaction {
                reactor,
                trigger: trigger_name.clone(),
                skill_name: entry.skill_name.clone(),
                target,
                check,
                check_detail,
                effect,
            })
        })
        .collect::<Result<Vec<LogEvent>>>()?;

    append_events(world, events)
}

/// 把 log 事件序列 append 到 BattleLog Resource
fn append_events(world: &mut World, events: Vec<LogEvent>) -> Result<()> {
    get_resource_mut::<BattleLog>(world, "請先呼叫 spawn_level")?
        .into_inner()
        .0
        .extend(events);
    Ok(())
}

/// 從單筆 `EffectEntry` 解析出 log 所需的 target/check/check_detail/effect
fn build_log_parts(
    world: &mut World,
    entry: &EffectEntry,
) -> Result<(LogTarget, LogCheck, Option<LogCheckDetail>, LogEffect)> {
    let target = resolve_log_target(world, entry.target, &entry.effect)?;
    let check = to_log_check(entry.check);
    let check_detail = entry.check_detail.as_ref().map(to_log_check_detail);
    let effect = to_log_effect(&entry.effect);
    Ok((target, check, check_detail, effect))
}

/// 查單位 ID 的 type name 名稱快照（查不到時 fail fast）
fn resolve_unit_name(world: &mut World, id: ID) -> Result<TypeName> {
    resolve_occupant_name(world, Occupant::Unit(id))
}

/// 查 `Occupant` 的 type name 名稱快照（查不到時 fail fast）
fn resolve_occupant_name(world: &mut World, occupant: Occupant) -> Result<TypeName> {
    let entity = find_entity_by_occupant(world, occupant)?;
    Ok(get_component!(world.entity(entity), OccupantTypeName)?
        .0
        .clone())
}

/// 解析 log 目標的名稱快照（查不到目標名稱時 fail fast）
///
/// 召喚效果（`SpawnObject`）的目標名稱直接取自 effect 帶的 `object_type`，
/// 不查 World：同一格可疊多個物件，事後查 World 無法可靠取得「剛召喚的那個」
/// （ECS query 迭代順序與隨機 ID 都不代表 spawn 時間序）。effect 本身已帶名稱，
/// 直接用之，語意精確且繞開多物件歧義。
fn resolve_log_target(
    world: &mut World,
    target: CheckTarget,
    effect: &ResolvedEffect,
) -> Result<LogTarget> {
    if let ResolvedEffect::SpawnObject { object_type } = effect {
        return Ok(LogTarget::Object {
            name: object_type.clone(),
        });
    }

    match target {
        CheckTarget::Unit(id) => Ok(LogTarget::Unit {
            name: resolve_unit_name(world, id)?,
        }),
        CheckTarget::Position(pos) => Ok(resolve_position_target(world, pos)),
    }
}

/// 解析位置目標：該格有物件則記物件名稱快照，否則視為空地
fn resolve_position_target(world: &mut World, pos: Position) -> LogTarget {
    let object_name: Option<TypeName> = world
        .query_filtered::<(&Position, &OccupantTypeName), With<Object>>()
        .iter(world)
        .find(|(object_pos, _)| **object_pos == pos)
        .map(|(_, type_name)| type_name.0.clone());

    match object_name {
        Some(name) => LogTarget::Object { name },
        None => LogTarget::EmptyGround,
    }
}

/// `CheckResult`（動畫用）→ `LogCheck`（log 用）
fn to_log_check(check: CheckResult) -> LogCheck {
    match check {
        CheckResult::Auto => LogCheck::Auto,
        CheckResult::Hit { crit } => LogCheck::Hit { crit },
        CheckResult::Block { crit } => LogCheck::Block { crit },
        CheckResult::Evade => LogCheck::Evade,
        CheckResult::Resisted => LogCheck::Resisted,
        CheckResult::Affected => LogCheck::Affected,
    }
}

/// `CheckDetail`（動畫用）→ `LogCheckDetail`（log 用）
fn to_log_check_detail(detail: &CheckDetail) -> LogCheckDetail {
    LogCheckDetail {
        accuracy_source: detail.accuracy_source.clone(),
        defense_type: detail.defense_type,
        attacker_accuracy: detail.attacker_accuracy,
        defender_evasion: detail.defender_evasion,
        defender_block: detail.defender_block,
        crit_rate: detail.crit_rate,
        roll: detail.roll,
    }
}

/// `ResolvedEffect`（動畫用）→ `LogEffect`（log 用，HpChange 取最終值）
fn to_log_effect(effect: &ResolvedEffect) -> LogEffect {
    match effect {
        ResolvedEffect::NoEffect => LogEffect::None,
        ResolvedEffect::HpChange { final_amount, .. } => LogEffect::HpChange {
            amount: *final_amount,
        },
        ResolvedEffect::SpawnObject { object_type } => LogEffect::SpawnObject {
            object_type: object_type.clone(),
        },
        ResolvedEffect::ApplyBuff(buff_name) => LogEffect::ApplyBuff {
            buff_name: buff_name.clone(),
        },
    }
}

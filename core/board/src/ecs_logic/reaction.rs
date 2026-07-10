//! ECS 反應系統操作函數

use super::{get_component, get_component_mut};
use crate::domain::alias::{ID, SkillName};
use crate::domain::core_types::PendingReaction;
use crate::ecs_logic::query::{
    build_faction_alliance_map, build_objects_on_board, build_unit_stats_on_board,
    find_entity_by_occupant, get_reaction_skill_data, get_resource, get_resource_mut,
    read_attribute_bundle, resolve_alliance,
};
use crate::ecs_logic::skill::apply_effect_entries;
use crate::ecs_types::components::{
    CurrentMp, Occupant, Position, ReactionPoint, Skills, Unit, UnitFaction,
};
use crate::ecs_types::resources::{Board, GameData, ReactionState};
use crate::error::{DataError, ReactionError, Result, UnitError};
use crate::logic::skill::UnitInfo;
use crate::logic::skill::skill_execution::{CombatStats, EffectEntry, resolve_effect_tree};
use crate::logic::skill::skill_reaction::{TakesDamageUnitInfo, collect_takes_damage_reactions};
use bevy_ecs::prelude::{Entity, With, World};
use rand::RngExt;
use std::collections::{HashMap, HashSet};

/// 反應執行結果
#[derive(Debug)]
pub enum ProcessReactionResult {
    /// 成功執行一個反應，附帶效果清單與觸發者
    ///
    /// 單次反應只對應單一 trigger，
    /// 供呼叫端產生反應 log（`append_reaction_log`）時使用。
    Executed {
        effects: Vec<EffectEntry>,
        trigger: Occupant,
    },
    /// 需要玩家決策（新的 pending reactions 已寫入 ReactionState）
    NeedDecision,
    /// 所有反應已處理完畢
    Done,
}

/// 取得目前待決策的反應清單
///
/// 若 ReactionState 不存在或 pending 為空，回傳空 Vec
pub fn get_pending_reactions(world: &World) -> Vec<PendingReaction> {
    match world.get_resource::<ReactionState>() {
        Some(state) => state.pending.clone(),
        None => vec![],
    }
}

/// 設定反應決策，將 pending 轉換為 queue
///
/// - `decisions`：[(reactor, skill_name)] 按執行順序排列，未出現的 reactor 視為放棄
/// - 若 ReactionState 不存在或 pending 為空，回傳 Err
/// - 若 reactor 不在 pending 清單中，或技能不在可用清單中，回傳 Err
pub fn set_reactions(world: &mut World, decisions: Vec<(Occupant, SkillName)>) -> Result<()> {
    let state = world
        .get_resource::<ReactionState>()
        .ok_or(ReactionError::NoPendingReactions)?;
    if state.pending.is_empty() {
        return Err(ReactionError::NoPendingReactions.into());
    }

    let pending = state.pending.clone();

    let mut queue = Vec::new();
    for (reactor, skill_name) in decisions {
        let entry = pending
            .iter()
            .find(|r| r.reactor == reactor)
            .ok_or(ReactionError::ReactorNotFound { occupant: reactor })?;

        if !entry.available_skills.contains(&skill_name) {
            return Err(UnitError::SkillNotFound { skill_name }.into());
        }

        queue.push((reactor, skill_name, entry.trigger));
    }

    // 反轉讓 pop() 可以按原始順序執行（O(1) 而非 remove(0) 的 O(n)）
    queue.reverse();

    let mut state = world
        .get_resource_mut::<ReactionState>()
        .ok_or(ReactionError::NoPendingReactions)?;
    state.pending = vec![];
    state.decided = queue;

    Ok(())
}

/// 執行 queue 中的下一個反應
pub fn process_reactions(world: &mut World) -> Result<ProcessReactionResult> {
    // === 讀取階段 ===
    let popped = match world.get_resource_mut::<ReactionState>() {
        None => return Ok(ProcessReactionResult::Done),
        Some(mut state) => state.decided.pop(),
    };

    let (reactor, skill_name, trigger) = match popped {
        None => {
            let has_pending = world
                .get_resource::<ReactionState>()
                .map_or(false, |s| !s.pending.is_empty());
            if has_pending {
                return Ok(ProcessReactionResult::NeedDecision);
            }
            world.remove_resource::<ReactionState>();
            return Ok(ProcessReactionResult::Done);
        }
        Some(entry) => entry,
    };

    let reactor_entity = find_entity_by_occupant(world, reactor)?;
    let trigger_entity = find_entity_by_occupant(world, trigger)?;

    let (reactor_pos, reactor_faction, reactor_mp, reactor_reaction_point, reactor_attributes) = {
        let entity_ref = world.entity(reactor_entity);
        let pos = *get_component!(entity_ref, Position)?;
        let faction = get_component!(entity_ref, UnitFaction)?.0;
        let mp = get_component!(entity_ref, CurrentMp)?.0;
        let reaction_point = get_component!(entity_ref, ReactionPoint)?.0;
        let attributes = read_attribute_bundle(&entity_ref)?;
        (pos, faction, mp, reaction_point, attributes)
    };

    let trigger_pos = *get_component!(world.entity(trigger_entity), Position)?;

    let faction_to_alliance = build_faction_alliance_map(world)?;
    let reactor_alliance = resolve_alliance(&faction_to_alliance, reactor_faction)?;

    let game_data = get_resource::<GameData>(world, "請先呼叫 parse_and_insert_game_data")?;
    let (_triggering, effects, cost, skill_tags) = get_reaction_skill_data(game_data, &skill_name)?;

    let board = *get_resource::<Board>(world, "請先呼叫 spawn_level")?;

    let unit_stats_on_board = build_unit_stats_on_board(world, &faction_to_alliance)?;
    let objects_on_board = build_objects_on_board(world);

    let unit_reaction_info: HashMap<Occupant, TakesDamageUnitInfo> = {
        let unit_entities: Vec<Entity> = world
            .query_filtered::<Entity, With<Unit>>()
            .iter(world)
            .collect();
        let mut map = HashMap::new();
        for unit_entity in unit_entities {
            let entity_ref = world.entity(unit_entity);
            let occupant = *get_component!(entity_ref, Occupant)?;
            let pos = *get_component!(entity_ref, Position)?;
            let remaining_reaction_point = get_component!(entity_ref, ReactionPoint)?.0;
            let skill_names = get_component!(entity_ref, Skills)?.0.clone();
            map.insert(
                occupant,
                TakesDamageUnitInfo {
                    pos,
                    remaining_reaction_point,
                    skill_names,
                },
            );
        }
        map
    };

    if reactor_reaction_point <= 0 {
        return Err(UnitError::InsufficientReactionPoint {
            current: reactor_reaction_point,
        }
        .into());
    }
    if reactor_mp < cost as i32 {
        return Err(UnitError::InsufficientMp {
            cost,
            current: reactor_mp,
        }
        .into());
    }

    let reactor_id = match reactor {
        Occupant::Unit(id) => id,
        Occupant::Object(_) => {
            return Err(DataError::InternalError {
                message: format!("反應者 {:?} 不是單位", reactor),
            }
            .into());
        }
    };

    let reactor_stats = CombatStats {
        unit_info: UnitInfo {
            occupant: reactor,
            faction_id: reactor_faction,
            alliance_id: reactor_alliance,
        },
        attribute: reactor_attributes,
    };

    let mut rng = rand::rng();
    let entries = resolve_effect_tree(
        reactor_id,
        &skill_name,
        &skill_tags,
        &effects,
        &reactor_stats,
        reactor_pos,
        trigger_pos,
        &unit_stats_on_board,
        &objects_on_board,
        board,
        &mut || rng.random_range(1..=100),
        false,
    )?;

    let game_data = get_resource::<GameData>(world, "請先呼叫 parse_and_insert_game_data")?;
    let new_pending = collect_takes_damage_reactions(
        &entries,
        reactor,
        reactor_pos,
        game_data,
        &unit_reaction_info,
        &unit_stats_on_board,
    );

    let mut used_ids: HashSet<ID> = world
        .query::<&Occupant>()
        .iter(world)
        .map(|occ| match occ {
            Occupant::Unit(id) | Occupant::Object(id) => *id,
        })
        .collect();

    {
        let mut entity_mut = world.entity_mut(reactor_entity);
        {
            let mut mp = get_component_mut!(entity_mut, CurrentMp)?;
            mp.0 -= cost as i32;
        }
        {
            let mut reaction_point = get_component_mut!(entity_mut, ReactionPoint)?;
            reaction_point.0 -= 1;
        }
    }

    apply_effect_entries(world, &entries, &mut used_ids)?;

    {
        let mut state_mut = get_resource_mut::<ReactionState>(world, "ReactionState 應存在")?;
        if !new_pending.is_empty() {
            state_mut.pending = new_pending;
        } else if state_mut.decided.is_empty() {
            drop(state_mut);
            world.remove_resource::<ReactionState>();
        }
    }

    Ok(ProcessReactionResult::Executed {
        effects: entries,
        trigger,
    })
}

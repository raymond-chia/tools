use crate::domain::alias::ID;
use crate::domain::core_types::{EndLevelCondition, LevelOutcome, OutcomeBranches};
use crate::ecs_logic::query::get_resource;
use crate::ecs_types::components::UnitFaction;
use crate::ecs_types::resources::EndConditionConfig;
use crate::error::Result;
use bevy_ecs::prelude::World;
use std::collections::HashSet;

/// 判定關卡結局（defeat 優先於 victory）並回傳結果
///
/// 存活 faction 集合只撈一次，victory、defeat 兩份 `OutcomeBranches` 共用。
pub fn resolve_level_outcome(world: &mut World) -> Result<LevelOutcome> {
    // === 讀取階段 ===
    let alive_factions: HashSet<ID> = world
        .query::<&UnitFaction>()
        .iter(world)
        .map(|faction| faction.0)
        .collect();
    let end_condition_config = get_resource::<EndConditionConfig>(world, "請先呼叫 spawn_level")?;

    // === 純邏輯階段 ===
    let outcome = match find_triggered_branch(&end_condition_config.defeat, &alive_factions) {
        Some(key) => LevelOutcome::Defeat(key),
        None => match find_triggered_branch(&end_condition_config.victory, &alive_factions) {
            Some(key) => LevelOutcome::Victory(key),
            None => LevelOutcome::Undetermined,
        },
    };

    Ok(outcome)
}

/// TODO match arms 夠多後重構
fn is_end_level_condition_met(condition: &EndLevelCondition, alive_factions: &HashSet<ID>) -> bool {
    match condition {
        EndLevelCondition::EliminateFaction(faction_id) => !alive_factions.contains(faction_id),
    }
}

/// TODO is_end_level_condition_met 夠複雜後重構
fn find_triggered_branch(
    branches: &OutcomeBranches,
    alive_factions: &HashSet<ID>,
) -> Option<String> {
    branches
        .iter()
        .find(|(_, conditions)| {
            conditions
                .iter()
                .all(|condition| is_end_level_condition_met(condition, alive_factions))
        })
        .map(|(key, _)| key.clone())
}

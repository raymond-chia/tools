//! ECS 反應系統操作函數

use crate::domain::alias::SkillName;
use crate::domain::core_types::PendingReaction;
use crate::ecs_types::components::Occupant;
use crate::ecs_types::resources::ReactionState;
use crate::error::{ReactionError, Result, UnitError};
use crate::logic::skill::skill_execution::EffectEntry;
use bevy_ecs::prelude::World;

/// 反應執行結果
#[derive(Debug)]
pub enum ProcessReactionResult {
    /// 成功執行一個反應，附帶效果清單
    Executed { effects: Vec<EffectEntry> },
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

    let mut state = world
        .get_resource_mut::<ReactionState>()
        .ok_or(ReactionError::NoPendingReactions)?;
    state.pending = vec![];
    state.queue = queue;

    Ok(())
}

/// 執行 queue 中的下一個反應
pub fn process_reactions(world: &mut World) -> Result<ProcessReactionResult> {
    unimplemented!("process_reactions 尚未實作")
}

//! ECS 反應系統操作函數

use crate::domain::alias::SkillName;
use crate::domain::core_types::PendingReaction;
use crate::ecs_types::components::Occupant;
use crate::ecs_types::resources::ReactionState;
use crate::error::Result;
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
pub fn set_reactions(world: &mut World, _decisions: Vec<(Occupant, SkillName)>) -> Result<()> {
    unimplemented!("set_reactions 尚未實作")
}

/// 執行 queue 中的下一個反應
pub fn process_reactions(world: &mut World) -> Result<ProcessReactionResult> {
    unimplemented!("process_reactions 尚未實作")
}

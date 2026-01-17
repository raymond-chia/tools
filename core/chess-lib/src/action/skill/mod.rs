//! skill 模組：技能系統相關功能
//!
//! 本模組負責技能效果、技能施放與解析邏輯。
//! 僅處理技能本身，不負責戰鬥流程、AI 決策或棋盤初始化。

pub use selection::*;
pub use skills_lib::{AttackResult, DefenseResult, SaveResult};
pub use targeting::*;
// 重新導出供 reaction 系統使用
pub(in crate::action) use casting::cast_skill_internal;

mod casting;
mod effect_application;
mod hit_resolution;
mod selection;
mod targeting;

// ============================================================================
// 戰鬥系統常數
// ============================================================================

/// 大失敗門檻：擲骰 <= 此值時完全閃避
pub(super) const CRITICAL_FAILURE_THRESHOLD: i32 = 5;

/// 大成功門檻：擲骰 > 此值時完全命中
pub(super) const CRITICAL_SUCCESS_THRESHOLD: i32 = 95;

/// 爆擊傷害倍率（固定 2.0 倍）
pub(super) const CRITICAL_HIT_MULTIPLIER: i32 = 2;

/// 格擋減傷基礎百分比（未裝備任何格擋減傷技能時）
pub(super) const MIN_BLOCK_REDUCTION_PERCENT: i32 = 0;

/// 格擋減傷最大百分比（技能加成上限）
pub(super) const MAX_BLOCK_REDUCTION_PERCENT: i32 = 80;

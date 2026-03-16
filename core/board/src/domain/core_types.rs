//! 基本資料類型定義

use crate::ecs_types::components::Occupant;
use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;

// ============================================================================
// 屬性系統
// ============================================================================

/// 定義屬性列表的 macro（單一來源）
///
/// 格式：(欄位名, Attribute enum variant)
/// - `Attribute` enum
macro_rules! define_attributes {
    ($(($field:ident, $variant:ident)),* $(,)?) => {
        /// 角色屬性類型
        #[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, EnumIter)]
        pub enum Attribute {
            #[default]
            $($variant,)*
        }
    };
}

define_attributes!(
    (hp, Hp),
    (mp, Mp),
    (initiative, Initiative),
    (hit, Hit),
    (evasion, Evasion),
    (block, Block),
    (block_protection, BlockProtection),
    (physical_attack, PhysicalAttack),
    (magical_attack, MagicalAttack),
    (magical_dc, MagicalDc),
    (fortitude, Fortitude),
    (reflex, Reflex),
    (will, Will),
    (movement, Movement),
    (reaction, Reaction),
);

/// 單位在回合表中的資訊
#[derive(Debug, Clone)]
pub struct TurnEntry {
    pub occupant: Occupant,
    pub initiative: i32, // 原始 INI
    pub roll: i32,
    pub total: i32,      // INI + roll（主排序，顯示用）
    pub tiebreaker: f64, // INI*10 + 1 if player + 0.xxx（次排序，隱藏）
    pub has_acted: bool,
}

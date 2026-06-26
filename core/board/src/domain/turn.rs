//! 回合系統資料型別定義

use crate::ecs_types::components::Occupant;

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

//! 回合順序計算邏輯（純邏輯，不操作 World）

use crate::domain::core_types::TurnEntry;
use crate::ecs_types::components::Occupant;
use crate::error::{BoardError, Result};
use std::cmp::Ordering;

/// 計算順序的輸入資料
pub struct TurnOrderInput {
    pub occupant: Occupant,
    pub initiative: i32,
    pub is_player: bool,
}

/// 計算一輪的行動順序（純邏輯，不操作 World）
///
/// # 參數
/// - `inputs`: 參與回合的單位列表
/// - `rng_int`: 產生 1~6 整數的隨機函數
/// - `rng_float`: 產生 0.001~0.999 小數的隨機函數
///
/// # 排序規則
/// 1. 主排序：INI + 骰子結果（降序，高分先行動）
/// 2. 次排序：tiebreaker（降序）
///    - 玩家單位：INI * 10 + 1 + 隨機小數
///    - 敵方單位：INI * 10 + 隨機小數
pub fn calculate_turn_order(
    inputs: &[TurnOrderInput],
    rng_int: &mut impl FnMut() -> i32,
    rng_float: &mut impl FnMut() -> f64,
) -> Vec<TurnEntry> {
    let mut entries: Vec<TurnEntry> = inputs
        .iter()
        .map(|input| {
            let roll = rng_int();
            let total = input.initiative + roll;
            let tiebreaker_base = input.initiative as f64 * 10.0;
            let tiebreaker_bonus = if input.is_player { 1.0 } else { 0.0 };
            let tiebreaker = tiebreaker_base + tiebreaker_bonus + rng_float();

            TurnEntry {
                occupant: input.occupant,
                initiative: input.initiative,
                roll,
                total,
                tiebreaker,
                has_acted: false,
            }
        })
        .collect();

    // 按 total 降序排序，若相同則按 tiebreaker 降序
    entries.sort_by(|a, b| match b.total.cmp(&a.total) {
        Ordering::Equal => b
            .tiebreaker
            .partial_cmp(&a.tiebreaker)
            .unwrap_or(Ordering::Equal),
        other => other,
    });

    entries
}

/// 將當前單位延後到 target_index 位置（只能往後）
///
/// # 參數
/// - `entries`: 回合表
/// - `target_index`: 單位最終要到的索引位置
///
/// # 錯誤
/// - 若沒有未行動的單位則回傳錯誤
/// - 若 target_index 小於等於當前單位索引則回傳錯誤
pub fn delay_unit(entries: &mut Vec<TurnEntry>, target_index: usize) -> Result<()> {
    let current_index = next_active_index(entries).ok_or(BoardError::NoActiveUnit)?;

    if target_index <= current_index {
        return Err(BoardError::InvalidDelayTarget {
            current: current_index,
            target: target_index,
        }
        .into());
    }

    if target_index >= entries.len() {
        return Err(BoardError::InvalidDelayTarget {
            current: current_index,
            target: target_index,
        }
        .into());
    }

    // 將當前單位移到 target_index，並將中間的單位往前移一格
    entries[current_index..target_index + 1].rotate_left(1);
    Ok(())
}

/// 取得下一個未行動的單位
pub fn next_active_unit(entries: &[TurnEntry]) -> Option<Occupant> {
    next_active_index(entries).and_then(|idx| entries.get(idx).map(|e| e.occupant))
}

/// 移除指定 Occupant 的單位
pub fn remove_unit(entries: &mut Vec<TurnEntry>, occupant: Occupant) -> Result<TurnEntry> {
    entries
        .iter()
        .position(|entry| entry.occupant == occupant)
        .ok_or_else(|| BoardError::OccupantNotFound { occupant }.into())
        .map(|idx| entries.remove(idx))
}

/// 取得下一個未行動的單位索引
fn next_active_index(entries: &[TurnEntry]) -> Option<usize> {
    entries.iter().position(|entry| !entry.has_acted)
}

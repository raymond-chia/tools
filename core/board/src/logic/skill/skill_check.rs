use crate::domain::constants::{FORCED_FAILURE_UPPER, FORCED_SUCCESS_LOWER};

// ============================================================================
// 命中判定
// ============================================================================

/// 命中骰結果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitCheckResult {
    Hit { crit: bool },
    Block { crit: bool },
    Evade,
}

/// 命中判定的完整輸出，附帶骰值與門檻供 log 顯示
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitResult {
    pub check: HitCheckResult,
    pub roll: i32,
    pub evasion_threshold: i32,
    pub block_threshold: i32,
    pub crit_rate: i32,
}

/// 解析命中判定
///
/// 判定流程：
/// 1. 強制閃避（骰 1~5）
/// 2. 強制命中（骰 96~100）
/// 3. 計算閃避門檻 = defender_evasion - attacker_hit
/// 4. 計算格擋門檻 = 閃避門檻 + defender_block
/// 5. 骰 < 閃避門檻 → Evade
/// 6. 骰 < 格擋門檻 → Block
/// 7. 否則 → Hit
/// 8. Hit 或 Block 時，骰 ≥ 100 - crit_rate → crit
///
/// 魔法命中判定可透過 resolve_hit(magical_accuracy, resistance, 0, 0, rng) 實現，
/// 其中 Evade 對應 Resisted，Hit { crit: false } 對應 Affected。
pub(crate) fn resolve_hit(
    attacker_hit: i32,
    defender_evasion: i32,
    defender_block: i32,
    crit_rate: i32,
    rng_int: &mut impl FnMut() -> i32,
) -> HitResult {
    let roll = rng_int();
    let evasion_threshold = defender_evasion - attacker_hit;
    let block_threshold = evasion_threshold + defender_block;

    let check = compute_hit_result(roll, evasion_threshold, block_threshold, crit_rate);

    HitResult {
        check,
        roll,
        evasion_threshold,
        block_threshold,
        crit_rate,
    }
}

fn compute_hit_result(
    roll: i32,
    evasion_threshold: i32,
    block_threshold: i32,
    crit_rate: i32,
) -> HitCheckResult {
    if roll <= FORCED_FAILURE_UPPER {
        return HitCheckResult::Evade;
    }
    if roll >= FORCED_SUCCESS_LOWER {
        let crit = is_crit(roll, crit_rate);
        return HitCheckResult::Hit { crit };
    }
    if roll < evasion_threshold {
        return HitCheckResult::Evade;
    }
    let crit = is_crit(roll, crit_rate);
    if roll < block_threshold {
        return HitCheckResult::Block { crit };
    }
    HitCheckResult::Hit { crit }
}

fn is_crit(roll: i32, crit_rate: i32) -> bool {
    if crit_rate <= 0 {
        return false;
    }
    let crit_threshold = 100 - crit_rate;
    roll >= crit_threshold
}

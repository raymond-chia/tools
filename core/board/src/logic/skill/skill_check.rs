use crate::domain::constants::{FORCED_FAILURE_UPPER, FORCED_SUCCESS_LOWER};
use crate::domain::core_types::HitCheckBreakdowns;

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

/// 命中判定的輸出，附帶骰值供呼叫端組裝顯示資料
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitResult {
    pub check: HitCheckResult,
    pub roll: i32,
}

/// 解析命中判定
///
/// 判定流程：
/// 1. 強制閃避（骰 1~5）
/// 2. 強制命中（骰 96~100）
/// 3. 計算閃避門檻 = defender_evasion - attacker_hit
/// 4. 計算格擋門檻 = 閃避門檻 + defender_block
/// 5. 骰 ≤ 閃避門檻 → Evade（命中須大於閃避門檻才不算閃避）
/// 6. 骰 ≤ 格擋門檻 → Block
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

    HitResult { check, roll }
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
    if roll <= evasion_threshold {
        return HitCheckResult::Evade;
    }
    let crit = is_crit(roll, crit_rate);
    if roll <= block_threshold {
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

// ============================================================================
// 命中機率（預覽用）
// ============================================================================

/// d100 均勻分布下各命中結果的百分點數（總和恆為 100）
///
/// 不含爆擊（預覽只需正常命中傷害），三分項對應 resolve_hit 的三種結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitProbabilities {
    pub hit: i32,
    pub block: i32,
    pub evade: i32,
    /// 爆擊率，直接為 crit_rate（供 UI 顯示，不影響三分項）
    pub crit: i32,
}

/// 計算命中機率（與 compute_hit_result 的門檻邏輯一致）
///
/// 判定域為 d100，每格 1 百分點：
/// 1. 強制閃避段 [1, FORCED_FAILURE_UPPER] 恆為 Evade
/// 2. 強制命中段 [FORCED_SUCCESS_LOWER, 100] 恆為 Hit
/// 3. 中間段依 evasion_threshold / block_threshold 切成 Evade / Block / Hit
pub(crate) fn hit_probabilities(breakdowns: &HitCheckBreakdowns) -> HitProbabilities {
    let attacker_hit = breakdowns.attacker_accuracy.total;
    let defender_evasion = breakdowns.defender_evasion.total;
    let defender_block = breakdowns.defender_block.total;
    let crit_rate = breakdowns.crit;

    let evasion_threshold = defender_evasion - attacker_hit;
    let block_threshold = evasion_threshold + defender_block;

    // 中間段的閉區間 [middle_lower, middle_upper]
    let middle_lower = FORCED_FAILURE_UPPER + 1;
    let middle_upper = FORCED_SUCCESS_LOWER - 1;

    // 中間段內：roll ≤ evasion_threshold → Evade
    let middle_evade = count_at_or_below(middle_lower, middle_upper, evasion_threshold);
    // 中間段內：evasion_threshold < roll ≤ block_threshold → Block
    let middle_block =
        count_at_or_below(middle_lower, middle_upper, block_threshold) - middle_evade;
    // 中間段內剩餘 → Hit
    let middle_total = middle_upper - middle_lower + 1;
    let middle_hit = middle_total - middle_evade - middle_block;

    let forced_evade = FORCED_FAILURE_UPPER;
    let forced_hit = 100 - FORCED_SUCCESS_LOWER + 1;

    HitProbabilities {
        hit: middle_hit + forced_hit,
        block: middle_block,
        evade: middle_evade + forced_evade,
        crit: crit_rate,
    }
}

/// 計算閉區間 [lower, upper] 內滿足 roll ≤ threshold 的整數格子數
fn count_at_or_below(lower: i32, upper: i32, threshold: i32) -> i32 {
    let effective_upper = threshold.min(upper);
    (effective_upper - lower + 1).max(0)
}

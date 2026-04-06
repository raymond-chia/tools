use crate::domain::constants::{FORCED_FAILURE_UPPER, FORCED_SUCCESS_LOWER};

// ============================================================================
// 命中判定
// ============================================================================

/// 命中骰結果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitResult {
    Hit { crit: bool },
    Block { crit: bool },
    Evade,
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
pub(crate) fn resolve_hit(
    attacker_hit: i32,
    defender_evasion: i32,
    defender_block: i32,
    crit_rate: i32,
    rng_int: &mut impl FnMut() -> i32,
) -> HitResult {
    let roll = rng_int();

    // 強制閃避
    if roll <= FORCED_FAILURE_UPPER {
        return HitResult::Evade;
    }

    // 強制命中
    if roll >= FORCED_SUCCESS_LOWER {
        let crit = is_crit(roll, crit_rate);
        return HitResult::Hit { crit };
    }

    let evasion_threshold = defender_evasion - attacker_hit;
    let block_threshold = evasion_threshold + defender_block;

    if roll < evasion_threshold {
        return HitResult::Evade;
    }

    let crit = is_crit(roll, crit_rate);
    if roll < block_threshold {
        return HitResult::Block { crit };
    }
    HitResult::Hit { crit }
}

// ============================================================================
// DC 豁免判定
// ============================================================================

/// DC 豁免結果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DcResult {
    Saved,
    Failed,
}

/// 解析 DC 豁免判定
///
/// 判定流程：
/// 1. 強制失敗（骰 1~5）
/// 2. 強制成功（骰 96~100）
/// 3. defender_save + roll ≥ attacker_dc → Saved
/// 4. 否則 → Failed
pub(crate) fn resolve_dc(
    attacker_dc: i32,
    defender_save: i32,
    rng_int: &mut impl FnMut() -> i32,
) -> DcResult {
    let roll = rng_int();

    if roll <= FORCED_FAILURE_UPPER {
        return DcResult::Failed;
    }

    if roll >= FORCED_SUCCESS_LOWER {
        return DcResult::Saved;
    }

    if defender_save + roll >= attacker_dc {
        DcResult::Saved
    } else {
        DcResult::Failed
    }
}

fn is_crit(roll: i32, crit_rate: i32) -> bool {
    if crit_rate <= 0 {
        return false;
    }
    let crit_threshold = 100 - crit_rate;
    roll >= crit_threshold
}

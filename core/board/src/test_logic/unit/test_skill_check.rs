use crate::logic::skill::skill_check::{HitResult, resolve_hit};

// ============================================================================
// 命中判定（一般、強制、暴擊）
// ============================================================================

#[test]
fn test_resolve_hit() {
    // (attacker_hit, defender_evasion, defender_block, crit_rate, hit_roll, 預期結果)
    let test_data = [
        // --- 一般命中 / 格擋 / 閃避 ---
        // 閃避 (30) + 格擋 (20) = 50
        // 命中
        (20, 30, 20, 0, 90, HitResult::Hit { crit: false }),
        (20, 30, 20, 0, 60, HitResult::Hit { crit: false }),
        (20, 30, 20, 0, 31, HitResult::Hit { crit: false }),
        // 命中（邊界）
        (20, 30, 20, 0, 30, HitResult::Hit { crit: false }),
        // 格擋
        (20, 30, 20, 0, 29, HitResult::Block { crit: false }),
        (20, 30, 20, 0, 20, HitResult::Block { crit: false }),
        (20, 30, 20, 0, 11, HitResult::Block { crit: false }),
        // 格擋（邊界）
        (20, 30, 20, 0, 10, HitResult::Block { crit: false }),
        // 閃避
        (20, 30, 20, 0, 9, HitResult::Evade),
        (20, 30, 20, 0, 8, HitResult::Evade),
        (20, 30, 20, 0, 7, HitResult::Evade),
        // 其他數值組合
        // 閃避 (50) + 格擋 (50) = 100
        // 命中
        (30, 50, 50, 0, 90, HitResult::Hit { crit: false }),
        (30, 50, 50, 0, 80, HitResult::Hit { crit: false }),
        (30, 50, 50, 0, 71, HitResult::Hit { crit: false }),
        // 命中（邊界）
        (30, 50, 50, 0, 70, HitResult::Hit { crit: false }),
        // 格擋
        (30, 50, 50, 0, 69, HitResult::Block { crit: false }),
        (30, 50, 50, 0, 40, HitResult::Block { crit: false }),
        (30, 50, 50, 0, 21, HitResult::Block { crit: false }),
        // 格擋（邊界）
        (30, 50, 50, 0, 20, HitResult::Block { crit: false }),
        // 閃避
        (30, 50, 50, 0, 19, HitResult::Evade),
        (30, 50, 50, 0, 10, HitResult::Evade),
        (30, 50, 50, 0, 9, HitResult::Evade),
        //
        // --- 強制命中（骰子 96~100）---
        // 數值上會閃避，但骰子 96 強制命中
        // 閃避歸零不會暴擊
        (0, 999, 0, 0, 100, HitResult::Hit { crit: false }),
        (0, 999, 0, 0, 99, HitResult::Hit { crit: false }),
        (0, 999, 0, 0, 98, HitResult::Hit { crit: false }),
        (0, 999, 0, 0, 97, HitResult::Hit { crit: false }),
        (0, 999, 0, 0, 96, HitResult::Hit { crit: false }),
        // 骰子 95 不觸發強制命中，正常閃避
        (0, 999, 0, 0, 95, HitResult::Evade),
        //
        // --- 強制閃避（骰子 1~5）---
        // 數值上會命中，但骰子 1~5 強制閃避
        (999, 0, 0, 0, 1, HitResult::Evade),
        (999, 0, 0, 0, 2, HitResult::Evade),
        (999, 0, 0, 0, 3, HitResult::Evade),
        (999, 0, 0, 0, 4, HitResult::Evade),
        (999, 0, 0, 0, 5, HitResult::Evade),
        // 骰子 6 不觸發強制閃避，正常命中
        (999, 0, 0, 0, 6, HitResult::Hit { crit: false }),
        //
        // --- 暴擊（命中時）---
        // 暴擊門檻 10
        // 暴擊
        (50, 30, 20, 10, 100, HitResult::Hit { crit: true }),
        (50, 30, 20, 10, 95, HitResult::Hit { crit: true }),
        (50, 30, 20, 10, 90, HitResult::Hit { crit: true }),
        // 不暴擊
        (50, 30, 20, 10, 89, HitResult::Hit { crit: false }),
        (50, 30, 20, 10, 80, HitResult::Hit { crit: false }),
        (50, 30, 20, 10, 70, HitResult::Hit { crit: false }),
        // 暴擊門檻 20
        // 暴擊
        (50, 30, 20, 20, 100, HitResult::Hit { crit: true }),
        (50, 30, 20, 20, 90, HitResult::Hit { crit: true }),
        (50, 30, 20, 20, 80, HitResult::Hit { crit: true }),
        // 不暴擊
        (50, 30, 20, 20, 79, HitResult::Hit { crit: false }),
        (50, 30, 20, 20, 60, HitResult::Hit { crit: false }),
        (50, 30, 20, 20, 50, HitResult::Hit { crit: false }),
        //
        // --- 暴擊 + 格擋 ---
        // 格擋
        (50, 30, 50, 100, 10, HitResult::Block { crit: true }),
        (50, 30, 50, 100, 20, HitResult::Block { crit: true }),
        (50, 30, 50, 100, 29, HitResult::Block { crit: true }),
        // 暴擊
        (50, 30, 50, 100, 30, HitResult::Hit { crit: true }),
        // 不同數值
        // 格擋
        (50, 30, 60, 100, 20, HitResult::Block { crit: true }),
        (50, 30, 60, 100, 30, HitResult::Block { crit: true }),
        (50, 30, 60, 100, 39, HitResult::Block { crit: true }),
        // 暴擊
        (50, 30, 60, 100, 40, HitResult::Hit { crit: true }),
        //
        // --- 閃避時不觸發暴擊 ---
        // 暴擊率 100%，但閃避了就不該暴擊
        (20, 30, 20, 100, 30, HitResult::Hit { crit: true }),
        (20, 30, 20, 100, 10, HitResult::Block { crit: true }),
        (20, 30, 20, 100, 9, HitResult::Evade),
        (20, 30, 20, 100, 6, HitResult::Evade),
        (20, 30, 20, 100, 1, HitResult::Evade),
        // 不同數值
        (20, 50, 50, 100, 80, HitResult::Hit { crit: true }),
        (20, 50, 50, 100, 30, HitResult::Block { crit: true }),
        (20, 50, 50, 100, 29, HitResult::Evade),
        (20, 50, 50, 100, 20, HitResult::Evade),
        (20, 50, 50, 100, 10, HitResult::Evade),
    ];

    for (attacker_hit, defender_evasion, defender_block, crit_rate, hit_roll, expected) in test_data
    {
        let mut hit_roll_fn = || hit_roll;
        let result = resolve_hit(
            attacker_hit,
            defender_evasion,
            defender_block,
            crit_rate,
            &mut hit_roll_fn,
        );
        assert_eq!(
            result, expected,
            "hit={attacker_hit}, eva={defender_evasion}, blk={defender_block}, crit_rate={crit_rate}, hit_roll={hit_roll}\n \
                search: {attacker_hit}, {defender_evasion}, {defender_block}, {crit_rate}, {hit_roll}"
        );
    }
}

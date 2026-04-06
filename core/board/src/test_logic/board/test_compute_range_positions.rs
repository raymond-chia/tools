//! compute_range_positions 測試

use crate::ecs_types::components::Position;
use crate::logic::skill::skill_range::compute_range_positions;
use crate::test_helpers::level_builder::load_from_ascii;
use std::collections::HashSet;

/// 從 markers 收集所有被標記為 R（範圍內）的格子
fn collect_range(markers: &std::collections::HashMap<String, Vec<Position>>) -> HashSet<Position> {
    markers.get("+").into_iter().flatten().copied().collect()
}

#[test]
fn test_compute_range_positions() {
    let test_data = [
        (
            0,
            0,
            "
            . . . . . .
            P . . . . .
            . . . . . .
            . . . . . .
            . . . . . .
            . . . . . .
            ",
        ),
        (
            0,
            0,
            "
            . . . . . .
            . . . . . .
            . . . . . .
            . . . . . .
            . . . . P .
            . . . . . .
            ",
        ),
        (
            0,
            1,
            "
            . . . . . .
            . + . . . .
            + P + . . .
            . + . . . .
            . . . . . .
            . . . . . .
            ",
        ),
        (
            0,
            1,
            "
            . . . . . .
            . . . . . .
            . . . . . .
            . . . . . +
            . . . . + P
            . . . . . +
            ",
        ),
        (
            1,
            1,
            "
            . . . . . .
            . . . . . .
            . . . . . .
            . . . . . .
            + . . . . .
            P + . . . .
            ",
        ),
        (
            1,
            1,
            "
            . . . . . .
            . . . . . .
            . . . + . .
            . . + P + .
            . . . + . .
            . . . . . .
            ",
        ),
        (
            0,
            2,
            "
            . . . + . .
            . . + + + .
            . + + P + +
            . . + + + .
            . . . + . .
            . . . . . .
            ",
        ),
        (
            1,
            2,
            "
            . . . + + P
            . . . . + +
            . . . . . +
            . . . . . .
            . . . . . .
            . . . . . .
            ",
        ),
        (
            1,
            2,
            "
            . . . . . .
            . . . . . .
            . + . . . .
            + + + . . .
            + P + + . .
            + + + . . .
            ",
        ),
        (
            2,
            2,
            "
            . . . . . .
            . . . + . .
            . . + . + .
            . + . P . +
            . . + . + .
            . . . + . .
            ",
        ),
        (
            2,
            2,
            "
            . . . . . .
            . . . . . .
            . . . . + .
            . . . + . +
            . . + . P .
            . . . + . +
            ",
        ),
    ];

    for (min_range, max_range, ascii) in test_data {
        let (board, markers) = load_from_ascii(ascii).expect(&format!(
            "載入棋盤失敗：min={}, max={}",
            min_range, max_range
        ));
        let caster = markers["P"][0];
        let mut expected_set = collect_range(&markers);
        if min_range == 0 {
            expected_set.insert(caster); // 包含施法者自己
        }

        let result = compute_range_positions(caster, (min_range, max_range), board);
        let result_set: HashSet<_> = result.into_iter().collect();

        assert_eq!(
            result_set, expected_set,
            "測試失敗：min_range={}, max_range={}, ascii={}",
            min_range, max_range, ascii
        );
    }
}

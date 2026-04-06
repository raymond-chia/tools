//! compute_affected_positions 測試

use crate::domain::core_types::Area;
use crate::ecs_types::components::Position;
use crate::logic::skill::skill_range::compute_affected_positions;
use crate::test_helpers::level_builder::load_from_ascii;
use std::collections::{HashMap, HashSet};

/// 從 markers 收集所有被影響的格子（C + T + A）
fn collect_affected(markers: &HashMap<String, Vec<Position>>) -> HashSet<Position> {
    ["C", "T", "A"]
        .iter()
        .flat_map(|key| markers.get(*key).into_iter().flatten().copied())
        .collect()
}

#[test]
fn test_compute_affected_positions() {
    let test_data = [
        // Diamond 測試
        (
            Area::Diamond { radius: 0 },
            "
            . . . . . .
            . . . . . .
            . . . . . .
            . . . C . .
            . . . . . .
            . . . . . .
            ",
        ),
        (
            Area::Diamond { radius: 0 },
            "
            . . . . . .
            . . . . . .
            . . . . . .
            . . . . . .
            . . C . . .
            . . . . . .
            ",
        ),
        (
            Area::Diamond { radius: 1 },
            "
            . . . . . .
            . . . . . .
            . . . A . .
            . . A C A .
            . . . A . .
            . . . . . .
            ",
        ),
        (
            Area::Diamond { radius: 1 },
            "
            C A . . . .
            A . . . . .
            . . . . . .
            . . . . . .
            . . . . . .
            . . . . . .
            ",
        ),
        (
            Area::Diamond { radius: 1 },
            "
            . . . . . .
            . . . . . A
            . . . . A C
            . . . . . A
            . . . . . .
            . . . . . .
            ",
        ),
        (
            Area::Diamond { radius: 2 },
            "
            . . A . . .
            . A A A . .
            A A C A A .
            . A A A . .
            . . A . . .
            . . . . . .
            ",
        ),
        (
            Area::Diamond { radius: 2 },
            "
            . . . . . .
            . . . . . .
            . . . . . .
            . . . . . A
            . . . . A A
            . . . A A C
            ",
        ),
        (
            Area::Diamond { radius: 2 },
            "
            . A A A . .
            A A C A A .
            . A A A . .
            . . A . . .
            . . . . . .
            . . . . . .
            ",
        ),
        (
            Area::Diamond { radius: 2 },
            "
            . . . . . .
            . . . . . .
            . . . . A .
            . . . A A A
            . . A A C A
            . . . A A A
            ",
        ),
        (
            Area::Diamond { radius: 3 },
            "
            . . . A . .
            . . A A A .
            . A A A A A
            A A A C A A
            . A A A A A
            . . A A A .
            ",
        ),
        // Cross 測試
        (
            Area::Cross { length: 0 },
            "
            . . . . . .
            . . . . . .
            . . . . . .
            . . . . . C
            . . . . . .
            . . . . . .
            ",
        ),
        (
            Area::Cross { length: 0 },
            "
            . . . . . .
            . . . . . .
            . . C . . .
            . . . . . .
            . . . . . .
            . . . . . .
            ",
        ),
        (
            Area::Cross { length: 1 },
            "
            . . . . . .
            . . . . . .
            . . . . . .
            . . A . . .
            . A C A . .
            . . A . . .
            ",
        ),
        (
            Area::Cross { length: 1 },
            "
            C A . . . .
            A . . . . .
            . . . . . .
            . . . . . .
            . . . . . .
            . . . . . .
            ",
        ),
        (
            Area::Cross { length: 1 },
            "
            . . . . . .
            . . . . . .
            . . . . . .
            . . . . . .
            . . . . . A
            . . . . A C
            ",
        ),
        (
            Area::Cross { length: 2 },
            "
            . . . . . .
            A . . . . .
            A . . . . .
            C A A . . .
            A . . . . .
            A . . . . .
            ",
        ),
        (
            Area::Cross { length: 2 },
            "
            . . . . . .
            . . . . . .
            . . . . . .
            . . . . . A
            . . . . . A
            . . . A A C
            ",
        ),
        (
            Area::Cross { length: 2 },
            "
            . . . . . .
            . . . . . .
            . . . . . A
            . . . . . A
            . . . A A C
            . . . . . A
            ",
        ),
        (
            Area::Cross { length: 3 },
            "
            . A . . . .
            A C A A A .
            . A . . . .
            . A . . . .
            . A . . . .
            . . . . . .
            ",
        ),
        (
            Area::Cross { length: 3 },
            "
            . . . . . .
            . . . . . A
            . . . . . A
            . . . . . A
            . . A A A C
            . . . . . A
            ",
        ),
        // Line 測試：C=caster, T=target 決定方向
        (
            Area::Line { length: 1 },
            "
            . . . . . .
            . . . . . .
            . . . . . .
            . . . T . .
            . . . C . .
            . . . . . .
            ",
        ),
        (
            Area::Line { length: 1 },
            "
            . . . . . .
            . C . . . .
            . T . . . .
            . . . . . .
            . . . . . .
            . . . . . .
            ",
        ),
        (
            Area::Line { length: 1 },
            "
            . . . . . .
            . . . . . .
            . . . . . .
            . . . . . .
            . . T C . .
            . . . . . .
            ",
        ),
        (
            Area::Line { length: 1 },
            "
            . . . . . .
            . . . . . .
            . . . . . .
            . . . . . .
            . . . . . .
            . . . . C T
            ",
        ),
        (
            Area::Line { length: 2 },
            "
            . . . . . .
            . . . T . .
            . . . A . .
            . . . C . .
            . . . . . .
            . . . . . .
            ",
        ),
        (
            Area::Line { length: 2 },
            "
            . . . . . .
            . . . . . .
            . . . . . .
            . . . . . .
            . . . . C .
            . . . . T .
            ",
        ),
        (
            Area::Line { length: 2 },
            "
            . . . . . .
            . . . . . .
            . . . . . .
            T C . . . .
            . . . . . .
            . . . . . .
            ",
        ),
        (
            Area::Line { length: 2 },
            "
            . . . . . .
            . . . . . .
            . . . . . .
            . . C A T .
            . . . . . .
            . . . . . .
            ",
        ),
        (
            Area::Line { length: 2 },
            "
            . . . . . .
            . . . . . .
            . . . . . .
            . . C T A .
            . . . . . .
            . . . . . .
            ",
        ),
        (
            Area::Line { length: 3 },
            "
            . . . . . .
            . . . . T .
            . . . . A .
            . . . . A .
            . . . . C .
            . . . . . .
            ",
        ),
        (
            Area::Line { length: 3 },
            "
            . . . . . .
            . . C . . .
            . . A . . .
            . . A . . .
            . . T . . .
            . . . . . .
            ",
        ),
        (
            Area::Line { length: 3 },
            "
            . . . . . .
            . . . . . .
            . . . . . .
            . . . . . .
            T C . . . .
            . . . . . .
            ",
        ),
        (
            Area::Line { length: 3 },
            "
            . . . . . .
            . . . . . .
            . C A A T .
            . . . . . .
            . . . . . .
            . . . . . .
            ",
        ),
        (
            Area::Line { length: 3 },
            "
            . . . . . .
            . . . . . .
            . . . . . .
            . . . . . .
            . . C A T A
            . . . . . .
            ",
        ),
        (
            Area::Line { length: 3 },
            "
            . . . . . .
            . . . . . .
            . C T A A .
            . . . . . .
            . . . . . .
            . . . . . .
            ",
        ),
    ];

    for (shape, ascii) in test_data {
        let (board, markers) = load_from_ascii(ascii).expect(&format!("載入棋盤失敗：{ascii}"));
        let caster = markers["C"][0];
        let target = markers.get("T").map(|v| v[0]).unwrap_or(caster);
        let expected_set = collect_affected(&markers);

        let result = compute_affected_positions(&shape, caster, target, board)
            .expect(&format!("計算失敗：{ascii}"));
        let result_set: HashSet<_> = result.into_iter().collect();
        assert_eq!(result_set, expected_set, "測試失敗：{ascii}");
    }
}

//! compute_affected_positions 測試

use board::ecs_types::components::Position;
use board::ecs_types::resources::Board;
use board::loader_schema::AoeShape;
use board::logic::skill::compute_affected_positions;
use std::collections::HashSet;

#[test]
fn test_compute_affected_positions() {
    let board = Board {
        width: 6,
        height: 6,
    };

    // (description, AoeInput, expected_positions)
    let test_data = [
        // Diamond 測試
        (
            "Diamond radius=0",
            (AoeShape::Diamond { radius: 0 }, (3, 3), (3, 3)),
            vec![(3, 3)],
        ),
        (
            "Diamond radius=1",
            (AoeShape::Diamond { radius: 1 }, (3, 3), (3, 3)),
            vec![(3, 3), (3, 2), (3, 4), (2, 3), (4, 3)],
        ),
        (
            "Diamond radius=1 邊界裁切",
            (AoeShape::Diamond { radius: 1 }, (0, 0), (0, 0)),
            vec![(0, 0), (0, 1), (1, 0)],
        ),
        (
            "Diamond radius=2 邊界裁切",
            (AoeShape::Diamond { radius: 2 }, (5, 5), (5, 5)),
            vec![(5, 5), (5, 4), (5, 3), (4, 5), (3, 5), (4, 4)],
        ),
        // Cross 測試
        (
            "Cross length=0",
            (AoeShape::Cross { length: 0 }, (5, 3), (5, 3)),
            vec![(5, 3)],
        ),
        (
            "Cross length=1",
            (AoeShape::Cross { length: 1 }, (2, 4), (2, 4)),
            vec![(2, 4), (2, 3), (2, 5), (1, 4), (3, 4)],
        ),
        (
            "Cross length=1 邊界裁切",
            (AoeShape::Cross { length: 1 }, (0, 3), (0, 3)),
            vec![(0, 3), (0, 2), (0, 4), (1, 3)],
        ),
        (
            "Cross length=2 邊界裁切",
            (AoeShape::Cross { length: 2 }, (5, 5), (5, 5)),
            vec![(5, 5), (5, 4), (5, 3), (4, 5), (3, 5)],
        ),
        // Line 測試
        (
            "Line 向上 length=2",
            (AoeShape::Line { length: 2 }, (3, 3), (3, 1)),
            vec![(3, 3), (3, 2), (3, 1)],
        ),
        (
            "Line 向下 length=1",
            (AoeShape::Line { length: 1 }, (3, 4), (3, 5)),
            vec![(3, 4), (3, 5)],
        ),
        (
            "Line 向左 length=2",
            (AoeShape::Line { length: 2 }, (1, 3), (0, 3)),
            vec![(1, 3), (0, 3)],
        ),
        (
            "Line 向右 length=3",
            (AoeShape::Line { length: 3 }, (2, 4), (5, 4)),
            vec![(2, 4), (3, 4), (4, 4), (5, 4)],
        ),
    ];

    for (description, input, expected) in test_data {
        let aoe = input.0;
        let caster_pos = Position {
            x: input.1.0,
            y: input.1.1,
        };
        let target_pos = Position {
            x: input.2.0,
            y: input.2.1,
        };
        let result = compute_affected_positions(&aoe, caster_pos, target_pos, board)
            .expect(&format!("計算失敗：{}", description));
        let result_set: HashSet<_> = result.into_iter().collect();
        let expected_set: HashSet<_> = expected
            .into_iter()
            .map(|(x, y)| Position { x, y })
            .collect();
        assert_eq!(result_set, expected_set, "測試失敗：{description}");
    }
}

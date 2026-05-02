//! 可移動範圍測試

use crate::domain::alias::MovementCost;
use crate::domain::constants::{BASIC_MOVEMENT_COST, IMPASSABLE_MOVEMENT_COST, PLAYER_ALLIANCE_ID};
use crate::ecs_types::components::Position;
use crate::logic::movement::{
    Direction, Mover, ReachableInfo, reachable_positions, reconstruct_path, step_in_direction,
};
use crate::test_helpers::level_builder::load_from_ascii;
use std::collections::HashSet;

const NORMAL_COST: MovementCost = BASIC_MOVEMENT_COST;
const WATER_COST: MovementCost = BASIC_MOVEMENT_COST * 3;

// ============================================================================
// step_in_direction 測試
// ============================================================================

#[test]
fn test_step_in_direction_normal() {
    let test_data = [
        (
            r#"
E . .
S . .
. . .
    "#,
            Direction::Up,
        ),
        (
            r#"
. . .
S . .
E . .
    "#,
            Direction::Down,
        ),
        (
            r#"
. . .
. . .
E S .
    "#,
            Direction::Left,
        ),
        (
            r#"
. . .
S E .
. . .
    "#,
            Direction::Right,
        ),
        (
            r#"
E . .
S . .
    "#,
            Direction::Up,
        ),
        (
            r#"
E
S
    "#,
            Direction::Up,
        ),
    ];

    for (idx, (ascii, direction)) in test_data.iter().enumerate() {
        let (board, markers) = load_from_ascii(ascii).unwrap();
        let pos = markers["S"][0];
        let expected = markers["E"][0];
        let result = step_in_direction(board, pos, *direction);
        assert_eq!(result, Some(expected), "Case {}", idx);
    }
}

#[test]
fn test_step_in_direction_out_of_bound() {
    let test_data = [
        (
            r#"
S . .
. . .
. . .
    "#,
            vec![Direction::Up, Direction::Left],
        ),
        (
            r#"
. . .
. . .
S . .
    "#,
            vec![Direction::Down, Direction::Left],
        ),
        (
            r#"
. . .
. . .
. . S
    "#,
            vec![Direction::Down, Direction::Right],
        ),
        (
            r#"
. . S
. . .
. . .
    "#,
            vec![Direction::Up, Direction::Right],
        ),
        (
            r#"
. . S
    "#,
            vec![Direction::Up, Direction::Down, Direction::Right],
        ),
        (
            r#"
S
.
.
    "#,
            vec![Direction::Up, Direction::Left, Direction::Right],
        ),
        (
            r#"
S
    "#,
            vec![
                Direction::Up,
                Direction::Down,
                Direction::Left,
                Direction::Right,
            ],
        ),
    ];

    for (idx, (ascii, directions)) in test_data.iter().enumerate() {
        let (board, markers) = load_from_ascii(ascii).unwrap();
        let pos = markers["S"][0];
        for d in directions {
            let result = step_in_direction(board, pos, *d);
            assert_eq!(result, None, "Case {}, direction {:?}", idx, d);
        }
    }
}

// ============================================================================
// reachable_positions 測試
// ============================================================================

#[test]
fn test_reachable_positions_out_of_bound() {
    let ascii = r#"
. . 
. . 
            "#;
    let (board, _) = load_from_ascii(ascii).unwrap();
    for from in [
        Position { x: 0, y: 2 },
        Position { x: 2, y: 0 },
        Position { x: 2, y: 1 },
    ] {
        let mover = Mover {
            pos: from,
            faction_alliance: PLAYER_ALLIANCE_ID,
        };
        let result = reachable_positions(board, mover, NORMAL_COST, |_| None, |_| NORMAL_COST);
        assert!(
            result.is_err(),
            "From position {:?} should be out of bound",
            from
        );
    }
}

#[test]
fn test_reachable_positions_normal() {
    let test_data = [
        (
            r#"
S . . . .
. . . . .
. . . . .
. . . . .
. . . . .
            "#,
            NORMAL_COST * 1,
            vec![
                (
                    Position { x: 1, y: 0 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        passthrough_only: false,
                        prev: Position { x: 0, y: 0 },
                    },
                ),
                (
                    Position { x: 0, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        passthrough_only: false,
                        prev: Position { x: 0, y: 0 },
                    },
                ),
            ],
        ),
        (
            r#"
. . . . .
. . . . .
. . . . .
. . . . .
. . . . S
            "#,
            NORMAL_COST * 1,
            vec![
                (
                    Position { x: 3, y: 4 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        passthrough_only: false,
                        prev: Position { x: 4, y: 4 },
                    },
                ),
                (
                    Position { x: 4, y: 3 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        passthrough_only: false,
                        prev: Position { x: 4, y: 4 },
                    },
                ),
            ],
        ),
        (
            r#"
. . . . .
S . . . .
. . . . .
. . . . .
. . . . .
            "#,
            NORMAL_COST * 1,
            vec![
                (
                    Position { x: 0, y: 0 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        passthrough_only: false,
                        prev: Position { x: 0, y: 1 },
                    },
                ),
                (
                    Position { x: 1, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        passthrough_only: false,
                        prev: Position { x: 0, y: 1 },
                    },
                ),
                (
                    Position { x: 0, y: 2 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        passthrough_only: false,
                        prev: Position { x: 0, y: 1 },
                    },
                ),
            ],
        ),
    ];

    for (idx, (ascii, budget, expected)) in test_data.iter().enumerate() {
        let (board, markers) = load_from_ascii(ascii).unwrap();
        let from = markers["S"][0];
        let mover = Mover {
            pos: from,
            faction_alliance: PLAYER_ALLIANCE_ID,
        };
        let result = reachable_positions(board, mover, *budget, |_| None, |_| NORMAL_COST)
            .unwrap_or_else(|e| panic!("Case {} failed: {:?}", idx, e));
        let expected_set: HashSet<_> = expected.iter().cloned().collect();
        let result_set: HashSet<_> = result
            .iter()
            .filter(|(_, v)| v.passthrough_only == false)
            .map(|(k, v)| (*k, *v))
            .collect();
        assert_eq!(result_set, expected_set, "Case {} mismatch", idx);
    }
}

#[test]
fn test_reachable_positions_with_enemy() {
    let test_data = [
        (
            r#"
S E . . .
. . . . .
. . . . .
            "#,
            NORMAL_COST * 1,
            vec![(
                Position { x: 0, y: 1 },
                ReachableInfo {
                    cost: NORMAL_COST * 1,
                    passthrough_only: false,
                    prev: Position { x: 0, y: 0 },
                },
            )],
        ),
        (
            r#"
S E . . .
. . . . .
. . . . .
            "#,
            NORMAL_COST * 2,
            vec![
                (
                    Position { x: 0, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        passthrough_only: false,
                        prev: Position { x: 0, y: 0 },
                    },
                ),
                (
                    Position { x: 1, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 2,
                        passthrough_only: false,
                        prev: Position { x: 0, y: 1 },
                    },
                ),
                (
                    Position { x: 0, y: 2 },
                    ReachableInfo {
                        cost: NORMAL_COST * 2,
                        passthrough_only: false,
                        prev: Position { x: 0, y: 1 },
                    },
                ),
            ],
        ),
        (
            r#"
. . .
. E .
. . S
            "#,
            NORMAL_COST * 2,
            vec![
                (
                    Position { x: 2, y: 0 },
                    ReachableInfo {
                        cost: NORMAL_COST * 2,
                        passthrough_only: false,
                        prev: Position { x: 2, y: 1 },
                    },
                ),
                (
                    Position { x: 2, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        passthrough_only: false,
                        prev: Position { x: 2, y: 2 },
                    },
                ),
                (
                    Position { x: 0, y: 2 },
                    ReachableInfo {
                        cost: NORMAL_COST * 2,
                        passthrough_only: false,
                        prev: Position { x: 1, y: 2 },
                    },
                ),
                (
                    Position { x: 1, y: 2 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        passthrough_only: false,
                        prev: Position { x: 2, y: 2 },
                    },
                ),
            ],
        ),
    ];

    for (idx, (ascii, budget, expected)) in test_data.iter().enumerate() {
        let (board, markers) = load_from_ascii(ascii).unwrap();
        let from = markers["S"][0];
        let enemy_pos = markers["E"][0];

        let mover = Mover {
            pos: from,
            faction_alliance: PLAYER_ALLIANCE_ID,
        };

        let get_occupant = |pos: Position| {
            if pos == enemy_pos {
                Some(PLAYER_ALLIANCE_ID + 1)
            } else {
                None
            }
        };

        let result = reachable_positions(board, mover, *budget, get_occupant, |_| NORMAL_COST)
            .unwrap_or_else(|e| panic!("Case {} failed: {:?}", idx, e));
        let expected_set: HashSet<_> = expected.iter().cloned().collect();
        let result_set: HashSet<_> = result
            .iter()
            .filter(|(_, v)| v.passthrough_only == false)
            .map(|(k, v)| (*k, *v))
            .collect();
        assert_eq!(result_set, expected_set, "Case {} mismatch", idx);
    }
}

#[test]
fn test_reachable_positions_with_ally() {
    let test_data = [
        (
            r#"
S A . . .
. . . . .
. . . . .
            "#,
            NORMAL_COST * 1,
            vec![(
                Position { x: 0, y: 1 },
                ReachableInfo {
                    cost: NORMAL_COST * 1,
                    passthrough_only: false,
                    prev: Position { x: 0, y: 0 },
                },
            )],
        ),
        (
            r#"
S A . . .
. A . . .
. . . . .
            "#,
            NORMAL_COST * 2,
            vec![
                (
                    Position { x: 2, y: 0 },
                    ReachableInfo {
                        cost: NORMAL_COST * 2,
                        passthrough_only: false,
                        prev: Position { x: 1, y: 0 },
                    },
                ),
                (
                    Position { x: 0, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        passthrough_only: false,
                        prev: Position { x: 0, y: 0 },
                    },
                ),
                (
                    Position { x: 0, y: 2 },
                    ReachableInfo {
                        cost: NORMAL_COST * 2,
                        passthrough_only: false,
                        prev: Position { x: 0, y: 1 },
                    },
                ),
            ],
        ),
        (
            r#"
. . S
. A A
. . .
            "#,
            NORMAL_COST * 2,
            vec![
                (
                    Position { x: 0, y: 0 },
                    ReachableInfo {
                        cost: NORMAL_COST * 2,
                        passthrough_only: false,
                        prev: Position { x: 1, y: 0 },
                    },
                ),
                (
                    Position { x: 1, y: 0 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        passthrough_only: false,
                        prev: Position { x: 2, y: 0 },
                    },
                ),
                (
                    Position { x: 2, y: 2 },
                    ReachableInfo {
                        cost: NORMAL_COST * 2,
                        passthrough_only: false,
                        prev: Position { x: 2, y: 1 },
                    },
                ),
            ],
        ),
    ];

    for (idx, (ascii, budget, expected)) in test_data.iter().enumerate() {
        let (board, markers) = load_from_ascii(ascii).unwrap();
        let from = markers["S"][0];
        let ally_positions = &markers["A"];

        let mover = Mover {
            pos: from,
            faction_alliance: PLAYER_ALLIANCE_ID,
        };

        let get_occupant = |pos: Position| {
            if ally_positions.contains(&pos) {
                Some(PLAYER_ALLIANCE_ID)
            } else {
                None
            }
        };

        let result = reachable_positions(board, mover, *budget, get_occupant, |_| NORMAL_COST)
            .unwrap_or_else(|e| panic!("Case {} failed: {:?}", idx, e));

        let expected_set: HashSet<_> = expected.iter().cloned().collect();
        let result_set: HashSet<_> = result
            .iter()
            .filter(|(_, v)| v.passthrough_only == false)
            .map(|(k, v)| (*k, *v))
            .collect();
        assert_eq!(result_set, expected_set, "Case {} mismatch", idx);
    }
}

#[test]
fn test_reachable_positions_with_walls() {
    let test_data = [
        (
            r#"
S # . . .
. # . . .
. . . . .
            "#,
            NORMAL_COST * 1,
            vec![(
                Position { x: 0, y: 1 },
                ReachableInfo {
                    cost: NORMAL_COST * 1,
                    passthrough_only: false,
                    prev: Position { x: 0, y: 0 },
                },
            )],
        ),
        (
            r#"
S . . . .
. # . . .
. . . . .
            "#,
            NORMAL_COST * 2,
            vec![
                (
                    Position { x: 1, y: 0 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        passthrough_only: false,
                        prev: Position { x: 0, y: 0 },
                    },
                ),
                (
                    Position { x: 2, y: 0 },
                    ReachableInfo {
                        cost: NORMAL_COST * 2,
                        passthrough_only: false,
                        prev: Position { x: 1, y: 0 },
                    },
                ),
                (
                    Position { x: 0, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        passthrough_only: false,
                        prev: Position { x: 0, y: 0 },
                    },
                ),
                (
                    Position { x: 0, y: 2 },
                    ReachableInfo {
                        cost: NORMAL_COST * 2,
                        passthrough_only: false,
                        prev: Position { x: 0, y: 1 },
                    },
                ),
            ],
        ),
        (
            r#"
S # . . .
# . . . .
. . . . .
            "#,
            NORMAL_COST * 2,
            vec![],
        ),
    ];

    for (idx, (ascii, budget, expected)) in test_data.iter().enumerate() {
        let (board, markers) = load_from_ascii(ascii).unwrap();
        let from = markers["S"][0];
        let wall_positions = &markers["#"];

        let mover = Mover {
            pos: from,
            faction_alliance: PLAYER_ALLIANCE_ID,
        };

        let get_terrain_cost = |pos: Position| {
            if wall_positions.contains(&pos) {
                IMPASSABLE_MOVEMENT_COST
            } else {
                NORMAL_COST
            }
        };

        let result = reachable_positions(board, mover, *budget, |_| None, get_terrain_cost)
            .unwrap_or_else(|e| panic!("Case {} failed: {:?}", idx, e));
        let expected_set: HashSet<_> = expected.iter().cloned().collect();
        let result_set: HashSet<_> = result
            .iter()
            .filter(|(_, v)| v.passthrough_only == false)
            .map(|(k, v)| (*k, *v))
            .collect();
        assert_eq!(result_set, expected_set, "Case {} mismatch", idx);
    }
}

#[test]
fn test_reachable_positions_with_water() {
    let test_data = [
        (
            r#"
S w . . .
w . . . .
. . . . .
            "#,
            NORMAL_COST * 2,
            vec![],
        ),
        (
            r#"
S w . . .
. w . . .
. . . . .
            "#,
            NORMAL_COST * 2,
            vec![
                (
                    Position { x: 0, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        passthrough_only: false,
                        prev: Position { x: 0, y: 0 },
                    },
                ),
                (
                    Position { x: 0, y: 2 },
                    ReachableInfo {
                        cost: NORMAL_COST * 2,
                        passthrough_only: false,
                        prev: Position { x: 0, y: 1 },
                    },
                ),
            ],
        ),
        (
            r#"
S w . . .
. w . . .
. . . . .
            "#,
            NORMAL_COST * 4,
            vec![
                (
                    Position { x: 1, y: 0 },
                    ReachableInfo {
                        cost: NORMAL_COST * 3,
                        passthrough_only: false,
                        prev: Position { x: 0, y: 0 },
                    },
                ),
                (
                    Position { x: 2, y: 0 },
                    ReachableInfo {
                        cost: NORMAL_COST * 4,
                        passthrough_only: false,
                        prev: Position { x: 1, y: 0 },
                    },
                ),
                (
                    Position { x: 0, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        passthrough_only: false,
                        prev: Position { x: 0, y: 0 },
                    },
                ),
                (
                    Position { x: 1, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 4,
                        passthrough_only: false,
                        prev: Position { x: 0, y: 1 },
                    },
                ),
                (
                    Position { x: 0, y: 2 },
                    ReachableInfo {
                        cost: NORMAL_COST * 2,
                        passthrough_only: false,
                        prev: Position { x: 0, y: 1 },
                    },
                ),
                (
                    Position { x: 1, y: 2 },
                    ReachableInfo {
                        cost: NORMAL_COST * 3,
                        passthrough_only: false,
                        prev: Position { x: 0, y: 2 },
                    },
                ),
                (
                    Position { x: 2, y: 2 },
                    ReachableInfo {
                        cost: NORMAL_COST * 4,
                        passthrough_only: false,
                        prev: Position { x: 1, y: 2 },
                    },
                ),
            ],
        ),
        (
            r#"
S w w . .
. . . . .
            "#,
            NORMAL_COST * 7,
            vec![
                (
                    Position { x: 1, y: 0 },
                    ReachableInfo {
                        cost: NORMAL_COST * 3,
                        passthrough_only: false,
                        prev: Position { x: 0, y: 0 },
                    },
                ),
                (
                    Position { x: 2, y: 0 },
                    ReachableInfo {
                        cost: NORMAL_COST * 6,
                        passthrough_only: false,
                        prev: Position { x: 1, y: 0 },
                    },
                ),
                (
                    Position { x: 3, y: 0 },
                    ReachableInfo {
                        cost: NORMAL_COST * 5,
                        passthrough_only: false,
                        prev: Position { x: 3, y: 1 },
                    },
                ),
                (
                    Position { x: 4, y: 0 },
                    ReachableInfo {
                        cost: NORMAL_COST * 6,
                        passthrough_only: false,
                        prev: Position { x: 3, y: 0 },
                    },
                ),
                (
                    Position { x: 0, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        passthrough_only: false,
                        prev: Position { x: 0, y: 0 },
                    },
                ),
                (
                    Position { x: 1, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 2,
                        passthrough_only: false,
                        prev: Position { x: 0, y: 1 },
                    },
                ),
                (
                    Position { x: 2, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 3,
                        passthrough_only: false,
                        prev: Position { x: 1, y: 1 },
                    },
                ),
                (
                    Position { x: 3, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 4,
                        passthrough_only: false,
                        prev: Position { x: 2, y: 1 },
                    },
                ),
                (
                    Position { x: 4, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 5,
                        passthrough_only: false,
                        prev: Position { x: 3, y: 1 },
                    },
                ),
            ],
        ),
    ];

    for (idx, (ascii, budget, expected)) in test_data.iter().enumerate() {
        let (board, markers) = load_from_ascii(ascii).unwrap();
        let from = markers["S"][0];
        let water_positions = &markers["w"];

        let mover = Mover {
            pos: from,
            faction_alliance: PLAYER_ALLIANCE_ID,
        };

        let get_terrain_cost = |pos: Position| {
            if water_positions.contains(&pos) {
                WATER_COST
            } else {
                NORMAL_COST
            }
        };

        let result = reachable_positions(board, mover, *budget, |_| None, get_terrain_cost)
            .unwrap_or_else(|e| panic!("Case {} failed: {:?}", idx, e));
        let expected_set: HashSet<_> = expected.iter().cloned().collect();
        let result_set: HashSet<_> = result
            .iter()
            .filter(|(_, v)| v.passthrough_only == false)
            .map(|(k, v)| (*k, *v))
            .collect();
        assert_eq!(result_set, expected_set, "Case {} mismatch", idx);
    }
}

// ============================================================================
// reconstruct_path 測試
// ============================================================================

#[test]
fn test_reconstruct_path() {
    let test_data = [
        // (描述, ascii, budget, 預期路徑)
        (
            "相鄰一格-上方",
            r#"
            . T . .
            . S . .
            . . . .
            "#,
            vec![(1, 1), (1, 0)],
        ),
        (
            "相鄰一格-右方",
            r#"
            . . . .
            . S T .
            . . . .
            "#,
            vec![(1, 1), (2, 1)],
        ),
        (
            "相鄰一格-下方",
            r#"
            . . . .
            . S . .
            . T . .
            "#,
            vec![(1, 1), (1, 2)],
        ),
        (
            "相鄰一格-左方",
            r#"
            . . . .
            T S . .
            . . . .
            "#,
            vec![(1, 1), (0, 1)],
        ),
        (
            "直線-右兩格",
            r#"
            S . T . .
            . . . . .
            "#,
            vec![(0, 0), (1, 0), (2, 0)],
        ),
        (
            "直線-右三格",
            r#"
            S . . T .
            . . . . .
            "#,
            vec![(0, 0), (1, 0), (2, 0), (3, 0)],
        ),
        (
            "直線-右四格",
            r#"
            . . . . .
            S . . . T
            "#,
            vec![(0, 1), (1, 1), (2, 1), (3, 1), (4, 1)],
        ),
        (
            "直線-上三格",
            r#"
            T .
            . .
            . .
            S .
            "#,
            vec![(0, 3), (0, 2), (0, 1), (0, 0)],
        ),
        (
            "直線-下三格",
            r#"
            S .
            . .
            . .
            T .
            "#,
            vec![(0, 0), (0, 1), (0, 2), (0, 3)],
        ),
        (
            "直線-左三格",
            r#"
            . . . .
            T . . S
            "#,
            vec![(3, 1), (2, 1), (1, 1), (0, 1)],
        ),
        (
            "中間有敵人-右方有敵人",
            r#"
            S E . .
            . T . .
            . . . .
            "#,
            vec![(0, 0), (0, 1), (1, 1)],
        ),
        (
            "中間有敵人-下方有敵人",
            r#"
            S . . .
            E T . .
            . . . .
            "#,
            vec![(0, 0), (1, 0), (1, 1)],
        ),
        (
            "中間有友軍-右方有敵人",
            r#"
            S E . .
            A T . .
            . . . .
            "#,
            vec![(0, 0), (0, 1), (1, 1)],
        ),
        (
            "中間有友軍-下方有敵人",
            r#"
            S A . .
            E T . .
            . . . .
            "#,
            vec![(0, 0), (1, 0), (1, 1)],
        ),
        (
            "中間有友軍-上方有敵人",
            r#"
            T E . .
            A S . .
            . . . .
            "#,
            vec![(1, 1), (0, 1), (0, 0)],
        ),
        (
            "中間有友軍-左方有敵人",
            r#"
            T A . .
            E S . .
            . . . .
            "#,
            vec![(1, 1), (1, 0), (0, 0)],
        ),
        (
            "繞過牆壁-L",
            r#"
            . . # . .
            . S # . .
            . . . . T
            "#,
            vec![(1, 1), (1, 2), (2, 2), (3, 2), (4, 2)],
        ),
        (
            "繞過牆壁-Z",
            r#"
            . . # . .
            . S . # .
            . # . . T
            "#,
            vec![(1, 1), (2, 1), (2, 2), (3, 2), (4, 2)],
        ),
        (
            "有水-直走",
            r#"
            . . # . .
            . S w . T
            . . . . .
            "#,
            vec![(1, 1), (2, 1), (3, 1), (4, 1)],
        ),
        (
            "有水-繞路",
            r#"
            . . # . .
            . S w w T
            . . . . .
            "#,
            vec![(1, 1), (1, 2), (2, 2), (3, 2), (4, 2), (4, 1)],
        ),
    ];

    for (desc, ascii, expected) in test_data {
        let (board, markers) = load_from_ascii(ascii).expect("load_from_ascii 應成功");
        let start = markers["S"][0];
        let target = markers["T"][0];
        let enemy = markers.get("E").map(|v| v[0]);
        let ally = markers.get("A").map(|v| v[0]);
        let wall_positions: Vec<Position> = markers.get("#").cloned().unwrap_or_default();
        let water_positions: Vec<Position> = markers.get("w").cloned().unwrap_or_default();

        let mover = Mover {
            pos: start,
            faction_alliance: PLAYER_ALLIANCE_ID,
        };
        let get_occupant = |pos: Position| {
            if enemy == Some(pos) {
                Some(999)
            } else if ally == Some(pos) {
                Some(PLAYER_ALLIANCE_ID)
            } else {
                None
            }
        };
        let get_terrain_cost = |pos: Position| {
            if wall_positions.contains(&pos) {
                IMPASSABLE_MOVEMENT_COST
            } else if water_positions.contains(&pos) {
                WATER_COST
            } else {
                NORMAL_COST
            }
        };
        let budget = NORMAL_COST * 10;

        let reachable = reachable_positions(board, mover, budget, get_occupant, get_terrain_cost)
            .unwrap_or_else(|e| panic!("Case {} reachable failed: {:?}", desc, e));

        let path = reconstruct_path(&reachable, start, target);
        let expected: Vec<Position> = expected
            .into_iter()
            .map(|(x, y)| Position { x, y })
            .collect();
        assert_eq!(path, expected, "Case {} path mismatch", desc);
    }
}

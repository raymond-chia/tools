//! 可移動範圍測試

use board::alias::MovementCost;
use board::component::{Faction, Position};
use board::constants::{BASIC_MOVEMENT_COST, IMPASSABLE_MOVEMENT_COST};
use board::loader::load_from_ascii;
use board::logic::movement::{
    Direction, Mover, ReachableInfo, reachable_positions, step_in_direction,
};
use std::collections::HashSet;

const NORMAL_COST: MovementCost = BASIC_MOVEMENT_COST;
const WATER_COST: MovementCost = BASIC_MOVEMENT_COST * 3;

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
        let (board, _, markers) = load_from_ascii(ascii).unwrap();
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
        let (board, _, markers) = load_from_ascii(ascii).unwrap();
        let pos = markers["S"][0];
        for d in directions {
            let result = step_in_direction(board, pos, *d);
            assert_eq!(result, None, "Case {}, direction {:?}", idx, d);
        }
    }
}

#[test]
fn test_reachable_positions_out_of_bound() {
    let ascii = r#"
. . 
. . 
            "#;
    let (board, _, _) = load_from_ascii(ascii).unwrap();
    for from in [
        Position { x: 0, y: 2 },
        Position { x: 2, y: 0 },
        Position { x: 2, y: 1 },
    ] {
        let mover = Mover {
            pos: from,
            faction: Faction(1),
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
                        prev: Position { x: 0, y: 0 },
                    },
                ),
                (
                    Position { x: 0, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
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
                        prev: Position { x: 4, y: 4 },
                    },
                ),
                (
                    Position { x: 4, y: 3 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
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
                        prev: Position { x: 0, y: 1 },
                    },
                ),
                (
                    Position { x: 1, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        prev: Position { x: 0, y: 1 },
                    },
                ),
                (
                    Position { x: 0, y: 2 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        prev: Position { x: 0, y: 1 },
                    },
                ),
            ],
        ),
    ];

    for (idx, (ascii, budget, expected)) in test_data.iter().enumerate() {
        let (board, _, markers) = load_from_ascii(ascii).unwrap();
        let from = markers["S"][0];
        let mover = Mover {
            pos: from,
            faction: Faction(1),
        };
        let result = reachable_positions(board, mover, *budget, |_| None, |_| NORMAL_COST)
            .unwrap_or_else(|e| panic!("Case {} failed: {:?}", idx, e));
        let expected_set: HashSet<_> = expected.iter().cloned().collect();
        let result_set: HashSet<_> = result.iter().map(|(k, v)| (*k, *v)).collect();
        assert_eq!(result_set, expected_set, "Case {} mismatch", idx);
    }
}

#[test]
fn test_reachable_positions_with_enemy() {
    let test_data = [
        (
            r#"
S X . . .
. . . . .
. . . . .
            "#,
            NORMAL_COST * 1,
            vec![(
                Position { x: 0, y: 1 },
                ReachableInfo {
                    cost: NORMAL_COST * 1,
                    prev: Position { x: 0, y: 0 },
                },
            )],
        ),
        (
            r#"
S X . . .
. . . . .
. . . . .
            "#,
            NORMAL_COST * 2,
            vec![
                (
                    Position { x: 0, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        prev: Position { x: 0, y: 0 },
                    },
                ),
                (
                    Position { x: 1, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 2,
                        prev: Position { x: 0, y: 1 },
                    },
                ),
                (
                    Position { x: 0, y: 2 },
                    ReachableInfo {
                        cost: NORMAL_COST * 2,
                        prev: Position { x: 0, y: 1 },
                    },
                ),
            ],
        ),
        (
            r#"
. . .
. X .
. . S
            "#,
            NORMAL_COST * 2,
            vec![
                (
                    Position { x: 2, y: 0 },
                    ReachableInfo {
                        cost: NORMAL_COST * 2,
                        prev: Position { x: 2, y: 1 },
                    },
                ),
                (
                    Position { x: 2, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        prev: Position { x: 2, y: 2 },
                    },
                ),
                (
                    Position { x: 0, y: 2 },
                    ReachableInfo {
                        cost: NORMAL_COST * 2,
                        prev: Position { x: 1, y: 2 },
                    },
                ),
                (
                    Position { x: 1, y: 2 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        prev: Position { x: 2, y: 2 },
                    },
                ),
            ],
        ),
    ];

    for (idx, (ascii, budget, expected)) in test_data.iter().enumerate() {
        let (board, _, markers) = load_from_ascii(ascii).unwrap();
        let from = markers["S"][0];
        let enemy_pos = markers["X"][0];

        let mover = Mover {
            pos: from,
            faction: Faction(1),
        };

        let get_occupant = |pos: Position| {
            if pos == enemy_pos {
                Some(Faction(2))
            } else {
                None
            }
        };

        let result = reachable_positions(board, mover, *budget, get_occupant, |_| NORMAL_COST)
            .unwrap_or_else(|e| panic!("Case {} failed: {:?}", idx, e));
        let expected_set: HashSet<_> = expected.iter().cloned().collect();
        let result_set: HashSet<_> = result.iter().map(|(k, v)| (*k, *v)).collect();
        assert_eq!(result_set, expected_set, "Case {} mismatch", idx);
    }
}

#[test]
fn test_reachable_positions_with_ally() {
    let test_data = [
        (
            r#"
S X . . .
. . . . .
. . . . .
            "#,
            NORMAL_COST * 1,
            vec![(
                Position { x: 0, y: 1 },
                ReachableInfo {
                    cost: NORMAL_COST * 1,
                    prev: Position { x: 0, y: 0 },
                },
            )],
        ),
        (
            r#"
S X . . .
. X . . .
. . . . .
            "#,
            NORMAL_COST * 2,
            vec![
                (
                    Position { x: 2, y: 0 },
                    ReachableInfo {
                        cost: NORMAL_COST * 2,
                        prev: Position { x: 1, y: 0 },
                    },
                ),
                (
                    Position { x: 0, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        prev: Position { x: 0, y: 0 },
                    },
                ),
                (
                    Position { x: 0, y: 2 },
                    ReachableInfo {
                        cost: NORMAL_COST * 2,
                        prev: Position { x: 0, y: 1 },
                    },
                ),
            ],
        ),
        (
            r#"
. . S
. X X
. . .
            "#,
            NORMAL_COST * 2,
            vec![
                (
                    Position { x: 0, y: 0 },
                    ReachableInfo {
                        cost: NORMAL_COST * 2,
                        prev: Position { x: 1, y: 0 },
                    },
                ),
                (
                    Position { x: 1, y: 0 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        prev: Position { x: 2, y: 0 },
                    },
                ),
                (
                    Position { x: 2, y: 2 },
                    ReachableInfo {
                        cost: NORMAL_COST * 2,
                        prev: Position { x: 2, y: 1 },
                    },
                ),
            ],
        ),
    ];

    for (idx, (ascii, budget, expected)) in test_data.iter().enumerate() {
        let (board, _, markers) = load_from_ascii(ascii).unwrap();
        let from = markers["S"][0];
        let ally_positions = &markers["X"];

        let mover = Mover {
            pos: from,
            faction: Faction(1),
        };

        let get_occupant = |pos: Position| {
            if ally_positions.contains(&pos) {
                Some(Faction(1))
            } else {
                None
            }
        };

        let result = reachable_positions(board, mover, *budget, get_occupant, |_| NORMAL_COST)
            .unwrap_or_else(|e| panic!("Case {} failed: {:?}", idx, e));

        let expected_set: HashSet<_> = expected.iter().cloned().collect();
        let result_set: HashSet<_> = result.iter().map(|(k, v)| (*k, *v)).collect();
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
                        prev: Position { x: 0, y: 0 },
                    },
                ),
                (
                    Position { x: 2, y: 0 },
                    ReachableInfo {
                        cost: NORMAL_COST * 2,
                        prev: Position { x: 1, y: 0 },
                    },
                ),
                (
                    Position { x: 0, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        prev: Position { x: 0, y: 0 },
                    },
                ),
                (
                    Position { x: 0, y: 2 },
                    ReachableInfo {
                        cost: NORMAL_COST * 2,
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
        let (board, _, markers) = load_from_ascii(ascii).unwrap();
        let from = markers["S"][0];
        let wall_positions = &markers["#"];

        let mover = Mover {
            pos: from,
            faction: Faction(1),
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
        let result_set: HashSet<_> = result.iter().map(|(k, v)| (*k, *v)).collect();
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
                        prev: Position { x: 0, y: 0 },
                    },
                ),
                (
                    Position { x: 0, y: 2 },
                    ReachableInfo {
                        cost: NORMAL_COST * 2,
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
                        prev: Position { x: 0, y: 0 },
                    },
                ),
                (
                    Position { x: 2, y: 0 },
                    ReachableInfo {
                        cost: NORMAL_COST * 4,
                        prev: Position { x: 1, y: 0 },
                    },
                ),
                (
                    Position { x: 0, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        prev: Position { x: 0, y: 0 },
                    },
                ),
                (
                    Position { x: 1, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 4,
                        prev: Position { x: 0, y: 1 },
                    },
                ),
                (
                    Position { x: 0, y: 2 },
                    ReachableInfo {
                        cost: NORMAL_COST * 2,
                        prev: Position { x: 0, y: 1 },
                    },
                ),
                (
                    Position { x: 1, y: 2 },
                    ReachableInfo {
                        cost: NORMAL_COST * 3,
                        prev: Position { x: 0, y: 2 },
                    },
                ),
                (
                    Position { x: 2, y: 2 },
                    ReachableInfo {
                        cost: NORMAL_COST * 4,
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
                        prev: Position { x: 0, y: 0 },
                    },
                ),
                (
                    Position { x: 2, y: 0 },
                    ReachableInfo {
                        cost: NORMAL_COST * 6,
                        prev: Position { x: 1, y: 0 },
                    },
                ),
                (
                    Position { x: 3, y: 0 },
                    ReachableInfo {
                        cost: NORMAL_COST * 5,
                        prev: Position { x: 3, y: 1 },
                    },
                ),
                (
                    Position { x: 4, y: 0 },
                    ReachableInfo {
                        cost: NORMAL_COST * 6,
                        prev: Position { x: 3, y: 0 },
                    },
                ),
                (
                    Position { x: 0, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 1,
                        prev: Position { x: 0, y: 0 },
                    },
                ),
                (
                    Position { x: 1, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 2,
                        prev: Position { x: 0, y: 1 },
                    },
                ),
                (
                    Position { x: 2, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 3,
                        prev: Position { x: 1, y: 1 },
                    },
                ),
                (
                    Position { x: 3, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 4,
                        prev: Position { x: 2, y: 1 },
                    },
                ),
                (
                    Position { x: 4, y: 1 },
                    ReachableInfo {
                        cost: NORMAL_COST * 5,
                        prev: Position { x: 3, y: 1 },
                    },
                ),
            ],
        ),
    ];

    for (idx, (ascii, budget, expected)) in test_data.iter().enumerate() {
        let (board, _, markers) = load_from_ascii(ascii).unwrap();
        let from = markers["S"][0];
        let water_positions = &markers["w"];

        let mover = Mover {
            pos: from,
            faction: Faction(1),
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
        let result_set: HashSet<_> = result.iter().map(|(k, v)| (*k, *v)).collect();
        assert_eq!(result_set, expected_set, "Case {} mismatch", idx);
    }
}

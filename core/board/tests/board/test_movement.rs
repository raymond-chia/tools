//! 移動路徑測試

use board::alias::MovementCost;
use board::component::{Board, Faction, Position};
use board::loader::load_from_ascii;
use board::logic::BASIC_MOVEMENT_COST;
use board::logic::movement::{Direction, Mover, manhattan_path, step_in_direction};

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
fn test_manhattan_path_valid() {
    let test_data = [
        (
            r#"
S . . E .
. . . . .
. . . . .
. . . . .
. . . . .
            "#,
            vec![
                Position { x: 1, y: 0 },
                Position { x: 2, y: 0 },
                Position { x: 3, y: 0 },
            ],
        ),
        (
            r#"
S . . . .
. . . . .
. . . . .
E . . . .
. . . . .
            "#,
            vec![
                Position { x: 0, y: 1 },
                Position { x: 0, y: 2 },
                Position { x: 0, y: 3 },
            ],
        ),
        (
            r#"
S . . . .
. . . . .
. . E . .
. . . . .
. . . . .
            "#,
            vec![
                Position { x: 1, y: 0 },
                Position { x: 2, y: 0 },
                Position { x: 2, y: 1 },
                Position { x: 2, y: 2 },
            ],
        ),
        (
            r#"
E . . . .
. . . . .
. . . . .
. . . S .
. . . . .
            "#,
            vec![
                Position { x: 2, y: 3 },
                Position { x: 1, y: 3 },
                Position { x: 0, y: 3 },
                Position { x: 0, y: 2 },
                Position { x: 0, y: 1 },
                Position { x: 0, y: 0 },
            ],
        ),
        (
            r#"
. . . . . E .
. . . . . . .
. . . . . . .
. . . . . . .
. . . . . . .
. S . . . . .
. . . . . . .
            "#,
            vec![
                Position { x: 2, y: 5 },
                Position { x: 3, y: 5 },
                Position { x: 4, y: 5 },
                Position { x: 5, y: 5 },
                Position { x: 5, y: 4 },
                Position { x: 5, y: 3 },
                Position { x: 5, y: 2 },
                Position { x: 5, y: 1 },
                Position { x: 5, y: 0 },
            ],
        ),
        (
            r#"
E . . . .
. . . S .
            "#,
            vec![
                Position { x: 2, y: 1 },
                Position { x: 1, y: 1 },
                Position { x: 0, y: 1 },
                Position { x: 0, y: 0 },
            ],
        ),
    ];

    for (idx, (ascii, expected)) in test_data.iter().enumerate() {
        let (board, _, markers) = load_from_ascii(ascii).unwrap();
        let from = markers["S"][0];
        let to = markers["E"][0];
        let mover = Mover {
            pos: from,
            faction: Faction(1),
        };
        let (path, _cost) = manhattan_path(board, mover, to, |_| None, |_| 1)
            .unwrap_or_else(|e| panic!("Case {} failed: {:?}", idx, e));
        assert_eq!(path, *expected, "Case {} path mismatch", idx);
    }
}

#[test]
fn test_manhattan_path_out_of_bounds() {
    let test_data = [
        (
            // 起點超出邊界
            Board {
                width: 3,
                height: 3,
            },
            Position { x: 5, y: 0 },
            Position { x: 1, y: 1 },
        ),
        (
            // 終點超出邊界
            Board {
                width: 3,
                height: 3,
            },
            Position { x: 0, y: 0 },
            Position { x: 5, y: 5 },
        ),
    ];

    for (idx, (board, from, to)) in test_data.iter().enumerate() {
        let mover = Mover {
            pos: *from,
            faction: Faction(1),
        };
        let result = manhattan_path(*board, mover, *to, |_| None, |_| 1);
        assert!(result.is_err(), "Case {} should fail", idx);
    }
}

#[test]
fn test_manhattan_path_edge_cases() {
    // 無移動：起點等於終點
    let ascii = r#"
. . . . .
. S . . .
. . . . .
. . . . .
. . . . .
    "#;
    let (board, _, markers) = load_from_ascii(ascii).unwrap();
    let from = markers["S"][0];
    let to = from;
    let mover = Mover {
        pos: from,
        faction: Faction(1),
    };
    let (path, cost) = manhattan_path(board, mover, to, |_| None, |_| 1).unwrap();
    assert_eq!(path, vec![]);
    assert_eq!(cost, 0);
}

#[test]
fn test_manhattan_path_around_enemy() {
    let test_data = [
        (
            r#"
S X . E
. . . .
. . . .
            "#,
            vec![
                Position { x: 0, y: 1 },
                Position { x: 1, y: 1 },
                Position { x: 2, y: 1 },
                Position { x: 3, y: 1 },
                Position { x: 3, y: 0 },
            ],
        ),
        (
            r#"
S X . . . E
. . . . . .
. . . . . .
            "#,
            vec![
                Position { x: 0, y: 1 },
                Position { x: 1, y: 1 },
                Position { x: 2, y: 1 },
                Position { x: 3, y: 1 },
                Position { x: 4, y: 1 },
                Position { x: 5, y: 1 },
                Position { x: 5, y: 0 },
            ],
        ),
        (
            r#"
S . . . .
X . . . .
. . . . E
            "#,
            vec![
                Position { x: 1, y: 0 },
                Position { x: 2, y: 0 },
                Position { x: 3, y: 0 },
                Position { x: 4, y: 0 },
                Position { x: 4, y: 1 },
                Position { x: 4, y: 2 },
            ],
        ),
    ];

    for (idx, (ascii, expected_path)) in test_data.iter().enumerate() {
        let (board, _, markers) = load_from_ascii(ascii).unwrap();
        let from = markers["S"][0];
        let to = markers["E"][0];
        let obstacle_pos = markers["X"][0];

        let mover = Mover {
            pos: from,
            faction: Faction(1),
        };

        let get_occupant = |pos: Position| {
            if pos == obstacle_pos {
                Some(Faction(2))
            } else {
                None
            }
        };

        let (path, _cost) = manhattan_path(board, mover, to, get_occupant, |_| 1)
            .unwrap_or_else(|e| panic!("Case {} failed: {:?}", idx, e));
        assert_eq!(path, *expected_path, "Case {} path mismatch", idx);
    }
}

#[test]
fn test_manhattan_path_through_ally() {
    // 友軍可穿越，不阻擋路徑
    let test_data = [
        (
            r#"
S X . E
. . . .
. . . .
            "#,
            vec![
                Position { x: 1, y: 0 },
                Position { x: 2, y: 0 },
                Position { x: 3, y: 0 },
            ],
        ),
        (
            r#"
S . . .
X . . .
. . . E
            "#,
            vec![
                Position { x: 1, y: 0 },
                Position { x: 2, y: 0 },
                Position { x: 3, y: 0 },
                Position { x: 3, y: 1 },
                Position { x: 3, y: 2 },
            ],
        ),
        (
            r#"
S X . .
. . . .
. . E .
            "#,
            vec![
                Position { x: 1, y: 0 },
                Position { x: 2, y: 0 },
                Position { x: 2, y: 1 },
                Position { x: 2, y: 2 },
            ],
        ),
    ];

    for (idx, (ascii, expected_path)) in test_data.iter().enumerate() {
        let (board, _, markers) = load_from_ascii(ascii).unwrap();
        let from = markers["S"][0];
        let to = markers["E"][0];
        let ally_pos = markers["X"][0];

        let mover = Mover {
            pos: from,
            faction: Faction(1),
        };

        let get_occupant = |pos: Position| {
            if pos == ally_pos {
                Some(Faction(1)) // 友軍（同陣營）
            } else {
                None
            }
        };

        let (path, _cost) = manhattan_path(board, mover, to, get_occupant, |_| 1)
            .unwrap_or_else(|e| panic!("Case {} failed: {:?}", idx, e));
        assert_eq!(path, *expected_path, "Case {} path mismatch", idx);
    }
}

#[test]
fn test_manhattan_path_unit_at_destination() {
    let test_data = [
        r#"
S . . X
. . . .
. . . .
        "#,
        r#"
S . . .
. . . .
. . . .
X . . .
        "#,
        r#"
S . . . .
. . . . .
. . X . .
        "#,
    ];

    for (idx, ascii) in test_data.iter().enumerate() {
        let (board, _, markers) = load_from_ascii(ascii).unwrap();
        let from = markers["S"][0];
        let to = markers["X"][0];

        let mover = Mover {
            pos: from,
            faction: Faction(1),
        };

        for faction in [Faction(1), Faction(2)] {
            let get_occupant = |pos: Position| {
                if pos == to { Some(faction) } else { None }
            };

            let result = manhattan_path(board, mover, to, get_occupant, |_| 1);
            assert!(
                result.is_err(),
                "Case {} should fail - destination blocked by unit of faction {:?}",
                idx,
                faction
            );
        }
    }
}

#[test]
fn test_manhattan_path_with_wall_obstacles() {
    // 測試牆壁（不可通行）阻擋路徑
    let test_data = [
        (
            r#"
S # . E
. # . .
. . . .
            "#,
            vec![
                Position { x: 0, y: 1 },
                Position { x: 0, y: 2 },
                Position { x: 1, y: 2 },
                Position { x: 2, y: 2 },
                Position { x: 3, y: 2 },
                Position { x: 3, y: 1 },
                Position { x: 3, y: 0 },
            ],
        ),
        (
            r#"
S . . . E
# . . . .
# . . . .
. . . . .
            "#,
            vec![
                Position { x: 1, y: 0 },
                Position { x: 2, y: 0 },
                Position { x: 3, y: 0 },
                Position { x: 4, y: 0 },
            ],
        ),
        (
            r#"
S . . E .
. # # . .
. . . . .
            "#,
            vec![
                Position { x: 1, y: 0 },
                Position { x: 2, y: 0 },
                Position { x: 3, y: 0 },
            ],
        ),
        (
            r#"
S . . . .
# # # . .
. E . . .
            "#,
            vec![
                Position { x: 1, y: 0 },
                Position { x: 2, y: 0 },
                Position { x: 3, y: 0 },
                Position { x: 3, y: 1 },
                Position { x: 3, y: 2 },
                Position { x: 2, y: 2 },
                Position { x: 1, y: 2 },
            ],
        ),
    ];

    for (idx, (ascii, expected)) in test_data.iter().enumerate() {
        let (board, _, markers) = load_from_ascii(ascii).unwrap();
        let from = markers["S"][0];
        let to = markers["E"][0];
        let wall_positions = &markers["#"];

        let mover = Mover {
            pos: from,
            faction: Faction(1),
        };

        let get_terrain_cost = |pos: Position| {
            if wall_positions.contains(&pos) {
                MovementCost::MAX // 牆壁不可通行
            } else {
                BASIC_MOVEMENT_COST // 普通地形
            }
        };

        let (path, _cost) = manhattan_path(board, mover, to, |_| None, get_terrain_cost)
            .unwrap_or_else(|e| panic!("Case {} failed: {:?}", idx, e));

        assert_eq!(path, *expected, "Case {} path mismatch", idx);
    }
}

#[test]
fn test_manhattan_path_wall_completely_blocks() {
    // 測試牆壁完全阻擋的情況
    let test_data = [
        r#"
S # E
# # .
. . .
            "#,
        r#"
S # . E
. # . .
. # . .
            "#,
        r#"
S # . E
. # # #
. . . .
            "#,
        r#"
S . .
# # #
E . .
            "#,
    ];

    for (idx, ascii) in test_data.iter().enumerate() {
        let (board, _, markers) = load_from_ascii(ascii).unwrap();
        let from = markers["S"][0];
        let to = markers["E"][0];
        let wall_positions = &markers["#"];

        let mover = Mover {
            pos: from,
            faction: Faction(1),
        };

        let get_terrain_cost = |pos: Position| {
            if wall_positions.contains(&pos) {
                MovementCost::MAX
            } else {
                BASIC_MOVEMENT_COST
            }
        };

        let result = manhattan_path(board, mover, to, |_| None, get_terrain_cost);
        assert!(result.is_err(), "Case {} should be unreachable", idx);
    }
}

#[test]
fn test_manhattan_path_with_water_terrain() {
    let test_data = [
        (
            r#"
S w . E
w w w .
. . . .
            "#,
            WATER_COST + NORMAL_COST * 2,
            vec![
                Position { x: 1, y: 0 },
                Position { x: 2, y: 0 },
                Position { x: 3, y: 0 },
            ],
        ),
        (
            r#"
S . . .
w . . E
. . . .
            "#,
            NORMAL_COST * 4,
            vec![
                Position { x: 1, y: 0 },
                Position { x: 2, y: 0 },
                Position { x: 3, y: 0 },
                Position { x: 3, y: 1 },
            ],
        ),
        (
            r#"
S w . .
. . . E
. . . .
            "#,
            NORMAL_COST * 4,
            vec![
                Position { x: 0, y: 1 },
                Position { x: 1, y: 1 },
                Position { x: 2, y: 1 },
                Position { x: 3, y: 1 },
            ],
        ),
        (
            r#"
S w w E 
. . . .
. . . .
            "#,
            NORMAL_COST * 5,
            vec![
                Position { x: 0, y: 1 },
                Position { x: 1, y: 1 },
                Position { x: 2, y: 1 },
                Position { x: 3, y: 1 },
                Position { x: 3, y: 0 },
            ],
        ),
        (
            r#"
S w w w E
. w . . .
. . w . .
            "#,
            WATER_COST + NORMAL_COST * 5,
            vec![
                Position { x: 0, y: 1 },
                Position { x: 1, y: 1 },
                Position { x: 2, y: 1 },
                Position { x: 3, y: 1 },
                Position { x: 4, y: 1 },
                Position { x: 4, y: 0 },
            ],
        ),
        (
            r#"
S w w w E
. w . w .
. . . . .
            "#,
            NORMAL_COST * 8,
            vec![
                Position { x: 0, y: 1 },
                Position { x: 0, y: 2 },
                Position { x: 1, y: 2 },
                Position { x: 2, y: 2 },
                Position { x: 3, y: 2 },
                Position { x: 4, y: 2 },
                Position { x: 4, y: 1 },
                Position { x: 4, y: 0 },
            ],
        ),
    ];

    for (idx, (ascii, expected_cost, expected_path)) in test_data.iter().enumerate() {
        let (board, _, markers) = load_from_ascii(ascii).unwrap();
        let from = markers["S"][0];
        let to = markers["E"][0];
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

        // enough movement cost
        let (path, cost) = manhattan_path(board, mover, to, |_| None, get_terrain_cost)
            .unwrap_or_else(|e| panic!("Case {} failed: {:?}", idx, e));
        assert_eq!(path, *expected_path, "Case {} path mismatch", idx);
        assert_eq!(cost, *expected_cost, "Case {} cost mismatch", idx);
    }
}

#[test]
fn test_manhattan_path_mixed_wall_and_water() {
    let test_data = [
        (
            r#"
S . # . E
. w # w .
. . . . .
            "#,
            NORMAL_COST * 8,
            vec![
                Position { x: 0, y: 1 },
                Position { x: 0, y: 2 },
                Position { x: 1, y: 2 },
                Position { x: 2, y: 2 },
                Position { x: 3, y: 2 },
                Position { x: 4, y: 2 },
                Position { x: 4, y: 1 },
                Position { x: 4, y: 0 },
            ],
        ),
        (
            r#"
S . w . E
. . . # .
. . . # .
            "#,
            WATER_COST + NORMAL_COST * 3,
            vec![
                Position { x: 1, y: 0 },
                Position { x: 2, y: 0 },
                Position { x: 3, y: 0 },
                Position { x: 4, y: 0 },
            ],
        ),
        (
            r#"
S # . E 
w w . .
. w . .
            "#,
            WATER_COST * 2 + NORMAL_COST * 3,
            vec![
                Position { x: 0, y: 1 },
                Position { x: 1, y: 1 },
                Position { x: 2, y: 1 },
                Position { x: 3, y: 1 },
                Position { x: 3, y: 0 },
            ],
        ),
        (
            r#"
S # . E 
w w w .
. . . .
            "#,
            WATER_COST + NORMAL_COST * 6,
            vec![
                Position { x: 0, y: 1 },
                Position { x: 0, y: 2 },
                Position { x: 1, y: 2 },
                Position { x: 2, y: 2 },
                Position { x: 3, y: 2 },
                Position { x: 3, y: 1 },
                Position { x: 3, y: 0 },
            ],
        ),
    ];

    for (idx, (ascii, expected_cost, expected_path)) in test_data.iter().enumerate() {
        let (board, _, markers) = load_from_ascii(ascii).unwrap();
        let from = markers["S"][0];
        let to = markers["E"][0];
        let wall_positions = &markers["#"];
        let water_positions = &markers["w"];

        let mover = Mover {
            pos: from,
            faction: Faction(1),
        };

        let get_terrain_cost = |pos: Position| {
            if wall_positions.contains(&pos) {
                MovementCost::MAX // 牆壁不可通行
            } else if water_positions.contains(&pos) {
                WATER_COST
            } else {
                NORMAL_COST
            }
        };

        // enough movement cost
        let (path, cost) = manhattan_path(board, mover, to, |_| None, get_terrain_cost)
            .unwrap_or_else(|e| panic!("Case {} failed: {:?}", idx, e));
        assert_eq!(path, *expected_path, "Case {} path mismatch", idx);
        assert_eq!(cost, *expected_cost, "Case {} cost mismatch", idx);
    }
}

//! 移動路徑測試

use board::component::{Board, Faction, Position};
use board::loader::load_from_ascii;
use board::logic::movement::{Direction, Mover, manhattan_path, step_in_direction};

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
        let pos = markers["S"];
        let expected = markers["E"];
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
        let pos = markers["S"];
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
        let from = markers["S"];
        let to = markers["E"];
        let mover = Mover {
            pos: from,
            faction: Faction(1),
        };
        let path = manhattan_path(board, mover, to, |_| None)
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
        let result = manhattan_path(*board, mover, *to, |_| None);
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
    let from = markers["S"];
    let to = from;
    let mover = Mover {
        pos: from,
        faction: Faction(1),
    };
    let path = manhattan_path(board, mover, to, |_| None).unwrap();
    assert_eq!(path, vec![]);
}

#[test]
fn test_manhattan_path_around_obstacle() {
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
        let from = markers["S"];
        let to = markers["E"];
        let obstacle_pos = markers["X"];

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

        let path = manhattan_path(board, mover, to, get_occupant)
            .unwrap_or_else(|e| panic!("Case {} failed: {:?}", idx, e));
        assert_eq!(path, *expected_path, "Case {} path mismatch", idx);
    }
}

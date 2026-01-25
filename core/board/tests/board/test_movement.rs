//! 移動路徑測試

use board::component::{Board, Position};
use board::loader::load_from_ascii;
use board::system::movement::{manhattan_path, step_in_direction};
use board::typ::Direction;

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

    for (ascii, direction) in test_data {
        let (board, _, markers) = load_from_ascii(ascii).unwrap();
        let pos = markers["S"];
        let expected = markers["E"];
        let result = step_in_direction(board, pos, direction);
        assert_eq!(result, Some(expected));
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

    for (ascii, directions) in test_data {
        let (board, _, markers) = load_from_ascii(ascii).unwrap();
        let pos = markers["S"];
        for d in directions {
            let result = step_in_direction(board, pos, d);
            assert_eq!(result, None);
        }
    }
}

#[test]
fn test_manhattan_path_valid() {
    let test_data = [
        (
            // 水平移動：S → → → E
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
            // 垂直移動：S ↓ ↓ ↓ E
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
            // L 形移動：S → → ↓ ↓ E
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
            // 反向 L 形移動：S ← ← ↑ ↑ E
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
            // 反向 L 形移動：S ← ← ↑ ↑ E
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

    for (ascii, expected) in test_data {
        let (board, _, markers) = load_from_ascii(ascii).unwrap();
        let from = markers["S"];
        let to = markers["E"];
        let path = manhattan_path(board, from, to).unwrap();
        assert_eq!(path, expected);
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

    for (board, from, to) in test_data {
        let result = manhattan_path(board, from, to);
        assert!(result.is_err());
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
    let path = manhattan_path(board, from, to).unwrap();
    assert_eq!(path, vec![]);
}

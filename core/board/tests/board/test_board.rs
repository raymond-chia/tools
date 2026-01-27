use board::component::Position;
use board::loader::load_from_ascii;
use board::logic::board::is_valid_position;

#[test]
fn test_ascii_to_board_3x3() {
    // 3x3
    let ascii = r#"
. . .
. . .
. . .
    "#;

    let (board, positions, _markers) = load_from_ascii(ascii).unwrap();

    assert_eq!(board.width, 3);
    assert_eq!(board.height, 3);
    assert_eq!(positions.len(), 9);

    // 驗證所有位置都在棋盤內且有效
    for &pos in &positions {
        assert!(is_valid_position(board, pos));
    }

    // 邊界內有效
    assert!(is_valid_position(board, Position { x: 0, y: 0 }));
    assert!(is_valid_position(board, Position { x: 2, y: 2 }));
    assert!(is_valid_position(board, Position { x: 1, y: 1 }));

    // 邊界外無效
    assert!(!is_valid_position(board, Position { x: 3, y: 0 }));
    assert!(!is_valid_position(board, Position { x: 0, y: 3 }));
    assert!(!is_valid_position(board, Position { x: 3, y: 3 }));
}

#[test]
fn test_ascii_to_board_8x8() {
    // 8x8
    let ascii = r#"
. . . . . . . .
. . . . . . . .
. . . . . . . .
. . . . . . . .
. . . . . . . .
. . . . . . . .
. . . . . . . .
. . . . . . . .
    "#;

    let (board, positions, _markers) = load_from_ascii(ascii).unwrap();

    assert_eq!(board.width, 8);
    assert_eq!(board.height, 8);
    assert_eq!(positions.len(), 64);

    // 驗證所有位置都在棋盤內且有效
    for pos in positions {
        assert!(is_valid_position(board, pos));
    }

    // 驗證四個角都有效
    assert!(is_valid_position(board, Position { x: 0, y: 0 }));
    assert!(is_valid_position(board, Position { x: 7, y: 0 }));
    assert!(is_valid_position(board, Position { x: 0, y: 7 }));
    assert!(is_valid_position(board, Position { x: 7, y: 7 }));

    // 邊界外無效
    assert!(!is_valid_position(board, Position { x: 8, y: 0 }));
    assert!(!is_valid_position(board, Position { x: 0, y: 8 }));
    assert!(!is_valid_position(board, Position { x: 8, y: 8 }));
}

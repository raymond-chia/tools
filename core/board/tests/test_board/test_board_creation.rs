use board::types::Board;
use board::types::Pos;

#[test]
fn test_create_valid_board() {
    Board::new(10, 10).expect("應該成功建立有效棋盤");
}

#[test]
fn test_create_board_with_invalid_dimensions() {
    let result_zero_width = Board::new(0, 10);
    assert!(result_zero_width.is_err());

    let result_zero_height = Board::new(10, 0);
    assert!(result_zero_height.is_err());
}

#[test]
fn test_is_valid_position() {
    let board = Board::new(5, 5).expect("應該成功建立棋盤");

    assert!(board.is_valid_position(&Pos { x: 0, y: 0 }));
    assert!(board.is_valid_position(&Pos { x: 4, y: 4 }));
    assert!(!board.is_valid_position(&Pos { x: 5, y: 0 }));
    assert!(!board.is_valid_position(&Pos { x: 0, y: 5 }));
    assert!(!board.is_valid_position(&Pos { x: 10, y: 10 }));
}

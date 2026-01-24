use crate::common::SceneBuilder;

#[test]
fn test_parse_board_size() {
    let scene_str = r#"
        5x5 board

        . . . . .
        . . . . .
        . . . . .
        . . . . .
        . . . . .
    "#;

    let scene = SceneBuilder::parse(scene_str).expect("應能解析棋盤");
    assert_eq!(scene.width(), 5);
    assert_eq!(scene.height(), 5);
}

#[test]
fn test_parse_board_3x4() {
    let scene_str = r#"
        3x4 board

        . . .
        . . .
        . . .
        . . .
    "#;

    let scene = SceneBuilder::parse(scene_str).expect("應能解析棋盤");
    assert_eq!(scene.width(), 3);
    assert_eq!(scene.height(), 4);
}

#[test]
fn test_invalid_dimensions_format() {
    let scene_str = r#"
        5-5 board

        . . . . .
    "#;

    let result = SceneBuilder::parse(scene_str);
    assert!(result.is_err());
}

#[test]
fn test_zero_dimension() {
    let scene_str = r#"
        0x5 board
    "#;

    let result = SceneBuilder::parse(scene_str);
    assert!(result.is_err());
}

#[test]
fn test_row_width_mismatch() {
    let scene_str = r#"
        3x3 board

        . . .
        . .
        . . .
    "#;

    let result = SceneBuilder::parse(scene_str);
    assert!(result.is_err());
}

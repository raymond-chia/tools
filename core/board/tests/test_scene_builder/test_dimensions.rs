use crate::common::SceneBuilder;

#[test]
fn test_scene_builder_dimensions_success() {
    let cases = vec![
        (
            r#"
            5x5 board

            . . . . .
            . . . . .
            . . . . .
            . . . . .
            . . . . .
        "#,
            5,
            5,
        ),
        (
            r#"
            3x4 board

            . . .
            . . .
            . . .
            . . .
        "#,
            3,
            4,
        ),
    ];

    for (scene_str, expected_width, expected_height) in cases {
        let scene = SceneBuilder::parse(scene_str).expect("應能解析棋盤");
        assert_eq!(scene.width(), expected_width);
        assert_eq!(scene.height(), expected_height);
    }
}

#[test]
fn test_scene_builder_dimensions_failure() {
    let cases = vec![
        r#"
            5-5 board

            . . . . .
        "#,
        r#"
            0x5 board
        "#,
        r#"
            3x3 board

            . . .
            . .
            . . .
        "#,
    ];

    for scene_str in cases {
        let result = SceneBuilder::parse(scene_str);
        assert!(result.is_err());
    }
}

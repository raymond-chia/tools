use crate::ecs_types::components::Position;
use crate::logic::skill::line_of_sight::has_line_of_sight;
use crate::test_helpers::level_builder::load_from_ascii;
use std::collections::{HashMap, HashSet};

fn get_first(markers: &HashMap<String, Vec<Position>>, key: &str) -> Position {
    markers[key][0]
}

fn get_blocking(markers: &HashMap<String, Vec<Position>>) -> HashSet<Position> {
    markers
        .get("W")
        .map(|v| v.iter().copied().collect())
        .unwrap_or_default()
}

#[test]
fn test_los() {
    let test_data: &[(&str, bool, &str)] = &[
        (
            r#"
A . . . T
. . . . .
. . . . .
. . . . .
. . . . ."#,
            true,
            "水平直線無阻擋",
        ),
        (
            r#"
A . . . .
. . . . .
. . . . .
. . . . .
T . . . ."#,
            true,
            "垂直直線無阻擋",
        ),
        (
            r#"
A . . . .
. . . . .
. . . . .
. . . . .
. . . . T"#,
            true,
            "斜線無阻擋",
        ),
        (
            r#"
A . W . T
. . . . .
. . . . .
. . . . .
. . . . ."#,
            false,
            "水平中途阻擋",
        ),
        (
            r#"
A . . . .
. . . . .
W . . . .
. . . . .
T . . . ."#,
            false,
            "垂直中途阻擋",
        ),
        (
            r#"
A . . . .
. W . . .
. . . . .
. . . . .
. . . . T"#,
            false,
            "斜線中途阻擋",
        ),
        (
            r#"
A . . . T
W . W . .
. W . . .
. . . . .
. . . . ."#,
            true,
            "阻擋物不在直線上",
        ),
        (
            r#"
A . . . .
W . W . .
. W . . .
. . . . .
. . . . T"#,
            true,
            "阻擋物不在斜線上",
        ),
        (
            r#"
A . W . .
W . W . T
. W . . .
. . . . .
. . . . ."#,
            false,
            "奇怪斜率",
        ),
        (
            r#"
A . W . .
W . W . .
. W . . .
. . . . .
. T . . ."#,
            false,
            "奇怪斜率",
        ),
    ];

    for &(ascii, expected, label) in test_data {
        let (_, markers) = load_from_ascii(ascii).expect("ASCII 解析失敗");
        let from = get_first(&markers, "A");
        let to = get_first(&markers, "T");
        let blocking = get_blocking(&markers);
        assert_eq!(has_line_of_sight(from, to, &blocking), expected, "{label}");
    }
}

#[test]
fn test_los_blocker_on_endpoint() {
    // 起點有阻擋：blocking = A 的位置
    // 終點有阻擋：blocking = T 的位置
    // 起點等於終點且有阻擋：from = to = A 的位置，blocking = A 的位置
    let ascii_start_blocked = r#"
A . . . T"#;
    let ascii_end_blocked = r#"
A . . . T"#;
    let ascii_same_pos = r#"
A . . . ."#;

    let (_, m) = load_from_ascii(ascii_start_blocked).expect("ASCII 解析失敗");
    let from = get_first(&m, "A");
    let to = get_first(&m, "T");
    let blocking: HashSet<Position> = [from].into_iter().collect();
    assert!(!has_line_of_sight(from, to, &blocking), "起點有阻擋物");

    let (_, m) = load_from_ascii(ascii_end_blocked).expect("ASCII 解析失敗");
    let from = get_first(&m, "A");
    let to = get_first(&m, "T");
    let blocking: HashSet<Position> = [to].into_iter().collect();
    assert!(!has_line_of_sight(from, to, &blocking), "終點有阻擋物");

    let (_, m) = load_from_ascii(ascii_same_pos).expect("ASCII 解析失敗");
    let from = get_first(&m, "A");
    let to = from;
    let blocking: HashSet<Position> = [from].into_iter().collect();
    assert!(
        has_line_of_sight(from, to, &blocking),
        "起點等於終點應回傳 true"
    );
}

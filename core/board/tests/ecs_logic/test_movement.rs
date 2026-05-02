//! execute_move 整合測試

use bevy_ecs::world::World;
use board::domain::constants::BASIC_MOVEMENT_COST;
use board::ecs_logic::movement::execute_move;
use board::ecs_logic::turn::{end_current_turn, start_new_round};
use board::ecs_types::components::{Occupant, Position};
use board::test_helpers::level_builder::load_from_ascii;

/// 從 ASCII 建構 World 並回傳玩家單位 Occupant 與目的地座標
///
/// 目的地標記：T 或 T1/T2，按 T → T1 → T2 順序收集回傳
fn build_world(ascii: &str) -> (World, Occupant, Vec<Position>) {
    let (world, occupant, markers) = super::build_warrior_world(ascii);

    // 收集目的地：T → T1 → T2
    const MARKER_TARGET: &str = "T";
    const MARKER_TARGET_1: &str = "T1";
    const MARKER_TARGET_2: &str = "T2";
    let mut targets = Vec::new();
    for key in [MARKER_TARGET, MARKER_TARGET_1, MARKER_TARGET_2] {
        if let Some(positions) = markers.get(key) {
            targets.extend(positions);
        }
    }

    (world, occupant, targets)
}

// ============================================================================
// 合法移動
// ============================================================================

#[test]
fn test_execute_move_success() {
    let test_data = [
        (
            "基本移動-上方",
            r#"
. . T . .
. . P . .
. . . . ."#,
            vec![(2, 1), (2, 0)],
            BASIC_MOVEMENT_COST * 1,
        ),
        (
            "基本移動-下方",
            r#"
. . . . .
. . P . .
. . T . ."#,
            vec![(2, 1), (2, 2)],
            BASIC_MOVEMENT_COST * 1,
        ),
        (
            "基本移動-左方1",
            r#"
. . . . .
. T P . .
. . . . ."#,
            vec![(2, 1), (1, 1)],
            BASIC_MOVEMENT_COST * 1,
        ),
        (
            "基本移動-左方2",
            r#"
. . . . .
T . P . .
. . . . ."#,
            vec![(2, 1), (1, 1), (0, 1)],
            BASIC_MOVEMENT_COST * 2,
        ),
        (
            "基本移動-右方1",
            r#"
. . . . .
. . P T .
. . . . ."#,
            vec![(2, 1), (3, 1)],
            BASIC_MOVEMENT_COST * 1,
        ),
        (
            "基本移動-右方2",
            r#"
. . . . .
. . P . T
. . . . ."#,
            vec![(2, 1), (3, 1), (4, 1)],
            BASIC_MOVEMENT_COST * 2,
        ),
        (
            "穿過友軍-右方",
            r#"
P A T . .
. . . . .
. . . . ."#,
            vec![(0, 0), (1, 0), (2, 0)],
            BASIC_MOVEMENT_COST * 2,
        ),
        (
            "穿過友軍-下方",
            r#"
. P . . .
. A . . .
. T . . ."#,
            vec![(1, 0), (1, 1), (1, 2)],
            BASIC_MOVEMENT_COST * 2,
        ),
        (
            "穿過低移動消耗物件-右方",
            r#"
P p T . .
. . . . .
. . . . ."#,
            vec![(0, 0), (1, 0), (2, 0)],
            BASIC_MOVEMENT_COST * 2 + 10,
        ),
        (
            "穿過低移動消耗物件-下方",
            r#"
. . P . .
. . p . .
. . T . ."#,
            vec![(2, 0), (2, 1), (2, 2)],
            BASIC_MOVEMENT_COST * 2 + 10,
        ),
        (
            "穿過低移動消耗物件-繞路",
            r#"
. p p p .
P p p p T
. . . . ."#,
            vec![(0, 1), (0, 2), (1, 2), (2, 2), (3, 2), (4, 2), (4, 1)],
            BASIC_MOVEMENT_COST * 6,
        ),
    ];

    for (desc, ascii, expected_path, expected_cost) in test_data {
        let (mut world, occupant, targets) = build_world(ascii);
        let target = targets[0];

        start_new_round(&mut world).expect("start_new_round 應成功");

        let result = execute_move(&mut world, target);
        assert!(result.is_ok(), "Case '{}' 應成功：{:?}", desc, result);

        let move_result = result.expect("應成功");
        let expected_path: Vec<Position> = expected_path
            .into_iter()
            .map(|(x, y)| Position { x, y })
            .collect();
        assert_eq!(move_result.path, expected_path, "Case '{}' 路徑不符", desc);
        assert_eq!(move_result.cost, expected_cost, "Case '{}' 消耗不符", desc);

        // 驗證 Position 已更新
        let mut query = world.query::<(&Occupant, &Position)>();
        let (_, new_pos) = query
            .iter(&world)
            .find(|(occ, _)| **occ == occupant)
            .expect(&format!("Case '{}' 應找到移動後的單位", desc));
        assert_eq!(*new_pos, target, "Case '{}' 單位位置應更新為目標位置", desc);
    }
}

/// 移動後再次移動，MovementUsed 應累加
#[test]
fn test_execute_move_accumulates_movement_used() {
    let (mut world, occupant, targets) = build_world("P . T1 . T2 . . . . . . .");
    start_new_round(&mut world).expect("開始新回合應成功");

    for (i, target) in [targets[0], targets[1], targets[0], targets[1], targets[0]]
        .iter()
        .enumerate()
    {
        // 第 1 次移動
        let result = execute_move(&mut world, *target);
        assert!(result.is_ok(), "第 {} 次移動應成功：{:?}", i + 1, result);

        let move_result = result.expect("應成功");
        assert_eq!(move_result.cost, 20, "第 {} 次消耗應為 20", i + 1);

        // 驗證位置已更新
        let mut query = world.query::<(&Occupant, &Position)>();
        let (_, new_pos) = query
            .iter(&world)
            .find(|(occ, _)| **occ == occupant)
            .expect("應找到移動後的單位");
        assert_eq!(
            *new_pos,
            *target,
            "第 {} 次移動後位置應更新為目標位置",
            i + 1
        );
    }

    let result = execute_move(&mut world, targets[1]);
    assert!(result.is_err(), "總共超出 2 倍移動力，移動應失敗");

    end_current_turn(&mut world).expect("結束回合應成功");

    let result = execute_move(&mut world, targets[1]);
    assert!(result.is_ok(), "移動力重置後，可以再次移動");
}

// ============================================================================
// 目標不可到達（超出預算、被佔據）
// ============================================================================

#[test]
fn test_execute_move_target_unreachable() {
    let test_data = [
        (
            "超出預算",
            // warrior MOV=50, budget = 2 * 50 = 100
            // 嘗試移動 11 格（消耗 110 > 100），應失敗
            "P . . . . . . . . . . T",
            "T",
        ),
        (
            "目標是友軍格子",
            r#"
P A . . .
. . . . .
. . . . ."#,
            "A",
        ),
        (
            "目標是敵軍格子",
            r#"
P E . . .
. . . . .
. . . . ."#,
            "E",
        ),
        ("穿過敵人被擋住", "P E T", "T"),
        ("穿過高移動消耗物件被擋住", "P w T", "T"),
        (
            "穿過敵人跟高移動消耗物件，被擋住",
            r#"
P E . . .
w . . . T
. . . . ."#,
            "T",
        ),
    ];

    for (desc, ascii, target_marker) in test_data {
        let (_, markers) = load_from_ascii(ascii).expect("load_from_ascii 應成功");
        let (mut world, _, _) = build_world(ascii);
        let target = markers[target_marker][0];

        let result = execute_move(&mut world, target);
        assert!(result.is_err(), "Case '{}' 應回傳錯誤", desc);
    }
}

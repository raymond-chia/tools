//! 移動路徑規劃期預覽 preview_move_path 整合測試
//!
//! 預覽只針對「當前行動單位」：給定目標格，回傳這條移動路徑上的兩類警示資訊：
//! - reactions：整條路徑上所有會觸發藉機攻擊的反應者（BG3 的紅箭頭，沿途每處脫離都提示）。
//! - hazard_positions：路徑上經過的危險地面格（BG3 的路徑變色警示）。
//!
//! 預覽為唯讀，不改變 World、不產生 pending 反應。
//!
//! 注意：預覽回傳整條路徑的反應者，與實際移動 advance_move「走到第一個觸發就停」的
//! 邏輯不同。預覽是給玩家看完整風險，實際移動仍逐步停下等待反應處理。
//!
//! TDD 紅燈：preview_move_path 與 MovePathPreview 尚未實作，本檔案僅撰寫測試。

use super::constants::{
    OBJECT_TYPE_FOG, OBJECT_TYPE_SPIKE, OBJECTS_TOML, SKILLS_TOML, UNIT_TYPE_MAGE,
    UNIT_TYPE_WARRIOR, UNITS_TOML,
};
use bevy_ecs::prelude::{Entity, With, World};
use board::domain::constants::PLAYER_FACTION_ID;
use board::ecs_logic::loader::parse_and_insert_game_data;
use board::ecs_logic::movement::preview_move_path;
use board::ecs_logic::spawner::spawn_level;
use board::ecs_logic::turn::start_new_round;
use board::ecs_types::components::{
    Initiative, MaxReactionPoint, Occupant, Position, ReactionPoint, Unit,
};
use board::test_helpers::level_builder::{LevelBuilder, load_from_ascii};
use std::collections::HashMap;

const ENEMY_FACTION_ID: u32 = 2;

/// 移動者（P）的敵人 marker 與對應設定。
///
/// - `unit_type`: 敵人單位類型（warrior 有藉機攻擊技能，mage 沒有）。
/// - `reaction_points`: 敵人的剩餘反應次數。
struct EnemySpec {
    marker: &'static str,
    unit_type: &'static str,
    reaction_points: i32,
}

/// 建立含玩家 warrior（marker `P`）、若干敵人、霧氣（`f`）與危險地面（`p`/`p1`/`p2`）的 World。
fn build_preview_world(
    ascii: &str,
    enemies: &[EnemySpec],
) -> (World, HashMap<String, Vec<Position>>) {
    let (_, markers) = load_from_ascii(ascii).expect("load_from_ascii 應成功");

    let mut builder = LevelBuilder::from_ascii(ascii)
        .unit("P", UNIT_TYPE_WARRIOR, PLAYER_FACTION_ID)
        .object("f", OBJECT_TYPE_FOG)
        .object("p", OBJECT_TYPE_SPIKE)
        .object("p1", OBJECT_TYPE_SPIKE)
        .object("p2", OBJECT_TYPE_SPIKE);
    for enemy in enemies {
        builder = builder.unit(enemy.marker, enemy.unit_type, ENEMY_FACTION_ID);
    }
    let level_toml = builder.to_toml().expect("to_toml 應成功");

    let mut world = World::new();
    parse_and_insert_game_data(&mut world, UNITS_TOML, SKILLS_TOML, OBJECTS_TOML)
        .expect("parse_and_insert_game_data 應成功");
    spawn_level(&mut world, &level_toml, "test-level").expect("spawn_level 應成功");

    // 移動者 P 設為先手
    let player_pos = markers["P"][0];
    let player_entity = {
        let mut query = world.query_filtered::<(Entity, &Position), With<Unit>>();
        query
            .iter(&world)
            .find(|(_, p)| **p == player_pos)
            .map(|(e, _)| e)
            .expect("應找到移動者 P")
    };
    world.entity_mut(player_entity).insert(Initiative(100));

    // 依 EnemySpec 設定各敵人的反應點數
    for enemy in enemies {
        let enemy_pos = markers[enemy.marker][0];
        let enemy_entity = {
            let mut query = world.query_filtered::<(Entity, &Position), With<Unit>>();
            query
                .iter(&world)
                .find(|(_, p)| **p == enemy_pos)
                .map(|(e, _)| e)
                .expect("應找到敵人單位")
        };
        world
            .entity_mut(enemy_entity)
            .insert(MaxReactionPoint(enemy.reaction_points))
            .insert(ReactionPoint(enemy.reaction_points));
    }

    start_new_round(&mut world).expect("start_new_round 應成功");

    (world, markers)
}

/// 取得指定位置單位的 Occupant
fn find_occupant(world: &mut World, pos: Position) -> Occupant {
    let mut query = world.query_filtered::<(&Occupant, &Position), With<Unit>>();
    *query
        .iter(world)
        .find(|(_, p)| **p == pos)
        .map(|(occ, _)| occ)
        .expect("應找到指定位置的單位")
}

/// 一個 warrior 敵人、反應點數 1（正常會藉機攻擊）
fn active_warrior(marker: &'static str) -> EnemySpec {
    EnemySpec {
        marker,
        unit_type: UNIT_TYPE_WARRIOR,
        reaction_points: 1,
    }
}

/// 藉機攻擊預覽：驗證整條移動路徑上所有會觸發藉機攻擊的反應者。
///
/// 與實際移動不同，預覽回傳整條路徑的反應者（不是只到第一個觸發格）。
#[test]
fn preview_move_path_reactions_cases() {
    struct TestCase {
        name: &'static str,
        ascii: &'static str,
        enemies: Vec<EnemySpec>,
        /// 預期出現在預覽中的反應者 marker
        expected_reactor_markers: Vec<&'static str>,
    }

    let test_data = [
        TestCase {
            name: "路徑不經過任何敵人旁邊，無提示",
            ascii: r#"
P . . T
. . . .
E . . ."#,
            enemies: vec![active_warrior("E")],
            expected_reactor_markers: vec![],
        },
        TestCase {
            name: "路徑經過 1 個敵人旁邊，提示藉機攻擊",
            ascii: r#"
P .  . T
. E1 . ."#,
            enemies: vec![active_warrior("E1")],
            expected_reactor_markers: vec!["E1"],
        },
        TestCase {
            name: "路徑先後經過 2 個敵人旁邊（不同段），整條路徑都提示",
            ascii: r#"
P .  . .  T
. E1 . E2 ."#,
            enemies: vec![active_warrior("E1"), active_warrior("E2")],
            expected_reactor_markers: vec!["E1", "E2"],
        },
        TestCase {
            name: "路徑經過 2 個敵人旁邊（同一步同時觸發），提示兩次藉機攻擊",
            ascii: r#"
. E1 .
P .  T
. E2 ."#,
            enemies: vec![active_warrior("E1"), active_warrior("E2")],
            expected_reactor_markers: vec!["E1", "E2"],
        },
        TestCase {
            name: "經過敵人旁邊，但敵人無藉機攻擊技能（mage），無提示",
            ascii: r#"
P . . T
. E . ."#,
            enemies: vec![EnemySpec {
                marker: "E",
                unit_type: UNIT_TYPE_MAGE,
                reaction_points: 1,
            }],
            expected_reactor_markers: vec![],
        },
        TestCase {
            name: "經過敵人旁邊，但敵人反應次數用完，無提示",
            ascii: r#"
P . . T
. E . ."#,
            enemies: vec![EnemySpec {
                marker: "E",
                unit_type: UNIT_TYPE_WARRIOR,
                reaction_points: 0,
            }],
            expected_reactor_markers: vec![],
        },
        TestCase {
            name: "經過敵人旁邊，但路徑被霧氣擋住視線，無提示",
            ascii: r#"
P f . T
. E . ."#,
            enemies: vec![active_warrior("E")],
            expected_reactor_markers: vec![],
        },
        TestCase {
            name: "先後經過 2 個敵人，一個敵人用完反應次數",
            ascii: r#"
P .  . .  T
. E1 . E2 ."#,
            enemies: vec![
                EnemySpec {
                    marker: "E1",
                    unit_type: UNIT_TYPE_WARRIOR,
                    reaction_points: 0,
                },
                active_warrior("E2"),
            ],
            expected_reactor_markers: vec!["E2"],
        },
    ];

    for case in test_data {
        let (mut world, markers) = build_preview_world(case.ascii, &case.enemies);
        let target_pos = markers["T"][0];

        let expected_reactors: Vec<Occupant> = case
            .expected_reactor_markers
            .iter()
            .map(|marker| find_occupant(&mut world, markers[*marker][0]))
            .collect();

        let result = preview_move_path(&mut world, target_pos)
            .expect(&format!("[{}] preview_move_path 應成功", case.name));

        let actual_reactors: Vec<Occupant> = result.reactions.iter().map(|r| r.occupant).collect();

        assert_eq!(
            actual_reactors.len(),
            expected_reactors.len(),
            "[{}] 預覽的反應者數量不符：實際 {:?}，預期 {:?}",
            case.name,
            actual_reactors,
            expected_reactors
        );
        for expected in &expected_reactors {
            assert!(
                actual_reactors.contains(expected),
                "[{}] 預覽缺少預期反應者 {:?}",
                case.name,
                expected
            );
        }
    }
}

/// 危險地面預覽：驗證移動路徑上經過的危險地面格。
#[test]
fn preview_move_path_hazard_cases() {
    struct TestCase {
        name: &'static str,
        ascii: &'static str,
        /// 預期出現在預覽中的危險格 marker
        expected_hazard_markers: Vec<&'static str>,
    }

    let test_data = [
        TestCase {
            name: "路徑不經過危險地面，無危險格",
            ascii: r#"
P . . T
. . . ."#,
            expected_hazard_markers: vec![],
        },
        TestCase {
            name: "路徑經過 1 格危險地面",
            ascii: r#"
P p . T"#,
            expected_hazard_markers: vec!["p"],
        },
        TestCase {
            name: "路徑經過 2 格危險地面",
            ascii: r#"
P p1 p2 T"#,
            expected_hazard_markers: vec!["p1", "p2"],
        },
        TestCase {
            name: "路徑經過 1 格危險地面，1 格中立地面",
            ascii: r#"
P p f T"#,
            expected_hazard_markers: vec!["p"],
        },
        TestCase {
            name: "危險地面在可達範圍內但不在選定路徑上，無危險格",
            ascii: r#"
P . . T
p . . ."#,
            expected_hazard_markers: vec![],
        },
    ];

    for case in test_data {
        let (mut world, markers) = build_preview_world(case.ascii, &[]);
        let target_pos = markers["T"][0];

        let expected_hazards: Vec<Position> = case
            .expected_hazard_markers
            .iter()
            .map(|marker| markers[*marker][0])
            .collect();

        let result = preview_move_path(&mut world, target_pos)
            .expect(&format!("[{}] preview_move_path 應成功", case.name));

        assert_eq!(
            result.hazard_positions.len(),
            expected_hazards.len(),
            "[{}] 預覽的危險格數量不符：實際 {:?}，預期 {:?}",
            case.name,
            result.hazard_positions,
            expected_hazards
        );
        for expected in &expected_hazards {
            assert!(
                result.hazard_positions.contains(expected),
                "[{}] 預覽缺少預期危險格 {:?}",
                case.name,
                expected
            );
        }
    }
}

/// 同一路徑同時有危險地面與藉機攻擊：兩類警示都應回傳（案例 E）。
#[test]
fn preview_move_path_hazard_and_reaction_together() {
    let ascii = r#"
P .  p T
. E .  ."#;
    let (mut world, markers) = build_preview_world(ascii, &[active_warrior("E")]);
    let target_pos = markers["T"][0];

    let expected_reactor = find_occupant(&mut world, markers["E"][0]);
    let expected_hazard = markers["p"][0];

    let result = preview_move_path(&mut world, target_pos).expect("preview_move_path 應成功");

    let actual_reactors: Vec<Occupant> = result.reactions.iter().map(|r| r.occupant).collect();
    assert!(
        actual_reactors.contains(&expected_reactor),
        "應回傳藉機攻擊反應者 {:?}，實際 {:?}",
        expected_reactor,
        actual_reactors
    );
    assert!(
        result.hazard_positions.contains(&expected_hazard),
        "應回傳危險格 {:?}，實際 {:?}",
        expected_hazard,
        result.hazard_positions
    );
}

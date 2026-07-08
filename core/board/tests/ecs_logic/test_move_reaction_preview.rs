//! 藉機攻擊規劃期預覽 preview_move_reactions 整合測試
//!
//! 預覽只針對「當前行動單位」：給定目標格，回傳這條移動路徑上會觸發藉機攻擊的
//! 反應者，供前端在玩家尚未確認移動前顯示警示（BG3 的紅箭頭）。
//!
//! 預覽為唯讀，不改變 World、不產生 pending 反應。
//!
//! TDD 紅燈：preview_move_reactions 尚未實作，本檔案僅撰寫測試。

use super::constants::{
    OBJECT_TYPE_FOG, OBJECTS_TOML, SKILLS_TOML, UNIT_TYPE_MAGE, UNIT_TYPE_WARRIOR, UNITS_TOML,
};
use bevy_ecs::prelude::{Entity, With, World};
use board::domain::constants::PLAYER_FACTION_ID;
use board::ecs_logic::loader::parse_and_insert_game_data;
use board::ecs_logic::movement::preview_move_reactions;
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

/// 建立含玩家 warrior（marker `P`）、若干敵人與霧氣（marker `F`）的 World。
///
/// - 移動者 P 為 warrior，Initiative 設為 100 確保先手（成為當前行動單位）。
/// - 每個敵人依 `EnemySpec` 設定單位類型與反應點數。
/// - `F` 為霧氣 object（blocks_sight，可通行）。
fn build_preview_world(
    ascii: &str,
    enemies: &[EnemySpec],
) -> (World, HashMap<String, Vec<Position>>) {
    let (_, markers) = load_from_ascii(ascii).expect("load_from_ascii 應成功");

    let mut builder = LevelBuilder::from_ascii(ascii)
        .unit("P", UNIT_TYPE_WARRIOR, PLAYER_FACTION_ID)
        .object("F", OBJECT_TYPE_FOG);
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

/// 藉機攻擊預覽：驗證移動路徑上會觸發藉機攻擊的反應者。
///
/// | # | 情境                                   | 預期反應者 |
/// |---|----------------------------------------|-----------|
/// | 1 | 路徑不經過任何敵人旁邊                 | 無         |
/// | 2 | 路徑經過 1 個敵人旁邊                   | E1         |
/// | 3 | 路徑經過 2 個敵人旁邊                   | E1、E2     |
/// | 4 | 經過敵人旁邊，但敵人（mage）無藉機攻擊 | 無         |
/// | 5 | 經過敵人旁邊，但敵人反應次數用完（0）  | 無         |
/// | 6 | 經過敵人旁邊，但路徑被霧氣擋住視線     | 無         |
///
/// 佈局慣例：P 為移動者、T 為目標格、E/E1/E2 為敵人、M 為 mage 敵人、F 為霧氣。
#[test]
fn preview_move_reactions_cases() {
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
            name: "路徑經過 2 個敵人旁邊，提示兩次藉機攻擊",
            ascii: r#"
P .  . .  T
. E1 . E2 ."#,
            enemies: vec![active_warrior("E1"), active_warrior("E2")],
            expected_reactor_markers: vec!["E1"],
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
P F . T
. E . ."#,
            enemies: vec![active_warrior("E")],
            expected_reactor_markers: vec![],
        },
        TestCase {
            name: "路徑經過 2 個敵人旁邊，一個敵人用完反應次數",
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

        let result = preview_move_reactions(&mut world, target_pos)
            .expect(&format!("[{}] preview_move_reactions 應成功", case.name));

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

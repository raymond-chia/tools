//! 藉機攻擊受視線（霧氣 object）影響的整合測試
//!
//! 霧氣（fog）是 blocks_sight = true 且可通行（movement_cost = 0）的 object。
//! 反應者對移動者的藉機攻擊，應在反應者與觸發格之間被霧氣阻擋時無法觸發。
//!
//! 視線基準：反應者 ↔ 觸發格（移動者此刻所在的那一格），比照 execute_skill 的
//! caster ↔ target 判定。
//!
//! TDD 紅燈：現行 advance_move 呼叫 collect_move_reactions 時不傳任何視線資訊，
//! 因此霧氣不影響藉機攻擊；下列「霧氣阻擋應無反應」的案例會失敗，待實作補上視線判定後轉綠。

use super::constants::{OBJECT_TYPE_FOG, OBJECTS_TOML, SKILLS_TOML, UNIT_TYPE_WARRIOR, UNITS_TOML};
use bevy_ecs::prelude::{Entity, With, World};
use board::domain::constants::PLAYER_FACTION_ID;
use board::ecs_logic::loader::parse_and_insert_game_data;
use board::ecs_logic::movement::{advance_move, plan_move};
use board::ecs_logic::reaction::get_pending_reactions;
use board::ecs_logic::spawner::spawn_level;
use board::ecs_logic::turn::start_new_round;
use board::ecs_types::components::{
    Initiative, MaxReactionPoint, Occupant, Position, ReactionPoint, Unit,
};
use board::test_helpers::level_builder::{LevelBuilder, load_from_ascii};
use std::collections::HashMap;

const ENEMY_FACTION_ID: u32 = 2;

/// 建立含 P（玩家 warrior）、E（敵人 warrior）與霧氣的 World。
///
/// marker 慣例：
/// - `P` 玩家 warrior、`E` 敵人 warrior。
/// - `F` 霧氣 object（單獨一格霧氣）。
/// - `EF` 敵人 warrior **與** 霧氣同格（攻擊者被霧氣覆蓋）：由 unit 與 object
///   共用同一 marker，`to_toml` 會為該格同時產生 unit 與 object placement。
/// - `PF` 玩家 warrior **與** 霧氣同格（移動起點被霧氣覆蓋，用於全覆蓋案例）。
///
/// - 移動者（P 或 PF）的 Initiative 設為 100，確保先手。
/// - 所有單位 ReactionPoint 設為 1。
fn build_fog_world(ascii: &str) -> (World, HashMap<String, Vec<Position>>) {
    let (_, markers) = load_from_ascii(ascii).expect("load_from_ascii 應成功");

    let level_toml = LevelBuilder::from_ascii(ascii)
        .unit("P", UNIT_TYPE_WARRIOR, PLAYER_FACTION_ID)
        .unit("PF", UNIT_TYPE_WARRIOR, PLAYER_FACTION_ID)
        .unit("E", UNIT_TYPE_WARRIOR, ENEMY_FACTION_ID)
        .unit("EF", UNIT_TYPE_WARRIOR, ENEMY_FACTION_ID)
        .object("F", OBJECT_TYPE_FOG)
        .object("PF", OBJECT_TYPE_FOG)
        .object("EF", OBJECT_TYPE_FOG)
        .to_toml()
        .expect("to_toml 應成功");

    let mut world = World::new();
    parse_and_insert_game_data(&mut world, UNITS_TOML, SKILLS_TOML, OBJECTS_TOML)
        .expect("parse_and_insert_game_data 應成功");
    spawn_level(&mut world, &level_toml, "test-level").expect("spawn_level 應成功");

    // 移動者可能標為 P 或 PF（PF = 移動者與霧氣同格），取存在的那個作為先手單位
    let mover_pos = markers
        .get("P")
        .or_else(|| markers.get("PF"))
        .map(|v| v[0])
        .expect("應有移動者 marker（P 或 PF）");
    let mover_entity = {
        let mut query = world.query_filtered::<(Entity, &Position), With<Unit>>();
        query
            .iter(&world)
            .find(|(_, p)| **p == mover_pos)
            .map(|(e, _)| e)
            .expect("應找到移動者")
    };
    world.entity_mut(mover_entity).insert(Initiative(100));

    let all_unit_entities: Vec<Entity> = {
        let mut query = world.query_filtered::<Entity, With<Unit>>();
        query.iter(&world).collect()
    };
    for entity in all_unit_entities {
        world
            .entity_mut(entity)
            .insert(MaxReactionPoint(1))
            .insert(ReactionPoint(1));
    }

    start_new_round(&mut world).expect("start_new_round 應成功");

    (world, markers)
}

fn find_occupant(world: &mut World, pos: Position) -> Occupant {
    let mut query = world.query_filtered::<(&Occupant, &Position), With<Unit>>();
    *query
        .iter(world)
        .find(|(_, p)| **p == pos)
        .map(|(occ, _)| occ)
        .expect("應找到指定位置的單位")
}

/// 藉機攻擊受霧氣（視線）影響。
///
/// 五個案例皆為「敵人藉機攻擊移動者」，差別在霧氣位置：
///
/// | # | 情境                 | 攻擊者格 / 移動者格   | 預期待反應數 |
/// |---|----------------------|----------------------|-------------|
/// | 1 | 沒有霧氣             | E / P                | 1（成功攻擊）|
/// | 2 | 全戰場覆蓋霧氣       | EF(=E+霧) / PF(=P+霧) | 0（無法攻擊）|
/// | 3 | 整條移動路徑覆蓋霧氣 | E / P                | 0（無法攻擊）|
/// | 4 | 攻擊者被霧氣覆蓋     | EF(=E+霧) / P        | 0（無法攻擊）|
/// | 5 | 霧氣在其他地方       | E / P                | 1（可以攻擊）|
///
/// 地圖佈局：移動者在攻擊者旁（range 1），往 T 移動離開藉機攻擊範圍。
/// 觸發格為移動者起點（離開該格觸發），攻擊者與該格相鄰。
#[test]
fn attack_of_opportunity_blocked_by_fog_cases() {
    struct TestCase {
        name: &'static str,
        ascii: &'static str,
        /// 移動者所在 marker（P 或 PF）
        mover_marker: &'static str,
        /// 攻擊者所在 marker（E 或 EF）
        attacker_marker: &'static str,
        expected_pending: usize,
    }

    let test_data = [
        TestCase {
            name: "沒有霧氣，成功藉機攻擊",
            ascii: r#"
E P . T
. . . ."#,
            mover_marker: "P",
            attacker_marker: "E",
            expected_pending: 1,
        },
        TestCase {
            name: "全戰場覆蓋霧氣，無法攻擊",
            ascii: r#"
EF PF F T
F  F  F F"#,
            mover_marker: "PF",
            attacker_marker: "EF",
            expected_pending: 0,
        },
        TestCase {
            name: "整條移動路徑覆蓋霧氣，無法攻擊",
            ascii: r#"
E PF F T
. .  . ."#,
            mover_marker: "P",
            attacker_marker: "E",
            expected_pending: 0,
        },
        TestCase {
            name: "攻擊者被霧氣覆蓋，無法攻擊",
            ascii: r#"
EF P . T
.  . . ."#,
            mover_marker: "P",
            attacker_marker: "EF",
            expected_pending: 0,
        },
        TestCase {
            name: "霧氣在其他地方，可以攻擊",
            ascii: r#"
E P . T
. . . F"#,
            mover_marker: "P",
            attacker_marker: "E",
            expected_pending: 1,
        },
    ];

    for case in test_data {
        let (mut world, markers) = build_fog_world(case.ascii);

        let attacker_occupant = find_occupant(&mut world, markers[case.attacker_marker][0]);
        let target_pos = markers["T"][0];
        let _ = case.mover_marker;

        plan_move(&mut world, target_pos).expect(&format!("[{}] plan_move 應成功", case.name));
        advance_move(&mut world).expect(&format!("[{}] advance_move 應成功", case.name));

        let pending = get_pending_reactions(&world);
        assert_eq!(
            pending.len(),
            case.expected_pending,
            "[{}] 待反應者數量應為 {}",
            case.name,
            case.expected_pending
        );

        if case.expected_pending == 1 {
            assert_eq!(
                pending[0].reactor, attacker_occupant,
                "[{}] 反應者應為攻擊者",
                case.name
            );
        }
    }
}

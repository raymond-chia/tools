//! SkillTargeting 測試：start / add / get / cancel

use super::constants::{
    SKILL_DIAMOND_AOE, SKILL_WARRIOR, SKILL_WARRIOR_ACTIVE_2, SKILL_WARRIOR_ACTIVE_4,
};
use bevy_ecs::prelude::{Entity, World};
use board::ecs_logic::query::get_skill_targeting;
use board::ecs_logic::skill::{add_skill_target, cancel_skill_targeting, start_skill_targeting};
use board::ecs_logic::turn::start_new_round;
use board::ecs_types::components::{CurrentMp, Occupant, Position};
use board::error::{BoardError, ErrorKind, UnitError};
use std::collections::HashMap;

fn build_warrior_world(ascii: &str, mp: i32) -> (World, Occupant, HashMap<String, Vec<Position>>) {
    let (mut world, occupant, markers) = super::build_warrior_world(ascii);
    prepare_caster(&mut world, occupant, mp);
    (world, occupant, markers)
}

fn build_mage_world(ascii: &str, mp: i32) -> (World, Occupant, HashMap<String, Vec<Position>>) {
    let (mut world, occupant, markers) = super::build_mage_world(ascii);
    prepare_caster(&mut world, occupant, mp);
    (world, occupant, markers)
}

fn prepare_caster(world: &mut World, occupant: Occupant, mp: i32) {
    start_new_round(world).expect("start_new_round 應成功");
    let entity = get_entity_by_occupant(world, occupant);
    world.entity_mut(entity).insert(CurrentMp(mp));
}

fn get_entity_by_occupant(world: &mut World, occupant: Occupant) -> Entity {
    let mut query = world.query::<(Entity, &Occupant)>();
    query
        .iter(world)
        .find(|(_, occ)| **occ == occupant)
        .map(|(e, _)| e)
        .expect("應找到指定單位的 Entity")
}

// ============================================================================

const ASCII_LAYOUT: &str = "
. . . . .
. P E E .
. . E . .
. . . . .
";

#[test]
fn test_start_skill_targeting_unavailable() {
    enum ExpectedError {
        SkillNotFound,
        InsufficientMp,
    }

    let test_data = [
        ("nonexistent", 10, ExpectedError::SkillNotFound, "未知技能"),
        (
            SKILL_WARRIOR,
            10,
            ExpectedError::SkillNotFound,
            "被動技能不可選",
        ),
        (
            SKILL_WARRIOR_ACTIVE_2,
            0,
            ExpectedError::InsufficientMp,
            "MP 不足",
        ),
    ];

    for (skill_name, mp, expected, note) in test_data {
        let (mut world, _, _) = build_warrior_world(ASCII_LAYOUT, mp);
        let err = start_skill_targeting(&mut world, &skill_name.to_string())
            .expect_err(&format!("{note} 應失敗"));
        match expected {
            ExpectedError::SkillNotFound => assert!(
                matches!(err.kind(), ErrorKind::Unit(UnitError::SkillNotFound { .. })),
                "{note}：錯誤應為 SkillNotFound，實際: {:?}",
                err.kind()
            ),
            ExpectedError::InsufficientMp => assert!(
                matches!(
                    err.kind(),
                    ErrorKind::Unit(UnitError::InsufficientMp { .. })
                ),
                "{note}：錯誤應為 InsufficientMp，實際: {:?}",
                err.kind()
            ),
        }
    }
}

#[derive(Debug)]
enum AddExpect {
    Ok,
    OutOfRange,
    CountFull,
    FilterMismatch,
}

#[test]
fn test_add_skill_target_unit() {
    // warrior-active-4: range=[1,1], count=2, allow_same_target=false, filter=Enemy
    let ascii = "
    . . A . .
    . E P E .
    . . E . .
    . . . . .
    ";
    let (mut world, _, markers) = build_warrior_world(ascii, 10);
    let player_pos = markers["P"][0];
    let enemies = markers["E"].clone();
    let ally_pos = markers["A"][0];

    start_skill_targeting(&mut world, &SKILL_WARRIOR_ACTIVE_4.to_string())
        .expect("start_skill_targeting 應成功");

    // ! 每一輪會影響後續測試
    // (目標位置, 預期結果, 該步驟後 picked 內容, 說明)
    let test_data: [(Position, AddExpect, Vec<Position>, &str); 6] = [
        (enemies[0], AddExpect::Ok, vec![enemies[0]], "首次加入 E0"),
        (
            enemies[0],
            AddExpect::Ok,
            vec![enemies[0]],
            "重複位置應被忽略",
        ),
        (
            player_pos,
            AddExpect::OutOfRange,
            vec![enemies[0]],
            "距離 0 超出 [1,1]",
        ),
        (
            ally_pos,
            AddExpect::FilterMismatch,
            vec![enemies[0]],
            "盟友不符合 Enemy filter",
        ),
        (
            enemies[1],
            AddExpect::Ok,
            vec![enemies[0], enemies[1]],
            "加入第二個目標 E1",
        ),
        (
            enemies[2],
            AddExpect::CountFull,
            vec![enemies[0], enemies[1]],
            "已達 count=2 上限",
        ),
    ];

    for (pos, expected, picked_after, note) in test_data {
        let result = add_skill_target(&mut world, pos);
        match expected {
            AddExpect::Ok => {
                result.unwrap_or_else(|e| panic!("{note} 應成功，實際: {:?}", e.kind()));
            }
            AddExpect::OutOfRange => {
                let err = result.expect_err(&format!("{note} 應失敗"));
                assert!(
                    matches!(err.kind(), ErrorKind::Board(BoardError::OutOfRange { .. })),
                    "{note}：錯誤應為 OutOfRange，實際: {:?}",
                    err.kind()
                );
            }
            AddExpect::CountFull => {
                let err = result.expect_err(&format!("{note} 應失敗"));
                assert!(
                    matches!(
                        err.kind(),
                        ErrorKind::Board(BoardError::TargetCountFull { .. })
                    ),
                    "{note}：錯誤應為 TargetCountFull，實際: {:?}",
                    err.kind()
                );
            }
            AddExpect::FilterMismatch => {
                let err = result.expect_err(&format!("{note} 應失敗"));
                assert!(
                    matches!(
                        err.kind(),
                        ErrorKind::Board(BoardError::TargetFilterMismatch { .. })
                    ),
                    "{note}：錯誤應為 TargetFilterMismatch，實際: {:?}",
                    err.kind()
                );
            }
        }
        let targeting = get_skill_targeting(&world).expect("應有 SkillTargeting resource");
        assert_eq!(targeting.picked, picked_after, "{note}：picked 不符");
    }
}

#[test]
fn test_add_skill_target_ground() {
    // diamond-aoe-1: range=[1,2], selection=Ground, filter=AllyExceptCaster, count=1
    // Ground 下應略過 filter，敵/友/空格皆可；射程外與 count 滿仍應報錯
    let ascii = "
    f . . . .
    . g P E .
    . . A . .
    . . . . .
    ";
    let (mut world, _, markers) = build_mage_world(ascii, 10);
    let player_pos = markers["P"][0];
    let enemy_pos = markers["E"][0];
    let ally_pos = markers["A"][0];
    let empty_in_range = markers["g"][0]; // 射程內的空格
    let far_pos = markers["f"][0]; // 超出射程的空格

    #[derive(Debug)]
    enum Expect {
        Ok,
        OutOfRange,
        CountFull,
    }

    // 每筆皆獨立啟動，避免 count 互相干擾
    let cases: [(Position, Expect, &str); 6] = [
        (
            enemy_pos,
            Expect::Ok,
            "Ground+AllyExceptCaster：敵人位置應可選",
        ),
        (
            ally_pos,
            Expect::Ok,
            "Ground+AllyExceptCaster：盟友位置應可選",
        ),
        (empty_in_range, Expect::Ok, "Ground：空格應可選"),
        (player_pos, Expect::OutOfRange, "自身位置距離 0 超出 [1,2]"),
        (far_pos, Expect::OutOfRange, "遠距離超出 [1,2]"),
        (ally_pos, Expect::CountFull, "已達 count=1 上限"),
    ];

    for (pos, expected, note) in cases {
        cancel_skill_targeting(&mut world);
        start_skill_targeting(&mut world, &SKILL_DIAMOND_AOE.to_string())
            .expect("start_skill_targeting 應成功");
        if matches!(expected, Expect::CountFull) {
            add_skill_target(&mut world, enemy_pos).expect("預填第一個目標應成功");
        }
        let result = add_skill_target(&mut world, pos);
        match expected {
            Expect::Ok => {
                result.unwrap_or_else(|e| panic!("{note} 應成功，實際: {:?}", e.kind()));
            }
            Expect::OutOfRange => {
                let err = result.expect_err(&format!("{note} 應失敗"));
                assert!(
                    matches!(err.kind(), ErrorKind::Board(BoardError::OutOfRange { .. })),
                    "{note}：錯誤應為 OutOfRange，實際: {:?}",
                    err.kind()
                );
            }
            Expect::CountFull => {
                let err = result.expect_err(&format!("{note} 應失敗"));
                assert!(
                    matches!(
                        err.kind(),
                        ErrorKind::Board(BoardError::TargetCountFull { .. })
                    ),
                    "{note}：錯誤應為 TargetCountFull，實際: {:?}",
                    err.kind()
                );
            }
        }
    }
}

#[test]
fn test_add_skill_target_without_start() {
    let (mut world, _, markers) = build_warrior_world(ASCII_LAYOUT, 10);
    let enemy_pos = markers["E"][0];

    let err =
        add_skill_target(&mut world, enemy_pos).expect_err("未呼叫 start_skill_targeting 應失敗");
    // 未初始化 resource → MissingResource (DataError)
    assert!(
        matches!(err.kind(), ErrorKind::Data(_)),
        "錯誤應為 Data MissingResource，實際: {:?}",
        err.kind()
    );
}

#[test]
fn test_cancel_skill_targeting() {
    let (mut world, _, markers) = build_warrior_world(ASCII_LAYOUT, 10);
    let enemy_pos = markers["E"][0];

    // 啟動 → 取消 → 再次取消（冪等）
    start_skill_targeting(&mut world, &SKILL_WARRIOR_ACTIVE_2.to_string())
        .expect("start_skill_targeting 應成功");
    add_skill_target(&mut world, enemy_pos).expect("新增目標應成功");
    assert_eq!(
        get_skill_targeting(&world).expect("cancel 前應存在").picked,
        vec![enemy_pos],
        "cancel 前應存在"
    );

    cancel_skill_targeting(&mut world);
    assert!(get_skill_targeting(&world).is_err(), "cancel 後應移除");

    cancel_skill_targeting(&mut world);
    assert!(
        get_skill_targeting(&world).is_err(),
        "再次 cancel 應冪等，不 panic"
    );
}

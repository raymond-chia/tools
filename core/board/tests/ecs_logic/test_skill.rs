//! 技能執行測試

use super::constants::{SKILL_SUMMON_WALL_AOE, SKILL_WARRIOR_ACTIVE_2, UNIT_TYPE_WARRIOR};
use bevy_ecs::prelude::{Entity, World};
use board::domain::alias::MovementCost;
use board::domain::battle_log::{LogCheck, LogEffect, LogEvent, LogTarget};
use board::ecs_logic::battle_log::append_skill_log;
use board::ecs_logic::query::get_battle_log;
use board::ecs_logic::skill::execute_skill;
use board::ecs_logic::turn::start_new_round;
use board::ecs_types::components::{
    ActionState, CurrentMp, MovementPoint, Object, Occupant, OccupantTypeName, Position,
};
use board::error::{BoardError, ErrorKind, UnitError};
use board::logic::skill::skill_execution::{CheckTarget, EffectEntry, ResolvedEffect};
use std::collections::{HashMap, HashSet};

fn build_warrior_world(ascii: &str, mp: i32) -> (World, Occupant, HashMap<String, Vec<Position>>) {
    let (mut world, occupant, markers) = super::build_warrior_world(ascii);

    start_new_round(&mut world).expect("start_new_round 應成功");

    let entity = {
        let mut query = world.query::<(Entity, &Occupant)>();
        query
            .iter(&world)
            .find(|(_, occ)| **occ == occupant)
            .map(|(e, _)| e)
            .expect("應找到指定單位的 Entity")
    };
    world.entity_mut(entity).insert(CurrentMp(mp));

    (world, occupant, markers)
}

fn build_mage_world(ascii: &str) -> (World, HashMap<String, Vec<Position>>) {
    let (mut world, _, markers) = super::build_mage_world(ascii);

    start_new_round(&mut world).expect("start_new_round 應成功");

    (world, markers)
}

/// 設定指定單位的 ActionState
fn set_active_action_state(world: &mut World, occupant: Occupant, state: ActionState) {
    let entity = get_entity_by_occupant(world, occupant);
    world.entity_mut(entity).insert(state);
}

/// 取得當前 active 單位的 movement_point
fn read_movement_point(world: &mut World, occupant: Occupant) -> i32 {
    let entity = get_entity_by_occupant(world, occupant);
    world
        .entity(entity)
        .get::<MovementPoint>()
        .expect("應有 MovementPoint")
        .0
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
// execute_skill 測試
// ============================================================================

/// warrior-active-2（cost=2）在不同 ActionState / MP 組合下的行為
///
/// - ExactlyFullMovement：`Moved { cost == movement_point }` → 成功，扣 MP，進 Done
/// - OverFullMovement：`Moved { cost > movement_point }` → InsufficientActionPoint
/// - AlreadyDone：`Done` 且 MP 足夠 → InsufficientActionPoint
/// - InsufficientMp：可行動但 MP 不足 → InsufficientMp
#[test]
fn test_execute_skill_action_point_and_mp() {
    const SKILL_COST: i32 = 2;

    enum ActionStateSpec {
        MovedMatchesMovementPoint,
        MovedOverMovementPoint,
        Done,
        MovedZero,
    }

    enum Expected {
        Success,
        ActionPointError,
        MpError,
    }

    struct TestCase {
        name: &'static str,
        initial_mp: i32,
        action_state: ActionStateSpec,
        expected: Expected,
    }

    let test_data = [
        TestCase {
            name: "zero movement",
            initial_mp: SKILL_COST,
            action_state: ActionStateSpec::MovedZero,
            expected: Expected::Success,
        },
        TestCase {
            name: "exactly full movement",
            initial_mp: SKILL_COST,
            action_state: ActionStateSpec::MovedMatchesMovementPoint,
            expected: Expected::Success,
        },
        TestCase {
            name: "over full movement",
            initial_mp: SKILL_COST,
            action_state: ActionStateSpec::MovedOverMovementPoint,
            expected: Expected::ActionPointError,
        },
        TestCase {
            name: "already done",
            initial_mp: SKILL_COST,
            action_state: ActionStateSpec::Done,
            expected: Expected::ActionPointError,
        },
        TestCase {
            name: "insufficient mp",
            initial_mp: SKILL_COST - 1,
            action_state: ActionStateSpec::MovedZero,
            expected: Expected::MpError,
        },
    ];

    for case in test_data {
        let (mut world, player_occupant, markers) = build_warrior_world(
            "
            . . . .
            . P E .
            . . . .
            ",
            case.initial_mp,
        );
        let enemy_pos = markers["E"][0];

        let movement_point = read_movement_point(&mut world, player_occupant);
        let state = match case.action_state {
            ActionStateSpec::MovedMatchesMovementPoint => ActionState::Moved {
                cost: movement_point as MovementCost,
            },
            ActionStateSpec::MovedOverMovementPoint => ActionState::Moved {
                cost: movement_point as MovementCost + 1,
            },
            ActionStateSpec::Done => ActionState::Done,
            ActionStateSpec::MovedZero => ActionState::Moved { cost: 0 },
        };
        set_active_action_state(&mut world, player_occupant, state);

        let result = execute_skill(
            &mut world,
            &SKILL_WARRIOR_ACTIVE_2.to_string(),
            &[enemy_pos],
        );

        match (case.expected, result) {
            (Expected::Success, Ok(entries)) => {
                assert!(
                    !entries.is_empty(),
                    "[{}] 應產生至少 1 個效果條目",
                    case.name
                );

                let entity = get_entity_by_occupant(&mut world, player_occupant);
                let entity_ref = world.entity(entity);
                assert!(
                    matches!(entity_ref.get::<ActionState>(), Some(ActionState::Done)),
                    "[{}] 施放後 ActionState 應為 Done",
                    case.name
                );
                assert_eq!(
                    entity_ref.get::<CurrentMp>().expect("應有 CurrentMp").0,
                    case.initial_mp - SKILL_COST,
                    "[{}] CurrentMp 應扣除技能 cost",
                    case.name
                );
            }
            (Expected::ActionPointError, Err(err)) => {
                assert!(
                    matches!(
                        err.kind(),
                        ErrorKind::Unit(UnitError::InsufficientActionPoint { .. })
                    ),
                    "[{}] 錯誤類型應為 InsufficientActionPoint，實際: {:?}",
                    case.name,
                    err.kind()
                );
            }
            (Expected::MpError, Err(err)) => {
                assert!(
                    matches!(
                        err.kind(),
                        ErrorKind::Unit(UnitError::InsufficientMp { .. })
                    ),
                    "[{}] 錯誤類型應為 InsufficientMp，實際: {:?}",
                    case.name,
                    err.kind()
                );
            }
            (Expected::Success, Err(err)) => {
                panic!("[{}] 應成功但得到錯誤: {:?}", case.name, err.kind());
            }
            (Expected::ActionPointError | Expected::MpError, Ok(_)) => {
                panic!("[{}] 應失敗但卻成功", case.name);
            }
        }
    }
}

/// execute_skill 對單一敵人施放後，BattleLog 應 append 一筆技能 log 事件
///
/// warrior-active-2（直接 Leaf、無 Branch）對 enemy(warrior) 攻擊：
/// - caster 名稱快照 = warrior
/// - target 為單位（warrior），check = Auto，effect = HpChange（最終傷害值）
#[test]
fn test_execute_skill_appends_skill_log() {
    let (mut world, player_occupant, markers) = build_warrior_world(
        "
        . . . .
        . P E .
        . . . .
        ",
        10,
    );
    let enemy_pos = markers["E"][0];
    set_active_action_state(&mut world, player_occupant, ActionState::Moved { cost: 0 });

    let entries = execute_skill(
        &mut world,
        &SKILL_WARRIOR_ACTIVE_2.to_string(),
        &[enemy_pos],
    )
    .expect("施放應成功");

    // log 由呼叫端在施放後明確呼叫 append_skill_log 產生（core 不自動 append）
    append_skill_log(&mut world, &entries).expect("append_skill_log 應成功");

    // 預期：每筆 EffectEntry 對應一筆 LogEvent
    let log = get_battle_log(&world).expect("spawn_level 後應可取得 BattleLog");
    assert_eq!(
        log.len(),
        entries.len(),
        "log 筆數應與 EffectEntry 筆數一致（一筆 EffectEntry = 一筆 LogEvent）"
    );

    // warrior-active-2 對單一目標只產生一筆 HpChange entry
    assert_eq!(log.len(), 1, "warrior-active-2 對單目標應產生一筆 log");
    match &log[0] {
        LogEvent::Skill {
            caster,
            skill_name,
            target,
            check,
            effect,
            ..
        } => {
            assert_eq!(caster, UNIT_TYPE_WARRIOR, "caster 名稱快照應為 warrior");
            assert_eq!(
                skill_name, SKILL_WARRIOR_ACTIVE_2,
                "技能名應為 warrior-active-2"
            );
            assert_eq!(
                *target,
                LogTarget::Unit {
                    name: UNIT_TYPE_WARRIOR.to_string()
                },
                "目標名稱快照應為 warrior"
            );
            assert_eq!(*check, LogCheck::Auto, "warrior-active-2 無判定，應為 Auto");
            match effect {
                LogEffect::HpChange { amount } => {
                    assert!(*amount < 0, "傷害應為負值，實際 {}", amount);
                }
                other => panic!("應為 HpChange，實際 {:?}", other),
            }
        }
        other => panic!("應為 Skill 事件，實際 {:?}", other),
    }
}

#[test]
fn test_execute_skill_skill_not_found() {
    let (mut world, player_occupant, markers) = build_warrior_world(
        "
        . . . .
        . P E .
        . . . .
        ",
        10,
    );
    let enemy_pos = markers["E"][0];
    set_active_action_state(&mut world, player_occupant, ActionState::Moved { cost: 0 });

    let err = execute_skill(&mut world, &"not-existing-skill".to_string(), &[enemy_pos])
        .expect_err("不存在的技能應失敗");

    assert!(
        matches!(err.kind(), ErrorKind::Unit(UnitError::SkillNotFound { .. })),
        "錯誤應為 SkillNotFound，實際: {:?}",
        err.kind()
    );
}

/// 範圍召喚牆壁 — AOE 中的牆壁/單位格不召喚，沼澤/空格成功召喚
///
/// 佈局（P=mage，E=敵人，w=wall 不可通過，p=swamp 可通過高成本）：
/// ```text
/// .  .  .  .  .
/// .  w  .  p  .
/// .  E  P  .  .
/// .  .  .  .  .
/// ```
/// 目標位置 = P 自己。Diamond radius=1 覆蓋：上下左右 + 中心
/// - 中心 (P 所在)：有單位，不召喚
/// - 左 (E)：有單位，不召喚
/// - 上 (w)：不可通過物件，不召喚
/// - 右：空格，召喚
/// - 下：空格，召喚
#[test]
fn test_execute_skill_summon_wall_aoe() {
    let (mut world, markers) = build_mage_world(
        "
        .  .  .  .  .
        .  .  w  .  .
        .  E  P  p  .
        .  .  .  .  .
        ",
    );
    let caster_pos = markers["P"][0];
    let enemy_pos = markers["E"][0];
    let wall_pos = markers["w"][0];
    let _swamp_pos = markers["p"][0];

    // 中心瞄準自己所在格，Diamond radius=1 涵蓋的 5 格
    let affected = [
        Position {
            x: caster_pos.x,
            y: caster_pos.y - 1,
        },
        Position {
            x: caster_pos.x,
            y: caster_pos.y + 1,
        },
        Position {
            x: caster_pos.x - 1,
            y: caster_pos.y,
        },
        Position {
            x: caster_pos.x + 1,
            y: caster_pos.y,
        },
        caster_pos,
    ];

    // 預期：只有空格會召喚
    let occupied: HashSet<Position> = [caster_pos, enemy_pos, wall_pos].into_iter().collect();
    let expected_spawn_positions: HashSet<Position> = affected
        .iter()
        .copied()
        .filter(|p| !occupied.contains(p))
        .collect();
    assert_eq!(expected_spawn_positions.len(), 2, "預期有 2 個空格應召喚");

    let entries = execute_skill(
        &mut world,
        &SKILL_SUMMON_WALL_AOE.to_string(),
        &[caster_pos],
    )
    .expect("範圍召喚應成功");

    // 收集召喚事件的位置
    let spawn_positions: HashSet<Position> = entries
        .iter()
        .filter_map(|e: &EffectEntry| match (&e.effect, e.target) {
            (ResolvedEffect::SpawnObject { .. }, CheckTarget::Position(p)) => Some(p),
            _ => None,
        })
        .collect();

    assert_eq!(
        spawn_positions, expected_spawn_positions,
        "召喚位置只應為 AOE 內的空格"
    );

    // 驗證 World 中實際新增的 wall 物件數量
    let wall_count = world
        .query::<(&Object, &Position, &OccupantTypeName)>()
        .iter(&world)
        .filter(|(_, pos, name)| expected_spawn_positions.contains(pos) && name.0 == "wall")
        .count();
    assert_eq!(
        wall_count,
        expected_spawn_positions.len(),
        "World 中應在預期位置新增對應數量的 wall 物件"
    );

    // log：每個召喚 entry 轉成一筆 SpawnObject(wall)，target 名稱直接取自
    // effect 的 object_type（不查 World）——同格可疊多物件，事後查 World 取不到
    // 「剛召喚的那個」，故 target 與 effect 皆為 wall，不受該格原有 swamp 干擾。
    append_skill_log(&mut world, &entries).expect("append_skill_log 應成功");
    let log = get_battle_log(&world).expect("spawn_level 後應可取得 BattleLog");
    assert_eq!(
        log.len(),
        entries.len(),
        "log 筆數應與 EffectEntry 筆數一致"
    );
    let spawn_log_count = log
        .iter()
        .filter(|event| match event {
            LogEvent::Skill { target, effect, .. } => matches!(
                (target, effect),
                (LogTarget::Object { name }, LogEffect::SpawnObject { object_type })
                    if name == "wall" && object_type == "wall"
            ),
            _ => false,
        })
        .count();
    assert_eq!(
        spawn_log_count,
        expected_spawn_positions.len(),
        "每個召喚應產生一筆 target=wall + SpawnObject(wall) 的 log（取自 effect，不查 World）"
    );
}

/// 目標被牆擋住視線時，execute_skill 應回傳 NoLineOfSight 錯誤
///
/// 佈局（P=player，w=牆壁，E=敵人）：
/// ```text
/// . . . . .
/// . P w E .
/// . . . . .
/// ```
/// warrior-active-2 射程 [1, 2]，E 在距離 2 但被 w 擋住視線，施放應失敗
#[test]
fn test_execute_skill_blocked_by_wall() {
    let (mut world, player_occupant, markers) = build_warrior_world(
        "
        . . . . .
        . P w E .
        . . . . .
        ",
        10,
    );
    let enemy_pos = markers["E"][0];
    set_active_action_state(&mut world, player_occupant, ActionState::Moved { cost: 0 });

    let err = execute_skill(
        &mut world,
        &SKILL_WARRIOR_ACTIVE_2.to_string(),
        &[enemy_pos],
    )
    .expect_err("被牆擋住視線的目標應施放失敗");

    assert!(
        matches!(
            err.kind(),
            ErrorKind::Board(BoardError::NoLineOfSight { .. })
        ),
        "錯誤應為 NoLineOfSight，實際: {:?}",
        err.kind()
    );
}

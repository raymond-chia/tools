//! 反應系統整合測試

use super::constants::{
    OBJECTS_TOML, SKILL_WARRIOR_COUNTER, SKILL_WARRIOR_REACTION, SKILL_WARRIOR_REACTION_2,
    SKILLS_TOML, UNIT_TYPE_WARRIOR, UNIT_TYPE_WARRIOR_B, UNITS_TOML,
};
use bevy_ecs::prelude::{Entity, With, World};
use board::domain::alias::SkillName;
use board::domain::constants::PLAYER_FACTION_ID;
use board::domain::core_types::PendingReaction;
use board::ecs_logic::loader::parse_and_insert_game_data;
use board::ecs_logic::movement::execute_move;
use board::ecs_logic::reaction::{
    ProcessReactionResult, get_pending_reactions, process_reactions, set_reactions,
};
use board::ecs_logic::spawner::spawn_level;
use board::ecs_logic::turn::start_new_round;
use board::ecs_types::components::{Initiative, Occupant, Position, ReactionPoint, Unit};
use board::ecs_types::resources::ReactionState;
use board::logic::skill::skill_execution::EffectEntry;
use board::test_helpers::level_builder::{LevelBuilder, load_from_ascii};
use std::collections::HashMap;

const ENEMY_FACTION_ID: u32 = 2;

/// 建立含 P 和 E（均為 warrior）的 World，啟動新回合
///
/// - ascii 中 "P" 為玩家 warrior，"E" 為敵人 warrior
/// - P 的 Initiative 設為 100（確保先手）
/// - 回傳 (world, markers)，呼叫端自行從 markers 查 occupant
fn build_reaction_world(ascii: &str) -> (World, HashMap<String, Vec<Position>>) {
    let (_, markers) = load_from_ascii(ascii).expect("load_from_ascii 應成功");

    let level_toml = LevelBuilder::from_ascii(ascii)
        .unit("P", UNIT_TYPE_WARRIOR, PLAYER_FACTION_ID)
        .unit("E", UNIT_TYPE_WARRIOR, ENEMY_FACTION_ID)
        .unit("E1", UNIT_TYPE_WARRIOR, ENEMY_FACTION_ID)
        .unit("E2", UNIT_TYPE_WARRIOR_B, ENEMY_FACTION_ID)
        .to_toml()
        .expect("to_toml 應成功");

    let mut world = World::new();
    parse_and_insert_game_data(&mut world, UNITS_TOML, SKILLS_TOML, OBJECTS_TOML)
        .expect("parse_and_insert_game_data 應成功");
    spawn_level(&mut world, &level_toml, "test-level").expect("spawn_level 應成功");

    let player_pos = markers["P"][0];
    let player_entity = {
        let mut query = world.query::<(Entity, &Position)>();
        query
            .iter(&world)
            .find(|(_, p)| **p == player_pos)
            .map(|(e, _)| e)
            .expect("應找到玩家")
    };
    world.entity_mut(player_entity).insert(Initiative(100));

    // 為所有單位設定預設反應次數（各測試可在之後覆蓋）
    let all_unit_entities: Vec<Entity> = {
        let mut query = world.query_filtered::<Entity, With<Unit>>();
        query.iter(&world).collect()
    };
    for entity in all_unit_entities {
        world.entity_mut(entity).insert(ReactionPoint(1));
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

fn find_current_pos(world: &mut World, occupant: Occupant) -> Position {
    let mut query = world.query_filtered::<(&Occupant, &Position), With<Unit>>();
    *query
        .iter(world)
        .find(|(occ, _)| **occ == occupant)
        .map(|(_, pos)| pos)
        .expect("應找到指定 occupant 的位置")
}

fn find_entity(world: &mut World, occupant: Occupant) -> bevy_ecs::prelude::Entity {
    let mut query = world.query_filtered::<(bevy_ecs::prelude::Entity, &Occupant), With<Unit>>();
    query
        .iter(world)
        .find(|(_, occ)| **occ == occupant)
        .map(|(e, _)| e)
        .expect("應找到指定 occupant 的 entity")
}

// ============================================================================
// get_pending_reactions 測試
// ============================================================================

/// get_pending_reactions 在各種 Resource 狀態下的回傳行為
///
/// | # | Resource 狀態               | 預期                          |
/// |---|----------------------------|-------------------------------|
/// | 1 | Resource 不存在             | 回傳空 Vec                    |
/// | 2 | Resource 存在，reactions 空 | 回傳空 Vec                    |
/// | 3 | Resource 存在，1 個反應者   | 回傳含觸發者與技能清單的 1 筆 |
/// | 4 | Resource 存在，2 個反應者，觸發者相同 | 回傳 2 筆，各自正確  |
/// | 5 | Resource 存在，2 個反應者，觸發者不同 | 回傳 2 筆，各自觸發者正確 |
#[test]
fn test_get_pending_reactions() {
    const REACTOR_ID_1: u32 = 10;
    const REACTOR_ID_2: u32 = 20;
    const TRIGGER_ID_1: u32 = 91;
    const TRIGGER_ID_2: u32 = 92;

    struct TestCase {
        name: &'static str,
        resource: Option<ReactionState>,
        expected_len: usize,
    }

    let skill_a: SkillName = "warrior-reaction".to_string();
    let skill_b: SkillName = "melee-attack".to_string();

    let test_data = [
        TestCase {
            name: "resource 不存在",
            resource: None,
            expected_len: 0,
        },
        TestCase {
            name: "resource 存在但 reactions 為空",
            resource: Some(ReactionState {
                pending: vec![],
                queue: vec![],
            }),
            expected_len: 0,
        },
        TestCase {
            name: "1 個反應者",
            resource: Some(ReactionState {
                pending: vec![PendingReaction {
                    reactor: Occupant::Unit(REACTOR_ID_1),
                    trigger: Occupant::Unit(TRIGGER_ID_1),
                    available_skills: vec![skill_a.clone()],
                }],
                queue: vec![],
            }),
            expected_len: 1,
        },
        TestCase {
            name: "2 個反應者，觸發者相同",
            resource: Some(ReactionState {
                pending: vec![
                    PendingReaction {
                        reactor: Occupant::Unit(REACTOR_ID_1),
                        trigger: Occupant::Unit(TRIGGER_ID_1),
                        available_skills: vec![skill_a.clone()],
                    },
                    PendingReaction {
                        reactor: Occupant::Unit(REACTOR_ID_2),
                        trigger: Occupant::Unit(TRIGGER_ID_1),
                        available_skills: vec![skill_b.clone()],
                    },
                ],
                queue: vec![],
            }),
            expected_len: 2,
        },
        TestCase {
            name: "2 個反應者，觸發者不同",
            resource: Some(ReactionState {
                pending: vec![
                    PendingReaction {
                        reactor: Occupant::Unit(REACTOR_ID_1),
                        trigger: Occupant::Unit(TRIGGER_ID_1),
                        available_skills: vec![skill_a.clone(), skill_b.clone()],
                    },
                    PendingReaction {
                        reactor: Occupant::Unit(REACTOR_ID_2),
                        trigger: Occupant::Unit(TRIGGER_ID_2),
                        available_skills: vec![skill_b.clone()],
                    },
                ],
                queue: vec![],
            }),
            expected_len: 2,
        },
    ];

    for case in test_data {
        let mut world = World::new();

        if let Some(resource) = case.resource {
            let snapshot: Vec<(Occupant, Occupant, Vec<SkillName>)> = resource
                .pending
                .iter()
                .map(|r| (r.reactor, r.trigger, r.available_skills.clone()))
                .collect();

            world.insert_resource(resource);

            let result = get_pending_reactions(&world);

            assert_eq!(
                result.len(),
                case.expected_len,
                "[{}] 回傳數量應為 {}",
                case.name,
                case.expected_len
            );

            for (i, (reactor, trigger, skills)) in snapshot.iter().enumerate() {
                assert_eq!(
                    result[i].reactor, *reactor,
                    "[{}] 第 {} 個反應者 occupant 應正確",
                    case.name, i
                );
                assert_eq!(
                    result[i].trigger, *trigger,
                    "[{}] 第 {} 個反應者的觸發者應正確",
                    case.name, i
                );
                assert_eq!(
                    result[i].available_skills, *skills,
                    "[{}] 第 {} 個反應者技能清單應正確",
                    case.name, i
                );
            }
        } else {
            let result = get_pending_reactions(&world);
            assert_eq!(
                result.len(),
                case.expected_len,
                "[{}] resource 不存在時應回傳空 Vec",
                case.name
            );
        }
    }
}

// ============================================================================
// set_reactions 錯誤情境
// ============================================================================

/// set_reactions 在非法輸入下應回傳 Err
///
/// | # | 情境                              | 預期 |
/// |---|----------------------------------|------|
/// | 1 | Resource 不存在                  | Err  |
/// | 2 | ReactionState 為空            | Err  |
/// | 3 | Occupant 不在 pending 清單       | Err  |
/// | 4 | 技能不在該 reactor 的可用清單     | Err  |
#[test]
fn test_set_reactions_errors() {
    const REACTOR_ID: u32 = 10;
    const TRIGGER_ID: u32 = 91;

    let reactor = Occupant::Unit(REACTOR_ID);
    let skill_a: SkillName = "warrior-reaction".to_string();
    let skill_b: SkillName = "melee-attack".to_string();

    // Case 1：Resource 不存在
    {
        let mut world = World::new();
        let result = set_reactions(&mut world, vec![(reactor, skill_a.clone())]);
        assert!(result.is_err(), "Resource 不存在時應回傳 Err");
    }

    // Case 2：pending 為空
    {
        let mut world = World::new();
        world.insert_resource(ReactionState {
            pending: vec![],
            queue: vec![],
        });
        let result = set_reactions(&mut world, vec![(reactor, skill_a.clone())]);
        assert!(result.is_err(), "pending 為空時應回傳 Err");
    }

    // Case 3：Occupant 不在 pending 清單
    {
        let mut world = World::new();
        world.insert_resource(ReactionState {
            pending: vec![PendingReaction {
                reactor,
                trigger: Occupant::Unit(TRIGGER_ID),
                available_skills: vec![skill_a.clone()],
            }],
            queue: vec![],
        });
        let unknown = Occupant::Unit(99);
        let result = set_reactions(&mut world, vec![(unknown, skill_a.clone())]);
        assert!(result.is_err(), "未知 Occupant 應回傳 Err");
    }

    // Case 4：技能不在該 reactor 的可用清單
    {
        let mut world = World::new();
        world.insert_resource(ReactionState {
            pending: vec![PendingReaction {
                reactor,
                trigger: Occupant::Unit(TRIGGER_ID),
                available_skills: vec![skill_a.clone()],
            }],
            queue: vec![],
        });
        let result = set_reactions(&mut world, vec![(reactor, skill_b.clone())]);
        assert!(result.is_err(), "技能不在可用清單應回傳 Err");
    }
}

// ============================================================================
// process_reactions 錯誤情境
// ============================================================================

/// 補充錯誤情境：`set_reactions` 傳空決定後呼叫 `process_reactions` 應回傳 Err
#[test]
fn test_process_reactions_err_when_no_decisions() {
    let (mut world, markers) = build_reaction_world(
        r#"
P E . . ."#,
    );

    let enemy_occupant = find_occupant(&mut world, markers["E"][0]);

    // 建立有 1 個反應者的 ReactionState
    world.insert_resource(ReactionState {
        pending: vec![PendingReaction {
            reactor: enemy_occupant,
            trigger: Occupant::Unit(0),
            available_skills: vec![SKILL_WARRIOR_REACTION.to_string()],
        }],
        queue: vec![],
    });

    // 傳空決定（全部放棄）
    set_reactions(&mut world, vec![]).expect("空決定 set_reactions 應成功");

    // 沒有待執行項目，呼叫 process_reactions 應 Err
    let result = process_reactions(&mut world);
    assert!(
        result.is_err(),
        "無待執行反應時 process_reactions 應回傳 Err"
    );
}

// ============================================================================
// set_reactions + process_reactions 整合測試
// ============================================================================

/// 整合測試 1：移動路過 1 個敵人，敵人 UseSkill（warrior-reaction），反應後繼續移動
///
/// 地圖：
/// P E . . .
/// . . . T .
///
/// | # | E 的 ReactionPoint | 第二次移動觸發 | E 選擇 | 預期結果                    |
/// |---|-------------------|-------------|-------|----------------------------|
/// | 1 | 1                 | 否          | -     | 無反應，P 直達目的地          |
/// | 2 | 2                 | 是          | 使用  | 觸發第二次反應，Done          |
/// | 3 | 2                 | 是          | 放棄  | 放棄反應後 P 直達目的地       |
#[test]
fn test_reaction_single_use_skill() {
    struct TestCase {
        name: &'static str,
        enemy_reaction_point: i32,
        /// 第二次移動是否觸發待反應（有 ReactionState）
        second_move_has_pending: bool,
        /// 觸發後 E 是否使用技能（false 代表放棄）
        second_move_use_skill: bool,
    }

    const REACTION_POINT_1: i32 = 1;
    const REACTION_POINT_2: i32 = 2;

    let test_data = [
        TestCase {
            name: "E 剩 1 次反應次數，第二次移動不觸發",
            enemy_reaction_point: REACTION_POINT_1,
            second_move_has_pending: false,
            second_move_use_skill: false,
        },
        TestCase {
            name: "E 剩 2 次反應次數，第二次移動觸發且 E 使用技能",
            enemy_reaction_point: REACTION_POINT_2,
            second_move_has_pending: true,
            second_move_use_skill: true,
        },
        TestCase {
            name: "E 剩 2 次反應次數，第二次移動觸發但 E 放棄，P 直達目的地",
            enemy_reaction_point: REACTION_POINT_2,
            second_move_has_pending: true,
            second_move_use_skill: false,
        },
    ];

    for case in test_data {
        let (mut world, markers) = build_reaction_world(
            r#"
P E . . .
. . . T ."#,
        );

        let player_occupant = find_occupant(&mut world, markers["P"][0]);
        let enemy_occupant = find_occupant(&mut world, markers["E"][0]);
        let target_pos = markers["T"][0];

        // 設定 E 的 ReactionPoint
        let enemy_entity = find_entity(&mut world, enemy_occupant);
        world
            .entity_mut(enemy_entity)
            .insert(ReactionPoint(case.enemy_reaction_point));

        const STOP_AFTER_FIRST_MOVE: Position = Position { x: 0, y: 1 };

        // ── 第一次移動 ──
        {
            execute_move(&mut world, target_pos).expect("第一次移動應成功");

            let player_pos = find_current_pos(&mut world, player_occupant);
            assert_eq!(
                player_pos, STOP_AFTER_FIRST_MOVE,
                "[{}] 第一次移動後 P 應停在觸發步的 to 位置",
                case.name
            );

            let pending = get_pending_reactions(&world);
            assert_eq!(
                pending.len(),
                1,
                "[{}] 第一次移動應有 1 個待反應者",
                case.name
            );
            assert_eq!(
                pending[0].reactor, enemy_occupant,
                "[{}] 反應者應為敵人",
                case.name
            );
            assert_eq!(
                pending[0].trigger, player_occupant,
                "[{}] 觸發者應為玩家",
                case.name
            );

            set_reactions(
                &mut world,
                vec![(enemy_occupant, pending[0].available_skills[0].clone())],
            )
            .expect("第一次 set_reactions 應成功");

            let result = process_reactions(&mut world).expect("第一次 process_reactions 應成功");
            match &result {
                ProcessReactionResult::Executed { effects } => {
                    assert!(
                        effects
                            .iter()
                            .any(|e| e.skill_name == SKILL_WARRIOR_REACTION),
                        "[{}] 第一次反應的技能應為 {}，effects：{:?}",
                        case.name,
                        SKILL_WARRIOR_REACTION,
                        effects,
                    );
                }
                _ => panic!(
                    "[{}] 第一次反應應回傳 Executed，實際：{:?}",
                    case.name, result
                ),
            }

            let result = process_reactions(&mut world).expect("反應鏈結束應成功");
            assert!(
                matches!(result, ProcessReactionResult::Done),
                "[{}] 第一次反應鏈結束應回傳 Done，實際：{:?}",
                case.name,
                result
            );
        }

        // ── 第二次移動 ──
        execute_move(&mut world, target_pos).expect("第二次移動應成功");

        let pending = get_pending_reactions(&world);

        if case.second_move_has_pending {
            const STOP_AFTER_SECOND_MOVE: Position = Position { x: 2, y: 1 };

            assert_eq!(
                pending.len(),
                1,
                "[{}] 第二次移動應有 1 個待反應者",
                case.name
            );
            assert_eq!(
                pending[0].reactor, enemy_occupant,
                "[{}] 反應者應為敵人",
                case.name
            );
            assert_eq!(
                pending[0].trigger, player_occupant,
                "[{}] 觸發者應為玩家",
                case.name
            );

            let player_pos = find_current_pos(&mut world, player_occupant);
            assert_eq!(
                player_pos, STOP_AFTER_SECOND_MOVE,
                "[{}] 第二次移動觸發反應後 P 應停在觸發步的 to 位置",
                case.name
            );

            if case.second_move_use_skill {
                set_reactions(
                    &mut world,
                    vec![(enemy_occupant, pending[0].available_skills[0].clone())],
                )
                .expect("第二次 set_reactions（使用技能）應成功");

                let result =
                    process_reactions(&mut world).expect("第二次 process_reactions 應成功");
                match &result {
                    ProcessReactionResult::Executed { effects } => {
                        assert!(
                            effects
                                .iter()
                                .any(|e| e.skill_name == SKILL_WARRIOR_REACTION),
                            "[{}] 第二次反應的技能應為 {}，effects：{:?}",
                            case.name,
                            SKILL_WARRIOR_REACTION,
                            effects,
                        );
                    }
                    _ => panic!(
                        "[{}] 第二次反應應回傳 Executed，實際：{:?}",
                        case.name, result
                    ),
                }

                let result = process_reactions(&mut world).expect("第二次反應鏈結束應成功");
                assert!(
                    matches!(result, ProcessReactionResult::Done),
                    "[{}] 第二次反應鏈結束應回傳 Done，實際：{:?}",
                    case.name,
                    result
                );
            } else {
                // E 放棄反應，P 應繼續移動並直達目的地
                set_reactions(&mut world, vec![]).expect("第二次 set_reactions（放棄）應成功");

                execute_move(&mut world, target_pos).expect("第三次移動應成功");
                let player_pos = find_current_pos(&mut world, player_occupant);
                assert_eq!(
                    player_pos, target_pos,
                    "[{}] E 放棄反應後 P 應直達目的地",
                    case.name
                );
            }
        } else {
            assert_eq!(pending.len(), 0, "[{}] 第二次移動不應觸發反應", case.name);

            let player_pos = find_current_pos(&mut world, player_occupant);
            assert_eq!(
                player_pos, target_pos,
                "[{}] 第二次移動無反應，P 應到達目標位置",
                case.name
            );
        }
    }
}

/// 整合測試 2：移動路過 2 個敵人，兩人用不同技能，分別以不同順序執行
///
/// 地圖：P(0,0)  E1(1,0)  E2(2,0)  .(3,0)  T(4,0)
/// - E1 為 warrior，擁有 warrior-reaction（AttackOfOpportunity）
/// - E2 為 warrior-b，擁有 warrior-reaction-2（AttackOfOpportunity）
/// P 移動到 T，路過兩者的 range=1 範圍
///
/// 組合（table-driven）：
/// - 組合 A：順序 [E1, E2]，E1 先執行 warrior-reaction，E2 後執行 warrior-reaction-2
/// - 組合 B：順序 [E2, E1]，E2 先執行 warrior-reaction-2，E1 後執行 warrior-reaction
/// - 組合 C：順序 [E2, E1]，兩人皆用 warrior-reaction
/// - 組合 D：只有 E1 使用技能，E2 放棄 → 只有一次 Executed
/// - 組合 E：只有 E2 使用技能，E1 放棄 → 只有一次 Executed
#[test]
fn test_reaction_two_reactors_different_skills_and_order() {
    struct TestCase {
        name: &'static str,
        decisions: fn(Occupant, Occupant) -> Vec<(Occupant, SkillName)>,
        expected_first: fn(Occupant, Occupant) -> Occupant,
        expected_first_skill: &'static str,
        expected_second: fn(Occupant, Occupant) -> Occupant,
        expected_second_skill: &'static str,
        second_is_done: bool,
    }

    let test_data = [
        TestCase {
            name: "組合 A：順序 [E1, E2]",
            decisions: |e1, e2| {
                vec![
                    (e1, SKILL_WARRIOR_REACTION.to_string()),
                    (e2, SKILL_WARRIOR_REACTION_2.to_string()),
                ]
            },
            expected_first: |e1, _| e1,
            expected_first_skill: SKILL_WARRIOR_REACTION,
            expected_second: |_, e2| e2,
            expected_second_skill: SKILL_WARRIOR_REACTION_2,
            second_is_done: false,
        },
        TestCase {
            name: "組合 B：順序 [E2, E1]",
            decisions: |e1, e2| {
                vec![
                    (e2, SKILL_WARRIOR_REACTION_2.to_string()),
                    (e1, SKILL_WARRIOR_REACTION.to_string()),
                ]
            },
            expected_first: |_, e2| e2,
            expected_first_skill: SKILL_WARRIOR_REACTION_2,
            expected_second: |e1, _| e1,
            expected_second_skill: SKILL_WARRIOR_REACTION,
            second_is_done: false,
        },
        TestCase {
            name: "組合 C：順序 [E2, E1]，兩人皆用 warrior-reaction",
            decisions: |e1, e2| {
                vec![
                    (e2, SKILL_WARRIOR_REACTION.to_string()),
                    (e1, SKILL_WARRIOR_REACTION.to_string()),
                ]
            },
            expected_first: |_, e2| e2,
            expected_first_skill: SKILL_WARRIOR_REACTION,
            expected_second: |e1, _| e1,
            expected_second_skill: SKILL_WARRIOR_REACTION,
            second_is_done: false,
        },
        TestCase {
            name: "組合 D：只有 E1 使用技能，E2 放棄",
            decisions: |e1, _e2| vec![(e1, SKILL_WARRIOR_REACTION.to_string())],
            expected_first: |e1, _| e1,
            expected_first_skill: SKILL_WARRIOR_REACTION,
            expected_second: |e1, _| e1,
            expected_second_skill: SKILL_WARRIOR_REACTION,
            second_is_done: true,
        },
        TestCase {
            name: "組合 E：只有 E2 使用技能，E1 放棄",
            decisions: |_e1, e2| vec![(e2, SKILL_WARRIOR_REACTION_2.to_string())],
            expected_first: |_, e2| e2,
            expected_first_skill: SKILL_WARRIOR_REACTION_2,
            expected_second: |_, e2| e2,
            expected_second_skill: SKILL_WARRIOR_REACTION_2,
            second_is_done: true,
        },
    ];

    for case in test_data {
        let (mut world, markers) = build_reaction_world(
            r#"
. E1 . . .
P .  . . T
. E2 . ."#,
        );

        let e1_occupant = find_occupant(&mut world, markers["E1"][0]);
        let e2_occupant = find_occupant(&mut world, markers["E2"][0]);
        let target_pos = markers["T"][0];

        // P 移動到 T，路過 E1(x=1) 和 E2(x=2) 的 range=1 範圍
        execute_move(&mut world, target_pos).expect("移動應成功");

        // 應有 2 個待反應者
        let pending = get_pending_reactions(&world);
        assert_eq!(pending.len(), 2, "[{}] 應有 2 個待反應者", case.name);

        let decisions = (case.decisions)(e1_occupant, e2_occupant);
        set_reactions(&mut world, decisions)
            .expect(&format!("[{}] set_reactions 應成功", case.name));

        // 第一次執行
        let result = process_reactions(&mut world)
            .expect(&format!("[{}] 第一次 process_reactions 應成功", case.name));
        match &result {
            ProcessReactionResult::Executed { effects } => {
                let expected_reactor = (case.expected_first)(e1_occupant, e2_occupant);
                let expected_caster_id = match expected_reactor {
                    Occupant::Unit(id) => id,
                    Occupant::Object(_) => panic!("expected_reactor 不應為 Object"),
                };
                assert_eq!(
                    effects[0].caster, expected_caster_id,
                    "[{}] 第一個執行者應為 {:?}，effects：{:?}",
                    case.name, expected_reactor, effects,
                );
                assert_eq!(
                    effects[0].skill_name, case.expected_first_skill,
                    "[{}] 第一次執行的技能應為 {}，effects：{:?}",
                    case.name, case.expected_first_skill, effects,
                );
            }
            _ => panic!("[{}] 第一次應回傳 Executed，實際：{:?}", case.name, result),
        }

        // 第二次執行：若 second_is_done 則預期 Done，否則預期 Executed
        let result = process_reactions(&mut world)
            .expect(&format!("[{}] 第二次 process_reactions 應成功", case.name));
        if case.second_is_done {
            assert!(
                matches!(result, ProcessReactionResult::Done),
                "[{}] 放棄者跳過後應回傳 Done，實際：{:?}",
                case.name,
                result
            );
        } else {
            match &result {
                ProcessReactionResult::Executed { effects } => {
                    let expected_reactor = (case.expected_second)(e1_occupant, e2_occupant);
                    let expected_caster_id = match expected_reactor {
                        Occupant::Unit(id) => id,
                        Occupant::Object(_) => panic!("expected_reactor 不應為 Object"),
                    };
                    assert_eq!(
                        effects[0].caster, expected_caster_id,
                        "[{}] 第二個執行者應為 {:?}，effects：{:?}",
                        case.name, expected_reactor, effects,
                    );
                    assert_eq!(
                        effects[0].skill_name, case.expected_second_skill,
                        "[{}] 第二次執行的技能應為 {}，effects：{:?}",
                        case.name, case.expected_second_skill, effects,
                    );
                }
                _ => panic!("[{}] 第二次應回傳 Executed，實際：{:?}", case.name, result),
            }

            // 第三次：Done
            let result = process_reactions(&mut world)
                .expect(&format!("[{}] 第三次 process_reactions 應成功", case.name));
            assert!(
                matches!(result, ProcessReactionResult::Done),
                "[{}] 最後應回傳 Done，實際：{:?}",
                case.name,
                result
            );
        }
    }
}

/// 整合測試 3：E 反應攻擊 P，P 有 warrior-counter（TakesDamage）觸發新反應
///
/// 地圖：P(0,0)  E(1,0)  .(2,0)  T(3,0)
/// P 移動到 T → E 觸發 AttackOfOpportunity 反應
/// E 執行 warrior-reaction 攻擊 P → P 的 warrior-counter（TakesDamage）觸發
///
/// 流程：
/// execute_move → set_reactions(E warrior-reaction) →
/// process_reactions → Executed（E 攻擊 P）→
/// process_reactions → NeedDecision（P 的 counter 觸發）→
/// set_reactions(P UseSkill warrior-counter) →
/// process_reactions → Executed（P 反擊 E）→
/// process_reactions → Done
#[test]
fn test_reaction_chain_counter() {
    let (mut world, markers) = build_reaction_world(
        r#"
P E . .
. . . T"#,
    );

    let player_occupant = find_occupant(&mut world, markers["P"][0]);
    let enemy_occupant = find_occupant(&mut world, markers["E"][0]);
    let target_pos = markers["T"][0];

    // P 移動到 T，觸發 E 的 AttackOfOpportunity
    execute_move(&mut world, target_pos).expect("移動應成功");

    // 第一輪：E 有待反應
    let pending = get_pending_reactions(&world);
    assert_eq!(pending.len(), 1, "第一輪應有 1 個待反應者（E）");
    assert_eq!(pending[0].reactor, enemy_occupant, "反應者應為 E");
    assert_eq!(pending[0].trigger, player_occupant, "觸發者應為 P");

    set_reactions(
        &mut world,
        vec![(enemy_occupant, pending[0].available_skills[0].clone())],
    )
    .expect("第一輪 set_reactions 應成功");

    // E 執行 warrior-reaction 攻擊 P
    let result = process_reactions(&mut world).expect("第一次 process_reactions 應成功");
    match &result {
        ProcessReactionResult::Executed { effects } => {
            assert_eq!(
                effects[0].skill_name, SKILL_WARRIOR_REACTION,
                "E 反應的技能應為 {}，effects：{:?}",
                SKILL_WARRIOR_REACTION, effects,
            );
        }
        _ => panic!("E 反應應回傳 Executed，實際：{:?}", result),
    }

    // P 被攻擊，warrior-counter（TakesDamage）觸發 → NeedDecision
    let result = process_reactions(&mut world).expect("第二次 process_reactions 應成功");
    assert!(
        matches!(result, ProcessReactionResult::NeedDecision),
        "P 的 counter 觸發應回傳 NeedDecision，實際：{:?}",
        result
    );

    // 取得新的 pending reactions（P 的 counter）
    let pending = get_pending_reactions(&world);
    assert_eq!(pending.len(), 1, "第二輪應有 1 個待反應者（P）");
    assert_eq!(pending[0].reactor, player_occupant, "反應者應為 P");

    // P 決定使用 warrior-counter 反擊 E
    set_reactions(
        &mut world,
        vec![(player_occupant, pending[0].available_skills[0].clone())],
    )
    .expect("第二輪 set_reactions 應成功");

    // P 執行反擊
    let result = process_reactions(&mut world).expect("第三次 process_reactions 應成功");
    match &result {
        ProcessReactionResult::Executed { effects } => {
            assert_eq!(
                effects[0].skill_name, SKILL_WARRIOR_COUNTER,
                "P 反擊的技能應為 {}，effects：{:?}",
                SKILL_WARRIOR_COUNTER, effects,
            );
        }
        _ => panic!("P 反擊應回傳 Executed，實際：{:?}", result),
    }

    // 無新反應，Done
    let result = process_reactions(&mut world).expect("第四次 process_reactions 應成功");
    assert!(
        matches!(result, ProcessReactionResult::Done),
        "反應鏈結束應回傳 Done，實際：{:?}",
        result
    );
}

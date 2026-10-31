use crate::support::{
    ACTOR_ID, BLOCKER_ID, TARGET_ID, TEST_SKILL_ID, ascii_board, marked_cell, skill_test_data, unit,
};
use game_core::{
    ActiveSkillEffect, ActiveSkillPreview, AttackResult, CombatLogEvent, Command, Game, Phase,
    PowerSource, Team, authoring,
};

const THIRD_ID: i64 = 4;
const OUTSIDE_ID: i64 = 5;

fn data(diagram: &str) -> (authoring::Definitions, authoring::Map) {
    let (mut definitions, mut map) = skill_test_data(
        1,
        1,
        ActiveSkillEffect::Sweep {
            power_source: PowerSource::Physical,
            hit_bonus: 0,
            power_bonus: 0,
        },
        Team::Enemy("bandits".into()),
        diagram,
    );
    let actor = definitions
        .unit_types
        .iter_mut()
        .find(|unit| unit.id == "test_actor")
        .expect("測試資料應有施放者");
    actor.physical_power = 6;
    actor.hit = 100;
    if let Some(blocker) = map.units.iter_mut().find(|unit| unit.id == BLOCKER_ID) {
        blocker.team = Team::Player;
    }
    let layout = ascii_board(diagram);
    for (marker, id) in [('C', THIRD_ID), ('O', OUTSIDE_ID)] {
        if layout.markers.contains_key(&marker) {
            let position = marked_cell(&layout, marker);
            let mut placement = map
                .units
                .iter()
                .find(|unit| unit.id == TARGET_ID)
                .expect("測試棋盤應有 T")
                .clone();
            placement.id = id;
            placement.x = position.x;
            placement.y = position.y;
            map.units.push(placement);
        }
    }
    (definitions, map)
}

fn start(definitions: authoring::Definitions, map: authoring::Map) -> Game {
    let mut game = Game::from_authoring(definitions, map, authoring::Difficulty::Easy)
        .expect("橫掃測試資料應可載入");
    game.command(Command::Start).expect("測試戰鬥應可開始");
    // 種子 2 的前三次 d20 固定為 7、14、10；開始戰鬥後再設定，避開先攻擲骰。
    game.set_random_seed(2);
    game
}

// 驗證四個方向都只攻擊前方三格，友軍也受傷，身後與施放者不受傷且只消耗一次行動。
#[test]
fn sweep_hits_front_row_including_allies_in_four_directions() {
    for diagram in [
        "
        .....
        .TBC.
        ..A..
        ..O..
        .....
        ",
        "
        .....
        ..O..
        ..A..
        .CBT.
        .....
        ",
        "
        .....
        .C...
        .BAO.
        .T...
        .....
        ",
        "
        .....
        ...T.
        .OAB.
        ...C.
        .....
        ",
    ] {
        let center = marked_cell(&ascii_board(diagram), 'B');
        let (definitions, map) = data(diagram);
        let mut game = start(definitions, map);
        let preview = game
            .preview_skill(ACTOR_ID, center, TEST_SKILL_ID)
            .expect("橫掃應可預覽友軍所在的中心格");
        match preview {
            ActiveSkillPreview::Area { cells, targets } => {
                assert_eq!(cells.len(), 3);
                assert_eq!(
                    targets
                        .iter()
                        .map(|target| target.target)
                        .collect::<Vec<_>>(),
                    vec![TARGET_ID, BLOCKER_ID, THIRD_ID]
                );
                assert_eq!(targets.iter().all(|target| target.hit_damage == 6), true);
            }
            _ => panic!("橫掃應回傳範圍預覽"),
        }
        let snapshot = game
            .command(Command::Skill {
                actor: ACTOR_ID,
                x: center.x,
                y: center.y,
                skill: TEST_SKILL_ID.into(),
            })
            .expect("橫掃應成功結算");
        let targets: Vec<_> = snapshot
            .log
            .iter()
            .filter_map(|event| match event {
                CombatLogEvent::Skill { target, .. } => Some(*target),
                _ => None,
            })
            .collect();
        assert_eq!(targets, vec![TARGET_ID, BLOCKER_ID, THIRD_ID]);
        for id in [TARGET_ID, BLOCKER_ID, THIRD_ID] {
            assert_eq!(unit(&snapshot, id).hp, 94, "範圍內的 {id} 應扣 6 HP");
        }
        assert_eq!(unit(&snapshot, OUTSIDE_ID).hp, 100);
        assert_eq!(unit(&snapshot, ACTOR_ID).hp, 100);
        assert_eq!(snapshot.turn.phase, Phase::Ended);
    }
}

// 驗證地圖邊緣裁切、空中心格與大型目標去重，斜角與界外中心不可施放且不消耗行動。
#[test]
fn sweep_handles_edges_empty_centers_and_large_targets() {
    for (diagram, expected_cells) in [
        (
            "
        A.T
        D..
        ...
        ",
            2,
        ),
        (
            "
        .....
        .TT..
        .TT..
        .AD..
        .....
        ",
            3,
        ),
    ] {
        let layout = ascii_board(diagram);
        let center = marked_cell(&layout, 'D');
        let (definitions, map) = data(diagram);
        let mut game = start(definitions, map);
        match game
            .preview_skill(ACTOR_ID, center, TEST_SKILL_ID)
            .expect("空中心格可預覽")
        {
            ActiveSkillPreview::Area { cells, targets } => {
                assert_eq!(cells.len(), expected_cells);
                assert_eq!(targets.len(), if expected_cells == 2 { 0 } else { 1 });
            }
            _ => panic!("應回傳範圍預覽"),
        }
        let invalid = game_core::GridPos { x: -1, y: 0 };
        assert_eq!(
            game.preview_skill(ACTOR_ID, invalid, TEST_SKILL_ID)
                .is_err(),
            true
        );
        let actor = marked_cell(&layout, 'A');
        assert_eq!(
            game.command(Command::Skill {
                actor: ACTOR_ID,
                x: actor.x + 1,
                y: actor.y + 1,
                skill: TEST_SKILL_ID.into(),
            })
            .is_err(),
            true
        );
        assert_eq!(game.snapshot(None).turn.phase, Phase::Ready);
        let snapshot = game
            .command(Command::Skill {
                actor: ACTOR_ID,
                x: center.x,
                y: center.y,
                skill: TEST_SKILL_ID.into(),
            })
            .expect("空中心格可施放橫掃");
        assert_eq!(
            snapshot
                .log
                .iter()
                .filter(|event| matches!(event, CombatLogEvent::Skill { .. }))
                .count(),
            if expected_cells == 2 { 0 } else { 1 }
        );
    }
}

// 驗證固定擲出 7 時，橫掃的威力來源與正負加成產生明確的命中結果及剩餘 HP。
#[test]
fn sweep_uses_selected_power_for_preview_and_damage() {
    struct Case {
        name: &'static str,
        power_source: PowerSource,
        base_hit: i32,
        hit_bonus: i32,
        power_bonus: i32,
        preview_damage: i32,
        result: AttackResult,
        damage: i32,
        remaining_hp: i32,
    }
    let cases = [
        // 7 + 3 達到閃避門檻 10，物理威力 6 扣除 6 HP。
        Case {
            name: "物理命中",
            power_source: PowerSource::Physical,
            base_hit: 0,
            hit_bonus: 3,
            power_bonus: 0,
            preview_damage: 6,
            result: AttackResult::Hit,
            damage: 6,
            remaining_hp: 94,
        },
        // 7 + 3 命中，魔法威力 17 加上 3，扣除 20 HP。
        Case {
            name: "魔法威力加成",
            power_source: PowerSource::Magical,
            base_hit: 0,
            hit_bonus: 3,
            power_bonus: 3,
            preview_damage: 20,
            result: AttackResult::Hit,
            damage: 20,
            remaining_hp: 80,
        },
        // 沒有命中加成時，7 未達門檻 10，物理威力再高也不扣血。
        Case {
            name: "零命中加成",
            power_source: PowerSource::Physical,
            base_hit: 0,
            hit_bonus: 0,
            power_bonus: 4,
            preview_damage: 10,
            result: AttackResult::Dodge,
            damage: 0,
            remaining_hp: 100,
        },
        // 原本 7 + 5 可命中，負加成使 7 + 5 - 3 未達門檻 10，不扣血。
        Case {
            name: "負命中加成",
            power_source: PowerSource::Physical,
            base_hit: 5,
            hit_bonus: -3,
            power_bonus: 4,
            preview_damage: 10,
            result: AttackResult::Dodge,
            damage: 0,
            remaining_hp: 100,
        },
        // 命中後魔法威力 17 減去 50，傷害最低為 0。
        Case {
            name: "負威力加成",
            power_source: PowerSource::Magical,
            base_hit: 0,
            hit_bonus: 3,
            power_bonus: -50,
            preview_damage: 0,
            result: AttackResult::Hit,
            damage: 0,
            remaining_hp: 100,
        },
    ];
    for Case {
        name,
        power_source,
        base_hit,
        hit_bonus,
        power_bonus,
        preview_damage,
        result,
        damage,
        remaining_hp,
    } in cases
    {
        let diagram = "
        ...
        .T.
        .A.
        ";
        let (mut definitions, map) = skill_test_data(
            1,
            1,
            ActiveSkillEffect::Sweep {
                power_source,
                hit_bonus,
                power_bonus,
            },
            Team::Enemy("bandits".into()),
            diagram,
        );
        for unit in &mut definitions.unit_types {
            if unit.id == "test_actor" {
                unit.hit = base_hit;
                unit.physical_power = 6;
                unit.magical_power = 17;
            } else {
                unit.dodge = 10;
            }
        }
        let center = marked_cell(&ascii_board(diagram), 'T');
        let mut game = start(definitions, map);
        match game
            .preview_skill(ACTOR_ID, center, TEST_SKILL_ID)
            .expect("橫掃應可預覽")
        {
            ActiveSkillPreview::Area { cells: _, targets } => {
                assert_eq!(targets.len(), 1, "{name}");
                assert_eq!(targets[0].hit_damage, preview_damage, "{name}");
            }
            _ => panic!("橫掃應回傳範圍預覽"),
        }
        let snapshot = game
            .command(Command::Skill {
                actor: ACTOR_ID,
                x: center.x,
                y: center.y,
                skill: TEST_SKILL_ID.into(),
            })
            .expect("橫掃應成功結算");
        let event = snapshot
            .log
            .iter()
            .find(|event| matches!(event, CombatLogEvent::Skill { .. }))
            .expect("橫掃應留下攻擊日誌");
        match event {
            CombatLogEvent::Skill {
                roll,
                result: actual_result,
                damage: actual_damage,
                ..
            } => {
                assert_eq!(*roll, 7, "{name}：固定擲出 7");
                assert_eq!(*actual_result, result, "{name}");
                assert_eq!(*actual_damage, damage, "{name}");
            }
            _ => panic!("應取得攻擊日誌"),
        }
        assert_eq!(unit(&snapshot, TARGET_ID).hp, remaining_hp, "{name}");
    }
}

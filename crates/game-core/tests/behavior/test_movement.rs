use crate::support::{
    ACTOR_ID, AsciiBoard, TARGET_ID, ascii_board, marked_cell, skill_test_data, unit,
};
use game_core::{
    ActiveSkillEffect, Command, Game, GridPos, MovePreview, Phase, PowerSource, Team,
    TerrainEntryRule, TerrainLayer, TerrainTypeDef,
};

struct MovementPreviewCase {
    name: &'static str,
    movement: u32,
    interrupted: bool,
    path_length: usize,
    second_path_length: usize,
    total_cost: u32,
}

// 驗證玩家預覽優先第一段路徑；第一段只能踩尖刺時，不為避傷改用第二段。
#[test]
fn move_preview_chooses_safest_path_within_first_phase() {
    let cases = [
        MovementPreviewCase {
            name: "第一階段可以安全繞路",
            movement: 4,
            interrupted: false,
            path_length: 5,
            second_path_length: 0,
            total_cost: 4,
        },
        MovementPreviewCase {
            name: "第一段踩尖刺，不改走第二段安全繞路",
            movement: 2,
            interrupted: true,
            path_length: 2,
            second_path_length: 0,
            total_cost: 1,
        },
        MovementPreviewCase {
            name: "兩段仍無法安全繞路時停在尖刺",
            movement: 1,
            interrupted: true,
            path_length: 2,
            second_path_length: 0,
            total_cost: 1,
        },
    ];

    for MovementPreviewCase {
        name,
        movement,
        interrupted: expected_interrupted,
        path_length,
        second_path_length,
        total_cost: expected_total_cost,
    } in cases
    {
        let (game, actor_position, spikes, destination) = movement_preview_game(movement);
        let MovePreview {
            first,
            second,
            interrupted,
            total_cost,
        } = game
            .preview_move(ACTOR_ID, destination)
            .expect("目的地應能在第一階段朝目標移動");
        let expected_last = if expected_interrupted {
            spikes
        } else {
            destination
        };

        assert_eq!(interrupted, expected_interrupted, "{}", name);
        assert_eq!(first.first(), Some(&actor_position), "{}", name);
        assert_eq!(
            second.last().or_else(|| first.last()),
            Some(&expected_last),
            "{}",
            name
        );
        assert_eq!(first.len(), path_length, "{}", name);
        assert_eq!(first.contains(&spikes), expected_interrupted, "{}", name);
        assert_eq!(total_cost, expected_total_cost, "{}", name);
        assert_eq!(second.len(), second_path_length, "{}", name);
    }
}

// 驗證大型單位開場已踩中尖刺時首次移動仍扣血，後續持續路過時也再次扣血。
#[test]
fn large_unit_movement_damages_existing_terrain_coverage() {
    // 單位、地形與落點分層畫在同尺寸棋盤，讓尖刺可與大型單位重疊；路線為 A → T → A。
    let units = "
        AA..
        AA..
    ";
    let destination_layout = ascii_board(
        "
        .T..
        ....
    ",
    );
    let destination_board = destination_layout.board;
    let next = marked_cell(&destination_layout, 'T');
    for (name, terrain_diagram) in [
        (
            "左上角所在列",
            "
            .^..
            ....
        ",
        ),
        (
            "其他佔用列",
            "
            ....
            .^..
        ",
        ),
    ] {
        let unit_layout = ascii_board(units);
        let start = marked_cell(&unit_layout, 'A');
        let AsciiBoard {
            board: unit_board,
            markers,
            terrains: _,
        } = unit_layout;
        let AsciiBoard {
            board,
            markers: _,
            terrains,
        } = ascii_board(terrain_diagram);
        assert_eq!(unit_board, board, "單位圖與地形圖尺寸應相同");
        assert_eq!(unit_board, destination_board, "單位圖與落點圖尺寸應相同");
        let mut game = movement_game(
            4,
            &AsciiBoard {
                board: unit_board,
                markers,
                terrains,
            },
        );
        for (destination, expected_hp) in [(next, 97), (start, 94)] {
            let snapshot = game
                .command(Command::Move {
                    actor: ACTOR_ID,
                    x: destination.x,
                    y: destination.y,
                })
                .expect("大型單位應可移動至指定格");
            assert_eq!(unit(&snapshot, ACTOR_ID).hp, expected_hp, "{name}");
            assert_eq!(snapshot.log.len(), 1, "{name}：每次移動應回傳一次地形傷害");
            assert_eq!(
                (unit(&snapshot, ACTOR_ID).x, unit(&snapshot, ACTOR_ID).y),
                (destination.x, destination.y),
                "{name}"
            );
        }
    }
}

// 驗證第二段分次移動會扣除剩餘額度，範圍、預覽與執行均不可重新取得完整預算。
#[test]
fn second_movement_segment_preserves_remaining_budget() {
    let layout = ascii_board("A....");
    let mut game = movement_game(3, &layout);
    game.command(Command::Move {
        actor: ACTOR_ID,
        x: 4,
        y: 0,
    })
    .expect("移動四格應跨入第二段並剩餘兩格");
    let snapshot = game
        .command(Command::Move {
            actor: ACTOR_ID,
            x: 3,
            y: 0,
        })
        .expect("第二段應可移動一格");
    let beyond_budget = GridPos { x: 1, y: 0 };
    assert_eq!(snapshot.turn.move_remaining, 1);
    assert_eq!(snapshot.turn.can_move, true);
    assert_eq!(snapshot.turn.can_skill, false);
    assert_eq!(snapshot.second_reachable.contains(&beyond_budget), false);
    assert_eq!(game.preview_move(ACTOR_ID, beyond_budget).is_err(), true);
    assert_eq!(
        game.command(Command::Move {
            actor: ACTOR_ID,
            x: beyond_budget.x,
            y: beyond_budget.y
        })
        .is_err(),
        true
    );
}

// 驗證第一段剛好用完後可開始第二段，並依第二段消耗決定剩餘額度及技能狀態。
#[test]
fn second_movement_segment_starts_after_first_is_exhausted() {
    for (destination_x, remaining, expected_phase) in
        [(4, 2, Phase::Moving), (0, 0, Phase::AfterMove)]
    {
        let layout = ascii_board("A....");
        let mut game = movement_game(3, &layout);
        let snapshot = game
            .command(Command::Move {
                actor: ACTOR_ID,
                x: 3,
                y: 0,
            })
            .expect("第一段應可剛好用完");
        assert_eq!(snapshot.turn.can_skill, true);
        assert_eq!(snapshot.turn.can_move, true);
        let destination = GridPos {
            x: destination_x,
            y: 0,
        };
        assert_eq!(snapshot.second_reachable.contains(&destination), true);
        game.preview_move(ACTOR_ID, destination)
            .expect("第二段應可預覽");
        let snapshot = game
            .command(Command::Move {
                actor: ACTOR_ID,
                x: destination.x,
                y: destination.y,
            })
            .expect("第一段用完後仍應可執行第二段移動");
        assert_eq!(snapshot.turn.move_remaining, remaining);
        assert_eq!(snapshot.turn.phase, expected_phase);
        assert_eq!(snapshot.turn.can_move, remaining > 0);
        assert_eq!(snapshot.turn.can_skill, false);
    }
}

fn movement_preview_game(movement: u32) -> (Game, GridPos, GridPos, GridPos) {
    let layout = ascii_board(
        "
        ...
        A^T
        ...
    ",
    );
    let actor_position = marked_cell(&layout, 'A');
    let destination = marked_cell(&layout, 'T');
    // 尖刺位置取自棋盤的 `^`，地形欄位不公開，透過公開序列化格式讀取。
    let spikes: GridPos = serde_json::from_value(
        serde_json::to_value(&layout.terrains[0]).expect("地形配置應可序列化"),
    )
    .expect("地形配置應含座標");
    let game = movement_game(movement, &layout);
    (game, actor_position, spikes, destination)
}

fn movement_game(movement: u32, layout: &AsciiBoard) -> Game {
    let AsciiBoard {
        board,
        markers,
        terrains,
    } = layout;
    let (_, actor_size) = *markers.get(&'A').expect("棋盤應包含玩家");
    let (mut definitions, mut map) = skill_test_data(
        0,
        1,
        ActiveSkillEffect::Attack {
            power_source: PowerSource::Physical,
            ranged: false,
            hit_bonus: 100,
            power_bonus: 0,
        },
        Team::Enemy("test_enemy".into()),
        "AT",
    );
    let actor = &mut definitions.unit_types[0];
    actor.width = actor_size.0;
    actor.height = actor_size.1;
    actor.movement = movement;
    definitions.terrain_types.push(
        serde_json::from_value::<TerrainTypeDef>(serde_json::json!({
            "id": "spikes",
            "layer": TerrainLayer::Overlay,
            "entry_rule": TerrainEntryRule::Walkable,
            "damage": 3,
            "blocks_sight": false,
            "extra_movement_cost": 0,
            "dodge_penalty": 0,
            "block_penalty": 0
        }))
        .expect("測試地形資料應可載入"),
    );
    let GridPos { x, y } = marked_cell(layout, 'A');
    map.width = board.0 + 1;
    map.height = board.1;
    map.terrains = terrains.clone();
    map.units[0].x = x;
    map.units[0].y = y;
    map.units[1].id = TARGET_ID;
    map.units[1].x = board.0;
    map.units[1].y = 0;
    let mut game = Game::from_authoring(definitions, map, game_core::authoring::Difficulty::Easy)
        .expect("移動測試定義應有效");
    game.set_random_seed(1);
    game.command(Command::Start).expect("移動測試應可開始");
    game
}

// 驗證藉機攻擊可設定每輪零至多次反應，耗盡後不再觸發，並於下一輪重置。
#[test]
fn opportunity_attack_uses_configured_reactions_and_resets_each_round() {
    use crate::support::{add_passive_skill, end_turn_and_continue};
    use game_core::{CombatLogEvent, PassiveSkillEffect, authoring};
    for quota in [0, 1, 2] {
        let (mut definitions, map) = skill_test_data(
            1,
            1,
            ActiveSkillEffect::Attack {
                power_source: PowerSource::Physical,
                ranged: false,
                hit_bonus: 0,
                power_bonus: 0,
            },
            Team::Enemy("test_enemy".into()),
            "
        TA.....
        ......B
        ",
        );
        definitions.unit_types[0].movement = 6;
        definitions.unit_types[1].physical_power = 3;
        definitions.unit_types[1].hit = 100;
        add_passive_skill(
            &mut definitions,
            "test_target",
            "opportunity",
            &[PassiveSkillEffect::OpportunityAttack {
                reactions_per_round: quota,
            }],
        );
        let mut game = Game::from_authoring(definitions, map, authoring::Difficulty::Easy)
            .expect("反應次數資料應有效");
        game.command(Command::Start).expect("戰鬥應可開始");
        let mut hits = 0;
        for x in [3, 1, 3, 1, 3, 1] {
            game.set_random_seed(1);
            let snapshot = game
                .command(Command::Move {
                    actor: ACTOR_ID,
                    x,
                    y: 0,
                })
                .expect("移動應成功");
            hits += snapshot.log.iter().filter(|event| matches!(event, CombatLogEvent::Skill { skill, .. } if skill == "opportunity")).count();
            assert_eq!(unit(&snapshot, ACTOR_ID).x, x);
        }
        assert_eq!(hits, quota as usize);
        let mut next = end_turn_and_continue(&mut game);
        for _ in 0..3 {
            if next.turn.actor == Some(ACTOR_ID) {
                break;
            }
            next = end_turn_and_continue(&mut game);
        }
        assert_eq!((next.round, next.turn.actor), (2, Some(ACTOR_ID)));
        game.set_random_seed(1);
        let snapshot = game
            .command(Command::Move {
                actor: ACTOR_ID,
                x: 3,
                y: 0,
            })
            .expect("下一輪應可移動");
        assert_eq!(snapshot.log.iter().filter(|event| matches!(event, CombatLogEvent::Skill { skill, .. } if skill == "opportunity")).count(), usize::from(quota > 0));
    }
}

// 驗證逐格離開敵方攻擊範圍才觸發藉機攻擊；繞出再回來仍觸發，友軍不觸發，被擊倒則停止移動。
#[test]
fn opportunity_attack_respects_reach_alliances_and_lethal_interruptions() {
    use crate::support::add_passive_skill;
    use game_core::{CombatLogEvent, PassiveSkillEffect, authoring};
    let straight_path = "
        TA.....
        ......B
        ";
    for (name, diagram, reach, allied, hp, (x, y), expected_hits, survives) in [
        (
            "範圍內移動",
            straight_path,
            true,
            false,
            100,
            (2, 0),
            0,
            true,
        ),
        (
            "跨出長兵器範圍",
            straight_path,
            true,
            false,
            100,
            (3, 0),
            1,
            true,
        ),
        (
            "友軍不觸發",
            straight_path,
            false,
            true,
            100,
            (3, 0),
            0,
            true,
        ),
        (
            "擊倒前不離開原格",
            straight_path,
            false,
            false,
            3,
            (3, 0),
            1,
            false,
        ),
        // A 必須從右側繞過牆，途中離開 T 的兩格攻擊範圍，再回到 T 下方。
        (
            "起終點都在範圍內，途中繞出仍觸發",
            "
        TA.
        .#.
        ...
        ",
            true,
            false,
            100,
            (0, 1),
            1,
            true,
        ),
    ] {
        let (mut definitions, mut map) = skill_test_data(
            1,
            1,
            ActiveSkillEffect::Attack {
                power_source: PowerSource::Physical,
                ranged: false,
                hit_bonus: 0,
                power_bonus: 0,
            },
            Team::Enemy("test_enemy".into()),
            diagram,
        );
        definitions.unit_types[0].movement = 6;
        definitions.unit_types[0].hp = hp;
        definitions.unit_types[1].physical_power = 3;
        definitions.unit_types[1].hit = 100;
        if allied {
            map.units
                .iter_mut()
                .find(|unit| unit.id == TARGET_ID)
                .expect("應有反應者")
                .team = Team::Player;
        }
        let mut effects = vec![PassiveSkillEffect::OpportunityAttack {
            reactions_per_round: 1,
        }];
        if reach {
            effects.push(PassiveSkillEffect::MeleeReach);
        }
        add_passive_skill(&mut definitions, "test_target", "opportunity", &effects);
        let mut game = Game::from_authoring(definitions, map, authoring::Difficulty::Easy)
            .expect("反應範圍資料應有效");
        game.command(Command::Start).expect("戰鬥應可開始");
        game.set_random_seed(1);
        let snapshot = game
            .command(Command::Move {
                actor: ACTOR_ID,
                x,
                y,
            })
            .expect("應能結算移動與反應");
        assert_eq!(snapshot.log.iter().filter(|event| matches!(event, CombatLogEvent::Skill { skill, .. } if skill == "opportunity")).count(), expected_hits, "{name}");
        assert_eq!(
            snapshot.units.iter().any(|unit| unit.id == ACTOR_ID),
            survives,
            "{name}"
        );
        if survives {
            assert_eq!(
                (unit(&snapshot, ACTOR_ID).x, unit(&snapshot, ACTOR_ID).y),
                (x, y),
                "{name}"
            );
        } else {
            assert_eq!(snapshot.movements.len(), 0, "{name}");
        }
    }
}

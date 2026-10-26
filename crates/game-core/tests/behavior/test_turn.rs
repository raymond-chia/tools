use crate::support::{
    ACTOR_ID, BLOCKER_ID, TARGET_ID, TEST_SKILL_ID, ascii_board, marked_cell, skill_test_data,
};
use game_core::{ActiveSkillEffect, Command, Game, Phase, PowerSource, Team};

const BOARD: &str = "
    A......TB
";

enum Action {
    Move(i32),
    Skill,
    End,
    Delay,
}

struct TurnStageCase {
    name: &'static str,
    setup: Vec<Action>,
    phase: Phase,
    remaining: u32,
    destination: i32,
    can_move: bool,
    can_skill: bool,
    can_delay: bool,
}

fn turn_game() -> Game {
    let (mut definitions, map) = skill_test_data(
        1,
        8,
        ActiveSkillEffect::Attack {
            power_source: PowerSource::Physical,
            ranged: true,
            hit_bonus: 100,
            power_bonus: 0,
        },
        Team::Enemy("test_enemy".into()),
        BOARD,
    );
    for (unit, initiative) in definitions.unit_types.iter_mut().zip([100, 50, 0]) {
        unit.initiative = initiative;
        unit.movement = 3;
    }
    let mut game = Game::from_authoring(definitions, map, game_core::authoring::Difficulty::Easy)
        .expect("回合測試資料應有效");
    game.set_random_seed(1);
    game.command(Command::Start).expect("戰鬥應可開始");
    assert_eq!(game.snapshot(None).turn.actor, Some(ACTOR_ID));
    game
}

fn action_command(action: &Action) -> Command {
    match action {
        Action::Move(x) => Command::Move {
            actor: ACTOR_ID,
            x: *x,
            y: 0,
        },
        Action::Skill => {
            let target = marked_cell(&ascii_board(BOARD), 'T');
            Command::Skill {
                actor: ACTOR_ID,
                x: target.x,
                y: target.y,
                skill: TEST_SKILL_ID.into(),
            }
        }
        Action::End => Command::EndTurn { actor: ACTOR_ID },
        Action::Delay => Command::Delay {
            actor: ACTOR_ID,
            after: TARGET_ID,
        },
    }
}

// 驗證各階段可否移動、施放技能及延後：旗標、預覽與命令一致，拒絕操作不改變狀態。
#[test]
fn turn_stages_agree_with_available_actions() {
    let cases = [
        TurnStageCase {
            name: "尚未行動",
            setup: vec![],
            phase: Phase::Ready,
            remaining: 3,
            destination: 2,
            can_move: true,
            can_skill: true,
            can_delay: true,
        },
        TurnStageCase {
            name: "第一段未用完",
            setup: vec![Action::Move(1)],
            phase: Phase::Moving,
            remaining: 2,
            destination: 2,
            can_move: true,
            can_skill: true,
            can_delay: false,
        },
        TurnStageCase {
            name: "第一段用完",
            setup: vec![Action::Move(3)],
            phase: Phase::AfterMove,
            remaining: 0,
            destination: 4,
            can_move: true,
            can_skill: true,
            can_delay: false,
        },
        TurnStageCase {
            name: "第二段未用完",
            setup: vec![Action::Move(4)],
            phase: Phase::Moving,
            remaining: 2,
            destination: 5,
            can_move: true,
            can_skill: false,
            can_delay: false,
        },
        TurnStageCase {
            name: "第二段用完",
            setup: vec![Action::Move(6)],
            phase: Phase::AfterMove,
            remaining: 0,
            destination: 5,
            can_move: false,
            can_skill: false,
            can_delay: false,
        },
        TurnStageCase {
            name: "施放技能後",
            setup: vec![Action::Skill],
            phase: Phase::Ended,
            remaining: 3,
            destination: 2,
            can_move: false,
            can_skill: false,
            can_delay: false,
        },
        TurnStageCase {
            name: "主動結束後",
            setup: vec![Action::End],
            phase: Phase::Ended,
            remaining: 3,
            destination: 2,
            can_move: false,
            can_skill: false,
            can_delay: false,
        },
    ];
    for TurnStageCase {
        name,
        setup,
        phase,
        remaining,
        destination,
        can_move,
        can_skill,
        can_delay,
    } in cases
    {
        for (action, allowed, error_id, operation) in [
            (Action::Move(destination), can_move, "cannot_move", "移動"),
            (Action::Skill, can_skill, "cannot_use_skill", "施放技能"),
            (Action::Delay, can_delay, "delay_after_action", "延後"),
        ] {
            let mut game = turn_game();
            for action in &setup {
                game.command(action_command(action))
                    .expect("準備行動應成功");
            }
            let before = game.snapshot(None);
            assert_eq!(before.turn.phase, phase, "{name}");
            assert_eq!(before.turn.move_remaining, remaining, "{name}");
            assert_eq!(before.turn.can_move, can_move, "{name}：可否移動");
            assert_eq!(before.turn.can_skill, can_skill, "{name}：可否施放技能");
            assert_eq!(before.turn.can_delay, can_delay, "{name}：可否延後");
            match &action {
                Action::Move(x) => assert_eq!(
                    game.preview_move(ACTOR_ID, game_core::GridPos { x: *x, y: 0 })
                        .is_ok(),
                    can_move,
                    "{name}：移動預覽"
                ),
                Action::Skill => assert_eq!(
                    game.preview_skill(
                        ACTOR_ID,
                        marked_cell(&ascii_board(BOARD), 'T'),
                        TEST_SKILL_ID
                    )
                    .is_ok(),
                    can_skill,
                    "{name}：技能預覽"
                ),
                Action::Delay | Action::End => {}
            }
            let result = game.command(action_command(&action));
            assert_eq!(result.is_ok(), allowed, "{name}：{operation}");
            if !allowed {
                assert_eq!(
                    result.err().expect("禁止的行動應回傳錯誤").id(),
                    error_id,
                    "{name}：{operation}"
                );
                assert_eq!(
                    serde_json::to_value(game.snapshot(None)).expect("快照應可序列化"),
                    serde_json::to_value(before).expect("快照應可序列化"),
                    "{name}：拒絕{operation}不得改變狀態"
                );
            }
        }
    }
}

// 驗證延後會插到指定單位之後，其他單位順序不變，再輪到自己時仍保有完整行動。
#[test]
fn delay_reorders_turn_without_spending_actions() {
    for (after, expected) in [
        (TARGET_ID, vec![TARGET_ID, ACTOR_ID, BLOCKER_ID]),
        (BLOCKER_ID, vec![TARGET_ID, BLOCKER_ID, ACTOR_ID]),
    ] {
        let mut game = turn_game();
        let snapshot = game
            .command(Command::Delay {
                actor: ACTOR_ID,
                after,
            })
            .expect("尚未行動時應可延後");
        let order: Vec<_> = snapshot
            .turn
            .actor
            .into_iter()
            .chain(snapshot.turn_order)
            .collect();
        assert_eq!(order, expected);
        for id in expected.into_iter().take_while(|id| *id != ACTOR_ID) {
            game.command(Command::EndTurn { actor: id })
                .expect("應可結束其他單位回合");
            game.command(Command::Continue).expect("應可推進至下一單位");
        }
        let snapshot = game.snapshot(None);
        assert_eq!(snapshot.turn.actor, Some(ACTOR_ID));
        assert_eq!(snapshot.turn.phase, Phase::Ready);
        assert_eq!(snapshot.turn.move_remaining, 3);
        assert_eq!(
            (snapshot.turn.can_move, snapshot.turn.can_skill),
            (true, true)
        );
    }
}

// 驗證延後到自己或不存在的單位會被拒絕，且不改變行動順序或回合狀態。
#[test]
fn invalid_delay_target_preserves_turn() {
    for (after, error_id) in [
        (ACTOR_ID, "invalid_delay_target"),
        (999, "missing_delay_target"),
    ] {
        let mut game = turn_game();
        let before = serde_json::to_value(game.snapshot(None)).expect("快照應可序列化");
        let error = game
            .command(Command::Delay {
                actor: ACTOR_ID,
                after,
            })
            .err()
            .expect("無效延後目標應被拒絕");
        assert_eq!(error.id(), error_id);
        assert_eq!(
            serde_json::to_value(game.snapshot(None)).expect("快照應可序列化"),
            before
        );
    }
}

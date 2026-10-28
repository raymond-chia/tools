use crate::support::{
    ACTOR_ID, BLOCKER_ID, TARGET_ID, TEST_SKILL_ID, ascii_board, end_turn_and_continue,
    marked_cell, skill_test_data, unit,
};
use game_core::{
    ActiveSkillEffect, ActiveSkillPreview, BattleMode, Command, Game, GridPos, Phase, PowerSource,
    Snapshot, Team, authoring,
};

const STEP_THROUGH_THE_SEDGE_ID: &str = "test_step_through_the_sedge";

// 驗證踏莎行可抵達視線內一至三格的空格（含斜向），可穿過敵我單位，抵達後才等待出刀。
#[test]
fn step_through_the_sedge_moves_before_attacking_and_passes_through_units() {
    for (name, diagram, blocker_team) in [
        (
            "水平三格穿過敵人",
            "
            AB.DT
        ",
            Team::Enemy("test_enemy".into()),
        ),
        (
            "水平三格穿過隊友",
            "
            AB.DT
        ",
            Team::Player,
        ),
        (
            "垂直三格",
            "
            A.
            ..
            ..
            DT
        ",
            Team::Enemy("test_enemy".into()),
        ),
        (
            "斜向兩格",
            "
            A...
            .D.T
        ",
            Team::Enemy("test_enemy".into()),
        ),
        (
            "斜向三格",
            "
            A...
            ..DT
        ",
            Team::Enemy("test_enemy".into()),
        ),
        (
            "水平一格",
            "
            ADT
        ",
            Team::Enemy("test_enemy".into()),
        ),
    ] {
        let mut game = step_through_the_sedge_game(diagram, blocker_team);
        let before = game.command(Command::Start).expect("測試戰鬥應可開始");
        let destination = marked_cell(&ascii_board(diagram), 'D');
        let after = game
            .command(skill_at(STEP_THROUGH_THE_SEDGE_ID, destination))
            .expect("踏莎行應能抵達視線內三格以內的合法空格");
        assert_eq!(position(&after, ACTOR_ID), destination, "{name}");
        assert_eq!(
            unit(&after, TARGET_ID).hp,
            unit(&before, TARGET_ID).hp,
            "{name}"
        );
        assert_eq!(after.log.len(), 0, "尚未出刀，不應產生攻擊日誌：{name}");
        assert_eq!(after.turn.actor, Some(ACTOR_ID), "{name}");
        assert_eq!(after.turn.can_move, false, "技能開始後不能普通移動：{name}");
        assert_eq!(after.turn.can_end_turn, true, "應可放棄出刀：{name}");
        if diagram.contains('B') {
            assert_eq!(
                position(&after, BLOCKER_ID),
                position(&before, BLOCKER_ID),
                "{name}"
            );
            assert_eq!(
                unit(&after, BLOCKER_ID).hp,
                unit(&before, BLOCKER_ID).hp,
                "{name}"
            );
        }
    }
}

// 驗證視線被牆遮擋、有人占用的落點、超出三格（含斜向）、原地及越界都被拒絕，且戰局狀態不變。
#[test]
fn invalid_step_through_the_sedge_destination_does_not_spend_action() {
    for (name, diagram, destination) in [
        (
            "不能穿牆",
            "
            A#.DT
        ",
            GridPos { x: 3, y: 0 },
        ),
        (
            "落點不能有人",
            "
            A.B.T
        ",
            GridPos { x: 2, y: 0 },
        ),
        (
            "不能超過三格",
            "
            A...DT
        ",
            GridPos { x: 4, y: 0 },
        ),
        (
            "斜向不能超過三格",
            "
            A..T
            ....
            ..D.
        ",
            GridPos { x: 2, y: 2 },
        ),
        (
            "斜向視線不能被牆擋住",
            "
            A...
            .#DT
        ",
            GridPos { x: 2, y: 1 },
        ),
        (
            "不能原地施放",
            "
            A..T
        ",
            GridPos { x: 0, y: 0 },
        ),
        (
            "不能越界",
            "
            A..T
        ",
            GridPos { x: -1, y: 0 },
        ),
    ] {
        let mut game = step_through_the_sedge_game(diagram, Team::Enemy("test_enemy".into()));
        let before = game.command(Command::Start).expect("測試戰鬥應可開始");
        assert_eq!(
            game.command(skill_at(STEP_THROUGH_THE_SEDGE_ID, destination))
                .is_err(),
            true,
            "{name}"
        );
        assert_eq!(
            snapshot_value(&game.snapshot(None)),
            snapshot_value(&before),
            "{name}"
        );
    }
}

// 驗證踏莎行後只能接續該技能的近戰攻擊，普通移動、其他技能及遠處目標都不能改變等待狀態。
#[test]
fn step_through_the_sedge_pending_attack_rejects_other_actions() {
    let diagram = "
        A..D.T
        ......
    ";
    let mut game = step_through_the_sedge_game(diagram, Team::Enemy("test_enemy".into()));
    game.command(Command::Start).expect("測試戰鬥應可開始");
    let pending = game
        .command(skill_at(
            STEP_THROUGH_THE_SEDGE_ID,
            marked_cell(&ascii_board(diagram), 'D'),
        ))
        .expect("踏莎行應先完成移動");
    let destination = position(&pending, ACTOR_ID);
    let target = position(&pending, TARGET_ID);
    for command in [
        Command::Move {
            actor: ACTOR_ID,
            x: destination.x,
            y: 1,
        },
        skill_at(
            TEST_SKILL_ID,
            GridPos {
                x: destination.x + 1,
                y: 0,
            },
        ),
        skill_at(STEP_THROUGH_THE_SEDGE_ID, target),
        skill_at(
            STEP_THROUGH_THE_SEDGE_ID,
            GridPos {
                x: destination.x + 1,
                y: 0,
            },
        ),
    ] {
        assert_eq!(game.command(command).is_err(), true);
        assert_eq!(
            snapshot_value(&game.snapshot(None)),
            snapshot_value(&pending)
        );
    }
}

// 驗證出刀從落點計算距離且只攻擊一次，傷害與同位置、同骰序的普通近戰完全一致。
#[test]
fn step_through_the_sedge_follow_up_matches_normal_melee_attack() {
    let diagram = "
        A..DT
    ";
    let mut game = step_through_the_sedge_game(diagram, Team::Enemy("test_enemy".into()));
    game.command(Command::Start).expect("測試戰鬥應可開始");
    let pending = game
        .command(skill_at(
            STEP_THROUGH_THE_SEDGE_ID,
            marked_cell(&ascii_board(diagram), 'D'),
        ))
        .expect("踏莎行應先抵達落點");
    let target = position(&pending, TARGET_ID);
    let mut normal = melee_game_at_destination(diagram);
    normal
        .command(Command::Start)
        .expect("普通近戰對照戰鬥應可開始");
    game.set_random_seed(1);
    normal.set_random_seed(1);
    let normal_after = normal
        .command(skill_at(TEST_SKILL_ID, target))
        .expect("普通近戰應能攻擊相鄰敵人");
    let after = game
        .command(skill_at(STEP_THROUGH_THE_SEDGE_ID, target))
        .expect("踏莎行後應能攻擊落點旁的敵人");
    assert_eq!(
        unit(&after, TARGET_ID).hp,
        unit(&normal_after, TARGET_ID).hp
    );
    assert_eq!(after.log.len(), pending.log.len() + 1);
    assert_eq!(after.turn.phase, Phase::Ended);
    assert_eq!((after.turn.can_move, after.turn.can_skill), (false, false));
    assert_eq!(
        game.command(skill_at(STEP_THROUGH_THE_SEDGE_ID, target))
            .is_err(),
        true
    );
}

// 驗證踏莎行後可不出刀直接結束回合，保留落點且敵人不受傷，也不能再移動或施放技能。
#[test]
fn step_through_the_sedge_can_end_turn_without_attacking() {
    let diagram = "
        A..DT
    ";
    let mut game = step_through_the_sedge_game(diagram, Team::Enemy("test_enemy".into()));
    game.command(Command::Start).expect("測試戰鬥應可開始");
    let pending = game
        .command(skill_at(
            STEP_THROUGH_THE_SEDGE_ID,
            marked_cell(&ascii_board(diagram), 'D'),
        ))
        .expect("踏莎行應完成移動");
    let after = game
        .command(Command::EndTurn { actor: ACTOR_ID })
        .expect("應可放棄出刀並結束回合");
    assert_eq!(position(&after, ACTOR_ID), position(&pending, ACTOR_ID));
    assert_eq!(unit(&after, TARGET_ID).hp, unit(&pending, TARGET_ID).hp);
    assert_eq!(after.log.len(), pending.log.len());
    assert_eq!(after.turn.phase, Phase::Ended);
    assert_eq!((after.turn.can_move, after.turn.can_skill), (false, false));
    assert_eq!(
        game.command(Command::Move {
            actor: ACTOR_ID,
            x: 2,
            y: 0
        })
        .is_err(),
        true
    );
}

// 驗證落點夾擊提供單體出刀預覽且不改變戰局，實際攻擊採用相同加成。
#[test]
fn step_through_the_sedge_follow_up_uses_landing_position_for_flanking() {
    let diagram = "
        A..D
        ...T
        ...B
    ";
    let mut game = step_through_the_sedge_game(diagram, Team::Player);
    game.command(Command::Start).expect("測試戰鬥應可開始");
    // 從上方落點攻擊目標；支援者站在目標下方。
    let destination = marked_cell(&ascii_board(diagram), 'D');
    game.command(skill_at(STEP_THROUGH_THE_SEDGE_ID, destination))
        .expect("踏莎行應完成移動");
    let before = snapshot_value(&game.snapshot(None));
    let target = position(&game.snapshot(None), TARGET_ID);
    let preview = game
        .preview_skill(ACTOR_ID, target, STEP_THROUGH_THE_SEDGE_ID)
        .expect("踏莎行後應能預覽落點旁的敵人");
    let hit_remaining_hp = match preview {
        ActiveSkillPreview::Attack(preview) => preview.hit_remaining_hp,
        ActiveSkillPreview::Area { .. } | ActiveSkillPreview::Healing(_) => {
            panic!("踏莎行後應產生攻擊預覽")
        }
    };
    assert_eq!(snapshot_value(&game.snapshot(None)), before);
    assert_eq!(hit_remaining_hp, 99);
    game.set_random_seed(1);
    let after = game
        .command(skill_at(STEP_THROUGH_THE_SEDGE_ID, target))
        .expect("踏莎行出刀應成功");
    let log = serde_json::to_value(&after.log).expect("攻擊日誌應可序列化");
    assert_eq!(log[0]["flanking_modifier"], 2);
    assert_eq!(log[0]["skill"], STEP_THROUGH_THE_SEDGE_ID);
}

// 驗證踏莎行後不能攻擊相鄰隊友，拒絕後仍可選擇相鄰敵人出刀。
#[test]
fn step_through_the_sedge_follow_up_cannot_attack_ally() {
    let diagram = "
        A..DT
        ...B.
    ";
    let mut game = step_through_the_sedge_game(diagram, Team::Player);
    game.command(Command::Start).expect("測試戰鬥應可開始");
    game.command(skill_at(
        STEP_THROUGH_THE_SEDGE_ID,
        marked_cell(&ascii_board(diagram), 'D'),
    ))
    .expect("踏莎行應完成移動");
    let before = game.snapshot(None);
    assert_eq!(
        game.command(skill_at(
            STEP_THROUGH_THE_SEDGE_ID,
            position(&before, BLOCKER_ID)
        ))
        .is_err(),
        true
    );
    assert_eq!(
        snapshot_value(&game.snapshot(None)),
        snapshot_value(&before)
    );
    game.command(skill_at(
        STEP_THROUGH_THE_SEDGE_ID,
        position(&before, TARGET_ID),
    ))
    .expect("拒絕砍隊友後仍可攻擊敵人");
}

// 驗證放棄出刀後下一回合可再次踏莎行，沒有冷卻或魔力門檻，也沒有殘留的待出刀狀態。
#[test]
fn step_through_the_sedge_is_available_again_next_turn() {
    let diagram = "
        A..D....T
    ";
    let mut game = step_through_the_sedge_game(diagram, Team::Enemy("test_enemy".into()));
    game.command(Command::Start).expect("測試戰鬥應可開始");
    game.command(skill_at(
        STEP_THROUGH_THE_SEDGE_ID,
        marked_cell(&ascii_board(diagram), 'D'),
    ))
    .expect("沒有相鄰敵人也可踏莎行");
    let mut next = end_turn_and_continue(&mut game);
    for _ in 0..4 {
        if next.turn.actor == Some(ACTOR_ID) && next.turn.phase != Phase::Ended {
            break;
        }
        next = game
            .command(Command::Continue)
            .expect("應可推進至施放者下一回合");
    }
    assert_eq!(next.turn.actor, Some(ACTOR_ID));
    assert_eq!(next.turn.can_skill, true);
    let after = game
        .command(skill_at(STEP_THROUGH_THE_SEDGE_ID, GridPos { x: 0, y: 0 }))
        .expect("下一回合應能再次踏莎行返回起點");
    assert_eq!(position(&after, ACTOR_ID), GridPos { x: 0, y: 0 });
}

// 驗證探索位移觸發遭遇後仍可出刀或放棄，完成行動才開始戰鬥排序。
#[test]
fn exploration_step_finishes_before_combat_starts() {
    for (diagram, max_range, attack) in [
        (
            "
            A..D.......T
            ",
            3,
            false,
        ),
        (
            "
            A.........DT
            ",
            10,
            true,
        ),
    ] {
        let (mut definitions, map) =
            step_through_the_sedge_data(diagram, Team::Enemy("test_enemy".into()));
        let mut skill =
            serde_json::to_value(&definitions.active_skills[1]).expect("測試技能應可序列化");
        skill["max_range"] = max_range.into();
        definitions.active_skills[1] =
            serde_json::from_value(skill).expect("測試技能射程應符合作者格式");
        let mut game = Game::from_authoring(definitions, map, authoring::Difficulty::Easy)
            .expect("探索測試資料應可載入");
        let before = game.command(Command::Start).expect("探索應可開始");
        assert_eq!(before.battle_mode == BattleMode::Exploring, true);
        let pending = game
            .command(skill_at(
                STEP_THROUGH_THE_SEDGE_ID,
                marked_cell(&ascii_board(diagram), 'D'),
            ))
            .expect("探索中應可踏莎行");
        assert_eq!(pending.battle_mode == BattleMode::AttackPending, true);
        assert_eq!(pending.turn.phase, Phase::PendingStrike);
        assert_eq!(pending.turn.actor, Some(ACTOR_ID));
        assert_eq!(pending.round, before.round);
        assert_eq!(pending.log.len(), 0);
        let command = if attack {
            skill_at(STEP_THROUGH_THE_SEDGE_ID, position(&pending, TARGET_ID))
        } else {
            Command::EndTurn { actor: ACTOR_ID }
        };
        let after = game.command(command).expect("完成行動後應可開始戰鬥");
        assert_eq!(after.battle_mode == BattleMode::Combat, true);
        assert_eq!(after.round, before.round + 1);
        assert_eq!(after.turn.actor, Some(ACTOR_ID));
        assert_eq!(after.turn.phase, Phase::Ready);
        assert_eq!(after.skill_ranges.len(), 2);
        let attacks = after
            .log
            .iter()
            .filter(|event| matches!(event, game_core::CombatLogEvent::Skill { .. }))
            .count();
        assert_eq!(attacks, usize::from(attack));
    }
}

// 驗證探索中切換隊友觸發戰鬥後，新回合會清除原角色尚未完成的待出刀狀態。
#[test]
fn combat_start_clears_another_exploration_units_pending_strike() {
    let diagram = "
        AD.............T
        ....B...........
    ";
    let (mut definitions, map) = step_through_the_sedge_data(diagram, Team::Player);
    let blocker = definitions
        .unit_types
        .iter_mut()
        .find(|unit| unit.id == "test_blocker")
        .expect("測試應有隊友種類");
    blocker.movement = 5;
    let mut game = Game::from_authoring(definitions, map, authoring::Difficulty::Easy)
        .expect("切換隊友測試資料應可載入");
    game.command(Command::Start).expect("探索應可開始");
    let pending = game
        .command(skill_at(
            STEP_THROUGH_THE_SEDGE_ID,
            marked_cell(&ascii_board(diagram), 'D'),
        ))
        .expect("原角色應可先位移並等待出刀");
    assert_eq!(pending.battle_mode == BattleMode::Exploring, true);
    assert_eq!(pending.turn.phase, Phase::PendingStrike);
    game.command(Command::SelectUnit { actor: BLOCKER_ID })
        .expect("探索時應可切換隊友");
    let combat = game
        .command(Command::Move {
            actor: BLOCKER_ID,
            x: 6,
            y: 1,
        })
        .expect("隊友接近敵人應觸發戰鬥");
    assert_eq!(combat.battle_mode == BattleMode::Combat, true);
    assert_eq!(combat.turn.actor, Some(ACTOR_ID));
    assert_eq!(combat.turn.phase, Phase::Ready);
    assert_eq!(combat.skill_ranges.len(), 2);
    let after = game
        .command(skill_at(STEP_THROUGH_THE_SEDGE_ID, GridPos { x: 2, y: 0 }))
        .expect("新回合踏莎行應重新從位移開始");
    assert_eq!(position(&after, ACTOR_ID), GridPos { x: 2, y: 0 });
    assert_eq!(after.turn.phase, Phase::PendingStrike);
}

fn step_through_the_sedge_game(diagram: &str, blocker_team: Team) -> Game {
    let (definitions, map) = step_through_the_sedge_data(diagram, blocker_team);
    Game::from_authoring(definitions, map, authoring::Difficulty::Easy)
        .expect("踏莎行專用測試資料應可載入")
}

// 共用踏莎行測試資料，個別案例只調整所需的作者設定。
fn step_through_the_sedge_data(
    diagram: &str,
    blocker_team: Team,
) -> (authoring::Definitions, authoring::Map) {
    let (mut definitions, mut map) = skill_test_data(
        1,
        1,
        melee_effect(),
        Team::Enemy("test_enemy".into()),
        diagram,
    );
    definitions.active_skills.push(
        serde_json::from_value(serde_json::json!({
            "id": STEP_THROUGH_THE_SEDGE_ID,
            "min_range": 1,
            "max_range": 3,
            "effect": "move_then_attack"
        }))
        .expect("核心應支援踏莎行技能的作者格式"),
    );
    definitions.unit_types[0]
        .active_skills
        .push(STEP_THROUGH_THE_SEDGE_ID.into());
    definitions.unit_types[0].movement = 5;
    definitions.passive_skills.push(
        serde_json::from_value(serde_json::json!({
            "id": "test_flanking", "effect": "flanking", "hit_bonus": 2
        }))
        .expect("測試夾擊被動應有效"),
    );
    definitions.unit_types[0]
        .passive_skills
        .push("test_flanking".into());
    if let Some(blocker) = map.units.iter_mut().find(|unit| unit.id == BLOCKER_ID) {
        blocker.team = blocker_team;
    }
    (definitions, map)
}

fn melee_game_at_destination(diagram: &str) -> Game {
    let (definitions, mut map) = skill_test_data(
        1,
        1,
        melee_effect(),
        Team::Enemy("test_enemy".into()),
        diagram,
    );
    let destination = marked_cell(&ascii_board(diagram), 'D');
    let actor = map
        .units
        .iter_mut()
        .find(|unit| unit.id == ACTOR_ID)
        .expect("對照地圖應有施放者");
    actor.x = destination.x;
    actor.y = destination.y;
    Game::from_authoring(definitions, map, game_core::authoring::Difficulty::Easy)
        .expect("普通近戰對照資料應可載入")
}

fn melee_effect() -> ActiveSkillEffect {
    ActiveSkillEffect::Attack {
        power_source: PowerSource::Physical,
        ranged: false,
        hit_bonus: 0,
        power_bonus: 0,
    }
}

fn skill_at(skill: &str, cell: GridPos) -> Command {
    Command::Skill {
        actor: ACTOR_ID,
        x: cell.x,
        y: cell.y,
        skill: skill.into(),
    }
}

fn position(snapshot: &Snapshot, id: i64) -> GridPos {
    let actor = unit(snapshot, id);
    GridPos {
        x: actor.x,
        y: actor.y,
    }
}

fn snapshot_value(snapshot: &Snapshot) -> serde_json::Value {
    let mut value = serde_json::to_value(snapshot).expect("公開快照應可序列化");
    let fields = value.as_object_mut().expect("公開快照應為物件");
    // command 會取走單次呈現事件；比較持續狀態，不要求事件重送。
    for field in ["log", "movements", "presentation_events"] {
        fields.remove(field);
    }
    value
}

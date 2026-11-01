use crate::support::{
    ACTOR_ID, TARGET_ID, ascii_board, end_turn_and_continue, map_from_ascii_board, marked_cell,
};
use authoring::ConditionMode::{All, Any};
use authoring::{ConditionMode, OutcomeCondition, OutcomeGroup};
use game_core::{Command, Game, GridPos, Outcome, Team, authoring};

const DEFINITIONS: &str = include_str!("../data/utility_ai_definitions.toml");
const ALL_ENEMIES_DEFEATED: OutcomeCondition = OutcomeCondition::AllEnemiesDefeated;
const PLAYER_TEAM_DEFEATED: OutcomeCondition =
    OutcomeCondition::TeamDefeated { team: Team::Player };
const DIAGRAM: &str = "
    ...F
    .AT.
    ....
";

const TEAM_MEMBER_ID: i64 = 4;
const SECOND_TEAM_MEMBER_ID: i64 = 5;
const ALLY_ID: i64 = 6;
const CARGO_DIAGRAM: &str = "
    .......
    .BATCUE
    ..D....
";

fn group(mode: ConditionMode, conditions: &[OutcomeCondition]) -> OutcomeGroup {
    OutcomeGroup {
        mode,
        conditions: conditions.to_vec(),
    }
}

// 直接建立作者地圖，保留各案例指定的勝敗條件。
fn map_data(victory: OutcomeGroup, defeat: OutcomeGroup) -> authoring::Map {
    let layout = ascii_board(DIAGRAM);
    let mut map = map_from_ascii_board(
        "outcome_conditions",
        &layout,
        &[
            ('A', ACTOR_ID, "tank", Team::Player),
            (
                'T',
                TARGET_ID,
                "other_tank",
                Team::Enemy("test_enemy".into()),
            ),
        ],
    );
    map.victory = victory;
    map.defeat = defeat;
    map
}

// F 加入與玩家同盟的陣營；同盟單位不算敵人。
fn with_player_ally(mut map: authoring::Map) -> authoring::Map {
    let GridPos { x, y } = marked_cell(&ascii_board(DIAGRAM), 'F');
    let team = Team::Enemy("friends".into());
    map.units.push(authoring::UnitPlacement {
        facing: authoring::Facing::Right,
        id: ALLY_ID,
        min_difficulty: Some(authoring::Difficulty::Easy),
        unit_type: "target".into(),
        team: team.clone(),
        x,
        y,
    });
    map.alliances = vec![authoring::Alliance {
        teams: vec![Team::Player, team],
    }];
    map
}

fn game(map: authoring::Map, attacker: i64) -> Game {
    let mut definitions: authoring::Definitions =
        toml::from_str(DEFINITIONS).expect("測試定義應可載入");
    for unit in &mut definitions.unit_types {
        unit.hp = 1;
        unit.initiative = if (unit.id == "tank" && attacker == ACTOR_ID)
            || (unit.id == "other_tank" && attacker == TARGET_ID)
        {
            1000
        } else {
            0
        };
    }
    let mut game = Game::from_authoring(definitions, map, authoring::Difficulty::Easy)
        .expect("明確指定勝敗條件的地圖應可載入");
    game.set_random_seed(1);
    let snapshot = game.command(Command::Start).expect("戰鬥應可開始");
    assert_eq!(snapshot.turn.actor, Some(attacker));
    assert_eq!(snapshot.outcome, Outcome::Ongoing, "雙方存活，尚無條件成立");
    game
}

fn eliminate_opponent(game: &mut Game, attacker: i64) -> Outcome {
    let layout = ascii_board(DIAGRAM);
    let marker = if attacker == ACTOR_ID { 'T' } else { 'A' };
    let GridPos { x, y } = marked_cell(&layout, marker);
    game.command(Command::Skill {
        actor: attacker,
        x,
        y,
        skill: "melee".into(),
    })
    .expect("相鄰攻擊應擊倒唯一對手")
    .outcome
}

// B、D 屬於指定的 sentries team；T、U 屬於 bandits，T 取貨而 U 持續存活。
fn cargo_outcome_map(conditions: &[OutcomeCondition]) -> authoring::Map {
    let layout = ascii_board(CARGO_DIAGRAM);
    let mut map = map_from_ascii_board(
        "cargo_outcome",
        &layout,
        &[
            ('A', ACTOR_ID, "guard", Team::Player),
            (
                'B',
                TEAM_MEMBER_ID,
                "second_raider",
                Team::Enemy("sentries".into()),
            ),
            ('T', TARGET_ID, "raider", Team::Enemy("bandits".into())),
            ('U', 3, "guard", Team::Enemy("bandits".into())),
            (
                'D',
                SECOND_TEAM_MEMBER_ID,
                "guard",
                Team::Enemy("sentries".into()),
            ),
        ],
    );
    map.cargo = Some(authoring::CargoObjective {
        positions: vec![marked_cell(&layout, 'C')],
        carriers: Vec::new(),
        exits: vec![marked_cell(&layout, 'E')],
    });
    map.victory = group(All, conditions);
    map.defeat = group(Any, &[PLAYER_TEAM_DEFEATED]);
    map
}

// 透過正式文件載入入口驗證條件。
fn load_cargo_outcome(
    map: &authoring::Map,
    difficulty: authoring::Difficulty,
) -> Result<Game, game_core::GameError> {
    Game::from_documents(
        include_str!("../data/cargo_objective_definitions.toml"),
        &toml::to_string(map).expect("測試地圖應可序列化為 TOML"),
        difficulty,
    )
}

fn cargo_outcome_game(conditions: &[OutcomeCondition]) -> Game {
    let mut game = load_cargo_outcome(&cargo_outcome_map(conditions), authoring::Difficulty::Easy)
        .expect("貨物勝敗條件應可載入");
    game.set_random_seed(1);
    game
}

fn units_defeated(unit_ids: &[i64]) -> OutcomeCondition {
    OutcomeCondition::UnitsDefeated {
        unit_ids: unit_ids.to_vec(),
    }
}

fn with_min_difficulty(map: &mut authoring::Map, id: i64, difficulty: authoring::Difficulty) {
    map.units
        .iter_mut()
        .find(|unit| unit.id == id)
        .expect("測試地圖應有指定單位")
        .min_difficulty = Some(difficulty);
}

fn team_defeated_condition() -> OutcomeCondition {
    OutcomeCondition::TeamDefeated {
        team: Team::Enemy("sentries".into()),
    }
}

fn advance_to_guard(game: &mut Game) {
    for _ in 0..8 {
        let snapshot = game.snapshot(None);
        if snapshot.turn.actor == Some(ACTOR_ID) && snapshot.turn.phase != game_core::Phase::Ended {
            return;
        }
        assert_eq!(snapshot.outcome, Outcome::Ongoing);
        end_turn_and_continue(game);
    }
    panic!("護衛應在有限回合內取得行動權");
}

fn defeat_marked_enemy(game: &mut Game, marker: char) -> Outcome {
    advance_to_guard(game);
    let GridPos { x, y } = marked_cell(&ascii_board(CARGO_DIAGRAM), marker);
    game.command(Command::Skill {
        actor: ACTOR_ID,
        x,
        y,
        skill: "strike".into(),
    })
    .expect("護衛應可擊倒相鄰敵人")
    .outcome
}

// 驗證同樣只有一個條件成立時，勝敗兩組分別遵守自己的 all／any。
#[test]
fn partially_satisfied_groups_respect_each_match_mode() {
    for (name, victory, defeat, attacker, expected) in [
        // 敵人全滅但玩家未全滅，勝利 all 尚未滿足，不套用固定勝利。
        (
            "勝利 all 未齊",
            group(All, &[ALL_ENEMIES_DEFEATED, PLAYER_TEAM_DEFEATED]),
            group(Any, &[PLAYER_TEAM_DEFEATED]),
            ACTOR_ID,
            Outcome::Ongoing,
        ),
        // 相同戰況改成勝利 any，任一條件成立即勝利。
        (
            "勝利 any 成立",
            group(Any, &[ALL_ENEMIES_DEFEATED, PLAYER_TEAM_DEFEATED]),
            group(Any, &[PLAYER_TEAM_DEFEATED]),
            ACTOR_ID,
            Outcome::Victory,
        ),
        // 玩家全滅但敵人未全滅，失敗 all 尚未滿足，不套用固定敗北。
        (
            "失敗 all 未齊",
            group(Any, &[ALL_ENEMIES_DEFEATED]),
            group(All, &[PLAYER_TEAM_DEFEATED, ALL_ENEMIES_DEFEATED]),
            TARGET_ID,
            Outcome::Ongoing,
        ),
        // 相同戰況改成失敗 any，任一條件成立即敗北。
        (
            "失敗 any 成立",
            group(Any, &[ALL_ENEMIES_DEFEATED]),
            group(Any, &[PLAYER_TEAM_DEFEATED, ALL_ENEMIES_DEFEATED]),
            TARGET_ID,
            Outcome::Defeat,
        ),
    ] {
        let mut game = game(map_data(victory, defeat), attacker);
        assert_eq!(eliminate_opponent(&mut game, attacker), expected, "{name}");
    }
}

// 驗證貨物離場可設定為勝利或失敗，僅掠奪而尚未離場時兩者都不成立。
#[test]
fn cargo_escape_resolves_the_configured_outcome() {
    let layout = ascii_board(
        "
        U....
        .ACTE
        .....
        ",
    );
    for (name, victory, defeat, expected) in [
        // 貨物離場設定於勝利組，攜貨者離場後應獲勝。
        (
            "貨物離場判勝",
            group(All, &[OutcomeCondition::CargoEscaped]),
            group(Any, &[PLAYER_TEAM_DEFEATED]),
            Outcome::Victory,
        ),
        // 貨物離場設定於失敗組，攜貨者離場後應敗北。
        (
            "貨物離場判敗",
            group(All, &[ALL_ENEMIES_DEFEATED]),
            group(Any, &[OutcomeCondition::CargoEscaped]),
            Outcome::Defeat,
        ),
    ] {
        let mut map = map_from_ascii_board(
            "cargo_escape_outcome",
            &layout,
            &[
                ('A', ACTOR_ID, "guard", Team::Player),
                ('T', TARGET_ID, "raider", Team::Enemy("bandits".into())),
                ('U', 3, "second_raider", Team::Enemy("bandits".into())),
            ],
        );
        let GridPos { x, y } = marked_cell(&layout, 'C');
        let GridPos {
            x: exit_x,
            y: exit_y,
        } = marked_cell(&layout, 'E');
        map.cargo = Some(authoring::CargoObjective {
            carriers: Vec::new(),
            positions: vec![GridPos { x, y }],
            exits: vec![GridPos {
                x: exit_x,
                y: exit_y,
            }],
        });
        map.victory = victory;
        map.defeat = defeat;
        let definitions = toml::from_str(include_str!("../data/cargo_objective_definitions.toml"))
            .expect("測試定義應可載入");
        let mut game = Game::from_authoring(definitions, map, authoring::Difficulty::Easy)
            .expect("貨物勝敗條件地圖應可載入");
        game.set_random_seed(1);
        let snapshot = game.command(Command::Start).expect("戰鬥應可開始");
        assert_eq!(snapshot.turn.actor, Some(TARGET_ID), "{name}");
        assert_eq!(snapshot.outcome, Outcome::Ongoing, "{name}");
        let snapshot = game
            .command(Command::Skill {
                actor: TARGET_ID,
                x,
                y,
                skill: "loot".into(),
            })
            .expect("強盜應可掠奪相鄰貨物");
        assert_eq!(snapshot.outcome, Outcome::Ongoing, "{name}：掠奪後尚未離場");
        for actor in [3, ACTOR_ID, TARGET_ID] {
            let snapshot = end_turn_and_continue(&mut game);
            assert_eq!(snapshot.turn.actor, Some(actor), "{name}");
            assert_eq!(snapshot.outcome, Outcome::Ongoing, "{name}：離場前");
        }
        let snapshot = game
            .command(Command::Move {
                actor: TARGET_ID,
                x: exit_x,
                y: exit_y,
            })
            .expect("攜貨強盜應可抵達出口");
        assert_eq!(snapshot.outcome, expected, "{name}");
    }
}

// 驗證同次結算勝敗皆成立時失敗優先，後續命令也不能覆寫結果。
#[test]
fn defeat_has_priority_when_both_groups_match() {
    for (name, mode) in [("all", All), ("any", Any)] {
        let mut game = game(
            map_data(
                group(mode, &[ALL_ENEMIES_DEFEATED]),
                group(mode, &[ALL_ENEMIES_DEFEATED]),
            ),
            ACTOR_ID,
        );
        assert_eq!(
            eliminate_opponent(&mut game, ACTOR_ID),
            Outcome::Defeat,
            "{name}"
        );
        let snapshot = game
            .command(Command::Continue)
            .expect("戰鬥結束後結果應保持");
        assert_eq!(snapshot.outcome, Outcome::Defeat, "{name}");
    }
}

// 驗證其他 team 的死亡不算，指定 team 須全滅才勝利，其他 team 可繼續存活。
#[test]
fn only_the_specified_teams_elimination_satisfies_the_condition() {
    let mut game = cargo_outcome_game(&[team_defeated_condition()]);
    let snapshot = game.command(Command::Start).expect("戰鬥應可開始");
    assert_eq!(snapshot.outcome, Outcome::Ongoing);
    assert_eq!(defeat_marked_enemy(&mut game, 'T'), Outcome::Ongoing);
    assert_eq!(defeat_marked_enemy(&mut game, 'B'), Outcome::Ongoing);
    assert_eq!(defeat_marked_enemy(&mut game, 'D'), Outcome::Victory);
    assert_eq!(
        game.snapshot(None).units.iter().any(|unit| unit.id == 3),
        true
    );
}

// 驗證單獨要求無人攜貨時，貨物仍在地上且敵人皆存活也會立即勝利。
#[test]
fn uncarried_ground_cargo_satisfies_the_condition_at_start() {
    let mut game = cargo_outcome_game(&[OutcomeCondition::NoCargoCarried]);
    let snapshot = game.command(Command::Start).expect("戰鬥應可開始");
    assert_eq!(snapshot.outcome, Outcome::Victory);
    assert_eq!(snapshot.units.len(), 5);
    assert_eq!(
        snapshot.cargo.expect("地圖應有貨物").items,
        vec![game_core::CargoState::Ground(marked_cell(
            &ascii_board(CARGO_DIAGRAM),
            'C'
        ))],
    );
}

// 驗證指定陣營全滅或指定目標組倒下，搭配無人攜貨時皆須兩條件成立，且不受達成順序影響。
#[test]
fn elimination_and_no_carriers_must_both_be_satisfied() {
    for (target, condition) in [
        // 陣營版本：B、D 都倒下後才完成消滅條件。
        ("指定陣營", team_defeated_condition()),
        // 目標組版本：D 倒下不算，必須 B 倒下才完成消滅條件。
        ("指定目標組", units_defeated(&[TEAM_MEMBER_ID])),
    ] {
        for (order, first, second) in [
            // 消滅目標時仍有攜貨者，最後擊倒攜貨者才勝利。
            ("目標先倒下", 'B', 'T'),
            // 無人攜貨時目標仍存活，最後消滅目標才勝利。
            ("攜貨者先倒", 'T', 'B'),
        ] {
            let name = format!("{target}：{order}");
            let mut game =
                cargo_outcome_game(&[condition.clone(), OutcomeCondition::NoCargoCarried]);
            let snapshot = game.command(Command::Start).expect("戰鬥應可開始");
            assert_eq!(snapshot.outcome, Outcome::Ongoing, "{name}");
            assert_eq!(snapshot.turn.actor, Some(TARGET_ID), "{name}");
            let GridPos { x, y } = marked_cell(&ascii_board(CARGO_DIAGRAM), 'C');
            let snapshot = game
                .command(Command::Skill {
                    actor: TARGET_ID,
                    x,
                    y,
                    skill: "loot".into(),
                })
                .expect("強盜應可取得貨物");
            assert_eq!(snapshot.outcome, Outcome::Ongoing, "{name}");
            assert_eq!(
                snapshot.cargo.expect("地圖應有貨物").items,
                vec![game_core::CargoState::Carried(TARGET_ID)],
                "{name}"
            );
            assert_eq!(
                defeat_marked_enemy(&mut game, 'D'),
                Outcome::Ongoing,
                "{name}"
            );
            assert_eq!(
                defeat_marked_enemy(&mut game, first),
                Outcome::Ongoing,
                "{name}"
            );
            assert_eq!(
                defeat_marked_enemy(&mut game, second),
                Outcome::Victory,
                "{name}"
            );
            let snapshot = game.snapshot(None);
            assert_eq!(
                snapshot.units.iter().any(|unit| unit.id == 3),
                true,
                "{name}"
            );
            assert_eq!(
                snapshot.cargo.expect("地圖應有貨物").items,
                vec![game_core::CargoState::Ground(marked_cell(
                    &ascii_board(CARGO_DIAGRAM),
                    'T'
                ))],
                "{name}"
            );
        }
    }
}

// 驗證敵對陣營全滅即勝利，存活的玩家同盟不算敵人。
#[test]
fn surviving_player_allies_do_not_block_victory() {
    let map = with_player_ally(map_data(
        group(All, &[ALL_ENEMIES_DEFEATED]),
        group(Any, &[PLAYER_TEAM_DEFEATED]),
    ));
    let mut game = game(map, ACTOR_ID);
    assert_eq!(eliminate_opponent(&mut game, ACTOR_ID), Outcome::Victory);
}

// 驗證玩家單位開場已攜貨，無須拾取或擊殺敵人即可送達出口獲勝。
#[test]
fn initially_carried_cargo_can_be_delivered_without_looting() {
    let layout = ascii_board(
        "
        U....
        .A.TE
        .....
        ",
    );
    let mut map = map_from_ascii_board(
        "initially_carried_cargo",
        &layout,
        &[
            ('A', ACTOR_ID, "guard", Team::Player),
            ('T', TARGET_ID, "raider", Team::Player),
            ('U', 3, "second_raider", Team::Enemy("bandits".into())),
        ],
    );
    map.cargo = Some(authoring::CargoObjective {
        positions: Vec::new(),
        carriers: vec![TARGET_ID],
        exits: vec![marked_cell(&layout, 'E')],
    });
    map.victory = group(All, &[OutcomeCondition::CargoEscaped]);
    map.defeat = group(Any, &[units_defeated(&[TARGET_ID])]);
    let document = toml::to_string(&map).expect("測試文件應可轉成 TOML");
    let mut game = Game::from_documents(
        include_str!("../data/cargo_objective_definitions.toml"),
        &document,
        authoring::Difficulty::Easy,
    )
    .expect("開場攜貨地圖應可載入");
    game.set_random_seed(1);
    let snapshot = game.command(Command::Start).expect("護送應可開始");
    assert_eq!(snapshot.turn.actor, Some(TARGET_ID));
    assert_eq!(snapshot.outcome, Outcome::Ongoing);
    assert_eq!(
        snapshot.cargo.expect("地圖應有貨物").items,
        vec![game_core::CargoState::Carried(TARGET_ID)]
    );
    let GridPos { x, y } = marked_cell(&layout, 'E');
    let snapshot = game
        .command(Command::Move {
            actor: TARGET_ID,
            x,
            y,
        })
        .expect("玩家應可直接將貨物送達出口");
    assert_eq!(snapshot.outcome, Outcome::Victory);
    assert_eq!(
        snapshot.cargo.expect("地圖應有貨物").items,
        vec![game_core::CargoState::Escaped]
    );
    assert_eq!(
        snapshot
            .units
            .iter()
            .map(|unit| unit.id)
            .collect::<std::collections::HashSet<_>>(),
        [ACTOR_ID, 3].into_iter().collect(),
        "攜貨者離場，其餘單位全部存活"
    );
}

// 驗證目標組只要求本難度有出場的目標倒下，未出場的目標不阻擋勝利。
#[test]
fn target_group_only_requires_present_targets() {
    for (difficulty, steps) in [
        // 簡單：D 未出場，擊倒 B 即勝利。
        (authoring::Difficulty::Easy, vec![('B', Outcome::Victory)]),
        // 困難：D 出場，擊倒 B 後仍須擊倒 D 才勝利。
        (
            authoring::Difficulty::Hard,
            vec![('B', Outcome::Ongoing), ('D', Outcome::Victory)],
        ),
    ] {
        let mut map =
            cargo_outcome_map(&[units_defeated(&[TEAM_MEMBER_ID, SECOND_TEAM_MEMBER_ID])]);
        with_min_difficulty(&mut map, SECOND_TEAM_MEMBER_ID, authoring::Difficulty::Hard);
        let mut game = load_cargo_outcome(&map, difficulty).expect("依難度增減目標應可載入");
        // 種子 2 讓兩個難度的攻擊都不會擲出必定閃避的 1。
        game.set_random_seed(2);
        let snapshot = game.command(Command::Start).expect("戰鬥應可開始");
        assert_eq!(snapshot.outcome, Outcome::Ongoing, "{difficulty:?}");
        for (marker, expected) in steps {
            assert_eq!(
                defeat_marked_enemy(&mut game, marker),
                expected,
                "{difficulty:?}：擊倒 {marker}"
            );
        }
    }
}

// 驗證未出場的開場攜貨者不會帶貨進場，出場時則開場即攜貨。
#[test]
fn initial_carriers_follow_difficulty() {
    let ground = game_core::CargoState::Ground(marked_cell(&ascii_board(CARGO_DIAGRAM), 'C'));
    for (difficulty, expected) in [
        // 簡單：U 未出場，只有地上貨物。
        (authoring::Difficulty::Easy, vec![ground.clone()]),
        // 困難：U 出場，除地上貨物外另帶一份。
        (
            authoring::Difficulty::Hard,
            vec![ground.clone(), game_core::CargoState::Carried(3)],
        ),
    ] {
        let mut map = cargo_outcome_map(&[team_defeated_condition()]);
        with_min_difficulty(&mut map, 3, authoring::Difficulty::Hard);
        map.cargo.as_mut().expect("測試地圖應有貨物設定").carriers = vec![3];
        let mut game = load_cargo_outcome(&map, difficulty).expect("依難度增減攜貨者應可載入");
        let snapshot = game.command(Command::Start).expect("戰鬥應可開始");
        assert_eq!(
            snapshot.cargo.expect("地圖應有貨物設定").items,
            expected,
            "{difficulty:?}"
        );
    }
}

// 驗證目標組與開場攜貨的作者設定錯誤會被拒絕，並指出是哪一類設定有問題。
#[test]
fn invalid_target_groups_and_carriers_are_rejected() {
    let cases: [(&str, fn(&mut authoring::Map), &str); 7] = [
        (
            "目標組為空",
            |map| map.victory = group(All, &[units_defeated(&[])]),
            "invalid_target_units",
        ),
        (
            "目標組重複單位",
            |map| map.victory = group(All, &[units_defeated(&[TEAM_MEMBER_ID, TEAM_MEMBER_ID])]),
            "invalid_target_units",
        ),
        (
            "目標組指向不存在的單位",
            |map| map.victory = group(All, &[units_defeated(&[99])]),
            "missing_outcome_unit",
        ),
        (
            "簡單難度沒有任何目標出場",
            |map| {
                with_min_difficulty(map, TEAM_MEMBER_ID, authoring::Difficulty::Normal);
                map.victory = group(All, &[units_defeated(&[TEAM_MEMBER_ID])]);
            },
            "invalid_target_units",
        ),
        (
            "攜貨者不存在",
            |map| map.cargo.as_mut().expect("測試地圖應有貨物設定").carriers = vec![99],
            "invalid_cargo_carrier",
        ),
        (
            "攜貨者重複",
            |map| map.cargo.as_mut().expect("測試地圖應有貨物設定").carriers = vec![3, 3],
            "invalid_cargo_carrier",
        ),
        (
            "簡單難度沒有任何貨物",
            |map| {
                with_min_difficulty(map, 3, authoring::Difficulty::Hard);
                let cargo = map.cargo.as_mut().expect("測試地圖應有貨物設定");
                cargo.positions = Vec::new();
                cargo.carriers = vec![3];
            },
            "invalid_cargo_objective",
        ),
    ];
    for (name, change, expected) in cases {
        let mut map = cargo_outcome_map(&[OutcomeCondition::NoCargoCarried]);
        change(&mut map);
        let error = match load_cargo_outcome(&map, authoring::Difficulty::Hard) {
            Ok(_) => panic!("{name} 應拒絕載入"),
            Err(error) => error,
        };
        assert_eq!(error.id(), expected, "{name}");
    }
}

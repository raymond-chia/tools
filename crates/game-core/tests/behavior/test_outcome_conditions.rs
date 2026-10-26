use crate::support::{
    ACTOR_ID, TARGET_ID, ascii_board, end_turn_and_continue, map_from_ascii_board, marked_cell,
};
use game_core::{Command, Game, GridPos, Outcome, Team, authoring};
use serde_json::{Value, json};

const DEFINITIONS: &str = include_str!("../data/utility_ai_definitions.toml");
const ALL_ENEMIES_DEFEATED: &str = "all_enemies_defeated";
const PLAYER_TEAM_DEFEATED: &str = "team_defeated";
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

fn group(mode: &str, conditions: &[&str]) -> Value {
    json!({
        "mode": mode,
        "conditions": conditions.iter().map(|kind| {
            if *kind == PLAYER_TEAM_DEFEATED {
                json!({"type": kind, "team": "player"})
            } else {
                json!({"type": kind})
            }
        }).collect::<Vec<_>>()
    })
}

// 直接建立作者地圖，保留各案例指定的勝敗條件。
fn map_data(victory: Value, defeat: Value) -> authoring::Map {
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
    map.victory = serde_json::from_value(victory).expect("勝利條件應符合作者格式");
    map.defeat = serde_json::from_value(defeat).expect("失敗條件應符合作者格式");
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
fn team_cargo_document(conditions: Vec<Value>) -> String {
    let layout = ascii_board(CARGO_DIAGRAM);
    let map = map_from_ascii_board(
        "team_cargo_outcome",
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
    let map = toml::Value::try_from(&map).expect("測試地圖應可轉成 TOML 資料");
    let mut map = serde_json::to_value(map).expect("測試地圖應可序列化");
    map["cargo"] = json!({
        "positions": [marked_cell(&layout, 'C')],
        "exits": [marked_cell(&layout, 'E')]
    });
    map["victory"] = json!({"mode": "all", "conditions": conditions});
    map["defeat"] = group("any", &[PLAYER_TEAM_DEFEATED]);
    // 直接序列化輸入，讓正式載入入口驗證尚待實作的新條件。
    toml::to_string(&map).expect("測試地圖應可序列化為 TOML")
}

fn team_cargo_game(conditions: Vec<Value>) -> Game {
    let mut game = Game::from_documents(
        include_str!("../data/cargo_objective_definitions.toml"),
        &team_cargo_document(conditions),
        authoring::Difficulty::Easy,
    )
    .expect("指定 team 全滅與無人攜貨條件應可載入");
    game.set_random_seed(1);
    game
}

fn team_defeated_condition() -> Value {
    json!({"type": "team_defeated", "team": {"enemy": "sentries"}})
}

fn no_cargo_carried_condition() -> Value {
    json!({"type": "no_cargo_carried"})
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

// 驗證勝利與失敗各自選擇 all／any，公開格式轉換完整保留兩組條件。
#[test]
fn independent_outcome_groups_survive_authoring_conversion() {
    for (victory_mode, defeat_mode) in [
        // 勝敗都要求全部成立。
        ("all", "all"),
        // 勝利要求全部、失敗任意成立。
        ("all", "any"),
        // 勝利任意、失敗要求全部成立。
        ("any", "all"),
        // 勝敗都採任意成立。
        ("any", "any"),
    ] {
        let victory = group(victory_mode, &[ALL_ENEMIES_DEFEATED]);
        let defeat = group(defeat_mode, &[PLAYER_TEAM_DEFEATED]);
        let document = toml::to_string(&map_data(victory.clone(), defeat.clone()))
            .expect("測試地圖應可序列化為 TOML");
        let output = authoring::map_to_json(&document).expect("設定應可轉成 JSON");
        let output: Value = serde_json::from_str(&output).expect("結果應為有效 JSON");
        assert_eq!(output["victory"], victory, "勝利 {victory_mode}");
        assert_eq!(output["defeat"], defeat, "失敗 {defeat_mode}");
    }
}

// 驗證同樣只有一個條件成立時，勝敗兩組分別遵守自己的 all／any。
#[test]
fn partially_satisfied_groups_respect_each_match_mode() {
    for (name, victory, defeat, attacker, expected) in [
        // 敵人全滅但玩家未全滅，勝利 all 尚未滿足，不套用固定勝利。
        (
            "勝利 all 未齊",
            group("all", &[ALL_ENEMIES_DEFEATED, PLAYER_TEAM_DEFEATED]),
            group("any", &[PLAYER_TEAM_DEFEATED]),
            ACTOR_ID,
            Outcome::Ongoing,
        ),
        // 相同戰況改成勝利 any，任一條件成立即勝利。
        (
            "勝利 any 成立",
            group("any", &[ALL_ENEMIES_DEFEATED, PLAYER_TEAM_DEFEATED]),
            group("any", &[PLAYER_TEAM_DEFEATED]),
            ACTOR_ID,
            Outcome::Victory,
        ),
        // 玩家全滅但敵人未全滅，失敗 all 尚未滿足，不套用固定敗北。
        (
            "失敗 all 未齊",
            group("any", &[ALL_ENEMIES_DEFEATED]),
            group("all", &[PLAYER_TEAM_DEFEATED, ALL_ENEMIES_DEFEATED]),
            TARGET_ID,
            Outcome::Ongoing,
        ),
        // 相同戰況改成失敗 any，任一條件成立即敗北。
        (
            "失敗 any 成立",
            group("any", &[ALL_ENEMIES_DEFEATED]),
            group("any", &[PLAYER_TEAM_DEFEATED, ALL_ENEMIES_DEFEATED]),
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
            group("all", &["cargo_escaped"]),
            group("any", &[PLAYER_TEAM_DEFEATED]),
            Outcome::Victory,
        ),
        // 貨物離場設定於失敗組，攜貨者離場後應敗北。
        (
            "貨物離場判敗",
            group("all", &[ALL_ENEMIES_DEFEATED]),
            group("any", &["cargo_escaped"]),
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
            positions: vec![GridPos { x, y }],
            exits: vec![GridPos {
                x: exit_x,
                y: exit_y,
            }],
        });
        map.victory = serde_json::from_value(victory).expect("勝利條件應符合作者格式");
        map.defeat = serde_json::from_value(defeat).expect("失敗條件應符合作者格式");
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
    for mode in ["all", "any"] {
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
            "{mode}"
        );
        let snapshot = game
            .command(Command::Continue)
            .expect("戰鬥結束後結果應保持");
        assert_eq!(snapshot.outcome, Outcome::Defeat, "{mode}");
    }
}

// 驗證指定 team 與無人攜貨條件在公開作者格式轉換後完整保留。
#[test]
fn team_cargo_conditions_survive_authoring_conversion() {
    let conditions = vec![team_defeated_condition(), no_cargo_carried_condition()];
    let output = authoring::map_to_json(&team_cargo_document(conditions.clone()))
        .expect("新勝利條件應可轉成 JSON");
    let output: Value = serde_json::from_str(&output).expect("結果應為有效 JSON");
    assert_eq!(
        output["victory"],
        json!({"mode": "all", "conditions": conditions})
    );
}

// 驗證其他 team 的死亡不算，指定 team 須全滅才勝利，其他 team 可繼續存活。
#[test]
fn only_the_specified_teams_elimination_satisfies_the_condition() {
    let mut game = team_cargo_game(vec![team_defeated_condition()]);
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
    let mut game = team_cargo_game(vec![no_cargo_carried_condition()]);
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

// 驗證先消滅指定 team 或先擊倒攜貨者，都須等兩條件同時滿足才勝利。
#[test]
fn team_elimination_and_no_carriers_must_both_be_satisfied() {
    for (name, first, second) in [
        // 指定 team 全滅時仍有攜貨者，最後擊倒攜貨者才勝利。
        ("指定 team 先全滅", 'B', 'T'),
        // 無人攜貨時指定 team 仍有人存活，最後消滅指定 team 才勝利。
        ("攜貨者先倒", 'T', 'B'),
    ] {
        let mut game = team_cargo_game(vec![
            team_defeated_condition(),
            no_cargo_carried_condition(),
        ]);
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

// 驗證敵對陣營全滅即勝利，存活的玩家同盟不算敵人。
#[test]
fn surviving_player_allies_do_not_block_victory() {
    let map = with_player_ally(map_data(
        group("all", &[ALL_ENEMIES_DEFEATED]),
        group("any", &[PLAYER_TEAM_DEFEATED]),
    ));
    let mut game = game(map, ACTOR_ID);
    assert_eq!(eliminate_opponent(&mut game, ACTOR_ID), Outcome::Victory);
}

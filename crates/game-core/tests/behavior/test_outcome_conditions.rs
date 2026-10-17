use super::support::{
    ACTOR_ID, TARGET_ID, ascii_board, end_turn_and_continue, map_from_ascii_board, marked_cell,
};
use game_core::{Command, Game, GridPos, Outcome, Team, authoring};
use serde_json::{Value, json};

const DEFINITIONS: &str = include_str!("../data/utility_ai_definitions.toml");
const ENEMIES: &str = "all_enemies_defeated";
const PLAYERS: &str = "all_players_defeated";
const DIAGRAM: &str = "
    ....
    .AT.
    ....
";

fn group(mode: &str, conditions: &[&str]) -> Value {
    json!({
        "mode": mode,
        "conditions": conditions.iter().map(|kind| json!({"type": kind})).collect::<Vec<_>>()
    })
}

// 以作者地圖序列化 TOML，保留各案例指定的勝敗條件。
fn document(victory: Value, defeat: Value) -> String {
    let layout = ascii_board(DIAGRAM);
    let map = map_from_ascii_board(
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
    let mut map = serde_json::to_value(map).expect("測試地圖應可序列化");
    map["victory"] = victory;
    map["defeat"] = defeat;
    let map: authoring::Map = serde_json::from_value(map).expect("勝敗地圖應符合作者格式");
    toml::to_string(&map).expect("勝敗地圖應可序列化為 TOML")
}

fn game(victory: Value, defeat: Value, attacker: i64) -> Game {
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
    let definitions = toml::to_string(&definitions).expect("測試定義應可序列化");
    let mut game = Game::from_documents(
        &definitions,
        &document(victory, defeat),
        authoring::Difficulty::Easy,
    )
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
        let victory = group(victory_mode, &[ENEMIES]);
        let defeat = group(defeat_mode, &[PLAYERS]);
        let output = authoring::map_to_json(&document(victory.clone(), defeat.clone()))
            .expect("設定應可轉成 JSON");
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
            group("all", &[ENEMIES, PLAYERS]),
            group("any", &[PLAYERS]),
            ACTOR_ID,
            Outcome::Ongoing,
        ),
        // 相同戰況改成勝利 any，任一條件成立即勝利。
        (
            "勝利 any 成立",
            group("any", &[ENEMIES, PLAYERS]),
            group("any", &[PLAYERS]),
            ACTOR_ID,
            Outcome::Victory,
        ),
        // 玩家全滅但敵人未全滅，失敗 all 尚未滿足，不套用固定敗北。
        (
            "失敗 all 未齊",
            group("any", &[ENEMIES]),
            group("all", &[PLAYERS, ENEMIES]),
            TARGET_ID,
            Outcome::Ongoing,
        ),
        // 相同戰況改成失敗 any，任一條件成立即敗北。
        (
            "失敗 any 成立",
            group("any", &[ENEMIES]),
            group("any", &[PLAYERS, ENEMIES]),
            TARGET_ID,
            Outcome::Defeat,
        ),
    ] {
        let mut game = game(victory, defeat, attacker);
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
            group("any", &[PLAYERS]),
            Outcome::Victory,
        ),
        // 貨物離場設定於失敗組，攜貨者離場後應敗北。
        (
            "貨物離場判敗",
            group("all", &[ENEMIES]),
            group("any", &["cargo_escaped"]),
            Outcome::Defeat,
        ),
    ] {
        let map = map_from_ascii_board(
            "cargo_escape_outcome",
            &layout,
            &[
                ('A', ACTOR_ID, "guard", Team::Player),
                ('T', TARGET_ID, "raider", Team::Enemy("bandits".into())),
                ('U', 3, "second_raider", Team::Enemy("bandits".into())),
            ],
        );
        let mut map = serde_json::to_value(map).expect("測試地圖應可序列化");
        let GridPos { x, y } = marked_cell(&layout, 'C');
        let GridPos {
            x: exit_x,
            y: exit_y,
        } = marked_cell(&layout, 'E');
        map["cargo"] = json!({
            "positions": [{"x": x, "y": y}],
            "exits": [{"x": exit_x, "y": exit_y}]
        });
        map["victory"] = victory;
        map["defeat"] = defeat;
        let map: authoring::Map = serde_json::from_value(map).expect("測試地圖應符合作者格式");
        let document = toml::to_string(&map).expect("測試地圖應可序列化為 TOML");
        let mut game = Game::from_documents(
            include_str!("../data/cargo_objective_definitions.toml"),
            &document,
            authoring::Difficulty::Easy,
        )
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
        let mut game = game(group(mode, &[ENEMIES]), group(mode, &[ENEMIES]), ACTOR_ID);
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

use crate::support::{ACTOR_ID, TARGET_ID, ascii_board, map_from_ascii_board, marked_cell, unit};
use game_core::{Command, Game, Outcome, Team, authoring};
use serde_json::{Value, json};

const DEFINITIONS: &str = include_str!("../data/utility_ai_definitions.toml");
const OTHER_ENEMY_ID: i64 = 3;
const PLAYER_ID: i64 = 4;
const DIAGRAM: &str = "
    .........
    .TAB...P.
    .........
";

fn alliances() -> Value {
    json!([{"teams": [{"enemy": "leaders"}, {"enemy": "bandits"}]}])
}

fn map_document(reverse: bool, hostile_target: bool) -> String {
    let layout = ascii_board(DIAGRAM);
    let (actor_team, target_team) = if reverse {
        ("bandits", "leaders")
    } else {
        ("leaders", "bandits")
    };
    let map = map_from_ascii_board(
        "alliances",
        &layout,
        &[
            ('A', ACTOR_ID, "tank", Team::Enemy(actor_team.into())),
            ('T', TARGET_ID, "target", Team::Enemy(target_team.into())),
            (
                'B',
                OTHER_ENEMY_ID,
                "other_tank",
                Team::Enemy(
                    if hostile_target {
                        "outsiders"
                    } else {
                        actor_team
                    }
                    .into(),
                ),
            ),
            ('P', PLAYER_ID, "other_tank", Team::Player),
        ],
    );
    let map = toml::Value::try_from(&map).expect("測試地圖應可轉成 TOML 資料");
    let mut map = serde_json::to_value(map).expect("測試地圖應可序列化");
    map["alliances"] = alliances();
    toml::to_string(&map).expect("同盟地圖應可序列化")
}

fn game(reverse: bool, hostile_target: bool) -> Game {
    let mut definitions: authoring::Definitions =
        toml::from_str(DEFINITIONS).expect("測試定義應可載入");
    for unit in &mut definitions.unit_types {
        unit.movement = 0;
        unit.initiative = if unit.id == "tank" { 1000 } else { 0 };
        unit.hp = if unit.id == "target" { 1 } else { 100 };
    }
    let definitions = toml::to_string(&definitions).expect("測試定義應可序列化");
    let mut game = Game::from_documents(
        &definitions,
        &map_document(reverse, hostile_target),
        authoring::Difficulty::Easy,
    )
    .expect("同盟戰鬥應可載入");
    game.set_random_seed(1);
    let snapshot = game.command(Command::Start).expect("戰鬥應可開始");
    assert_eq!(snapshot.turn.actor, Some(ACTOR_ID));
    assert_eq!(snapshot.outcome, Outcome::Ongoing);
    // 固定攻擊擲骰，避免先攻消耗亂數後出現強制失敗。
    game.set_random_seed(1);
    game
}

fn attack_target(game: &mut Game) -> Result<game_core::Snapshot, game_core::GameError> {
    let game_core::GridPos { x, y } = marked_cell(&ascii_board(DIAGRAM), 'T');
    game.command(Command::Skill {
        actor: ACTOR_ID,
        x,
        y,
        skill: "melee".into(),
    })
}

// 驗證不同陣營間的同盟關係在公開作者格式轉換後完整保留。
#[test]
fn alliances_survive_authoring_conversion() {
    let output =
        authoring::map_to_json(&map_document(false, false)).expect("同盟地圖應可轉成 JSON");
    let output: Value = serde_json::from_str(&output).expect("輸出應為有效 JSON");
    assert_eq!(output["alliances"], alliances());
}

// 驗證同盟雙向禁止攻擊，拒絕時不扣血、不消耗行動，仍可攻擊敵對陣營。
#[test]
fn allied_factions_cannot_attack_each_other() {
    for reverse in [false, true] {
        let mut game = game(reverse, true);
        let before = serde_json::to_value(game.snapshot(None)).expect("快照應可序列化");
        let error = attack_target(&mut game)
            .err()
            .expect("不同陣營的同盟應禁止互相攻擊");
        assert_eq!(error.id(), "cannot_attack_ally");
        let after = serde_json::to_value(game.snapshot(None)).expect("快照應可序列化");
        assert_eq!(after["units"], before["units"]);
        assert_eq!(after["turn"], before["turn"]);
        let game_core::GridPos { x, y } = marked_cell(&ascii_board(DIAGRAM), 'B');
        let snapshot = game
            .command(Command::Skill {
                actor: ACTOR_ID,
                x,
                y,
                skill: "melee".into(),
            })
            .expect("拒絕攻擊同盟後應仍可攻擊敵對陣營");
        assert_eq!(unit(&snapshot, OTHER_ENEMY_ID).hp, 90);
    }
}

// 驗證同盟雙向不被 AI 攻擊；範圍內有敵人時只傷害敵人，只有同盟時不造成傷害。
#[test]
fn ai_only_attacks_hostile_factions_in_range() {
    for (name, hostile_target, expected_hp) in [
        // 同盟即使只剩一滴血，AI 仍應選擇範圍內的敵人。
        ("同盟與敵人都在範圍內", true, [1, 90, 100]),
        // 範圍內只有同盟時，AI 不應傷害同盟或遠方敵人。
        ("範圍內只有同盟", false, [1, 100, 100]),
    ] {
        for reverse in [false, true] {
            let mut game = game(reverse, hostile_target);
            let snapshot = game.command(Command::Continue).expect("AI 應可完成行動");
            let actual_hp = [TARGET_ID, OTHER_ENEMY_ID, PLAYER_ID].map(|id| unit(&snapshot, id).hp);
            assert_eq!(actual_hp, expected_hp, "{name}, reverse={reverse}");
        }
    }
}

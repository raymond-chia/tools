use crate::support::{ACTOR_ID, TARGET_ID, ascii_board, ascii_map, marked_cell, unit};
use game_core::{BattleMode, Command, Game, Team, authoring};

const DEFINITIONS: &str = include_str!("../data/utility_ai_definitions.toml");
const PARTNER_ID: i64 = 3;

// A 與 P 為玩家，T 為敵人；沿用專用測試定義與共用 ASCII 棋盤。
fn exploration_test_data(diagram: &str) -> (authoring::Definitions, authoring::Map) {
    let definitions = toml::from_str(DEFINITIONS).expect("探索測試定義應可載入");
    let mut units = vec![
        ('A', ACTOR_ID, "healer", Team::Player),
        ('T', TARGET_ID, "target", Team::Enemy("test_enemy".into())),
    ];
    if diagram.contains('P') {
        units.push(('P', PARTNER_ID, "healer", Team::Player));
    }
    (definitions, ascii_map("exploration_test", diagram, &units))
}

// 驗證單人與多人探索皆可一次走到遠處，並反覆移動，不需結束回合或切換隊員。
#[test]
fn exploration_moves_across_turns_without_manual_end_turn() {
    for (name, diagram) in [
        (
            "單人",
            "
            A..N.....D#T
            ",
        ),
        (
            "有尚未行動的隊友",
            "
            A..N.....D#T
            P...........
            ",
        ),
    ] {
        let layout = ascii_board(diagram);
        let (definitions, map) = exploration_test_data(diagram);
        let mut game = Game::from_authoring(definitions, map, authoring::Difficulty::Easy)
            .expect("探索測試地圖應可載入");
        game.command(Command::Start).expect("探索應可開始");
        for marker in ['D', 'N', 'D'] {
            let destination = marked_cell(&layout, marker);
            let snapshot = game
                .command(Command::Move {
                    actor: ACTOR_ID,
                    x: destination.x,
                    y: destination.y,
                })
                .expect("安全探索應可一次走到超過兩段移動額度的目的地");
            let actor = unit(&snapshot, ACTOR_ID);
            assert_eq!((actor.x, actor.y), (destination.x, destination.y), "{name}");
            assert_eq!(
                snapshot.battle_mode == BattleMode::Exploring,
                true,
                "{name}"
            );
            assert_eq!(snapshot.turn.actor, Some(ACTOR_ID), "{name}");
            assert_eq!(snapshot.turn.can_move, true, "{name}");
            assert_eq!(snapshot.turn.can_skill, true, "{name}");
        }
    }
}

// 驗證有隊友時，探索中同一人可連續治療三次且每次生效，不需手動換人或結束回合。
#[test]
fn exploration_reuses_skills_without_manual_end_turn() {
    let diagram = "
        A^.........#T
        P............
    ";
    let (definitions, map) = exploration_test_data(diagram);
    let mut definitions = toml::Value::try_from(definitions).expect("測試定義應可轉成作者資料");
    let heal = definitions["active_skills"]
        .as_array_mut()
        .expect("測試定義應包含技能")
        .iter_mut()
        .find(|skill| skill["id"].as_str() == Some("heal"))
        .expect("測試定義應包含治療");
    heal["power_bonus"] = 1.into();
    let definitions = definitions.try_into().expect("測試定義應符合作者格式");
    let mut game = Game::from_authoring(definitions, map, authoring::Difficulty::Easy)
        .expect("連續技能測試地圖應可載入");
    game.command(Command::Start).expect("探索應可開始");
    let snapshot = game
        .command(Command::Move {
            actor: ACTOR_ID,
            x: 1,
            y: 0,
        })
        .expect("玩家應可踩尖刺受傷");
    assert_eq!(unit(&snapshot, ACTOR_ID).hp, 97);
    for expected_hp in [98, 99, 100] {
        let snapshot = game
            .command(Command::Skill {
                actor: ACTOR_ID,
                x: 1,
                y: 0,
                skill: "heal".into(),
            })
            .expect("探索中同一人應能連續施放技能");
        assert_eq!(unit(&snapshot, ACTOR_ID).hp, expected_hp);
        assert_eq!(snapshot.battle_mode == BattleMode::Exploring, true);
        assert_eq!(snapshot.turn.actor, Some(ACTOR_ID));
        assert_eq!(snapshot.turn.can_skill, true);
        assert_eq!(snapshot.turn.can_move, true);
    }
}

// 驗證途中遇敵不立即停步，但最多走完本次剩餘的兩段額度；目的地較近則停在目的地。
#[test]
fn exploration_contact_finishes_current_movement_budget() {
    for (name, diagram, initial_move, expected_x) in [
        (
            "目的地在本次額度內",
            "
            A.D........T
            ",
            None,
            2,
        ),
        (
            "遠處目的地只走完本次額度",
            "
            A........D...T
            ",
            None,
            4,
        ),
        (
            "已花掉的移動力不補回",
            "
            A........D....T
            ",
            Some(1),
            4,
        ),
    ] {
        let layout = ascii_board(diagram);
        let destination = marked_cell(&layout, 'D');
        let (definitions, map) = exploration_test_data(diagram);
        let mut game = Game::from_authoring(definitions, map, authoring::Difficulty::Easy)
            .expect("接敵測試地圖應可載入");
        game.set_random_seed(1);
        let snapshot = game.command(Command::Start).expect("探索應可開始");
        assert_eq!(
            snapshot.battle_mode == BattleMode::Exploring,
            true,
            "{name}"
        );
        if let Some(x) = initial_move {
            let snapshot = game
                .command(Command::Move {
                    actor: ACTOR_ID,
                    x,
                    y: 0,
                })
                .expect("接敵前應可先走一格");
            assert_eq!(
                snapshot.battle_mode == BattleMode::Exploring,
                true,
                "{name}"
            );
        }
        let snapshot = game
            .command(Command::Move {
                actor: ACTOR_ID,
                x: destination.x,
                y: destination.y,
            })
            .expect("探索應可指定遠處目的地，接敵後才依本次額度停步");
        let actor = unit(&snapshot, ACTOR_ID);
        assert_eq!((actor.x, actor.y), (expected_x, 0), "{name}");
        assert_eq!(snapshot.battle_mode == BattleMode::Combat, true, "{name}");
        assert_eq!(snapshot.round, 1, "{name}");
        let path: Vec<_> = snapshot
            .movements
            .iter()
            .filter(|movement| movement.unit_id == ACTOR_ID)
            .flat_map(|movement| movement.path.iter().map(|cell| (cell.x, cell.y)))
            .collect();
        let expected_path: Vec<_> = (initial_move.unwrap_or(0)..=expected_x)
            .map(|x| (x, 0))
            .collect();
        assert_eq!(path, expected_path, "{name}：應逐格移動，不能瞬移");
    }
}

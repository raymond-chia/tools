use super::support::{ACTOR_ID, TARGET_ID};
use crate::*;

// 沿用測試目錄內的單位、技能與地形定義，不載入正式遊戲資料。
const DEFINITIONS: &str = include_str!("data/utility_ai_definitions.toml");

fn encounter_game(first_x: i32, offsets: &[i32], reverse: bool) -> Game {
    let mut definitions: toml::Value = toml::from_str(DEFINITIONS).expect("測試定義應為有效 TOML");
    let shot = definitions["skills"]
        .as_array_mut()
        .expect("測試定義應包含技能")
        .iter_mut()
        .find(|skill| skill["id"].as_str() == Some("shot"))
        .expect("測試定義應包含射擊技能");
    shot["max_range"] = 12.into();
    let mut units = vec![authoring::UnitPlacement {
        id: ACTOR_ID,
        unit_type: "archer".into(),
        team: Team::Player,
        x: 0,
        y: 0,
    }];
    for (index, offset) in offsets.iter().enumerate() {
        units.push(authoring::UnitPlacement {
            id: TARGET_ID + index as i64,
            unit_type: "target".into(),
            team: Team::Enemy("test_enemy".into()),
            x: first_x + offset,
            y: 0,
        });
    }
    if reverse {
        units.reverse();
    }
    let map = authoring::Map {
        name: "encounter_test".into(),
        width: 50,
        height: 1,
        terrains: Vec::new(),
        units,
    };
    let definitions = toml::to_string(&definitions).expect("測試定義應可序列化");
    let map = toml::to_string(&map).expect("測試地圖應可序列化");
    let mut game = Game::from_documents(&definitions, &map).expect("測試遭遇應可載入");
    game.set_random_seed(1);
    game
}

fn assert_participants(game: &Game, expected: &[i64], name: &str) {
    let crate::model::Encounter {
        participants,
        order: _,
        cursor: _,
        round: _,
    } = game.world.resource::<crate::model::Encounter>();
    let mut actual: Vec<_> = participants.iter().copied().collect();
    actual.sort();
    assert_eq!(actual, expected, "{name}");
}

// 驗證開始遭遇時以五格連鎖觸發整批敵人，超過五格的同陣營敵人維持待機，且不受載入順序影響。
#[test]
fn encounter_start_activates_connected_enemies() {
    for (name, offsets, expected) in [
        (
            "相鄰長列",
            vec![0, 1, 2, 3, 4, 5, 6, 12],
            vec![1, 2, 3, 4, 5, 6, 7, 8],
        ),
        ("五格邊界連鎖", vec![0, 5, 10, 16], vec![1, 2, 3, 4]),
        ("六格間隔不觸發", vec![0, 6, 7], vec![1, 2]),
        ("未接近時整批待機", vec![1, 2, 3], vec![1]),
    ] {
        for reverse in [false, true] {
            let mut game = encounter_game(10, &offsets, reverse);
            game.command(Command::Start).expect("遭遇應可開始");
            assert_participants(&game, &expected, &format!("{name}，反向載入 {reverse}"));
        }
    }
}

// 驗證探索移動接近第一隻敵人時會連鎖觸發五格相連的一批，較遠的同陣營敵人仍待機。
#[test]
fn encounter_movement_activates_connected_enemies() {
    let mut game = encounter_game(11, &[0, 5, 10, 16], false);
    game.command(Command::Start).expect("探索應可開始");
    assert_participants(&game, &[1], "移動前");
    let snapshot = game
        .command(Command::Move {
            actor: ACTOR_ID,
            x: 1,
            y: 0,
        })
        .expect("玩家應可接近敵人");
    assert!(
        snapshot.battle_mode == BattleMode::Combat,
        "接近敵人後應進入戰鬥"
    );
    assert_participants(&game, &[1, 2, 3, 4], "移動觸發");
}

// 驗證探索中攻擊遭遇範圍外的敵人也會以受攻擊者為起點連鎖觸發附近一批。
#[test]
fn encounter_attack_activates_connected_enemies() {
    let mut game = encounter_game(11, &[0, 5, 10, 16], false);
    game.command(Command::Start).expect("探索應可開始");
    assert_participants(&game, &[1], "攻擊前");
    game.command(Command::Skill {
        actor: ACTOR_ID,
        x: 11,
        y: 0,
        skill: "shot".into(),
    })
    .expect("玩家應可攻擊遭遇範圍外的敵人");
    assert_participants(&game, &[1, 2, 3, 4], "攻擊觸發");
}

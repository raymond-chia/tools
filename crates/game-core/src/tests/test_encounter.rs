use super::support::{ACTOR_ID, TARGET_ID, ascii_map};
use crate::*;

// 沿用測試目錄內的單位、技能與地形定義，不載入正式遊戲資料。
const DEFINITIONS: &str = include_str!("data/utility_ai_definitions.toml");

fn encounter_game(diagram: &str, reverse: bool) -> (Game, Vec<(char, i64)>) {
    let mut definitions: toml::Value = toml::from_str(DEFINITIONS).expect("測試定義應為有效 TOML");
    let shot = definitions["skills"]
        .as_array_mut()
        .expect("測試定義應包含技能")
        .iter_mut()
        .find(|skill| skill["id"].as_str() == Some("shot"))
        .expect("測試定義應包含射擊技能");
    shot["max_range"] = 12.into();
    // A 為玩家；依 T、U、V…順序給敵人實例 ID。
    let mut units = vec![('A', ACTOR_ID, "archer", Team::Player)];
    for (index, marker) in "TUVWXYZB".chars().enumerate() {
        if diagram.contains(marker) {
            units.push((
                marker,
                TARGET_ID + index as i64,
                "target",
                Team::Enemy("test_enemy".into()),
            ));
        }
    }
    let mut map = ascii_map("encounter_test", diagram, &units);
    if reverse {
        map.units.reverse();
    }
    let definitions = toml::to_string(&definitions).expect("測試定義應可序列化");
    let map = toml::to_string(&map).expect("測試地圖應可序列化");
    let mut game = Game::from_documents(&definitions, &map).expect("測試遭遇應可載入");
    game.set_random_seed(1);
    let markers = units
        .iter()
        .map(|(marker, id, _, _)| (*marker, *id))
        .collect();
    (game, markers)
}

fn assert_participants(game: &Game, markers: &[(char, i64)], expected: &str, name: &str) {
    let crate::model::Encounter {
        participants,
        order: _,
        cursor: _,
        round: _,
    } = game.world.resource::<crate::model::Encounter>();
    let mut actual: Vec<_> = participants
        .iter()
        .map(|id| {
            markers
                .iter()
                .find(|(_, unit_id)| unit_id == id)
                .expect("參戰單位應有對應的地圖代號")
                .0
        })
        .collect();
    actual.sort();
    let mut expected: Vec<_> = expected.chars().collect();
    expected.sort();
    assert_eq!(actual, expected, "{name}");
}

// 驗證開始遭遇時以十格連鎖觸發整批敵人，超過十格的同陣營敵人維持待機，且不受載入順序影響
#[test]
fn encounter_start_activates_connected_enemies() {
    for (name, diagram, expected) in [
        (
            "相鄰長列",
            "A.........TUVWXYZ.....B...........................",
            "ATUVWXYZB",
        ),
        (
            "十格邊界連鎖",
            "A.........T.........U.........V..........W........",
            "ATUV",
        ),
        (
            "十一格間隔不觸發",
            "A.........T..........UV...........................",
            "AT",
        ),
        (
            "原本六格間隔也參戰",
            "A.........T.....UV................................",
            "ATUV",
        ),
        (
            "未接近時整批待機",
            "A..........TUV....................................",
            "A",
        ),
    ] {
        for reverse in [false, true] {
            let (mut game, markers) = encounter_game(diagram, reverse);
            game.command(Command::Start).expect("遭遇應可開始");
            assert_participants(
                &game,
                &markers,
                expected,
                &format!("{name}，反向載入 {reverse}"),
            );
        }
    }
}

// 驗證探索移動接近第一隻敵人時會連鎖觸發十格相連的一批，較遠的同陣營敵人仍待機。
#[test]
fn encounter_movement_activates_connected_enemies() {
    let (mut game, markers) =
        encounter_game("A..........T.........U.........V..........W.......", false);
    game.command(Command::Start).expect("探索應可開始");
    assert_participants(&game, &markers, "A", "移動前");
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
    assert_participants(&game, &markers, "ATUV", "移動觸發");
}

// 驗證探索中攻擊遭遇範圍外的敵人也會以受攻擊者為起點連鎖觸發附近一批。
#[test]
fn encounter_attack_activates_connected_enemies() {
    let (mut game, markers) =
        encounter_game("A..........T.........U.........V..........W.......", false);
    game.command(Command::Start).expect("探索應可開始");
    assert_participants(&game, &markers, "A", "攻擊前");
    let target = game.entity(TARGET_ID).expect("ASCII 的 T 應建立目標單位");
    let GridPos { x, y } = game.world.get::<Pos>(target).expect("目標應有位置").0;
    game.command(Command::Skill {
        actor: ACTOR_ID,
        x,
        y,
        skill: "shot".into(),
    })
    .expect("玩家應可攻擊遭遇範圍外的敵人");
    assert_participants(&game, &markers, "ATUV", "攻擊觸發");
}

// 驗證參戰敵人移動接近待機同伴後，以相同十格規則連鎖參戰，十一格外的敵人仍待機。
#[test]
fn encounter_enemy_movement_activates_connected_enemies() {
    let definitions = toml::from_str(DEFINITIONS).expect("測試定義應為有效 TOML");
    let units = [
        ('A', ACTOR_ID, "target", Team::Player),
        ('T', TARGET_ID, "healer", Team::Enemy("test_enemy".into())),
        ('L', 3, "target", Team::Enemy("test_enemy".into())),
        ('S', 4, "target", Team::Enemy("test_enemy".into())),
        ('F', 5, "target", Team::Enemy("test_enemy".into())),
    ];
    let markers: Vec<_> = units
        .iter()
        .map(|(marker, id, _, _)| (*marker, *id))
        .collect();
    let map = ascii_map(
        "enemy_movement_encounter",
        "A.........T..........L.........S..........F.......",
        &units,
    );
    let mut game = Game::from_authoring(definitions, map).expect("敵人移動遭遇測試應可載入");
    game.set_random_seed(1);
    let ally = game.entity(3).expect("測試待機同伴應存在");
    game.world
        .get_mut::<Hp>(ally)
        .expect("同伴應具有 HP")
        .current = 20;
    game.command(Command::Start).expect("戰鬥應可開始");
    assert_participants(&game, &markers, "AT", "同伴尚在十一格外");
    assert_eq!(game.world.resource::<Turn>().actor, Some(TARGET_ID));
    game.command(Command::Continue)
        .expect("治癒者應可移向受傷同伴");
    let healer = game.entity(TARGET_ID).expect("治癒者應仍存在");
    assert_eq!(
        game.world.get::<Pos>(healer).expect("治癒者應有位置").0,
        GridPos { x: 14, y: 0 }
    );
    assert_participants(&game, &markers, "ATLS", "移動後啟動同伴並連鎖十格");
}

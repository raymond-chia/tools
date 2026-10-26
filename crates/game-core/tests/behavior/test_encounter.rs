use crate::support::{ACTOR_ID, TARGET_ID, ascii_map, assert_turn_order, unit};
use game_core::{BattleMode, Command, Game, Team, authoring};

// 沿用測試目錄內的單位、技能與地形定義，不載入正式遊戲資料。
const DEFINITIONS: &str = include_str!("../data/utility_ai_definitions.toml");
const ALLY_ID: i64 = 10;

fn encounter_game(diagram: &str, reverse: bool) -> (Game, Vec<(char, i64)>) {
    let mut definitions: toml::Value = toml::from_str(DEFINITIONS).expect("測試定義應為有效 TOML");
    let shot = definitions["active_skills"]
        .as_array_mut()
        .expect("測試定義應包含技能")
        .iter_mut()
        .find(|skill| skill["id"].as_str() == Some("shot"))
        .expect("測試定義應包含射擊技能");
    shot["max_range"] = 12.into();
    // A 為玩家；依 T、U、V…順序給敵人實例 ID；F 為與玩家同盟的陣營。
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
    let ally_team = Team::Enemy("friends".into());
    if diagram.contains('F') {
        units.push(('F', ALLY_ID, "target", ally_team.clone()));
    }
    let mut map = ascii_map("encounter_test", diagram, &units);
    map.alliances = vec![authoring::Alliance {
        teams: vec![Team::Player, ally_team],
    }];
    if reverse {
        map.units.reverse();
    }
    let definitions: authoring::Definitions =
        definitions.try_into().expect("測試定義應符合作者格式");
    let mut game = Game::from_authoring(definitions, map, authoring::Difficulty::Easy)
        .expect("測試遭遇應可載入");
    game.set_random_seed(1);
    let markers = units
        .iter()
        .map(|(marker, id, _, _)| (*marker, *id))
        .collect();
    (game, markers)
}

// 驗證開始遭遇只納入十格內可見及連鎖可見的敵人與所有玩家同盟，隔牆不觸發，且不受載入順序影響。
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
        (
            "只有同盟靠近不觸發",
            "AF................................................",
            "A",
        ),
        (
            "遠方同盟也參戰",
            "A.........T...................................F...",
            "ATF",
        ),
        (
            "隔牆不開戰",
            "
            A#T
            ...
            ...
            ",
            "A",
        ),
        (
            "與參戰敵人隔牆",
            "
            A.......T#W#Z
            .........#.#.
            ........U.V..
            ",
            "ATUVW",
        ),
    ] {
        for reverse in [false, true] {
            let (mut game, markers) = encounter_game(diagram, reverse);
            let snapshot = game.command(Command::Start).expect("遭遇應可開始");
            assert_eq!(
                snapshot.battle_mode == BattleMode::Combat,
                expected.contains('T'),
                "{name}，反向載入 {reverse}"
            );
            assert_turn_order(
                &game,
                &markers,
                expected,
                &format!("{name}，反向載入 {reverse}"),
            );
        }
    }
}

// 驗證探索移動接近敵人或走出遮擋後才開戰，並納入十格連鎖範圍內可見的敵人。
#[test]
fn encounter_movement_activates_connected_enemies() {
    for (name, diagram, x, y, expected) in [
        (
            "接近敵人",
            "A..........T.........U.........V..........W.......",
            1,
            0,
            "ATUV",
        ),
        (
            "走出遮擋",
            "
            A#T
            ...
            ...
            ",
            0,
            2,
            "AT",
        ),
    ] {
        let (mut game, markers) = encounter_game(diagram, false);
        let snapshot = game.command(Command::Start).expect("探索應可開始");
        assert_eq!(
            snapshot.battle_mode == BattleMode::Exploring,
            true,
            "{name}"
        );
        assert_turn_order(&game, &markers, "A", &format!("{name}：移動前"));
        let snapshot = game
            .command(Command::Move {
                actor: ACTOR_ID,
                x,
                y,
            })
            .expect("玩家應可移動到看見敵人的位置");
        assert_eq!(snapshot.battle_mode == BattleMode::Combat, true, "{name}");
        assert_turn_order(&game, &markers, expected, &format!("{name}：移動後"));
    }
}

// 驗證公開探索攻擊與結束回合後，以受攻擊者為起點連鎖觸發附近敵人加入行動順序。
#[test]
fn encounter_attack_activates_connected_enemies() {
    let (mut game, markers) =
        encounter_game("A..........T.........U.........V..........W.......", false);
    game.command(Command::Start).expect("探索應可開始");
    assert_turn_order(&game, &markers, "A", "攻擊前");
    let snapshot = game.snapshot(None);
    let target = unit(&snapshot, TARGET_ID);
    let (x, y) = (target.x, target.y);
    game.command(Command::Skill {
        actor: ACTOR_ID,
        x,
        y,
        skill: "shot".into(),
    })
    .expect("玩家應可攻擊遭遇範圍外的敵人");
    game.command(Command::EndTurn { actor: ACTOR_ID })
        .expect("探索攻擊後應可結束回合進入戰鬥");
    assert_turn_order(&game, &markers, "ATUV", "攻擊觸發後的行動順序");
}

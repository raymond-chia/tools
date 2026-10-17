use super::support::{
    ACTOR_ID, AsciiBoard, TARGET_ID, ascii_board, end_turn_and_continue, map_from_ascii_board,
    marked_cell, unit,
};
use game_core::{CargoState, Command, Game, GridPos, Outcome, Phase, Team, authoring};
use serde_json::{Value, json};

const DEFINITIONS: &str = include_str!("../data/cargo_objective_definitions.toml");
const SECOND_RAIDER: i64 = 3;

// 以作者地圖序列化貨物防守 TOML，逐份提供貨物位置、撤離格及勝敗條件。
// C/F 是貨物位置、E 是撤離格、A 是護衛、T/U 是強盜、D 是移動目的地。
fn map_document(layout: &AsciiBoard) -> String {
    let AsciiBoard {
        board: _,
        markers,
        terrains: _,
    } = layout;
    let mut units = vec![
        ('A', ACTOR_ID, "guard", Team::Player),
        ('T', TARGET_ID, "raider", Team::Enemy("bandits".into())),
    ];
    if markers.contains_key(&'U') {
        units.push((
            'U',
            SECOND_RAIDER,
            "second_raider",
            Team::Enemy("bandits".into()),
        ));
    }
    let mut document = serde_json::to_value(map_from_ascii_board("cargo_defense", layout, &units))
        .expect("測試地圖應可序列化");
    let positions: Vec<_> = ['C', 'F']
        .into_iter()
        .filter(|marker| markers.contains_key(marker))
        .map(|marker| marked_cell(layout, marker))
        .collect();
    let GridPos {
        x: exit_x,
        y: exit_y,
    } = marked_cell(layout, 'E');
    document["cargo"] = json!({
        "positions": positions,
        "exits": [{"x": exit_x, "y": exit_y}]
    });
    document["victory"] = json!({
        "mode": "all",
        "conditions": [{"type": "all_enemies_defeated"}]
    });
    document["defeat"] = json!({
        "mode": "any",
        "conditions": [
            {"type": "all_players_defeated"},
            {"type": "cargo_escaped"}
        ]
    });
    let document: authoring::Map =
        serde_json::from_value(document).expect("測試地圖應符合作者格式");
    toml::to_string(&document).expect("測試地圖應可序列化為 TOML")
}

fn game(diagram: &str) -> (Game, AsciiBoard) {
    let layout = ascii_board(diagram);
    let mut game = Game::from_documents(
        DEFINITIONS,
        &map_document(&layout),
        authoring::Difficulty::Easy,
    )
    .expect("貨物防守地圖應可載入");
    game.set_random_seed(1);
    let snapshot = game.command(Command::Start).expect("防守戰應可開始");
    assert_eq!(snapshot.turn.actor, Some(TARGET_ID));
    assert_eq!(snapshot.outcome, Outcome::Ongoing);
    (game, layout)
}

fn loot(game: &mut Game, GridPos { x, y }: GridPos) {
    game.command(Command::Skill {
        actor: TARGET_ID,
        x,
        y,
        skill: "loot".into(),
    })
    .expect("相鄰強盜應可花一次行動取得貨物");
}

fn carrying(game: &Game, id: i64) -> Value {
    let snapshot = serde_json::to_value(game.snapshot(None)).expect("快照應可序列化");
    snapshot["units"]
        .as_array()
        .expect("快照應包含單位")
        .iter()
        .find(|unit| unit["id"] == id)
        .expect("攜貨者應仍在地圖內")["carrying_cargo"]
        .clone()
}

fn move_to(game: &mut Game, actor: i64, GridPos { x, y }: GridPos) {
    game.command(Command::Move { actor, x, y })
        .expect("案例指定的路線應可通行");
}

fn next_actor(game: &mut Game, expected: i64) {
    let snapshot = end_turn_and_continue(game);
    assert_eq!(snapshot.turn.actor, Some(expected));
}

fn next_raider_turn(game: &mut Game) {
    if game
        .snapshot(None)
        .units
        .iter()
        .any(|unit| unit.id == SECOND_RAIDER)
    {
        next_actor(game, SECOND_RAIDER);
    }
    next_actor(game, ACTOR_ID);
    next_actor(game, TARGET_ID);
}

fn strike(game: &mut Game, actor: i64, GridPos { x, y }: GridPos) {
    game.command(Command::Skill {
        actor,
        x,
        y,
        skill: "strike".into(),
    })
    .expect("相鄰攻擊應可擊倒目標");
}

// 驗證貨物目標在公開作者格式轉換後完整保留，不能被默默忽略。
#[test]
fn cargo_objective_survives_authoring_conversion() {
    let layout = ascii_board(
        "
        .....
        .ACTE
        .....
    ",
    );
    let source = map_document(&layout);
    let GridPos { x, y } = marked_cell(&layout, 'C');
    let GridPos {
        x: exit_x,
        y: exit_y,
    } = marked_cell(&layout, 'E');
    let output = authoring::map_to_json(&source).expect("貨物地圖應可轉成 JSON");
    let actual: Value = serde_json::from_str(&output).expect("轉換結果應為 JSON");
    assert_eq!(
        actual["cargo"],
        json!({
            "positions": [{"x": x, "y": y}], "exits": [{"x": exit_x, "y": exit_y}]
        })
    );
}

// 驗證取貨後本回合不能移動、空位置不能再取貨，另一份貨物可取得且下一輪才可移動。
#[test]
fn looting_ends_the_turn_and_preserves_other_cargo_until_next_round() {
    let (mut game, layout) = game(
        "
        .FUD.
        .ACTE
        .....
        ",
    );
    assert_eq!(
        game.snapshot(None).cargo.expect("地圖已設定貨物").items,
        vec![
            CargoState::Ground(marked_cell(&layout, 'C')),
            CargoState::Ground(marked_cell(&layout, 'F')),
        ]
    );
    loot(&mut game, marked_cell(&layout, 'C'));
    let GridPos { x, y } = marked_cell(&layout, 'D');
    let movement = game.command(Command::Move {
        actor: TARGET_ID,
        x,
        y,
    });
    assert_eq!(movement.is_err(), true, "T 取貨後本回合不能移動");
    let snapshot = game.snapshot(None);
    assert_eq!(snapshot.turn.phase, Phase::Ended);
    let carrier = unit(&snapshot, TARGET_ID);
    assert_eq!(
        GridPos {
            x: carrier.x,
            y: carrier.y
        },
        marked_cell(&layout, 'T')
    );
    assert_eq!(
        snapshot.cargo.expect("地圖已設定貨物").items,
        vec![
            CargoState::Carried(TARGET_ID),
            CargoState::Ground(marked_cell(&layout, 'F'))
        ]
    );

    next_actor(&mut game, SECOND_RAIDER);
    let GridPos { x, y } = marked_cell(&layout, 'C');
    let empty_loot = game.command(Command::Skill {
        actor: SECOND_RAIDER,
        x,
        y,
        skill: "loot".into(),
    });
    assert_eq!(empty_loot.is_err(), true, "U 不能從已取空的 C 取貨");
    let GridPos { x, y } = marked_cell(&layout, 'F');
    game.command(Command::Skill {
        actor: SECOND_RAIDER,
        x,
        y,
        skill: "loot".into(),
    })
    .expect("U 取空位置失敗後仍可從 F 取得另一份貨物");
    let GridPos { x, y } = marked_cell(&layout, 'C');
    let movement = game.command(Command::Move {
        actor: SECOND_RAIDER,
        x,
        y,
    });
    assert_eq!(movement.is_err(), true, "U 取貨後本回合不能移動");
    let snapshot = game.snapshot(None);
    assert_eq!(snapshot.turn.phase, Phase::Ended);
    let carrier = unit(&snapshot, SECOND_RAIDER);
    assert_eq!(
        GridPos {
            x: carrier.x,
            y: carrier.y
        },
        marked_cell(&layout, 'U')
    );
    assert_eq!(snapshot.outcome, Outcome::Ongoing);
    assert_eq!(
        snapshot.cargo.expect("地圖已設定貨物").items,
        vec![
            CargoState::Carried(TARGET_ID),
            CargoState::Carried(SECOND_RAIDER)
        ]
    );

    next_actor(&mut game, ACTOR_ID);
    next_actor(&mut game, TARGET_ID);
    move_to(&mut game, TARGET_ID, marked_cell(&layout, 'D'));
    next_actor(&mut game, SECOND_RAIDER);
    move_to(&mut game, SECOND_RAIDER, marked_cell(&layout, 'C'));
    let snapshot = game.snapshot(None);
    for (id, destination) in [(TARGET_ID, 'D'), (SECOND_RAIDER, 'C')] {
        let carrier = unit(&snapshot, id);
        assert_eq!(
            GridPos {
                x: carrier.x,
                y: carrier.y
            },
            marked_cell(&layout, destination)
        );
        assert_eq!(carrier.carrying_cargo, true);
    }
    assert_eq!(snapshot.outcome, Outcome::Ongoing);
}

// 驗證是否攜貨、移動落點與其他敵人是否存活，共同決定逃離是否判敗。
#[test]
fn raider_movement_resolves_cargo_escape() {
    for (name, diagram, should_loot, destination, expected_outcome) in [
        // 最後一名攜貨強盜抵達 E 仍判敗，不能因敵人清空而獲勝。
        (
            "最後一名攜貨者逃離",
            "
            .....
            .ACTE
            .....
            ",
            true,
            'E',
            Outcome::Defeat,
        ),
        // 其他強盜仍在場時，攜貨者抵達 E 同樣立即判敗。
        (
            "其他強盜仍在場時逃離",
            "
            U....
            .ACTE
            .....
            ",
            true,
            'E',
            Outcome::Defeat,
        ),
        // 未取貨的強盜抵達 E 不觸發貨物損失的敗北。
        (
            "空手抵達出口",
            "
            U....
            .ACTE
            .....
            ",
            false,
            'E',
            Outcome::Ongoing,
        ),
        // 攜貨者抵達非出口邊界 D 仍留在場上並保有貨物。
        (
            "攜貨抵達非出口邊界",
            "
            ...D.
            .ACTE
            .....
            ",
            true,
            'D',
            Outcome::Ongoing,
        ),
    ] {
        let (mut game, layout) = game(diagram);
        if should_loot {
            loot(&mut game, marked_cell(&layout, 'C'));
            next_raider_turn(&mut game);
        }
        move_to(&mut game, TARGET_ID, marked_cell(&layout, destination));
        let snapshot = game.snapshot(None);
        assert_eq!(snapshot.outcome, expected_outcome, "{name}");
        if expected_outcome == Outcome::Defeat {
            assert_eq!(
                snapshot.units.iter().any(|unit| unit.id == ACTOR_ID),
                true,
                "{name}"
            );
            assert_eq!(
                snapshot.units.iter().any(|unit| unit.id == TARGET_ID),
                false,
                "{name}"
            );
            let snapshot = game
                .command(Command::Continue)
                .expect("結束後命令應維持結果");
            assert_eq!(snapshot.outcome, Outcome::Defeat, "{name}");
        } else if should_loot {
            assert_eq!(carrying(&game, TARGET_ID), json!(true), "{name}");
        }
    }
}

// 驗證擊倒攜貨者後貨物留在陣亡格，並依其他強盜是否存活決定勝利。
#[test]
fn defeating_raider_preserves_cargo_and_resolves_outcome() {
    for (name, diagram, should_loot, expected_outcome) in [
        // 擊倒攜貨者後貨物掉落，其他強盜仍在場時戰鬥繼續。
        (
            "掉落貨物但仍有其他強盜",
            "
            U..D.
            .ACTE
            .....
            ",
            true,
            Outcome::Ongoing,
        ),
        // 貨物未被取走時擊倒最後一名強盜即獲勝。
        (
            "擊倒未掠奪的最後一名強盜",
            "
            ...D.
            .ACTE
            .....
            ",
            false,
            Outcome::Victory,
        ),
        // 擊倒最後一名攜貨者即獲勝，不必回收掉落貨物。
        (
            "擊倒最後一名攜貨者",
            "
            ...D.
            .ACTE
            .....
            ",
            true,
            Outcome::Victory,
        ),
    ] {
        let (mut game, layout) = game(diagram);
        if should_loot {
            loot(&mut game, marked_cell(&layout, 'C'));
        }
        if layout.markers.contains_key(&'U') {
            next_actor(&mut game, SECOND_RAIDER);
        }
        next_actor(&mut game, ACTOR_ID);
        move_to(&mut game, ACTOR_ID, marked_cell(&layout, 'D'));
        strike(&mut game, ACTOR_ID, marked_cell(&layout, 'T'));
        let snapshot = game.snapshot(None);
        assert_eq!(snapshot.outcome, expected_outcome, "{name}");
        assert_eq!(
            snapshot.units.iter().any(|unit| unit.id == TARGET_ID),
            false,
            "{name}"
        );
        let expected_position = marked_cell(&layout, if should_loot { 'T' } else { 'C' });
        assert_eq!(
            snapshot.cargo.expect("地圖已設定貨物").items,
            vec![CargoState::Ground(expected_position)],
            "{name}"
        );
    }
}

// 驗證攜貨者陣亡後，另一名強盜可從陣亡格取得掉落貨物。
#[test]
fn cargo_remains_at_the_carriers_death_position() {
    let (mut game, layout) = game(
        "
        U..D.
        .ACTE
        .....
        ",
    );
    loot(&mut game, marked_cell(&layout, 'C'));
    next_actor(&mut game, SECOND_RAIDER);
    next_actor(&mut game, ACTOR_ID);
    move_to(&mut game, ACTOR_ID, marked_cell(&layout, 'D'));
    strike(&mut game, ACTOR_ID, marked_cell(&layout, 'T'));
    next_actor(&mut game, SECOND_RAIDER);
    move_to(&mut game, SECOND_RAIDER, marked_cell(&layout, 'C'));
    let GridPos { x, y } = marked_cell(&layout, 'T');
    game.command(Command::Skill {
        actor: SECOND_RAIDER,
        x,
        y,
        skill: "loot".into(),
    })
    .expect("另一名強盜應可從攜貨者陣亡格取得掉落貨物");
    assert_eq!(carrying(&game, SECOND_RAIDER), json!(true));
    assert_eq!(game.snapshot(None).outcome, Outcome::Ongoing);
}

// 驗證攜貨者陣亡後，其他強盜仍不能從已取空的初始位置取得貨物。
#[test]
fn defeating_carrier_does_not_replenish_the_initial_cargo_position() {
    let (mut game, layout) = game(
        "
        U..D.
        .ACTE
        .....
        ",
    );
    loot(&mut game, marked_cell(&layout, 'C'));
    next_actor(&mut game, SECOND_RAIDER);
    next_actor(&mut game, ACTOR_ID);
    move_to(&mut game, ACTOR_ID, marked_cell(&layout, 'D'));
    strike(&mut game, ACTOR_ID, marked_cell(&layout, 'T'));
    next_actor(&mut game, SECOND_RAIDER);
    move_to(&mut game, SECOND_RAIDER, marked_cell(&layout, 'A'));
    let GridPos { x, y } = marked_cell(&layout, 'C');
    let result = game.command(Command::Skill {
        actor: SECOND_RAIDER,
        x,
        y,
        skill: "loot".into(),
    });
    assert_eq!(result.is_err(), true);
    assert_eq!(carrying(&game, SECOND_RAIDER), json!(false));
    assert_eq!(game.snapshot(None).outcome, Outcome::Ongoing);
}

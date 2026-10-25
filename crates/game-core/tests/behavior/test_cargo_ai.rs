use crate::support::{
    ACTOR_ID, AsciiBoard, TARGET_ID, ascii_board, end_turn_and_continue, map_from_ascii_board,
    marked_cell, unit,
};
use game_core::{CargoState, Command, Game, Phase, Snapshot, Team, authoring};

const DEFINITIONS: &str = include_str!("../data/utility_ai_definitions.toml");
const OTHER_ID: i64 = 3;

// 沿用 AI 專用資料與公開作者入口；C/F 是貨物、E 是出口、D 是預期停點。
fn setup(diagram: &str, movement: u32, has_loot: bool) -> (Game, AsciiBoard) {
    let layout = ascii_board(diagram);
    let mut definitions: authoring::Definitions =
        toml::from_str(DEFINITIONS).expect("AI 測試資料應可載入");
    definitions.active_skills.push(
        toml::from_str("id = 'loot'\nmin_range = 1\nmax_range = 1\neffect = 'loot'")
            .expect("測試掠奪技能應可載入"),
    );
    let raider = definitions
        .unit_types
        .iter_mut()
        .find(|unit| unit.id == "tank")
        .expect("測試資料應包含 tank");
    raider.movement = movement;
    if has_loot {
        raider.active_skills.push("loot".into());
    }
    let mut other = raider.clone();
    other.id = "other_raider".into();
    other.initiative = 200;
    definitions.unit_types.push(other);
    let mut units = vec![
        ('A', ACTOR_ID, "tank", Team::Enemy("bandits".into())),
        ('T', TARGET_ID, "target", Team::Player),
    ];
    if layout.markers.contains_key(&'B') {
        units.push(('B', OTHER_ID, "other_raider", Team::Enemy("bandits".into())));
    }
    let mut map = map_from_ascii_board("cargo_ai", &layout, &units);
    if layout.markers.contains_key(&'C') {
        map.cargo = Some(authoring::CargoObjective {
            positions: ['C', 'F']
                .into_iter()
                .filter(|marker| layout.markers.contains_key(marker))
                .map(|marker| marked_cell(&layout, marker))
                .collect(),
            exits: vec![marked_cell(&layout, 'E')],
        });
    }
    let mut game = Game::from_authoring(definitions, map, authoring::Difficulty::Easy)
        .expect("AI 貨物測試地圖應可載入");
    game.set_random_seed(1);
    game.command(Command::Start).expect("測試戰鬥應可開始");
    (game, layout)
}

fn act(game: &mut Game) -> Snapshot {
    assert_eq!(game.snapshot(None).turn.actor, Some(ACTOR_ID));
    let snapshot = game.command(Command::Continue).expect("AI 應可完成回合");
    assert_eq!(snapshot.turn.phase, Phase::Ended);
    snapshot
}

fn assert_position(snapshot: &Snapshot, layout: &AsciiBoard, marker: char) {
    let actor = unit(snapshot, ACTOR_ID);
    let expected = marked_cell(layout, marker);
    assert_eq!((actor.x, actor.y), (expected.x, expected.y));
}

fn take_cargo(game: &mut Game, layout: &AsciiBoard, actor: i64) {
    let position = marked_cell(layout, 'C');
    game.command(Command::Skill {
        actor,
        x: position.x,
        y: position.y,
        skill: "loot".into(),
    })
    .expect("準備情境時應可手動撿貨");
}

// 驗證原地或第一段移動能撿貨時，優先撿貨而不攻擊可打到的玩家。
#[test]
fn loot_before_attacking_when_reachable_in_one_segment() {
    for (name, diagram) in [
        (
            "原地撿貨",
            "
            TAC.E
        ",
        ),
        (
            "移動後撿貨",
            "
            A.DC.E
            ..T...
        ",
        ),
    ] {
        let (mut game, layout) = setup(diagram, 2, true);
        let snapshot = act(&mut game);
        assert_position(
            &snapshot,
            &layout,
            if name == "原地撿貨" { 'A' } else { 'D' },
        );
        assert_eq!(unit(&snapshot, ACTOR_ID).carrying_cargo, true, "{name}");
        assert_eq!(unit(&snapshot, TARGET_ID).hp, 100, "{name}：不得改選攻擊");
        assert_eq!(
            snapshot.cargo.as_ref().expect("應有貨物").items,
            vec![CargoState::Carried(ACTOR_ID)]
        );
    }
}

// 驗證需兩段移動時，能沿途攻擊就只走第一段；沒有攻擊機會才走兩段。
#[test]
fn approach_cargo_and_attack_instead_of_using_second_segment() {
    for (diagram, attacks) in [
        (
            "
            A.D..C.E
            ..T.....
        ",
            true,
        ),
        (
            "
            A...D.C.E
            T########
        ",
            false,
        ),
    ] {
        let (mut game, layout) = setup(diagram, 2, true);
        let snapshot = act(&mut game);
        assert_position(&snapshot, &layout, 'D');
        assert_eq!(
            unit(&snapshot, TARGET_ID).hp,
            if attacks { 90 } else { 100 }
        );
        assert_eq!(unit(&snapshot, ACTOR_ID).carrying_cargo, false);
    }
}

// 驗證依繞牆與粗糙地面的實際路程選擇 F，停在 D 撿貨。
#[test]
fn choose_cargo_by_walkable_path_cost() {
    for diagram in [
        "
            A#C..T
            .#....
            ......
            D.....
            F....E
        ",
        "
            A~.C.T
            ......
            D.....
            F....E
        ",
        "
            A~DF.T
            ~~~...
            ~~~...
            C....E
        ",
    ] {
        let (mut game, layout) = setup(diagram, 4, true);
        let snapshot = act(&mut game);
        assert_position(&snapshot, &layout, 'D');
        assert_eq!(
            snapshot.cargo.as_ref().expect("應有貨物").items,
            vec![
                CargoState::Ground(marked_cell(&layout, 'C')),
                CargoState::Carried(ACTOR_ID),
            ]
        );
        assert_eq!(unit(&snapshot, TARGET_ID).hp, 100);
    }
}

// 驗證近處貨物不可達時改撿其他貨物，所有貨物不可達時恢復攻擊。
#[test]
fn unreachable_cargo_does_not_block_other_actions() {
    for (diagram, carries, target_hp) in [
        (
            "
            A#C#T
            .####
            F...E
        ",
            true,
            100,
        ),
        (
            "
            AT#C#E
        ",
            false,
            90,
        ),
    ] {
        let (mut game, _) = setup(diagram, 2, true);
        let snapshot = act(&mut game);
        assert_eq!(unit(&snapshot, ACTOR_ID).carrying_cargo, carries);
        assert_eq!(unit(&snapshot, TARGET_ID).hp, target_hp);
    }
}

// 驗證另一名強盜先撿走貨物後，AI 改選剩餘貨物，不追原本的空格。
#[test]
fn reselect_cargo_after_another_raider_takes_it() {
    let (mut game, layout) = setup(
        "
        A.CB.T
        ......
        F....E
    ",
        2,
        true,
    );
    assert_eq!(game.snapshot(None).turn.actor, Some(OTHER_ID));
    take_cargo(&mut game, &layout, OTHER_ID);
    end_turn_and_continue(&mut game);
    let snapshot = act(&mut game);
    assert_eq!(
        snapshot.cargo.as_ref().expect("應有貨物").items,
        vec![CargoState::Carried(OTHER_ID), CargoState::Carried(ACTOR_ID),]
    );
}

// 驗證攜貨者朝出口前進，能順路攻擊時停下攻擊，否則用兩段移動逃跑。
#[test]
fn carrier_escapes_and_may_attack_on_the_way() {
    for (diagram, target_hp) in [
        (
            "
            CA.DF..E
            ...T....
        ",
            90,
        ),
        (
            "
            CA...D.E
            .T######
        ",
            100,
        ),
    ] {
        let (mut game, layout) = setup(diagram, 2, true);
        take_cargo(&mut game, &layout, ACTOR_ID);
        let snapshot = end_turn_and_continue(&mut game);
        assert_eq!(snapshot.turn.actor, Some(TARGET_ID));
        end_turn_and_continue(&mut game);
        // 固定攻擊骰，避免跨輪先攻擲骰使本案例剛好失手。
        game.set_random_seed(1);
        let snapshot = act(&mut game);
        assert_position(&snapshot, &layout, 'D');
        assert_eq!(unit(&snapshot, ACTOR_ID).carrying_cargo, true);
        assert_eq!(unit(&snapshot, TARGET_ID).hp, target_hp);
        let cargo = snapshot.cargo.as_ref().expect("應有貨物");
        assert_eq!(cargo.items[0], CargoState::Carried(ACTOR_ID));
        if layout.markers.contains_key(&'F') {
            assert_eq!(
                cargo.items[1],
                CargoState::Ground(marked_cell(&layout, 'F'))
            );
        }
    }
}

// 驗證沒有貨物、沒有地面貨物或沒有 Loot 技能時，仍使用原有攻擊行為。
#[test]
fn normal_combat_without_available_loot() {
    for (diagram, has_loot, taken) in [
        (
            "
            AT.E
        ",
            true,
            false,
        ),
        (
            "
            AT.CE
        ",
            false,
            false,
        ),
        (
            "
            AT.CBE
        ",
            true,
            true,
        ),
    ] {
        let (mut game, layout) = setup(diagram, 2, has_loot);
        if taken {
            take_cargo(&mut game, &layout, OTHER_ID);
            end_turn_and_continue(&mut game);
        }
        let snapshot = act(&mut game);
        assert_eq!(unit(&snapshot, TARGET_ID).hp, 90);
        assert_eq!(unit(&snapshot, ACTOR_ID).carrying_cargo, false);
    }
}

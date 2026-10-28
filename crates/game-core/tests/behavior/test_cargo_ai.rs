use crate::support::{
    ACTOR_ID, TARGET_ID, ascii_board, end_turn_and_continue, map_from_ascii_board, marked_cell,
    unit,
};
use game_core::{CargoState, Command, Game, Team, authoring};

const DEFINITIONS: &str = include_str!("../data/utility_ai_definitions.toml");
const OTHER_ID: i64 = 3;

// 驗證兩名強盜分工：較近者分回合趕路取貨，移動力由 4 降為 1 後仍專心撤離。
#[test]
fn raiders_split_roles_and_focus_on_looting_then_escaping() {
    // A 負責取貨，B 先行動，T 是玩家，C 是貨物，D 是取貨停點，E 是出口；P/R/S/V/W 是途中停點。
    let layout = ascii_board(
        "
        EAW.V.S.RPDC
        .T..........
        .B..........
    ",
    );
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
        .expect("測試資料應包含強盜種類");
    raider.active_skills.push("loot".into());
    let mut other = raider.clone();
    // 取貨位置距起點九格，兩段移動共八格仍無法在第一回合抵達。
    raider.movement = 4;
    other.id = "other_raider".into();
    other.initiative = 200;
    definitions.unit_types.push(other);
    let mut map = map_from_ascii_board(
        "cargo_ai",
        &layout,
        &[
            ('A', ACTOR_ID, "tank", Team::Enemy("bandits".into())),
            ('B', OTHER_ID, "other_raider", Team::Enemy("bandits".into())),
            ('T', TARGET_ID, "target", Team::Player),
        ],
    );
    map.cargo = Some(authoring::CargoObjective {
        positions: vec![marked_cell(&layout, 'C')],
        exits: vec![marked_cell(&layout, 'E')],
    });
    let mut game = Game::from_authoring(definitions, map, authoring::Difficulty::Easy)
        .expect("AI 貨物測試地圖應可載入");
    game.set_random_seed(1);
    let snapshot = game.command(Command::Start).expect("測試戰鬥應可開始");

    // B 雖然先行動，仍把貨物留給較近的 A，自己攻擊玩家。
    assert_eq!(snapshot.turn.actor, Some(OTHER_ID));
    let snapshot = game.command(Command::Continue).expect("B 應可完成戰鬥回合");
    assert_eq!(unit(&snapshot, OTHER_ID).carrying_cargo, false);
    assert_eq!(unit(&snapshot, TARGET_ID).hp, 90);
    end_turn_and_continue(&mut game);

    // 接著只讓 A 執行 AI；其他單位略過回合，固定觀察取貨與撤離的完整過程。
    for (stage, destination, expected_cargo) in [
        // 第一回合用完兩段移動，停在 P，尚未抵達取貨位置。
        (
            "走向貨物",
            'P',
            CargoState::Ground(marked_cell(&layout, 'C')),
        ),
        // 第二回合再走一格到 D 並取貨。
        ("取得貨物", 'D', CargoState::Carried(ACTOR_ID)),
        // 攜貨後每回合兩段共走兩格，先退到 R。
        ("撤離第一段", 'R', CargoState::Carried(ACTOR_ID)),
        // 繼續朝出口走兩格到 S，不停下攻擊。
        ("撤離第二段", 'S', CargoState::Carried(ACTOR_ID)),
        // 繼續朝出口走兩格到 V，不停下攻擊。
        ("撤離第三段", 'V', CargoState::Carried(ACTOR_ID)),
        // 繼續朝出口走兩格到 W，不停下攻擊。
        ("撤離第四段", 'W', CargoState::Carried(ACTOR_ID)),
        // 最後走兩格抵達出口並離場。
        ("帶貨逃走", 'E', CargoState::Escaped),
    ] {
        assert_eq!(game.snapshot(None).turn.actor, Some(ACTOR_ID), "{stage}");
        let snapshot = game
            .command(Command::Continue)
            .expect("A 應可完成取貨或撤離回合");
        if destination == 'E' {
            assert_eq!(
                snapshot.units.iter().any(|unit| unit.id == ACTOR_ID),
                false,
                "{stage}：攜貨者應離場"
            );
        } else {
            let actor = unit(&snapshot, ACTOR_ID);
            let expected_position = marked_cell(&layout, destination);
            assert_eq!(
                (actor.x, actor.y),
                (expected_position.x, expected_position.y),
                "{stage}"
            );
        }
        assert_eq!(
            snapshot.cargo.as_ref().expect("應有貨物").items,
            vec![expected_cargo],
            "{stage}"
        );
        assert_eq!(unit(&snapshot, TARGET_ID).hp, 90, "{stage}：A 不得停下攻擊");
        if destination != 'E' {
            for _ in 0..3 {
                let snapshot = end_turn_and_continue(&mut game);
                if snapshot.turn.actor == Some(ACTOR_ID) {
                    break;
                }
            }
        }
    }
}

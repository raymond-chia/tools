use crate::support::{
    ACTOR_ID, BLOCKER_ID, TARGET_ID, TEST_SKILL_ID, ascii_board, marked_cell, skill_test_data, unit,
};
use game_core::{
    ActiveSkillEffect, ActiveSkillPreview, CombatLogEvent, Command, Game, Phase, Team, authoring,
};

const THIRD_ID: i64 = 4;
const OUTSIDE_ID: i64 = 5;

fn data(diagram: &str) -> (authoring::Definitions, authoring::Map) {
    let (mut definitions, mut map) = skill_test_data(
        1,
        1,
        ActiveSkillEffect::Sweep,
        Team::Enemy("bandits".into()),
        diagram,
    );
    let actor = definitions
        .unit_types
        .iter_mut()
        .find(|unit| unit.id == "test_actor")
        .expect("測試資料應有施放者");
    actor.physical_power = 6;
    actor.hit = 100;
    if let Some(blocker) = map.units.iter_mut().find(|unit| unit.id == BLOCKER_ID) {
        blocker.team = Team::Player;
    }
    let layout = ascii_board(diagram);
    for (marker, id) in [('C', THIRD_ID), ('O', OUTSIDE_ID)] {
        if layout.markers.contains_key(&marker) {
            let position = marked_cell(&layout, marker);
            let mut placement = map
                .units
                .iter()
                .find(|unit| unit.id == TARGET_ID)
                .expect("測試棋盤應有 T")
                .clone();
            placement.id = id;
            placement.x = position.x;
            placement.y = position.y;
            map.units.push(placement);
        }
    }
    (definitions, map)
}

fn start(definitions: authoring::Definitions, map: authoring::Map) -> Game {
    let mut game = Game::from_authoring(definitions, map, authoring::Difficulty::Easy)
        .expect("橫掃測試資料應可載入");
    game.command(Command::Start).expect("測試戰鬥應可開始");
    game.set_random_seed(2);
    game
}

// 驗證四個方向都只攻擊前方三格，友軍也受傷，身後與施放者不受傷且只消耗一次行動。
#[test]
fn sweep_hits_front_row_including_allies_in_four_directions() {
    for diagram in [
        "
        .....
        .TBC.
        ..A..
        ..O..
        .....
        ",
        "
        .....
        ..O..
        ..A..
        .CBT.
        .....
        ",
        "
        .....
        .C...
        .BAO.
        .T...
        .....
        ",
        "
        .....
        ...T.
        .OAB.
        ...C.
        .....
        ",
    ] {
        let center = marked_cell(&ascii_board(diagram), 'B');
        let (definitions, map) = data(diagram);
        let mut game = start(definitions, map);
        let preview = game
            .preview_skill(ACTOR_ID, center, TEST_SKILL_ID)
            .expect("橫掃應可預覽友軍所在的中心格");
        match preview {
            ActiveSkillPreview::Area { cells, targets } => {
                assert_eq!(cells.len(), 3);
                assert_eq!(
                    targets
                        .iter()
                        .map(|target| target.target)
                        .collect::<Vec<_>>(),
                    vec![TARGET_ID, BLOCKER_ID, THIRD_ID]
                );
                assert_eq!(targets.iter().all(|target| target.hit_damage == 6), true);
            }
            _ => panic!("橫掃應回傳範圍預覽"),
        }
        let snapshot = game
            .command(Command::Skill {
                actor: ACTOR_ID,
                x: center.x,
                y: center.y,
                skill: TEST_SKILL_ID.into(),
            })
            .expect("橫掃應成功結算");
        let targets: Vec<_> = snapshot
            .log
            .iter()
            .filter_map(|event| match event {
                CombatLogEvent::Skill { target, .. } => Some(*target),
                _ => None,
            })
            .collect();
        assert_eq!(targets, vec![TARGET_ID, BLOCKER_ID, THIRD_ID]);
        for id in [TARGET_ID, BLOCKER_ID, THIRD_ID] {
            assert_eq!(unit(&snapshot, id).hp < 100, true, "範圍內的 {id} 應受傷");
        }
        assert_eq!(unit(&snapshot, OUTSIDE_ID).hp, 100);
        assert_eq!(unit(&snapshot, ACTOR_ID).hp, 100);
        assert_eq!(snapshot.turn.phase, Phase::Ended);
    }
}

// 驗證地圖邊緣裁切、空中心格與大型目標去重，斜角與界外中心不可施放且不消耗行動。
#[test]
fn sweep_handles_edges_empty_centers_and_large_targets() {
    for (diagram, expected_cells) in [
        (
            "
        A.T
        D..
        ...
        ",
            2,
        ),
        (
            "
        .....
        .TT..
        .TT..
        .AD..
        .....
        ",
            3,
        ),
    ] {
        let layout = ascii_board(diagram);
        let center = marked_cell(&layout, 'D');
        let (definitions, map) = data(diagram);
        let mut game = start(definitions, map);
        match game
            .preview_skill(ACTOR_ID, center, TEST_SKILL_ID)
            .expect("空中心格可預覽")
        {
            ActiveSkillPreview::Area { cells, targets } => {
                assert_eq!(cells.len(), expected_cells);
                assert_eq!(targets.len(), if expected_cells == 2 { 0 } else { 1 });
            }
            _ => panic!("應回傳範圍預覽"),
        }
        let invalid = game_core::GridPos { x: -1, y: 0 };
        assert_eq!(
            game.preview_skill(ACTOR_ID, invalid, TEST_SKILL_ID)
                .is_err(),
            true
        );
        let actor = marked_cell(&layout, 'A');
        assert_eq!(
            game.command(Command::Skill {
                actor: ACTOR_ID,
                x: actor.x + 1,
                y: actor.y + 1,
                skill: TEST_SKILL_ID.into(),
            })
            .is_err(),
            true
        );
        assert_eq!(game.snapshot(None).turn.phase, Phase::Ready);
        let snapshot = game
            .command(Command::Skill {
                actor: ACTOR_ID,
                x: center.x,
                y: center.y,
                skill: TEST_SKILL_ID.into(),
            })
            .expect("空中心格可施放橫掃");
        assert_eq!(
            snapshot
                .log
                .iter()
                .filter(|event| matches!(event, CombatLogEvent::Skill { .. }))
                .count(),
            if expected_cells == 2 { 0 } else { 1 }
        );
    }
}

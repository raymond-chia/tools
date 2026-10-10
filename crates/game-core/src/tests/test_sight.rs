use super::support::{
    ACTOR_ID, AsciiBoard, TARGET_ID, TEST_SKILL_ID, ascii_board, game_with_skill_on_ascii_map,
};
use crate::*;

fn sight_game(diagram: &str, effect: SkillEffect, target_team: Team) -> Game {
    game_with_skill_on_ascii_map(0, 20, effect, target_team, diagram)
}

fn attack() -> SkillEffect {
    SkillEffect::Attack {
        attack_bonus: 100,
        power_bonus: 0,
    }
}

// 驗證 Bresenham 直線、斜線、半格取捨、端點與雙向查詢一致，單位不遮擋視線。
#[test]
fn sight_lines_use_terrain_and_are_symmetric() {
    for (name, diagram, visible) in [
        ("水平", "A..T", true),
        (
            "垂直遮擋",
            "
            A
            #
            T
            ",
            false,
        ),
        ("水平遮擋", "A#T", false),
        (
            "斜線遮擋",
            "
            A..
            .#.
            ..T
            ",
            false,
        ),
        (
            "斜線外牆",
            "
            A#.
            ...
            ..T
            ",
            true,
        ),
        (
            "半格遮擋",
            "
            A..
            .#T
            ",
            false,
        ),
        ("單位不遮擋", "ABT", true),
    ] {
        let game = sight_game(diagram, attack(), Team::Enemy("test_enemy".into()));
        let actor = game.entity(ACTOR_ID).expect("施放者應存在");
        let target = game.entity(TARGET_ID).expect("目標應存在");
        let a = game.world.get::<Pos>(actor).expect("施放者應有位置").0;
        let t = game.world.get::<Pos>(target).expect("目標應有位置").0;
        let footprint = Footprint {
            width: 1,
            height: 1,
        };
        assert_eq!(
            crate::sight::has_sight(&game.world, a, footprint, t),
            visible,
            "{name}"
        );
        assert_eq!(
            crate::sight::has_sight(&game.world, t, footprint, a),
            visible,
            "{name}反向"
        );
        assert_eq!(
            crate::sight::has_sight(&game.world, a, footprint, a),
            true,
            "自身可見"
        );
    }
}

// 驗證攻擊、推擊、治療及地面技能的提示、預覽與施放都拒絕牆後瞄準格，失敗不消耗行動。
#[test]
fn all_skill_targets_require_sight() {
    for (effect, target_team) in [
        (attack(), Team::Enemy("test_enemy".into())),
        (
            SkillEffect::Push { attack_bonus: 100 },
            Team::Enemy("test_enemy".into()),
        ),
        (SkillEffect::Heal { power_bonus: 1 }, Team::Player),
        (
            SkillEffect::Mire {
                terrain: "wall".into(),
                duration: 2,
            },
            Team::Enemy("test_enemy".into()),
        ),
    ] {
        let ground = matches!(effect, SkillEffect::Mire { .. });
        let mut game = sight_game("A#T", effect, target_team);
        let actor = game.entity(ACTOR_ID).expect("施放者應存在");
        let target = game.entity(TARGET_ID).expect("目標應存在");
        let cell = game.world.get::<Pos>(target).expect("目標應有位置").0;
        assert_eq!(
            crate::skill::skill_ranges(&game.world, actor)[0]
                .cells
                .contains(&cell),
            false
        );
        game.start().expect("戰鬥應可開始");
        let skill = game.world.resource::<Skills>().definitions[TEST_SKILL_ID].clone();
        let preview_error = if ground {
            let origin = game.world.get::<Pos>(actor).expect("施放者應有位置").0;
            crate::skill::validate_cell_skill_from_position(
                &game.world,
                actor,
                origin,
                cell,
                &skill,
            )
            .err()
            .expect("牆後不能預覽地面技能")
        } else {
            game.preview_skill(ACTOR_ID, cell, TEST_SKILL_ID)
                .err()
                .expect("牆後不能預覽")
        };
        assert_eq!(preview_error.id(), "target_not_visible");
        assert_eq!(
            game.use_skill_at_cell(ACTOR_ID, cell, skill)
                .expect_err("牆後不能施放")
                .id(),
            "target_not_visible"
        );
        assert_eq!(game.world.resource::<Turn>().phase, Phase::Ready);
    }
}

// 驗證不同尺寸單位可共存，大型施放者可由任一佔用格瞄準，大型目標的不同格各自判斷視線。
#[test]
fn sight_uses_the_selected_cell_and_actor_footprint() {
    for (name, units, aim, visible) in [
        (
            "單格施放者被遮擋",
            "
            A#T
            ..T
            ...
            ",
            "
            ..X
            ...
            ...
            ",
            false,
        ),
        (
            "大型施放者可由底部瞄準",
            "
            AAA#T
            AAA..
            AAA..
            ",
            "
            ....X
            .....
            .....
            ",
            true,
        ),
        (
            "大型目標的另一格可見",
            "
            A#TT
            ..TT
            ....
            ",
            "
            ....
            ..X.
            ....
            ",
            true,
        ),
    ] {
        let cell = marked_cell(aim, 'X');
        let mut game = sight_game(units, attack(), Team::Enemy("test_enemy".into()));
        game.start().expect("戰鬥應可開始");
        let actor = game.entity(ACTOR_ID).expect("施放者應存在");
        let origin = game.world.get::<Pos>(actor).expect("施放者應有位置").0;
        let footprint = *game
            .world
            .get::<Footprint>(actor)
            .expect("施放者應有佔用尺寸");
        assert_eq!(
            crate::sight::has_sight(&game.world, origin, footprint, cell),
            visible,
            "{name}"
        );
        match game.preview_skill(ACTOR_ID, cell, TEST_SKILL_ID) {
            Ok(_) => assert_eq!(visible, true, "{name}：遮擋格不得預覽"),
            Err(error) => {
                assert_eq!(visible, false, "{name}：可見格應可預覽");
                assert_eq!(error.id(), "target_not_visible", "{name}");
            }
        }
    }
}

// 驗證射程和視線必須由同一格同時滿足。
#[test]
fn skill_range_and_sight_require_the_same_origin_cell() {
    let over_wall_diagram = "
        AA#..T
        AA....
        ";
    let minimum_range_diagram = "
        AA.T
        AA..
        ";
    for (name, min_range, max_range, diagram, expected_error) in [
        (
            "射程內的格被遮擋",
            0,
            4,
            over_wall_diagram,
            Some("target_not_visible"),
        ),
        ("可見格符合最大射程", 0, 5, over_wall_diagram, None),
        (
            "最近格過近但另一格符合最小射程",
            3,
            3,
            minimum_range_diagram,
            None,
        ),
        (
            "所有佔用格都過近",
            5,
            5,
            minimum_range_diagram,
            Some("target_too_close"),
        ),
        (
            "所有佔用格都過遠",
            0,
            1,
            minimum_range_diagram,
            Some("target_too_far"),
        ),
    ] {
        for (effect, target_team) in [
            (attack(), Team::Enemy("test_enemy".into())),
            (
                SkillEffect::Push { attack_bonus: 100 },
                Team::Enemy("test_enemy".into()),
            ),
            (SkillEffect::Heal { power_bonus: 1 }, Team::Player),
            (
                SkillEffect::Mire {
                    terrain: "wall".into(),
                    duration: 2,
                },
                Team::Enemy("test_enemy".into()),
            ),
        ] {
            let ground = matches!(effect, SkillEffect::Mire { .. });
            let mut game =
                game_with_skill_on_ascii_map(min_range, max_range, effect, target_team, diagram);
            let actor = game.entity(ACTOR_ID).expect("施放者應存在");
            let target = game.entity(TARGET_ID).expect("目標應存在");
            let cell = game.world.get::<Pos>(target).expect("目標應有位置").0;
            let origin = game.world.get::<Pos>(actor).expect("施放者應有位置").0;
            let footprint = *game
                .world
                .get::<Footprint>(actor)
                .expect("施放者應有佔用尺寸");
            let sight = crate::sight::check_sight_in_range(
                &game.world,
                origin,
                footprint,
                cell,
                min_range,
                max_range,
            );
            assert_eq!(
                sight.as_ref().err().map(|error| error.id()),
                expected_error,
                "{name}：視線 helper 應符合射程規則與錯誤分類"
            );
            assert_eq!(
                crate::skill::skill_ranges(&game.world, actor)[0]
                    .cells
                    .contains(&cell),
                expected_error.is_none(),
                "{name}：技能提示應符合射程與視線規則"
            );
            game.start().expect("戰鬥應可開始");
            let skill = game.world.resource::<Skills>().definitions[TEST_SKILL_ID].clone();
            let preview = if ground {
                crate::skill::validate_cell_skill_from_position(
                    &game.world,
                    actor,
                    origin,
                    cell,
                    &skill,
                )
                .map(|_| ())
            } else {
                game.preview_skill(ACTOR_ID, cell, TEST_SKILL_ID)
                    .map(|_| ())
            };
            let result = game.use_skill_at_cell(ACTOR_ID, cell, skill);
            if let Some(expected_error) = expected_error {
                assert_eq!(
                    preview.expect_err("沒有符合射程與視線的佔用格").id(),
                    expected_error,
                    "{name}：預覽應回傳正確錯誤分類"
                );
                assert_eq!(
                    result.expect_err("沒有符合射程與視線的佔用格").id(),
                    expected_error,
                    "{name}：施放應回傳正確錯誤分類"
                );
                assert_eq!(game.world.resource::<Turn>().phase, Phase::Ready);
            } else {
                preview.expect("射程內的可見佔用格應允許預覽");
                result.expect("射程內的可見佔用格應允許施放");
            }
        }
    }
}

fn marked_cell(diagram: &str, marker: char) -> GridPos {
    let AsciiBoard {
        board: _,
        markers,
        terrains: _,
    } = ascii_board(diagram);
    let ((x, y), _) = markers[&marker];
    GridPos { x, y }
}

// 驗證對可見地面施放土牆後遮擋後方、牆格仍可見，直到到期才恢復視線。
#[test]
fn temporary_terrain_blocks_sight_until_removed() {
    let diagram = "AXT";
    let mut game = sight_game(
        diagram,
        SkillEffect::Mire {
            terrain: "wall".into(),
            duration: 2,
        },
        Team::Enemy("test_enemy".into()),
    );
    game.start().expect("戰鬥應可開始");
    let origin = marked_cell(diagram, 'A');
    let middle = marked_cell(diagram, 'X');
    let target = marked_cell(diagram, 'T');
    let footprint = Footprint {
        width: 1,
        height: 1,
    };
    let skill = game.world.resource::<Skills>().definitions[TEST_SKILL_ID].clone();
    game.use_skill_at_cell(ACTOR_ID, middle, skill)
        .expect("可見地面應可施放土牆");

    for (round, target_visible) in [(1, false), (2, false), (3, true)] {
        assert_eq!(game.world.resource::<model::Encounter>().round, round);
        assert_eq!(
            crate::sight::has_sight(&game.world, origin, footprint, target),
            target_visible,
            "第 {round} 輪的後方視線應符合土牆到期狀態"
        );
        assert_eq!(
            crate::sight::has_sight(&game.world, origin, footprint, middle),
            true,
            "第 {round} 輪的土牆所在格應可見"
        );
        if round < 3 {
            for _ in 0..2 {
                let actor = game.world.resource::<Turn>().actor.expect("應有當前單位");
                game.command(Command::EndTurn { actor })
                    .expect("當前單位應可結束回合");
                game.command(Command::Continue)
                    .expect("結束回合後應可推進戰鬥");
            }
        }
    }
}

use super::support::{
    ACTOR_ID, AsciiBoard, TARGET_ID, TEST_SKILL_ID, ascii_board, sight_game_in_range,
};
use crate::model::{ActiveSkills, Footprint, Pos};
use crate::{ActiveSkillEffect, Command, Game, GridPos, PowerSource, Team};

fn sight_game(diagram: &str, effect: ActiveSkillEffect, target_team: Team) -> Game {
    sight_game_in_range(diagram, effect, target_team, 0, 20)
}

fn attack() -> ActiveSkillEffect {
    ActiveSkillEffect::Attack {
        power_source: PowerSource::Physical,
        ranged: false,
        hit_bonus: 100,
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
        game.command(Command::Start).expect("戰鬥應可開始");
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

fn marked_cell(diagram: &str, marker: char) -> GridPos {
    let AsciiBoard {
        board: _,
        markers,
        terrains: _,
    } = ascii_board(diagram);
    let ((x, y), _) = markers[&marker];
    GridPos { x, y }
}

// 驗證地面技能預覽拒絕遮擋、過近與過遠的格子，射程和視線須由同一佔用格滿足。
#[test]
fn ground_skill_preview_requires_range_and_sight() {
    for (name, diagram, min_range, max_range, expected_error) in [
        ("牆後目標", "A#T", 0, 20, Some("target_not_visible")),
        (
            "射程內的格被遮擋",
            "
            AA#..T
            AA....
        ",
            0,
            4,
            Some("target_not_visible"),
        ),
        (
            "可見格符合最大射程",
            "
            AA#..T
            AA....
        ",
            0,
            5,
            None,
        ),
        (
            "另一格符合最小射程",
            "
            AA.T
            AA..
        ",
            3,
            3,
            None,
        ),
        (
            "所有佔用格過近",
            "
            AA.T
            AA..
        ",
            5,
            5,
            Some("target_too_close"),
        ),
        (
            "所有佔用格過遠",
            "
            AA.T
            AA..
        ",
            0,
            1,
            Some("target_too_far"),
        ),
    ] {
        let game = sight_game_in_range(
            diagram,
            ActiveSkillEffect::Mire {
                terrain: "wall".into(),
                duration: 2,
            },
            Team::Enemy("test_enemy".into()),
            min_range,
            max_range,
        );
        let actor = game.entity(ACTOR_ID).expect("施放者應存在");
        let origin = game.world.get::<Pos>(actor).expect("施放者應有位置").0;
        let cell = marked_cell(diagram, 'T');
        let skill = &game.world.resource::<ActiveSkills>().definitions[TEST_SKILL_ID];
        let result = crate::skill::validate_cell_skill_from_position(
            &game.world,
            actor,
            origin,
            cell,
            skill,
        );
        assert_eq!(
            result.as_ref().err().map(|error| error.id()),
            expected_error,
            "{name}"
        );
    }
}

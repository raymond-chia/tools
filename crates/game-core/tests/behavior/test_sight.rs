use super::support::{
    ACTOR_ID, TARGET_ID, TEST_SKILL_ID, ascii_board, end_turn_and_continue, marked_cell,
    sight_game_in_range, unit,
};
use game_core::{Command, Game, GridPos, Phase, SkillEffect, Team};

fn sight_game(diagram: &str, effect: SkillEffect, target_team: Team) -> Game {
    sight_game_in_range(diagram, effect, target_team, 0, 20)
}

fn attack() -> SkillEffect {
    SkillEffect::Attack {
        hit_bonus: 100,
        power_bonus: 0,
    }
}

// 驗證攻擊、推擊、治療及地面技能的提示、預覽與施放都拒絕牆後瞄準格，失敗不消耗行動。
#[test]
fn all_skill_targets_require_sight() {
    for (effect, target_team) in [
        (attack(), Team::Enemy("test_enemy".into())),
        (
            SkillEffect::Push { hit_bonus: 100 },
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
        game.command(Command::Start).expect("戰鬥應可開始");
        let snapshot = game.snapshot(None);
        let target = unit(&snapshot, TARGET_ID);
        let cell = GridPos {
            x: target.x,
            y: target.y,
        };
        assert_eq!(
            game.snapshot(None).skill_ranges[0].cells.contains(&cell),
            false
        );
        if !ground {
            let error = game
                .preview_skill(ACTOR_ID, cell, TEST_SKILL_ID)
                .err()
                .expect("牆後不能預覽");
            assert_eq!(error.id(), "target_not_visible");
        }
        assert_eq!(
            game.command(Command::Skill {
                actor: ACTOR_ID,
                x: cell.x,
                y: cell.y,
                skill: TEST_SKILL_ID.into()
            })
            .err()
            .expect("牆後不能施放")
            .id(),
            "target_not_visible"
        );
        assert_eq!(game.snapshot(None).turn.phase, Phase::Ready);
    }
}

// 驗證公開技能範圍、預覽與施放要求同一佔用格同時滿足射程和視線。
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
                SkillEffect::Push { hit_bonus: 100 },
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
            let mut game = sight_game_in_range(diagram, effect, target_team, min_range, max_range);
            game.command(Command::Start).expect("戰鬥應可開始");
            let snapshot = game.snapshot(None);
            let target = unit(&snapshot, TARGET_ID);
            let cell = GridPos {
                x: target.x,
                y: target.y,
            };
            assert_eq!(
                game.snapshot(None).skill_ranges[0].cells.contains(&cell),
                expected_error.is_none(),
                "{name}：技能提示應符合射程與視線規則"
            );
            if !ground {
                let preview = game
                    .preview_skill(ACTOR_ID, cell, TEST_SKILL_ID)
                    .map(|_| ());
                assert_eq!(
                    preview.as_ref().err().map(|error| error.id()),
                    expected_error,
                    "{name}：預覽錯誤分類"
                );
            }
            let result = game.command(Command::Skill {
                actor: ACTOR_ID,
                x: cell.x,
                y: cell.y,
                skill: TEST_SKILL_ID.into(),
            });
            if let Some(expected_error) = expected_error {
                assert_eq!(
                    result.err().expect("沒有符合射程與視線的佔用格").id(),
                    expected_error,
                    "{name}：施放應回傳正確錯誤分類"
                );
                assert_eq!(game.snapshot(None).turn.phase, Phase::Ready);
            } else {
                result.expect("射程內的可見佔用格應允許施放");
            }
        }
    }
}

// 驗證公開土牆命令施放後當輪不能再用技能，之後後方格移出技能範圍、牆格仍可瞄準，到期後恢復範圍。
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
    game.command(Command::Start).expect("戰鬥應可開始");
    let layout = ascii_board(diagram);
    let middle = marked_cell(&layout, 'X');
    let target = marked_cell(&layout, 'T');
    game.command(Command::Skill {
        actor: ACTOR_ID,
        x: middle.x,
        y: middle.y,
        skill: TEST_SKILL_ID.into(),
    })
    .expect("可見地面應可施放土牆");

    assert_eq!(game.snapshot(None).round, 1);
    assert_eq!(
        game.snapshot(None)
            .terrain_effects
            .iter()
            .any(|terrain| terrain.x == middle.x
                && terrain.y == middle.y
                && terrain.effect == "wall"),
        true
    );
    for cell in [target, middle] {
        assert_eq!(
            game.preview_skill(ACTOR_ID, cell, TEST_SKILL_ID)
                .err()
                .map(|error| error.id()),
            Some("cannot_use_skill"),
            "第 1 輪施放後不能再使用技能"
        );
    }
    for (round, target_visible) in [(2, false), (3, true)] {
        for _ in 0..2 {
            end_turn_and_continue(&mut game);
        }
        let snapshot = game.snapshot(None);
        assert_eq!(snapshot.round, round);
        assert_eq!(
            snapshot.skill_ranges[0].cells.contains(&target),
            target_visible,
            "第 {round} 輪的技能範圍應符合土牆到期狀態"
        );
        assert_eq!(
            snapshot.skill_ranges[0].cells.contains(&middle),
            true,
            "第 {round} 輪的土牆所在格應可瞄準"
        );
        assert_eq!(
            snapshot
                .terrain_effects
                .iter()
                .any(|terrain| terrain.effect == "wall"),
            !target_visible,
            "土牆到期後應從公開地形快照移除"
        );
    }
}

use super::support::{ACTOR_ID, AsciiBoard, ascii_board, game_with_skill_on_ascii_map};
use crate::model::Exploration;
use crate::*;

struct MovementPreviewCase {
    name: &'static str,
    movement: u32,
    interrupted: bool,
    path_length: usize,
    second_path_length: usize,
    total_cost: u32,
}

// 驗證玩家預覽優先第一段路徑；第一段只能踩尖刺時，不為避傷改用第二段。
#[test]
fn move_preview_chooses_safest_path_within_first_phase() {
    let cases = [
        MovementPreviewCase {
            name: "第一階段可以安全繞路",
            movement: 4,
            interrupted: false,
            path_length: 5,
            second_path_length: 0,
            total_cost: 4,
        },
        MovementPreviewCase {
            name: "第一段踩尖刺，不改走第二段安全繞路",
            movement: 2,
            interrupted: true,
            path_length: 2,
            second_path_length: 0,
            total_cost: 1,
        },
        MovementPreviewCase {
            name: "兩段仍無法安全繞路時停在尖刺",
            movement: 1,
            interrupted: true,
            path_length: 2,
            second_path_length: 0,
            total_cost: 1,
        },
    ];

    for MovementPreviewCase {
        name,
        movement,
        interrupted: expected_interrupted,
        path_length,
        second_path_length,
        total_cost: expected_total_cost,
    } in cases
    {
        let (game, actor_position, spikes, destination) = movement_preview_game(movement);
        let MovePreview {
            first,
            second,
            interrupted,
            total_cost,
        } = game
            .preview_move(ACTOR_ID, destination)
            .expect("目的地應能在第一階段朝目標移動");
        let expected_last = if expected_interrupted {
            spikes
        } else {
            destination
        };

        assert_eq!(interrupted, expected_interrupted, "{}", name);
        assert_eq!(first.first(), Some(&actor_position), "{}", name);
        assert_eq!(
            second.last().or_else(|| first.last()),
            Some(&expected_last),
            "{}",
            name
        );
        assert_eq!(first.len(), path_length, "{}", name);
        assert_eq!(first.contains(&spikes), expected_interrupted, "{}", name);
        assert_eq!(total_cost, expected_total_cost, "{}", name);
        assert_eq!(second.len(), second_path_length, "{}", name);
    }
}

// 驗證大型單位開場已踩中尖刺時首次移動仍扣血，後續持續路過時也再次扣血。
#[test]
fn large_unit_movement_damages_existing_terrain_coverage() {
    // 單位、地形與落點分層畫在同尺寸棋盤，讓尖刺可與大型單位重疊；路線為 A → T → A。
    let units = "
        AA..
        AA..
    ";
    let AsciiBoard {
        board: destination_board,
        markers: destination_markers,
        terrains: _,
    } = ascii_board(
        "
        .T..
        ....
    ",
    );
    let (next, _) = *destination_markers.get(&'T').expect("落點圖應包含 T");
    let next = GridPos {
        x: next.0,
        y: next.1,
    };
    for (name, terrain_diagram) in [
        (
            "左上角所在列",
            "
            .^..
            ....
        ",
        ),
        (
            "其他佔用列",
            "
            ....
            .^..
        ",
        ),
    ] {
        let AsciiBoard {
            board: unit_board,
            markers,
            terrains: _,
        } = ascii_board(units);
        let AsciiBoard {
            board,
            markers: _,
            terrains,
        } = ascii_board(terrain_diagram);
        assert_eq!(unit_board, board, "單位圖與地形圖尺寸應相同");
        assert_eq!(unit_board, destination_board, "單位圖與落點圖尺寸應相同");
        let (start, _) = *markers.get(&'A').expect("單位圖應包含 A");
        let start = GridPos {
            x: start.0,
            y: start.1,
        };
        let mut game = movement_game(
            4,
            &AsciiBoard {
                board: unit_board,
                markers,
                terrains,
            },
        );
        let entity = game.entity(ACTOR_ID).expect("測試玩家應存在");
        for (path, expected_hp, expected_events) in
            [(vec![start, next], 97, 1), (vec![next, start], 94, 2)]
        {
            assert_eq!(game.execute_move_path(entity, ACTOR_ID, &path), 1, "{name}");
            assert_eq!(
                game.world.get::<Hp>(entity).expect("玩家應有 HP").current,
                expected_hp,
                "{name}"
            );
            assert_eq!(
                game.world.resource::<Log>().0.len(),
                expected_events,
                "{name}"
            );
        }
    }
}

// 驗證第二段分次移動會扣除剩餘額度，範圍、預覽與執行均不可重新取得完整預算。
#[test]
fn second_movement_segment_preserves_remaining_budget() {
    #[derive(Debug)]
    enum Check {
        RemainingBudget,
        MovementRange,
        Preview,
        Execution,
    }
    let mut failures = Vec::new();
    for check in [
        Check::RemainingBudget,
        Check::MovementRange,
        Check::Preview,
        Check::Execution,
    ] {
        let mut game = game_with_skill_on_ascii_map(
            1,
            1,
            SkillEffect::Push {
                attack_bonus: 100,
                power_bonus: 0,
            },
            "
            A....
            ..TB.
            ",
        );
        game.start().expect("測試戰鬥應可開始");
        let entity = game.entity(ACTOR_ID).expect("測試玩家應存在");
        game.world
            .get_mut::<Unit>(entity)
            .expect("玩家應有 Unit")
            .movement = 3;
        // 沿無障礙的上排跨入第二段，再分次移動。
        *game.world.resource_mut::<Turn>() = Turn {
            actor: Some(ACTOR_ID),
            phase: Phase::Ready,
            movement_remaining: 3,
            movement_segments_used: 0,
        };
        game.move_to(ACTOR_ID, GridPos { x: 4, y: 0 })
            .expect("移動四格應跨入第二段並剩餘兩格");
        game.move_to(ACTOR_ID, GridPos { x: 3, y: 0 })
            .expect("第二段應可移動一格");
        let beyond_budget = GridPos { x: 1, y: 0 };
        let correct = match check {
            Check::RemainingBudget => game.world.resource::<Turn>().movement_remaining == 1,
            Check::MovementRange => {
                let (_, second) = crate::movement::movement_ranges(
                    &game.world,
                    entity,
                    game.world.resource::<Turn>(),
                );
                !second.contains(&beyond_budget)
            }
            Check::Preview => game.preview_move(ACTOR_ID, beyond_budget).is_err(),
            Check::Execution => game.move_to(ACTOR_ID, beyond_budget).is_err(),
        };
        if !correct {
            failures.push(check);
        }
    }
    assert!(failures.is_empty(), "第二段預算未正確保留：{failures:?}");
}

// 驗證第一段剛好用完後可開始第二段，並依第二段消耗決定剩餘額度及技能狀態。
#[test]
fn second_movement_segment_starts_after_first_is_exhausted() {
    for (destination_x, remaining, expected_phase) in
        [(4, 2, Phase::Moving), (0, 0, Phase::AfterMove)]
    {
        let mut game = game_with_skill_on_ascii_map(
            1,
            1,
            SkillEffect::Push {
                attack_bonus: 100,
                power_bonus: 0,
            },
            "
            A....
            ..TB.
            ",
        );
        game.start().expect("測試戰鬥應可開始");
        let entity = game.entity(ACTOR_ID).expect("測試玩家應存在");
        game.world
            .get_mut::<Unit>(entity)
            .expect("玩家應有 Unit")
            .movement = 3;
        *game.world.resource_mut::<Turn>() = Turn {
            actor: Some(ACTOR_ID),
            phase: Phase::Ready,
            movement_remaining: 3,
            movement_segments_used: 0,
        };
        game.move_to(ACTOR_ID, GridPos { x: 3, y: 0 })
            .expect("第一段應可剛好用完");
        assert!(crate::skill::can_use_skill(game.world.resource::<Turn>()));
        let destination = GridPos {
            x: destination_x,
            y: 0,
        };
        let (_, second) =
            crate::movement::movement_ranges(&game.world, entity, game.world.resource::<Turn>());
        assert!(second.contains(&destination));
        game.preview_move(ACTOR_ID, destination)
            .expect("第二段應可預覽");
        game.move_to(ACTOR_ID, destination)
            .expect("第一段用完後仍應可執行第二段移動");
        let turn @ Turn {
            actor: _,
            phase,
            movement_remaining,
            movement_segments_used: _,
        } = game.world.resource::<Turn>();
        assert_eq!(*movement_remaining, remaining);
        assert_eq!(*phase, expected_phase);
        assert!(!crate::skill::can_use_skill(turn));
    }
}

fn movement_preview_game(movement: u32) -> (Game, GridPos, GridPos, GridPos) {
    let AsciiBoard {
        board,
        markers,
        terrains,
    } = ascii_board(
        "
        ...
        A^T
        ...
    ",
    );
    let (start, _) = *markers.get(&'A').expect("棋盤應包含玩家");
    let (target, _) = *markers.get(&'T').expect("棋盤應包含目的地");
    let actor_position = GridPos {
        x: start.0,
        y: start.1,
    };
    let destination = GridPos {
        x: target.0,
        y: target.1,
    };
    let spikes = GridPos {
        x: terrains[0].x,
        y: terrains[0].y,
    };
    let game = movement_game(
        movement,
        &AsciiBoard {
            board,
            markers,
            terrains,
        },
    );
    (game, actor_position, spikes, destination)
}

fn movement_game(movement: u32, layout: &AsciiBoard) -> Game {
    let AsciiBoard {
        board,
        markers,
        terrains,
    } = layout;
    let (start, actor_size) = *markers.get(&'A').expect("棋盤應包含玩家");
    let actor_position = GridPos {
        x: start.0,
        y: start.1,
    };
    let mut world = World::new();
    world.insert_resource(Board {
        width: board.0,
        height: board.1,
        terrains: terrains
            .iter()
            .cloned()
            .map(|TerrainPlacement { x, y, kind }| (GridPos { x, y }, vec![kind]))
            .collect(),
        terrain_types: HashMap::from([
            (
                "spikes".into(),
                TerrainTypeDef {
                    id: "spikes".into(),
                    layer: TerrainLayer::Overlay,
                    entry_rule: TerrainEntryRule::Walkable,
                    damage: 3,
                    extra_movement_cost: 0,
                    dodge_penalty: 0,
                    block_penalty: 0,
                },
            ),
            (
                "plain".into(),
                TerrainTypeDef {
                    id: "plain".into(),
                    layer: TerrainLayer::Ground,
                    entry_rule: TerrainEntryRule::Walkable,
                    damage: 0,
                    extra_movement_cost: 0,
                    dodge_penalty: 0,
                    block_penalty: 0,
                },
            ),
        ]),
    });
    world.insert_resource(Log::default());
    world.insert_resource(TemporaryTerrains::default());
    world.insert_resource(Exploration {
        mode: BattleMode::Combat,
        turns: HashMap::new(),
    });
    world.insert_resource(Turn {
        actor: Some(ACTOR_ID),
        phase: Phase::Ready,
        movement_remaining: movement,
        movement_segments_used: 0,
    });
    world.spawn((
        Id(ACTOR_ID),
        Pos(actor_position),
        Hp {
            current: 100,
            maximum: 100,
        },
        Footprint {
            width: actor_size.0,
            height: actor_size.1,
        },
        Unit {
            unit_type: "test_actor".into(),
            visual: "test_actor".into(),
            team: Team::Player,
            movement,
            initiative: 0,
            dodge: 0,
            block: 0,
            attack: 0,
            physical_power: 0,
            magical_power: 0,
            block_reduction: 0,
            equipment: EquipmentView {
                main_hand: String::new(),
                off_hand: String::new(),
                armor: String::new(),
                accessory: String::new(),
            },
            skills: Vec::new(),
        },
    ));
    Game {
        world,
        movements: Vec::new(),
    }
}

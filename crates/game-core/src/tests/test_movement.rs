use super::support::{ACTOR_ID, push_collision_game};
use crate::model::Exploration;
use crate::*;

struct MovementPreviewCase {
    name: &'static str,
    movement: u32,
    interrupted: bool,
    path_length: usize,
    total_cost: u32,
}

// 驗證完整移動預覽在第一階段預算內優先安全繞路，無法繞路時選擇危險路徑。
#[test]
fn move_preview_chooses_safest_path_within_first_phase() {
    let cases = [
        MovementPreviewCase {
            name: "第一階段可以安全繞路",
            movement: 4,
            interrupted: false,
            path_length: 5,
            total_cost: 4,
        },
        MovementPreviewCase {
            name: "第一階段無法安全繞路",
            movement: 2,
            interrupted: true,
            path_length: 2,
            total_cost: 1,
        },
    ];

    for MovementPreviewCase {
        name,
        movement,
        interrupted: expected_interrupted,
        path_length,
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
        assert_eq!(first.last(), Some(&expected_last), "{}", name);
        assert_eq!(first.len(), path_length, "{}", name);
        assert_eq!(first.contains(&spikes), expected_interrupted, "{}", name);
        assert_eq!(total_cost, expected_total_cost, "{}", name);
        assert!(second.is_empty(), "{}", name);
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
        let mut game = push_collision_game();
        game.start().expect("測試戰鬥應可開始");
        let entity = game.entity(ACTOR_ID).expect("測試玩家應存在");
        game.world
            .get_mut::<Unit>(entity)
            .expect("玩家應有 Unit")
            .movement = 3;
        // 沿無障礙的上排跨入第二段，再分次移動。
        game.world.get_mut::<Pos>(entity).expect("玩家應有位置").0 = GridPos { x: 0, y: 0 };
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
        let mut game = push_collision_game();
        game.start().expect("測試戰鬥應可開始");
        let entity = game.entity(ACTOR_ID).expect("測試玩家應存在");
        game.world
            .get_mut::<Unit>(entity)
            .expect("玩家應有 Unit")
            .movement = 3;
        game.world.get_mut::<Pos>(entity).expect("玩家應有位置").0 = GridPos { x: 0, y: 0 };
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
    let actor_position = GridPos { x: 0, y: 1 };
    let spikes = GridPos { x: 1, y: 1 };
    let destination = GridPos { x: 2, y: 1 };
    let mut world = World::new();
    world.insert_resource(Board {
        width: 3,
        height: 3,
        terrains: HashMap::from([(spikes, vec!["spikes".into()])]),
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
        Footprint {
            width: 1,
            height: 1,
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
    (
        Game {
            world,
            movements: Vec::new(),
        },
        actor_position,
        spikes,
        destination,
    )
}

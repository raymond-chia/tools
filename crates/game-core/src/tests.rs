use super::*;
use crate::model::Encounter;

// 驗證命中結果、不同固定格擋減傷與暴擊順序均符合傷害公式。
#[test]
fn attack_damage_uses_expected_formula() {
    let cases = [
        ("普通命中", 5, AttackResult::Hit, 2, false, 5),
        ("暴擊命中", 5, AttackResult::Hit, 2, true, 10),
        ("普通格擋", 5, AttackResult::Block, 2, false, 3),
        ("先格擋再暴擊", 5, AttackResult::Block, 2, true, 6),
        ("無格擋減傷", 7, AttackResult::Block, 0, false, 7),
        ("較低格擋減傷", 7, AttackResult::Block, 2, false, 5),
        ("較高格擋減傷", 7, AttackResult::Block, 4, false, 3),
        ("較高格擋減傷後暴擊", 7, AttackResult::Block, 4, true, 6),
        ("超額格擋減傷", 7, AttackResult::Block, 8, false, 0),
        ("完全格擋後暴擊", 7, AttackResult::Block, 8, true, 0),
        ("閃避後暴擊", 5, AttackResult::Dodge, 2, true, 0),
    ];

    for (name, base_damage, result, block_damage_reduction, critical, expected) in cases {
        assert_eq!(
            attack_damage(base_damage, result, block_damage_reduction, critical),
            expected,
            "{name}"
        );
    }
}

struct FlankingCase {
    name: &'static str,
    ranged: bool,
    range: i32,
    attacker_position: GridPos,
    supporter_position: GridPos,
    target_position: GridPos,
    target_footprint: Footprint,
    expected_modifier: i32,
}

// 驗證共用攻擊值下，技能類型、最小與最大射程、相對站位及大型目標占用格會正確決定包夾加成。
#[test]
fn attack_modifier_uses_expected_flanking_bonus() {
    let cases = [
        FlankingCase {
            name: "近距離相反側",
            ranged: false,
            range: 1,
            attacker_position: GridPos { x: 0, y: 0 },
            supporter_position: GridPos { x: 2, y: 0 },
            target_position: GridPos { x: 1, y: 0 },
            target_footprint: Footprint {
                width: 1,
                height: 1,
            },
            expected_modifier: 5 + gameplay_config::FLANKING_ATTACK_BONUS,
        },
        FlankingCase {
            name: "近距離錯誤站位",
            ranged: false,
            range: 1,
            attacker_position: GridPos { x: 0, y: 0 },
            supporter_position: GridPos { x: 1, y: 1 },
            target_position: GridPos { x: 1, y: 0 },
            target_footprint: Footprint {
                width: 1,
                height: 1,
            },
            expected_modifier: 5,
        },
        FlankingCase {
            name: "遠距離相反側",
            ranged: true,
            range: 1,
            attacker_position: GridPos { x: 0, y: 0 },
            supporter_position: GridPos { x: 2, y: 0 },
            target_position: GridPos { x: 1, y: 0 },
            target_footprint: Footprint {
                width: 1,
                height: 1,
            },
            expected_modifier: 5,
        },
        FlankingCase {
            name: "長兵器隔一格",
            ranged: false,
            range: 2,
            attacker_position: GridPos { x: 0, y: 0 },
            supporter_position: GridPos { x: 3, y: 0 },
            target_position: GridPos { x: 2, y: 0 },
            target_footprint: Footprint {
                width: 1,
                height: 1,
            },
            expected_modifier: 5 + gameplay_config::FLANKING_ATTACK_BONUS,
        },
        FlankingCase {
            name: "大型目標不同列的相反側",
            ranged: false,
            range: 1,
            attacker_position: GridPos { x: 1, y: 0 },
            supporter_position: GridPos { x: 4, y: 1 },
            target_position: GridPos { x: 2, y: 0 },
            target_footprint: Footprint {
                width: 2,
                height: 2,
            },
            expected_modifier: 5 + gameplay_config::FLANKING_ATTACK_BONUS,
        },
    ];

    for case in cases {
        let skill = attack_skill(case.ranged, case.range);
        let (world, attacker, target) = flanking_world(
            case.attacker_position,
            case.supporter_position,
            case.target_position,
            case.target_footprint,
            case.range,
        );

        assert_eq!(
            attack_modifier(&world, attacker, target, &skill),
            case.expected_modifier,
            "{}",
            case.name
        );
    }
}

struct MovementPreviewCase {
    name: &'static str,
    movement: u32,
    interrupted: bool,
    path_length: usize,
    total_cost: u32,
}

// 驗證第一階段會在預算內優先安全繞路，無法繞路時則選擇同階段的危險路徑。
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

    for case in cases {
        let (game, actor_position, spikes, destination) = movement_preview_game(case.movement);
        let preview = game
            .preview_move("actor", destination)
            .expect("目的地應能在第一階段朝目標移動");
        let expected_last = if case.interrupted {
            spikes
        } else {
            destination
        };

        assert_eq!(preview.interrupted, case.interrupted, "{}", case.name);
        assert_eq!(
            preview.first.first(),
            Some(&actor_position),
            "{}",
            case.name
        );
        assert_eq!(preview.first.last(), Some(&expected_last), "{}", case.name);
        assert_eq!(preview.first.len(), case.path_length, "{}", case.name);
        assert_eq!(
            preview.first.contains(&spikes),
            case.interrupted,
            "{}",
            case.name
        );
        assert_eq!(preview.total_cost, case.total_cost, "{}", case.name);
        assert!(preview.second.is_empty(), "{}", case.name);
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
        terrain_types: HashMap::from([(
            "spikes".into(),
            TerrainTypeDef {
                visual: "spikes".into(),
                passable: true,
                damage: 3,
                movement_cost_bonus: 0,
                dodge_penalty: 0,
                block_penalty: 0,
                forced_entry: ForcedEntry::None,
                effect_key: "TERRAIN_EFFECT_DAMAGE".into(),
                forced_entry_log_key: None,
            },
        )]),
    });
    world.insert_resource(TemporaryTerrains::default());
    world.insert_resource(Turn {
        actor: Some("actor".into()),
        phase: Phase::Ready,
        movement_remaining: movement,
        movement_segments_used: 0,
    });
    world.spawn((
        Id("actor".into()),
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
            power: 0,
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

fn attack_skill(ranged: bool, range: i32) -> SkillDef {
    SkillDef {
        id: if ranged {
            "ranged_attack".into()
        } else {
            "melee_attack".into()
        },
        ranged,
        min_range: 1,
        max_range: range,
        effect: SkillEffect::Attack {
            attack_bonus: 0,
            power_bonus: 0,
        },
    }
}

fn spawn_unit(
    world: &mut World,
    position: GridPos,
    footprint: Footprint,
    team: Team,
    skills: Vec<String>,
) -> Entity {
    world
        .spawn((
            Pos(position),
            footprint,
            Unit {
                unit_type: "test_unit".into(),
                visual: "test_unit".into(),
                team,
                movement: 0,
                initiative: 0,
                dodge: 0,
                block: 0,
                attack: 5,
                power: 1,
                skills,
            },
        ))
        .id()
}

fn flanking_world(
    attacker_position: GridPos,
    supporter_position: GridPos,
    target_position: GridPos,
    target_footprint: Footprint,
    melee_range: i32,
) -> (World, Entity, Entity) {
    let melee_skill = attack_skill(false, melee_range);
    let melee_skill_id = melee_skill.id.clone();
    let mut world = World::new();
    world.insert_resource(Skills {
        definitions: HashMap::from([(melee_skill_id.clone(), melee_skill)]),
    });
    let attacker = spawn_unit(
        &mut world,
        attacker_position,
        Footprint {
            width: 1,
            height: 1,
        },
        Team::Player,
        vec![melee_skill_id.clone()],
    );
    spawn_unit(
        &mut world,
        supporter_position,
        Footprint {
            width: 1,
            height: 1,
        },
        Team::Player,
        vec![melee_skill_id],
    );
    let target = spawn_unit(
        &mut world,
        target_position,
        target_footprint,
        Team::Enemy("test_enemy".into()),
        Vec::new(),
    );
    (world, attacker, target)
}

// 驗證推擊撞上另一單位時停止移動，並以實例 ID 記錄受碰撞傷害的單位。
#[test]
fn push_collision_damages_both_units() {
    let mut game = push_collision_game();
    game.start().expect("測試戰鬥應可開始");
    let skill = game
        .world
        .resource::<Skills>()
        .definitions
        .get("push")
        .expect("測試推擊技能應存在")
        .clone();
    game.use_skill("actor", "target", GridPos { x: 2, y: 1 }, skill)
        .expect("推擊應成功結算");

    let target = game.entity("target").expect("測試目標應存在");
    let blocker = game.entity("blocker").expect("測試碰撞單位應存在");
    let log = game.world.resource::<Log>();
    let (damage, collision_damage, collision_units) = match log.0.last() {
        Some(CombatLogEvent::Skill {
            damage,
            collision_damage,
            collision_units,
            push_blocked: true,
            pushed: false,
            ..
        }) => (*damage, *collision_damage, collision_units),
        _ => panic!("推擊應記錄單位碰撞且不移動"),
    };

    assert_eq!(collision_damage, gameplay_config::COLLISION_DAMAGE);
    assert_eq!(game.world.get::<Pos>(target).expect("目標應有位置").0.x, 2);
    assert_eq!(
        game.world.get::<Hp>(target).expect("目標應有 HP").current,
        100 - damage - collision_damage
    );
    assert_eq!(
        game.world
            .get::<Hp>(blocker)
            .expect("碰撞單位應有 HP")
            .current,
        100 - collision_damage
    );
    assert_eq!(collision_units.len(), 1);
    assert_eq!(collision_units[0].unit, "blocker");
    assert_eq!(collision_units[0].unit_type, "test_unit");
    assert_eq!(collision_units[0].remaining_hp, 100 - collision_damage);
}

// 驗證最小射程排除過近格子，施放失敗時會回傳固定 ID 與詳細訊息。
#[test]
fn skill_min_range_limits_preview_and_action() {
    let mut game = game_with_skill_range_and_effect(
        2,
        2,
        SkillEffect::Push {
            attack_bonus: 100,
            power_bonus: 0,
        },
    );
    let actor = game.entity("actor").expect("測試攻擊者應存在");
    let range = super::skill::skill_ranges(&game.world, actor);
    assert!(!range[0].cells.contains(&GridPos { x: 2, y: 1 }));
    assert!(range[0].cells.contains(&GridPos { x: 3, y: 1 }));

    game.start().expect("測試戰鬥應可開始");
    let skill = game.world.resource::<Skills>().definitions["push"].clone();
    let error = game
        .use_skill("actor", "target", GridPos { x: 2, y: 1 }, skill)
        .expect_err("過近的目標應被拒絕");
    assert_eq!(error.id(), "target_too_close");
    assert_eq!(error.message(), "目標距離太近");
}

// 驗證零射程治療可對自己施放，且治療量為施放者力量加技能加值。
#[test]
fn zero_range_heal_targets_self() {
    let mut game = game_with_skill_range_and_effect(0, 0, SkillEffect::Heal { power_bonus: 4 });
    let actor = game.entity("actor").expect("測試攻擊者應存在");
    let range = super::skill::skill_ranges(&game.world, actor);
    assert_eq!(range[0].cells, vec![GridPos { x: 1, y: 1 }]);

    game.start().expect("測試戰鬥應可開始");
    game.world
        .get_mut::<Hp>(actor)
        .expect("施放者應有生命值")
        .current = 90;
    let preview = game
        .preview_skill("actor", "actor", GridPos { x: 1, y: 1 }, "push")
        .expect("零距離治療應可預覽");
    match preview {
        SkillPreview::Healing(healing) => assert_eq!(healing.healing, 5),
        _ => panic!("治療技能應產生治療預覽"),
    }
    let skill = game.world.resource::<Skills>().definitions["push"].clone();
    game.use_skill("actor", "actor", GridPos { x: 1, y: 1 }, skill)
        .expect("零距離治療應可對自己施放");
    assert_eq!(
        game.world
            .get::<Hp>(actor)
            .expect("施放者應有生命值")
            .current,
        95
    );
}

// 驗證敵方沒有技能時回報錯誤，且空技能清單不會自動取得所有技能。
#[test]
fn enemy_without_skill_reports_error() {
    let mut game = push_collision_game();
    let enemy = game.entity("target").expect("測試敵方應存在");
    game.world
        .get_mut::<Unit>(enemy)
        .expect("敵方應有資料")
        .skills
        .clear();
    game.world.resource_mut::<Turn>().actor = Some("target".into());
    game.world.resource_mut::<Turn>().phase = Phase::Ready;
    game.world.resource_mut::<Encounter>().round = 1;
    let error = match game.command(Command::AutoStep) {
        Ok(_) => panic!("沒有技能的敵方應回報錯誤"),
        Err(error) => error,
    };
    assert_eq!(error.id(), "missing_ai_skill");
}

fn push_collision_game() -> Game {
    game_with_skill_range_and_effect(
        1,
        1,
        SkillEffect::Push {
            attack_bonus: 100,
            power_bonus: 0,
        },
    )
}

fn game_with_skill_range_and_effect(min_range: i32, max_range: i32, effect: SkillEffect) -> Game {
    let terrain = TerrainTypeDef {
        visual: "plain".into(),
        passable: true,
        damage: 0,
        movement_cost_bonus: 0,
        dodge_penalty: 0,
        block_penalty: 0,
        forced_entry: ForcedEntry::None,
        effect_key: "TERRAIN_EFFECT_NONE".into(),
        forced_entry_log_key: None,
    };
    let definition = Definition {
        map: MapDef {
            width: 5,
            height: 2,
            terrains: Vec::new(),
        },
        terrain_types: HashMap::from([
            ("plain".into(), terrain.clone()),
            ("rough".into(), terrain),
        ]),
        skills: vec![SkillDef {
            id: "push".into(),
            ranged: false,
            min_range,
            max_range,
            effect,
        }],
        units: vec![
            push_collision_unit("actor", Team::Player, 1),
            push_collision_unit("target", Team::Enemy("test_enemy".into()), 2),
            push_collision_unit("blocker", Team::Enemy("test_enemy".into()), 3),
        ],
    };
    Game::from_definition(definition).expect("測試戰鬥定義應有效")
}

fn push_collision_unit(id: &str, team: Team, x: i32) -> UnitDef {
    let initiative = if team == Team::Player { 100 } else { 0 };
    UnitDef {
        id: id.into(),
        unit_type: "test_unit".into(),
        visual: "test_unit".into(),
        team,
        x,
        y: 1,
        width: 1,
        height: 1,
        hp: 100,
        movement: 0,
        initiative,
        dodge: 0,
        block: 0,
        attack: 0,
        power: 1,
        skills: vec!["push".into()],
    }
}

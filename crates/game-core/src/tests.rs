use super::*;

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

// 驗證武器類型、射程、相對站位與大型目標占用格會正確決定包夾加成。
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
        costs: vec![1; 9],
        triggers: HashMap::from([(spikes, "spikes".into())]),
    });
    world.insert_resource(TemporaryTerrains::default());
    world.insert_resource(Turn {
        actor: Some("actor".into()),
        phase: Phase::Ready,
        remaining: movement,
        moves: 0,
    });
    world.spawn((
        Id("actor".into()),
        Pos(actor_position),
        Footprint {
            width: 1,
            height: 1,
        },
        Unit {
            name: "測試角色".into(),
            team: Team::Player,
            group: "測試群組".into(),
            movement,
            initiative: 0,
            dodge: 0,
            block: 0,
            melee: 0,
            ranged: 0,
            damage: 0,
            range: 1,
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
        name: "測試攻擊".into(),
        ranged,
        attack_bonus: 0,
        damage_bonus: 0,
        range: Some(range),
        duration: None,
        heal_amount: None,
        effect: SkillEffect::Attack,
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
                name: "測試單位".into(),
                team,
                group: "測試群組".into(),
                movement: 0,
                initiative: 0,
                dodge: 0,
                block: 0,
                melee: 5,
                ranged: 5,
                damage: 1,
                range: 1,
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
    world.insert_resource(Skills(HashMap::from([(
        melee_skill_id.clone(),
        melee_skill,
    )])));
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
        Team::Enemy,
        Vec::new(),
    );
    (world, attacker, target)
}

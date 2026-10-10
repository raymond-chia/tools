use super::support::{AsciiBoard, ascii_board};
use crate::*;

// 驗證命中結果、不同裝備格擋減傷與暴擊順序均符合傷害公式。
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

// 驗證被動夾擊加成、ASCII 站位、技能類型、射程及大型目標佔用格共同決定攻擊加值。
#[test]
fn attack_modifier_uses_expected_flanking_bonus() {
    for (name, diagram, ranged, min_range, max_range, flanking) in [
        // 驗證相鄰目標的兩側近戰單位提供包夾。
        (
            "近距離相反側",
            "
            ATB
            ",
            false,
            1,
            1,
            true,
        ),
        // 驗證支援者未站在相反側時不提供包夾。
        (
            "近距離錯誤站位",
            "
            AT
            .B
            ",
            false,
            1,
            1,
            false,
        ),
        // 驗證遠程攻擊不取得包夾加成。
        (
            "遠距離相反側",
            "
            ATB
            ",
            true,
            1,
            1,
            false,
        ),
        // 驗證長兵器可隔一格與另一側支援者形成包夾。
        (
            "長兵器隔一格",
            "
            A.TB
            ",
            false,
            1,
            2,
            true,
        ),
        // 驗證大型目標兩側即使位於不同列仍形成包夾。
        (
            "大型目標不同列的相反側",
            "
            .ATT.
            ..TTB
            ",
            false,
            1,
            1,
            true,
        ),
        // 驗證兩側單位都位於二至三格射程內時提供包夾。
        (
            "最小射程內的相反側",
            "
            A..T..B
            ",
            false,
            2,
            3,
            true,
        ),
        // 驗證大型攻擊者與支援者可從符合最小射程的佔用格提供包夾。
        (
            "大型單位的另一格符合最小射程",
            "
            AA.T..BB
            AA....BB
            ",
            false,
            3,
            3,
            true,
        ),
        // 驗證大型目標的較遠佔用格符合最小射程時仍提供包夾。
        (
            "大型目標的較遠格符合最小射程",
            "
            A.TT..B
            ..TT...
            ",
            false,
            3,
            3,
            true,
        ),
        // 驗證支援者距離不足最小射程時不提供包夾。
        (
            "支援者過近",
            "
            A..TB
            ",
            false,
            2,
            3,
            false,
        ),
        // 驗證攻擊者距離不足最小射程時不提供包夾。
        (
            "攻擊者過近",
            "
            AT..B
            ",
            false,
            2,
            3,
            false,
        ),
        // 驗證支援者被牆遮擋時不提供包夾。
        (
            "支援者被牆遮擋",
            "
            AT#B
            ",
            false,
            1,
            2,
            false,
        ),
    ] {
        let skill = attack_skill(ranged, min_range, max_range);
        for (passive_name, passive_id, expected_bonus) in [
            // 無被動技能時，即使形成夾擊也沒有加成。
            ("無被動", None, 0),
            // 一般夾擊被動提供 2 點攻擊加值。
            ("一般夾擊", Some("test_flanking"), 2),
            // 特殊夾擊被動提供 4 點攻擊加值，支援者不需要持有被動。
            ("特殊夾擊", Some("test_pack_flanking"), 4),
        ] {
            let (world, attacker, target) =
                flanking_world(diagram, min_range, max_range, passive_id);
            let expected_modifier = 5 + if flanking { expected_bonus } else { 0 };
            assert_eq!(
                attack_modifier(&world, attacker, target, &skill, 0),
                expected_modifier,
                "{name}／{passive_name}"
            );
        }
    }
}

fn attack_skill(ranged: bool, min_range: i32, max_range: i32) -> SkillDef {
    SkillDef {
        power_source: PowerSource::Physical,
        id: if ranged {
            "ranged_attack".into()
        } else {
            "melee_attack".into()
        },
        ranged,
        min_range,
        max_range,
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
                physical_power: 1,
                magical_power: 1,
                block_reduction: 0,
                equipment: EquipmentView {
                    main_hand: String::new(),
                    off_hand: String::new(),
                    armor: String::new(),
                    accessory: String::new(),
                },
                skills,
            },
        ))
        .id()
}

fn flanking_world(
    diagram: &str,
    min_range: i32,
    max_range: i32,
    passive_id: Option<&str>,
) -> (World, Entity, Entity) {
    let AsciiBoard {
        board,
        markers,
        terrains,
    } = ascii_board(diagram);
    let placement = |marker| {
        let ((x, y), (width, height)) = markers[&marker];
        (GridPos { x, y }, Footprint { width, height })
    };
    let (attacker_position, attacker_footprint) = placement('A');
    let (supporter_position, supporter_footprint) = placement('B');
    let (target_position, target_footprint) = placement('T');
    let melee_skill = attack_skill(false, min_range, max_range);
    let melee_skill_id = melee_skill.id.clone();
    let plain = TerrainTypeDef {
        id: "plain".into(),
        blocks_sight: false,
        layer: TerrainLayer::Ground,
        entry_rule: TerrainEntryRule::Walkable,
        damage: 0,
        extra_movement_cost: 0,
        dodge_penalty: 0,
        block_penalty: 0,
    };
    let mut wall = plain.clone();
    wall.id = "wall".into();
    wall.layer = TerrainLayer::Overlay;
    wall.entry_rule = TerrainEntryRule::Blocked;
    wall.blocks_sight = true;
    let mut world = World::new();
    world.insert_resource(TemporaryTerrains::default());
    world.insert_resource(Board {
        width: board.0,
        height: board.1,
        terrains: terrains
            .into_iter()
            .map(|TerrainPlacement { x, y, kind }| (GridPos { x, y }, vec![kind]))
            .collect(),
        terrain_types: HashMap::from([("plain".into(), plain), ("wall".into(), wall)]),
    });
    let mut definitions = HashMap::from([(melee_skill_id.clone(), melee_skill)]);
    // 每個案例都定義相同的兩種被動，只改變攻擊者持有的技能 ID。
    for (id, attack_bonus) in [("test_flanking", 2), ("test_pack_flanking", 4)] {
        definitions.insert(
            id.to_owned(),
            SkillDef {
                id: id.to_owned(),
                power_source: PowerSource::Physical,
                ranged: false,
                min_range: 0,
                max_range: 0,
                effect: SkillEffect::Flanking { attack_bonus },
            },
        );
    }
    world.insert_resource(Skills { definitions });
    let mut attacker_skills = vec![melee_skill_id.clone()];
    if let Some(id) = passive_id {
        attacker_skills.push(id.to_owned());
    }
    let attacker = spawn_unit(
        &mut world,
        attacker_position,
        attacker_footprint,
        Team::Player,
        attacker_skills,
    );
    spawn_unit(
        &mut world,
        supporter_position,
        supporter_footprint,
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

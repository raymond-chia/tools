use crate::support::{ACTOR_ID, BLOCKER_ID, TARGET_ID, TEST_SKILL_ID, skill_test_data, unit};
use game_core::{
    ActiveSkillEffect, ActiveSkillPreview, CombatLogEvent, Command, Game, GridPos, HealingPreview,
    PowerSource, Team, TerrainEntryRule, TerrainLayer, TerrainTypeDef,
};

// 驗證公開推擊命令撞上另一單位時停止移動，並回傳雙方傷害與碰撞日誌。
#[test]
fn push_collision_damages_both_units() {
    let mut game = game_with_skill_on_ascii_map(
        1,
        1,
        ActiveSkillEffect::Push {
            ranged: false,
            hit_bonus: 100,
        },
        Team::Enemy("test_enemy".into()),
        "
        .....
        .ATB.
        ",
    );
    game.command(Command::Start).expect("測試戰鬥應可開始");
    let before = game.snapshot(None);
    let target = unit(&before, TARGET_ID);
    let target_position = GridPos {
        x: target.x,
        y: target.y,
    };
    let snapshot = game
        .command(Command::Skill {
            actor: ACTOR_ID,
            x: target_position.x,
            y: target_position.y,
            skill: TEST_SKILL_ID.into(),
        })
        .expect("推擊應成功結算");
    let (damage, collision_damage, collision_units) = match snapshot.log.last() {
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
    assert_eq!(damage, 0);
    assert_eq!(collision_damage, 2);
    let target = unit(&snapshot, TARGET_ID);
    assert_eq!((target.x, target.y), (target_position.x, target_position.y));
    assert_eq!(target.hp, 98);
    assert_eq!(unit(&snapshot, BLOCKER_ID).hp, 98);
    assert_eq!(collision_units.len(), 1);
    let collision = serde_json::to_value(&collision_units[0]).expect("碰撞日誌應可序列化");
    assert_eq!(collision["unit"], BLOCKER_ID);
    assert_eq!(collision["unit_type"], "test_blocker");
    assert_eq!(collision["remaining_hp"], 98);
}

// 驗證公開技能範圍排除過近格子，施放失敗回傳固定 ID 與訊息且不消耗技能行動。
#[test]
fn skill_min_range_limits_preview_and_action() {
    let mut game = game_with_skill_on_ascii_map(
        2,
        2,
        ActiveSkillEffect::Push {
            ranged: false,
            hit_bonus: 100,
        },
        Team::Enemy("test_enemy".into()),
        "
        .....
        .ATB.
        ",
    );
    let snapshot = game.command(Command::Start).expect("測試戰鬥應可開始");
    let target = unit(&snapshot, TARGET_ID);
    let target_position = GridPos {
        x: target.x,
        y: target.y,
    };
    let blocker = unit(&snapshot, BLOCKER_ID);
    let blocker_position = GridPos {
        x: blocker.x,
        y: blocker.y,
    };
    assert_eq!(
        snapshot.skill_ranges[0].cells.contains(&target_position),
        false
    );
    assert_eq!(
        snapshot.skill_ranges[0].cells.contains(&blocker_position),
        true
    );
    let error = game
        .command(Command::Skill {
            actor: ACTOR_ID,
            x: target_position.x,
            y: target_position.y,
            skill: TEST_SKILL_ID.into(),
        })
        .err()
        .expect("過近的目標應被拒絕");
    assert_eq!(error.id(), "target_too_close");
    assert_eq!(error.message(), "目標距離太近");
    assert_eq!(game.snapshot(None).turn.can_skill, true);
}

// 驗證普通單位受傷後能自療，組合被動中的禁止治癒在預覽與施放一致生效。
#[test]
fn zero_range_heal_respects_unhealable_passive() {
    for unhealable in [false, true] {
        let (mut definitions, map) = skill_test_data(
            0,
            0,
            ActiveSkillEffect::Heal {
                power_source: PowerSource::Magical,
                power_bonus: 4,
            },
            Team::Player,
            "
        A.TB
        ^...
        ",
        );
        definitions.unit_types[0].movement = 1;
        if unhealable {
            definitions.passive_skills.push(
                serde_json::from_value(serde_json::json!({
                    "id": "mechanical", "effects": [{"effect": "unhealable"}, {"effect": "flanking", "hit_bonus": 3}]
                }))
                .expect("被動測試資料應可載入"),
            );
            definitions.unit_types[0]
                .passive_skills
                .push("mechanical".into());
        }
        definitions.terrain_types.push(
            serde_json::from_value::<TerrainTypeDef>(serde_json::json!({
                "id": "spikes",
                "layer": TerrainLayer::Overlay,
                "entry_rule": TerrainEntryRule::Walkable,
                "damage": 10,
                "blocks_sight": false,
                "extra_movement_cost": 0,
                "dodge_penalty": 0,
                "block_penalty": 0
            }))
            .expect("測試地形資料應可載入"),
        );
        let mut game =
            Game::from_authoring(definitions, map, game_core::authoring::Difficulty::Easy)
                .expect("治療測試應可載入");
        game.command(Command::Start).expect("測試戰鬥應可開始");
        let snapshot = game
            .command(Command::Move {
                actor: ACTOR_ID,
                x: 0,
                y: 1,
            })
            .expect("施放者應可移動至尖刺格受傷");
        let actor_position = GridPos { x: 0, y: 1 };
        assert_eq!(unit(&snapshot, ACTOR_ID).hp, 90);
        if unhealable {
            assert_eq!(snapshot.skill_ranges[0].cells, vec![actor_position]);
            assert_eq!(
                game.preview_skill(ACTOR_ID, actor_position, TEST_SKILL_ID)
                    .err()
                    .expect("無法接受治癒的單位不能預覽治療")
                    .id(),
                "target_unhealable"
            );
            let error = game
                .command(Command::Skill {
                    actor: ACTOR_ID,
                    x: 0,
                    y: 1,
                    skill: TEST_SKILL_ID.into(),
                })
                .err()
                .expect("無法接受治癒的單位不能施放治療");
            assert_eq!(error.id(), "target_unhealable");
            assert_eq!(unit(&game.snapshot(None), ACTOR_ID).hp, 90);
            continue;
        }
        assert_eq!(snapshot.skill_ranges[0].cells, vec![actor_position]);
        let preview = game
            .preview_skill(ACTOR_ID, actor_position, TEST_SKILL_ID)
            .expect("零距離治療應可預覽");
        match preview {
            ActiveSkillPreview::Healing(HealingPreview {
                target: _,
                target_type: _,
                target_hp: _,
                target_max_hp: _,
                target_mana: _,
                healing,
                remaining_hp: _,
                missing_hp: _,
                health_segments: _,
            }) => assert_eq!(healing, 5),
            _ => panic!("治療技能應產生治療預覽"),
        }
        let snapshot = game
            .command(Command::Skill {
                actor: ACTOR_ID,
                x: actor_position.x,
                y: actor_position.y,
                skill: TEST_SKILL_ID.into(),
            })
            .expect("零距離治療應可對自己施放");
        assert_eq!(unit(&snapshot, ACTOR_ID).hp, 95);
    }
}

// 單技能測試共用建局：A 為施放者，T 的陣營由案例指定、B 為敵人，各自尺寸與起點由 ASCII 決定。
fn game_with_skill_on_ascii_map(
    min_range: i32,
    max_range: i32,
    effect: ActiveSkillEffect,
    target_team: Team,
    diagram: &str,
) -> Game {
    let (definitions, map) = skill_test_data(min_range, max_range, effect, target_team, diagram);
    Game::from_authoring(definitions, map, game_core::authoring::Difficulty::Easy)
        .expect("測試戰鬥定義應有效")
}

// 驗證反應次數疊加、其他同類效果取較強值，且禁止治癒與技能說明一致。
#[test]
fn passive_skills_stack_reaction_counts_keep_strongest_other_effects() {
    let mut cases = Vec::new();
    for effect in ["collision_counter", "opportunity_attack"] {
        for selected in [vec!["first", "second"], vec!["second", "first"]] {
            // 碰撞與藉機攻擊次數各自相加，交換選取順序仍為三次，並保留禁止治療。
            let expected = if selected[0] == "first" {
                serde_json::json!([
                    {"id": "first", "effects": [{"effect": "unhealable"}, {"effect": effect, "reactions_per_round": 3}]}
                ])
            } else {
                serde_json::json!([
                    {"id": "second", "effects": [{"effect": effect, "reactions_per_round": 3}]},
                    {"id": "first", "effects": [{"effect": "unhealable"}]}
                ])
            };
            cases.push((
                serde_json::json!([{"effect": "unhealable"}, {"effect": effect, "reactions_per_round": 1}]),
                serde_json::json!([{"effect": effect, "reactions_per_round": 2}]),
                selected,
                expected,
            ));
        }
    }
    cases.extend([
        // 弱組合先選，較強夾擊只取代夾擊，保留禁止治療。
        (
            serde_json::json!([{"effect": "unhealable"}, {"effect": "flanking", "hit_bonus": 2}]),
            serde_json::json!([{"effect": "flanking", "hit_bonus": 4}]),
            vec!["first", "second"],
            serde_json::json!([
                {"id": "first", "effects": [{"effect": "unhealable"}]},
                {"id": "second", "effects": [{"effect": "flanking", "hit_bonus": 4}]}
            ]),
        ),
        // 強夾擊先選，較弱夾擊不能覆蓋它，禁止治療仍保留。
        (
            serde_json::json!([{"effect": "flanking", "hit_bonus": 4}]),
            serde_json::json!([{"effect": "unhealable"}, {"effect": "flanking", "hit_bonus": 2}]),
            vec!["first", "second"],
            serde_json::json!([
                {"id": "first", "effects": [{"effect": "flanking", "hit_bonus": 4}]},
                {"id": "second", "effects": [{"effect": "unhealable"}]}
            ]),
        ),
        // 交換勾選順序，仍取較強夾擊並保留禁止治療。
        (
            serde_json::json!([{"effect": "flanking", "hit_bonus": 4}]),
            serde_json::json!([{"effect": "unhealable"}, {"effect": "flanking", "hit_bonus": 2}]),
            vec!["second", "first"],
            serde_json::json!([
                {"id": "second", "effects": [{"effect": "unhealable"}]},
                {"id": "first", "effects": [{"effect": "flanking", "hit_bonus": 4}]}
            ]),
        ),
        // 相同禁止治療與同值夾擊只生效一次，保留先選技能的來源。
        (
            serde_json::json!([{"effect": "unhealable"}, {"effect": "flanking", "hit_bonus": 2}]),
            serde_json::json!([{"effect": "unhealable"}, {"effect": "flanking", "hit_bonus": 2}]),
            vec!["first", "second"],
            serde_json::json!([
                {"id": "first", "effects": [{"effect": "unhealable"}, {"effect": "flanking", "hit_bonus": 2}]}
            ]),
        ),
    ]);
    for (first_effects, second_effects, selected, expected) in cases {
        let (mut definitions, map) = skill_test_data(
            0,
            0,
            ActiveSkillEffect::Heal {
                power_source: PowerSource::Magical,
                power_bonus: 4,
            },
            Team::Enemy("test_enemy".into()),
            "
            A.T
        ",
        );
        definitions.passive_skills = serde_json::from_value(serde_json::json!([
            {"id": "first", "effects": first_effects},
            {"id": "second", "effects": second_effects}
        ]))
        .expect("組合被動應符合作者格式");
        definitions.unit_types[0].passive_skills =
            selected.into_iter().map(str::to_owned).collect();
        let mut game =
            Game::from_authoring(definitions, map, game_core::authoring::Difficulty::Easy)
                .expect("組合被動應可載入");
        let snapshot = game.command(Command::Start).expect("戰鬥應可開始");
        assert_eq!(
            serde_json::to_value(&unit(&snapshot, ACTOR_ID).passive_skills)
                .expect("被動資訊應可序列化"),
            expected
        );
        assert_eq!(
            game.preview_skill(ACTOR_ID, GridPos { x: 0, y: 0 }, TEST_SKILL_ID)
                .err()
                .expect("組合後仍應禁止治癒")
                .id(),
            "target_unhealable"
        );
    }
}

// 驗證雙重打擊逐擊擲骰、半傷捨去後格擋、獨立暴擊，且首擊擊倒仍正常完成行動。
#[test]
fn double_strike_resolves_independent_half_damage_hits() {
    use game_core::authoring::EquipmentSlot;
    use game_core::{AttackResult, authoring};
    for (
        name,
        dodge,
        block,
        reduction,
        hp,
        seed,
        expected,
        remaining,
        preview_hit,
        preview_block,
    ) in [
        (
            "攻擊兩次，每次造成一半傷害，小數捨去",
            0,
            0,
            0,
            100,
            1,
            Some(vec![
                (14, AttackResult::Hit, false, 2),
                (12, AttackResult::Hit, false, 2),
            ]),
            96,
            96,
            96,
        ),
        (
            "各次格擋減傷",
            0,
            100,
            1,
            100,
            1,
            Some(vec![
                (14, AttackResult::Block, false, 1),
                (12, AttackResult::Block, false, 1),
            ]),
            98,
            96,
            98,
        ),
        (
            "一擊命中一擊格擋",
            0,
            13,
            1,
            100,
            1,
            Some(vec![
                (14, AttackResult::Hit, false, 2),
                (12, AttackResult::Block, false, 1),
            ]),
            97,
            96,
            98,
        ),
        (
            "一擊命中一擊閃避",
            13,
            0,
            0,
            100,
            1,
            Some(vec![
                (14, AttackResult::Hit, false, 2),
                (12, AttackResult::Dodge, false, 0),
            ]),
            98,
            96,
            96,
        ),
        (
            "暴擊獨立結算",
            0,
            0,
            0,
            100,
            15,
            Some(vec![
                (20, AttackResult::Hit, true, 4),
                (8, AttackResult::Hit, false, 2),
            ]),
            94,
            96,
            96,
        ),
        (
            "首擊擊倒後技能正常完成",
            0,
            0,
            0,
            2,
            1,
            // 這個案例不在乎紀錄，能完成就好
            None,
            0,
            0,
            0,
        ),
    ] {
        let (mut definitions, map) = skill_test_data(
            1,
            1,
            ActiveSkillEffect::DoubleStrike {
                power_source: PowerSource::Physical,
                ranged: false,
                hit_bonus: 0,
                power_bonus: 0,
            },
            Team::Enemy("test_enemy".into()),
            "
        ATB
        ",
        );
        definitions.equipment.push(authoring::EquipmentDef {
            id: "guard".into(),
            slot: EquipmentSlot::Armor,
            hp: 0,
            physical_power: 0,
            magical_power: 0,
            block,
            block_reduction: reduction,
        });
        for kind in &mut definitions.unit_types {
            if kind.id == "test_actor" {
                kind.physical_power = 5;
            }
            if kind.id == "test_target" {
                kind.hp = hp;
                kind.dodge = dodge;
                kind.armor = "guard".into();
            }
        }
        let mut game = Game::from_authoring(definitions, map, authoring::Difficulty::Easy)
            .expect("雙重打擊資料應有效");
        game.command(Command::Start).expect("戰鬥應可開始");
        game.set_random_seed(seed);
        let preview = match game
            .preview_skill(ACTOR_ID, GridPos { x: 1, y: 0 }, TEST_SKILL_ID)
            .expect("應可預覽")
        {
            ActiveSkillPreview::Attack(preview) => preview,
            _ => panic!("應為攻擊預覽"),
        };
        assert_eq!(
            (preview.strikes, preview.hit_damage, preview.block_damage),
            (2, 2, (2 - reduction).max(0)),
            "{name}"
        );
        assert_eq!(
            (preview.hit_remaining_hp, preview.block_remaining_hp),
            (preview_hit, preview_block),
            "{name}"
        );
        let snapshot = game
            .command(Command::Skill {
                actor: ACTOR_ID,
                x: 1,
                y: 0,
                skill: TEST_SKILL_ID.into(),
            })
            .expect("應可雙重打擊");
        let strikes: Vec<_> = snapshot
            .log
            .iter()
            .filter_map(|event| match event {
                CombatLogEvent::Skill {
                    actor,
                    skill,
                    roll,
                    result,
                    critical,
                    damage,
                    ..
                } if *actor == ACTOR_ID && skill == TEST_SKILL_ID => {
                    Some((*roll, *result, *critical, *damage))
                }
                _ => None,
            })
            .collect();
        if let Some(expected) = expected {
            assert_eq!(strikes, expected, "{name}");
        }
        assert_eq!(
            snapshot
                .units
                .iter()
                .find(|unit| unit.id == TARGET_ID)
                .map_or(0, |unit| unit.hp),
            remaining,
            "{name}"
        );
        assert_eq!(
            snapshot.turn.phase,
            game_core::Phase::Ended,
            "{name}：整招結束行動"
        );
    }
}

// 使用共用被動資料驗證長兵器只增加近戰射程一次，施放、可選格與查看皆一致。
#[test]
fn melee_reach_changes_real_skill_range_without_changing_definition() {
    use crate::support::add_passive_skill;
    use game_core::authoring;
    for (name, effect, reaches, expected_range) in [
        (
            "無被動",
            ActiveSkillEffect::DoubleStrike {
                power_source: PowerSource::Physical,
                ranged: false,
                hit_bonus: 0,
                power_bonus: 0,
            },
            0,
            1,
        ),
        (
            "雙重打擊",
            ActiveSkillEffect::DoubleStrike {
                power_source: PowerSource::Physical,
                ranged: false,
                hit_bonus: 0,
                power_bonus: 0,
            },
            1,
            2,
        ),
        (
            "重複被動不疊加",
            ActiveSkillEffect::DoubleStrike {
                power_source: PowerSource::Physical,
                ranged: false,
                hit_bonus: 0,
                power_bonus: 0,
            },
            2,
            2,
        ),
        (
            "普通近戰",
            ActiveSkillEffect::Attack {
                power_source: PowerSource::Physical,
                ranged: false,
                hit_bonus: 0,
                power_bonus: 0,
            },
            1,
            2,
        ),
        (
            "遠程雙重打擊不增加",
            ActiveSkillEffect::DoubleStrike {
                power_source: PowerSource::Physical,
                ranged: true,
                hit_bonus: 0,
                power_bonus: 0,
            },
            1,
            1,
        ),
        (
            "遠程不增加",
            ActiveSkillEffect::Attack {
                power_source: PowerSource::Physical,
                ranged: true,
                hit_bonus: 0,
                power_bonus: 0,
            },
            1,
            1,
        ),
        (
            "治療不增加",
            ActiveSkillEffect::Heal {
                power_source: PowerSource::Magical,
                power_bonus: 0,
            },
            1,
            1,
        ),
        (
            "推擊",
            ActiveSkillEffect::Push {
                ranged: false,
                hit_bonus: 0,
            },
            1,
            2,
        ),
        (
            "橫掃",
            ActiveSkillEffect::Sweep {
                power_source: PowerSource::Physical,
                hit_bonus: 0,
                power_bonus: 0,
            },
            1,
            2,
        ),
    ] {
        let (mut definitions, mut map) = skill_test_data(
            1,
            1,
            effect.clone(),
            Team::Enemy("test_enemy".into()),
            "
        A.T.B
        ",
        );
        if matches!(effect, ActiveSkillEffect::Heal { .. }) {
            map.units
                .iter_mut()
                .find(|unit| unit.id == TARGET_ID)
                .expect("應有目標")
                .team = Team::Player;
        }
        for index in 0..reaches {
            let id = format!("reach_{index}");
            add_passive_skill(
                &mut definitions,
                "test_actor",
                &id,
                &[game_core::PassiveSkillEffect::MeleeReach],
            );
        }
        let original = serde_json::to_value(&definitions.active_skills[0]).expect("技能應可序列化");
        assert_eq!(original["max_range"], 1, "{name}：原始技能維持一格");
        let mut game = Game::from_authoring(definitions, map, authoring::Difficulty::Easy)
            .expect("長兵器資料應有效");
        game.command(Command::Start).expect("戰鬥應可開始");
        let snapshot = game.snapshot(None);
        let range = &snapshot.skill_ranges[0];
        assert_eq!(range.details.max_range, expected_range, "{name}");
        assert_eq!(
            unit(&snapshot, ACTOR_ID).active_skills[0].details.max_range,
            expected_range,
            "{name}：單位查看"
        );
        assert_eq!(
            range.cells.contains(&GridPos { x: 2, y: 0 }),
            expected_range == 2,
            "{name}：可選地格"
        );
        assert_eq!(
            game.preview_skill(ACTOR_ID, GridPos { x: 2, y: 0 }, TEST_SKILL_ID)
                .is_ok(),
            expected_range == 2,
            "{name}：預覽"
        );
        assert_eq!(
            game.command(Command::Skill {
                actor: ACTOR_ID,
                x: 2,
                y: 0,
                skill: TEST_SKILL_ID.into()
            })
            .is_ok(),
            expected_range == 2,
            "{name}：實際施放"
        );
    }
}

// 驗證火焰附傷欄位影響預覽與實際傷害，並沿用同次攻擊的格擋與暴擊。
#[test]
fn fire_claw_uses_configured_damage_in_preview_and_resolution() {
    use game_core::authoring::{self, EquipmentSlot};
    for (name, fire, block, reduction, seed, expected) in [
        ("沒有附傷", 0, 0, 0, 1, 4),
        ("一點附傷", 1, 0, 0, 1, 5),
        ("調整附傷", 3, 0, 0, 1, 7),
        ("格擋", 3, 100, 2, 1, 5),
        ("格擋", 3, 100, 100, 1, 0),
        ("暴擊", 3, 0, 0, 15, 14),
    ] {
        let (mut definitions, map) = skill_test_data(
            1,
            1,
            ActiveSkillEffect::FireClaw {
                power_source: PowerSource::Physical,
                ranged: false,
                hit_bonus: 0,
                power_bonus: 0,
                fire_damage: fire,
            },
            Team::Enemy("test_enemy".into()),
            "
        AT
        ",
        );
        definitions.unit_types[0].physical_power = 4;
        definitions.equipment.push(authoring::EquipmentDef {
            id: "guard".into(),
            slot: EquipmentSlot::Armor,
            hp: 0,
            physical_power: 0,
            magical_power: 0,
            block,
            block_reduction: reduction,
        });
        definitions.unit_types[1].armor = "guard".into();
        let mut game = Game::from_authoring(definitions, map, authoring::Difficulty::Easy)
            .expect("火焰爪資料應有效");
        game.command(Command::Start).expect("戰鬥應可開始");
        game.set_random_seed(seed);
        let preview = match game
            .preview_skill(ACTOR_ID, GridPos { x: 1, y: 0 }, TEST_SKILL_ID)
            .expect("應可預覽")
        {
            ActiveSkillPreview::Attack(preview) => preview,
            _ => panic!("應為攻擊預覽"),
        };
        assert_eq!(preview.hit_damage, 4 + fire, "{name}");
        let snapshot = game
            .command(Command::Skill {
                actor: ACTOR_ID,
                x: 1,
                y: 0,
                skill: TEST_SKILL_ID.into(),
            })
            .expect("應可施放火焰爪");
        let damage = snapshot
            .log
            .iter()
            .find_map(|event| match event {
                CombatLogEvent::Skill { damage, .. } => Some(*damage),
                _ => None,
            })
            .expect("應有攻擊日誌");
        assert_eq!(damage, expected, "{name}");
        assert_eq!(unit(&snapshot, TARGET_ID).hp, 100 - expected, "{name}");
    }
}

// 驗證碰撞反擊只攻擊直接碰撞的敵人，雙方免傷各自結算，不會反擊推擊施放者。
#[test]
fn collision_counter_targets_the_colliding_enemy() {
    use crate::support::add_passive_skill;
    use game_core::{PassiveSkillEffect, authoring};
    for (
        name,
        target_counter,
        blocker_counter,
        allied_blocker,
        expected_target,
        expected_blocker,
        expected_counters,
    ) in [
        (
            "被推者反擊碰撞者",
            true,
            false,
            true,
            100,
            93,
            vec![(TARGET_ID, BLOCKER_ID)],
        ),
        (
            "被撞者反擊碰撞者",
            false,
            true,
            true,
            93,
            100,
            vec![(BLOCKER_ID, TARGET_ID)],
        ),
        (
            "雙方各自免傷反擊",
            true,
            true,
            true,
            95,
            95,
            vec![(TARGET_ID, BLOCKER_ID), (BLOCKER_ID, TARGET_ID)],
        ),
        ("友軍碰撞只免傷", true, true, false, 100, 100, vec![]),
    ] {
        let (mut definitions, mut map) = skill_test_data(
            1,
            1,
            ActiveSkillEffect::Push {
                ranged: false,
                hit_bonus: 100,
            },
            Team::Enemy("test_enemy".into()),
            "
            ATB
            ",
        );
        if allied_blocker {
            map.units
                .iter_mut()
                .find(|unit| unit.id == BLOCKER_ID)
                .expect("應有碰撞者")
                .team = Team::Player;
        }
        for kind in &mut definitions.unit_types {
            kind.physical_power = 5;
            kind.hit = 100;
        }
        if target_counter {
            add_passive_skill(
                &mut definitions,
                "test_target",
                "counter_target",
                &[PassiveSkillEffect::CollisionCounter {
                    reactions_per_round: 1,
                }],
            );
        }
        if blocker_counter {
            add_passive_skill(
                &mut definitions,
                "test_blocker",
                "counter_blocker",
                &[PassiveSkillEffect::CollisionCounter {
                    reactions_per_round: 1,
                }],
            );
        }
        let mut game = Game::from_authoring(definitions, map, authoring::Difficulty::Easy)
            .expect("碰撞資料應有效");
        game.command(Command::Start).expect("戰鬥應可開始");
        game.set_random_seed(1);
        let snapshot = game
            .command(Command::Skill {
                actor: ACTOR_ID,
                x: 1,
                y: 0,
                skill: TEST_SKILL_ID.into(),
            })
            .expect("推擊應成功");
        assert_eq!(unit(&snapshot, TARGET_ID).hp, expected_target, "{name}");
        assert_eq!(unit(&snapshot, BLOCKER_ID).hp, expected_blocker, "{name}");
        assert_eq!(
            unit(&snapshot, ACTOR_ID).hp,
            100,
            "{name}：推擊者沒有直接碰撞"
        );
        let counters: Vec<_> = snapshot
            .log
            .iter()
            .filter_map(|event| match event {
                CombatLogEvent::Skill {
                    actor,
                    target,
                    skill,
                    ..
                } if skill.starts_with("counter_") => Some((*actor, *target)),
                _ => None,
            })
            .collect();
        assert_eq!(counters, expected_counters, "{name}");
        let log = serde_json::to_value(&snapshot.log[0]).expect("應可序列化碰撞日誌");
        assert_eq!(
            log["collision_damage"],
            if target_counter { 0 } else { 2 },
            "{name}"
        );
        assert_eq!(
            log["collision_units"][0]["damage"],
            if blocker_counter { 0 } else { 2 },
            "{name}"
        );
    }
}

// 驗證每次撞牆用掉一次免傷額度，用完後撞牆會扣血；設為零次時每次都扣血，下一輪恢復設定的免傷次數。
#[test]
fn collision_counter_wall_immunity_resets_each_round() {
    use crate::support::{add_passive_skill, end_turn_and_continue};
    use game_core::{PassiveSkillEffect, authoring};
    for (quota, expected_health) in [(0, [98, 96, 94]), (1, [100, 98, 98]), (2, [100, 100, 100])] {
        let (mut definitions, mut map) = skill_test_data(
            1,
            1,
            ActiveSkillEffect::Push {
                ranged: false,
                hit_bonus: 100,
            },
            Team::Enemy("test_enemy".into()),
            "
        ..#
        .AT
        ..B
        ",
        );
        // A 從左把 T 推向地圖邊界；B 從下把 T 推向岩壁，兩次都不產生直接碰撞敵人。
        map.units
            .iter_mut()
            .find(|unit| unit.id == BLOCKER_ID)
            .expect("應有第二名推擊者")
            .team = Team::Player;
        definitions
            .unit_types
            .iter_mut()
            .find(|unit| unit.id == "test_blocker")
            .expect("應有第二名推擊者")
            .initiative = 50;
        add_passive_skill(
            &mut definitions,
            "test_target",
            "counter",
            &[PassiveSkillEffect::CollisionCounter {
                reactions_per_round: quota,
            }],
        );
        let mut game = Game::from_authoring(definitions, map, authoring::Difficulty::Easy)
            .expect("撞牆測試資料應有效");
        game.command(Command::Start).expect("戰鬥應可開始");
        for (index, (actor, expected_hp)) in [ACTOR_ID, BLOCKER_ID, ACTOR_ID]
            .into_iter()
            .zip(expected_health)
            .enumerate()
        {
            assert_eq!(game.snapshot(None).turn.actor, Some(actor));
            game.set_random_seed(1);
            let snapshot = game
                .command(Command::Skill {
                    actor,
                    x: 2,
                    y: 1,
                    skill: TEST_SKILL_ID.into(),
                })
                .expect("推擊應成功");
            assert_eq!(unit(&snapshot, TARGET_ID).hp, expected_hp);
            assert_eq!(snapshot.log.iter().filter(|event| matches!(event, CombatLogEvent::Skill { skill, .. } if skill == "counter")).count(), 0);
            if index == 2 {
                break;
            }
            let mut snapshot = end_turn_and_continue(&mut game);
            if actor == BLOCKER_ID {
                snapshot = end_turn_and_continue(&mut game);
            }
            assert_eq!(
                snapshot.turn.actor,
                Some(if actor == ACTOR_ID {
                    BLOCKER_ID
                } else {
                    ACTOR_ID
                })
            );
        }
    }
}

// 驗證強制推移離開敵方近戰範圍時，不觸發藉機攻擊。
#[test]
fn pushing_does_not_provoke_opportunity_attacks() {
    use crate::support::add_passive_skill;
    use game_core::{PassiveSkillEffect, authoring};
    let (mut definitions, mut map) = skill_test_data(
        1,
        1,
        ActiveSkillEffect::Push {
            ranged: false,
            hit_bonus: 100,
        },
        Team::Enemy("test_enemy".into()),
        "
        ..B.
        .AT.
        ",
    );
    map.units
        .iter_mut()
        .find(|unit| unit.id == BLOCKER_ID)
        .expect("應有反應者")
        .team = Team::Player;
    add_passive_skill(
        &mut definitions,
        "test_blocker",
        "opportunity",
        &[PassiveSkillEffect::OpportunityAttack {
            reactions_per_round: 2,
        }],
    );
    let mut game = Game::from_authoring(definitions, map, authoring::Difficulty::Easy)
        .expect("推移反應資料應有效");
    game.command(Command::Start).expect("戰鬥應可開始");
    game.set_random_seed(1);
    let snapshot = game
        .command(Command::Skill {
            actor: ACTOR_ID,
            x: 2,
            y: 1,
            skill: TEST_SKILL_ID.into(),
        })
        .expect("應可推移敵人");
    assert_eq!(
        (unit(&snapshot, TARGET_ID).x, unit(&snapshot, TARGET_ID).y),
        (3, 1)
    );
    assert_eq!(unit(&snapshot, TARGET_ID).hp, 100);
    assert_eq!(snapshot.log.iter().filter(|event| matches!(event, CombatLogEvent::Skill { skill, .. } if skill == "opportunity")).count(), 0);
}

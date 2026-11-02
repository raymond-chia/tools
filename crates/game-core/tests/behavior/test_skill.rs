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

// 驗證單位專屬被動只覆蓋預設中的相同效果，禁止治癒與技能說明保持一致。
#[test]
fn passive_bundle_override_preserves_other_effects() {
    for (default_effects, unit_effects, expected) in [
        // 預設有禁止治療及夾擊 +2；專屬夾擊 +4 只替換夾擊，保留預設的禁止治療。
        (
            serde_json::json!([{"effect": "unhealable"}, {"effect": "flanking", "hit_bonus": 2}]),
            serde_json::json!([{"effect": "flanking", "hit_bonus": 4}]),
            serde_json::json!([
                {"id": "baseline", "effects": [{"effect": "unhealable"}]},
                {"id": "mechanical", "effects": [{"effect": "flanking", "hit_bonus": 4}]}
            ]),
        ),
        // 預設只有夾擊 +2；專屬組合提供禁止治療及夾擊 +4，預設被完全取代。
        (
            serde_json::json!([{"effect": "flanking", "hit_bonus": 2}]),
            serde_json::json!([{"effect": "unhealable"}, {"effect": "flanking", "hit_bonus": 4}]),
            serde_json::json!([
                {"id": "mechanical", "effects": [{"effect": "unhealable"}, {"effect": "flanking", "hit_bonus": 4}]}
            ]),
        ),
    ] {
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
            {"id": "baseline", "effects": default_effects},
            {"id": "mechanical", "effects": unit_effects}
        ]))
        .expect("組合被動應符合作者格式");
        definitions.default_passive_skills = vec!["baseline".into()];
        definitions.unit_types[0].passive_skills = vec!["mechanical".into()];
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

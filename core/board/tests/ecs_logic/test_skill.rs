//! 技能系統 ECS 操作測試

use super::constants::{
    SKILL_DIAMOND_AOE, SKILL_MELEE, SKILL_WARRIOR_ACTIVE_2, SKILL_WARRIOR_ACTIVE_4,
    UNIT_TYPE_WARRIOR,
};
use super::setup_world_with_level;
use crate::helpers::level_builder::LevelBuilder;
use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use board::domain::constants::PLAYER_FACTION_ID;
use board::ecs_logic::skill::{
    AvailableSkill, PreviewAffectedPositions, get_available_skills, get_skill_affected_positions,
};
use board::ecs_logic::turn::start_new_round;
use board::ecs_types::components::{ActionState, CurrentMp, Occupant, Position};
use std::collections::{HashMap, HashSet};

const ENEMY_FACTION_ID: u32 = 2;

fn build_warrior_world(ascii: &str, mp: i32) -> (World, Occupant, HashMap<String, Vec<Position>>) {
    let (mut world, occupant, markers) = super::build_warrior_world(ascii);

    start_new_round(&mut world).expect("start_new_round 應成功");

    let entity = {
        let mut query = world.query::<(Entity, &Occupant)>();
        query
            .iter(&world)
            .find(|(_, occ)| **occ == occupant)
            .map(|(e, _)| e)
            .expect("應找到指定單位的 Entity")
    };
    world.entity_mut(entity).insert(CurrentMp(mp));

    (world, occupant, markers)
}

fn build_mage_world(ascii: &str) -> (World, HashMap<String, Vec<Position>>) {
    let (mut world, _, markers) = super::build_mage_world(ascii);

    start_new_round(&mut world).expect("start_new_round 應成功");

    (world, markers)
}

/// 設定指定單位的 ActionState
fn set_active_action_state(world: &mut World, occupant: Occupant, state: ActionState) {
    let entity = {
        let mut query = world.query::<(Entity, &Occupant)>();
        query
            .iter(world)
            .find(|(_, occ)| **occ == occupant)
            .map(|(e, _)| e)
            .expect("應找到指定單位的 Entity")
    };
    world.entity_mut(entity).insert(state);
}

// ============================================================================
// MP 與 usable 判定
// ============================================================================

#[test]
fn test_usable_depends_on_mp() {
    let test_data = [
        // (current_mp, [cost=0 usable, cost=2 usable, cost=4 usable])
        (0, [true, false, false]),
        (1, [true, false, false]),
        (2, [true, true, false]),
        (3, [true, true, false]),
        (4, [true, true, true]),
        (10, [true, true, true]),
    ];
    let skill_names = [SKILL_MELEE, SKILL_WARRIOR_ACTIVE_2, SKILL_WARRIOR_ACTIVE_4];

    for (mp, expected_usable) in &test_data {
        let (mut world, _, _) = build_warrior_world(
            "
            . . . . .
            . P . . .
            . . . E .
            . . . . .
            ",
            *mp,
        );

        let skills_vec = get_available_skills(&mut world).expect("get_available_skills 應成功");
        let skills: HashMap<&str, _> = skills_vec.iter().map(|s| (s.name.as_str(), s)).collect();

        // 只應回傳 active 技能
        let expected_active: HashSet<&str> = HashSet::from(skill_names);
        let actual_names: HashSet<&str> = skills.keys().copied().collect();
        assert_eq!(
            actual_names, expected_active,
            "Active 技能不符，預期: {:?}，實際: {:?}",
            expected_active, actual_names,
        );

        for (i, name) in skill_names.iter().enumerate() {
            let skill = skills
                .get(*name)
                .expect(&format!("MP={} 時應包含技能 {}", mp, name));
            assert_eq!(
                skill.usable, expected_usable[i],
                "MP={} 時技能 {} 的 usable 應為 {}，實際為 {}",
                mp, name, expected_usable[i], skill.usable,
            );
        }
    }
}

// ============================================================================
// ActionState::Done 時全部 usable = false
// ============================================================================

#[test]
fn test_done_state_all_unusable() {
    let assert_usable = |skills: &Vec<AvailableSkill>, usable: bool, msg: &str| {
        assert_eq!(skills.len(), 3, "{} 仍應回傳 3 個 Active 技能", msg);

        for skill in skills {
            assert_eq!(
                skill.usable, usable,
                "{} 時技能 {} 應為 usable={}",
                msg, skill.name, usable,
            );
        }
    };

    for mp in [100, 1000] {
        let (mut world, player_occupant, _) = build_warrior_world(
            "
            . . . . .
            . P . . .
            . . . E .
            . . . . .
            ",
            mp,
        );

        let state = ActionState::Done;
        let msg = format!("MP={mp} {state:?} 狀態");
        set_active_action_state(&mut world, player_occupant, state);
        let skills = get_available_skills(&mut world).expect("get_available_skills 應成功");
        assert_usable(&skills, false, &msg);

        let state = ActionState::Moved { cost: 20 };
        let msg = format!("MP={mp} {state:?} 狀態");
        set_active_action_state(&mut world, player_occupant, state);
        let skills = get_available_skills(&mut world).expect("get_available_skills 應成功");
        assert_usable(&skills, true, &msg);

        let state = ActionState::Moved { cost: 49 };
        let msg = format!("MP={mp} {state:?} 狀態");
        set_active_action_state(&mut world, player_occupant, state);
        let skills = get_available_skills(&mut world).expect("get_available_skills 應成功");
        assert_usable(&skills, true, &msg);

        let state = ActionState::Moved { cost: 50 };
        let msg = format!("MP={mp} {state:?} 狀態");
        set_active_action_state(&mut world, player_occupant, state);
        let skills = get_available_skills(&mut world).expect("get_available_skills 應成功");
        assert_usable(&skills, true, &msg);

        let state = ActionState::Moved { cost: 51 };
        let msg = format!("MP={mp} {state:?} 狀態");
        set_active_action_state(&mut world, player_occupant, state);
        let skills = get_available_skills(&mut world).expect("get_available_skills 應成功");
        assert_usable(&skills, false, &msg);

        let state = ActionState::Moved { cost: 60 };
        let msg = format!("MP={mp} {state:?} 狀態");
        set_active_action_state(&mut world, player_occupant, state);
        let skills = get_available_skills(&mut world).expect("get_available_skills 應成功");
        assert_usable(&skills, false, &msg);
    }
}

// ============================================================================
// 沒有 active unit 時回傳錯誤
// ============================================================================

#[test]
fn test_no_active_unit_returns_error() {
    let level_toml = LevelBuilder::from_ascii(
        "
        . . . . .
        . P . . .
        . . . E .
        . . . . .
        ",
    )
    .unit("P", UNIT_TYPE_WARRIOR, PLAYER_FACTION_ID)
    .unit("E", UNIT_TYPE_WARRIOR, ENEMY_FACTION_ID)
    .to_toml()
    .expect("LevelBuilder::to_toml 應成功");
    let mut world = setup_world_with_level(&level_toml);

    // 不呼叫 start_new_round
    let result = get_available_skills(&mut world);
    assert!(result.is_err(), "沒有 active unit 時應回傳錯誤");
}

// ============================================================================
// 查詢可選目標格子（基於射程）
// ============================================================================

#[test]
fn test_skill_targetable_positions_various_ranges() {
    // (skill_name, ascii)
    let test_data = [
        // SKILL_MELEE range=[1,1]
        (
            SKILL_MELEE,
            "
            . . . . .
            . . + . .
            . + P + .
            . . + . .
            . . . . .
            ",
        ),
        (
            SKILL_MELEE,
            "
            P + . . .
            + . . . .
            . . . . .
            . . . . .
            . . . . .
            ",
        ),
        (
            SKILL_MELEE,
            "
            . . . + P
            . . . . +
            . . . . .
            . . . . .
            . . . . .
            ",
        ),
        (
            SKILL_MELEE,
            "
            . . . . .
            . . . . .
            . . . . .
            . . + . .
            . + P + .
            ",
        ),
        (
            SKILL_MELEE,
            "
            . . . . .
            . . . . .
            . . . + .
            . . + P +
            . . . + .
            ",
        ),
        // SKILL_WARRIOR_ACTIVE_2 range=[1,2]
        (
            SKILL_WARRIOR_ACTIVE_2,
            "
            . . + . .
            . + + + .
            + + P + +
            . + + + .
            . . + . .
            ",
        ),
        (
            SKILL_WARRIOR_ACTIVE_2,
            "
            P + + . .
            + + . . .
            + . . . .
            . . . . .
            . . . . .
            ",
        ),
        (
            SKILL_WARRIOR_ACTIVE_2,
            "
            . . + + P
            . . . + +
            . . . . +
            . . . . .
            . . . . .
            ",
        ),
        (
            SKILL_WARRIOR_ACTIVE_2,
            "
            . . . . .
            . . . . .
            . . + . .
            . + + + .
            + + P + +
            ",
        ),
        (
            SKILL_WARRIOR_ACTIVE_2,
            "
            . . . . .
            . . . + .
            . . + + +
            . + + P +
            . . + + +
            ",
        ),
    ];

    for (skill_name, ascii) in &test_data {
        let (mut world, _, markers) = build_warrior_world(ascii, 100);
        let positions: HashSet<_> = board::ecs_logic::skill::get_skill_targetable_positions(
            &mut world,
            &skill_name.to_string(),
        )
        .expect("get_skill_targetable_positions 應成功")
        .into_iter()
        .collect();

        let expected_positions: HashSet<_> = markers
            .get("+")
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();

        assert_eq!(
            positions,
            expected_positions,
            "技能 {} 佈局:\n{}\n應回傳 {} 個格子，實際 {}",
            skill_name,
            ascii,
            expected_positions.len(),
            positions.len()
        );
    }
}

// ============================================================================
// 預覽 AOE 影響範圍
// ============================================================================

#[test]
fn get_skill_affected_positions_diamond() {
    /// 從 markers 收集預期影響格子（T + A）
    fn collect_expected_affected(
        markers: &HashMap<String, Vec<Position>>,
    ) -> Result<HashSet<Position>, String> {
        let set: HashSet<_> = ["T", "A"]
            .iter()
            .flat_map(|key| markers.get(*key).into_iter().flatten().copied())
            .collect();
        // 假設只有 T 代表超出施放範圍
        if set.len() == 1 {
            Err("只有 T 代表超出施放範圍".to_string())
        } else {
            Ok(set)
        }
    }

    let test_data = [
        "
        . . . . .
        . . T . .
        . . . . .
        . . . P .
        . . . . .
        ",
        "
        . . . . .
        . . A . .
        . A T A .
        . P A . .
        . . . . .
        ",
        "
        . . . . .
        . . . A .
        . . A T A
        . . . P .
        . . . . .
        ",
        "
        . . . . .
        . . . . .
        . . . . A
        . . . P T
        . . . . A
        ",
    ];

    for ascii in &test_data {
        let (mut world, markers) = build_mage_world(ascii);
        let caster_pos = markers["P"][0];
        let target_pos = markers["T"][0];
        let expected = collect_expected_affected(&markers);

        let result =
            get_skill_affected_positions(&mut world, &SKILL_DIAMOND_AOE.to_string(), target_pos);
        match expected {
            Ok(mut expected) => {
                let PreviewAffectedPositions {
                    all_positions,
                    filtered_positions,
                } = result.expect(&format!("preview 應成功：{ascii}"));
                let all_positions = all_positions.into_iter().collect::<HashSet<_>>();
                let filtered_positions = filtered_positions.into_iter().collect::<HashSet<_>>();

                assert_eq!(filtered_positions, expected, "預覽 AOE 不符：{ascii}");
                // 本測試只有過濾 caster, 所以預期只要檢查是否差異在 caster
                if all_positions.contains(&caster_pos) {
                    expected.insert(caster_pos);
                }
                assert_eq!(all_positions, expected, "預覽 AOE 全部格子不符：{ascii}");
            }
            Err(msg) => assert!(result.is_err(), "預期失敗但成功了: {msg}\n佈局: {ascii}\n"),
        }
    }
}

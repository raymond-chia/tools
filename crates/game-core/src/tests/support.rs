use crate::*;

pub(super) const ACTOR_ID: i64 = 1;
pub(super) const TARGET_ID: i64 = 2;
pub(super) const BLOCKER_ID: i64 = 3;

pub(super) fn tank_ai_profile() -> authoring::AiProfile {
    authoring::AiProfile {
        id: "tank".into(),
        distance_preference: authoring::DistancePreference::Near,
        damage_weight: 10,
        healing_weight: 0,
        positioning_weight: 10,
        hit_weight: 0,
        pursuit_weight: 0,
    }
}

pub(super) fn push_collision_game() -> Game {
    game_with_skill_range_and_effect(
        1,
        1,
        SkillEffect::Push {
            attack_bonus: 100,
            power_bonus: 0,
        },
    )
}

pub(super) fn game_with_skill_range_and_effect(
    min_range: i32,
    max_range: i32,
    effect: SkillEffect,
) -> Game {
    let terrain = TerrainTypeDef {
        id: "plain".into(),
        layer: TerrainLayer::Ground,
        entry_rule: TerrainEntryRule::Walkable,
        damage: 0,
        extra_movement_cost: 0,
        dodge_penalty: 0,
        block_penalty: 0,
    };
    let mut rough = terrain.clone();
    rough.id = "rough".into();
    let definitions = authoring::Definitions {
        ai_profiles: vec![tank_ai_profile()],
        terrain_types: vec![terrain, rough],
        skills: vec![SkillDef {
            id: "push".into(),
            ranged: false,
            min_range,
            max_range,
            effect,
        }],
        unit_types: vec![
            push_collision_unit_type("test_player", 100),
            push_collision_unit_type("test_unit", 0),
        ],
    };
    let map = authoring::Map {
        name: "test_map".into(),
        width: 5,
        height: 2,
        terrains: Vec::new(),
        units: vec![
            push_collision_placement(ACTOR_ID, "test_player", Team::Player, 1),
            push_collision_placement(TARGET_ID, "test_unit", Team::Enemy("test_enemy".into()), 2),
            push_collision_placement(BLOCKER_ID, "test_unit", Team::Enemy("test_enemy".into()), 3),
        ],
    };
    match Game::from_authoring(definitions, map) {
        Ok(game) => game,
        Err(error) => panic!("測試戰鬥定義應有效：{}", error.message()),
    }
}

fn push_collision_unit_type(id: &str, initiative: i32) -> authoring::UnitType {
    authoring::UnitType {
        ai_profile: "tank".into(),
        id: id.into(),
        visual: id.into(),
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

fn push_collision_placement(
    id: i64,
    unit_type: &str,
    team: Team,
    x: i32,
) -> authoring::UnitPlacement {
    authoring::UnitPlacement {
        id,
        unit_type: unit_type.into(),
        team,
        x,
        y: 1,
    }
}

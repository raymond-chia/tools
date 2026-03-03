use super::constants::{
    OBJECT_TYPE_PIT, OBJECT_TYPE_WALL, OBJECTS_TOML, SKILL_MELEE, SKILL_WARRIOR, SKILLS_TOML,
    UNIT_TYPE_MAGE, UNIT_TYPE_WARRIOR, UNITS_TOML,
};
use bevy_ecs::prelude::World;
use board::ecs_logic::loader::parse_and_insert_game_data;
use board::ecs_types::resources::GameData;

#[test]
fn test_parse_and_insert_game_data_sets_resource() {
    let mut world = World::new();

    let result = parse_and_insert_game_data(&mut world, UNITS_TOML, SKILLS_TOML, OBJECTS_TOML);
    assert!(
        result.is_ok(),
        "parse_and_insert_game_data 應成功：{:?}",
        result
    );

    let game_data = world.get_resource::<GameData>();
    assert!(game_data.is_some(), "GameData resource 應已存入 World");

    let game_data = game_data.expect("GameData resource 應已存入 World");
    assert_eq!(game_data.skill_map.len(), 3, "skill_map 應包含 3 個技能");
    assert!(
        game_data.skill_map.contains_key(SKILL_WARRIOR),
        "skill_map 應包含 {SKILL_WARRIOR}"
    );
    assert!(
        game_data.skill_map.contains_key(SKILL_MELEE),
        "skill_map 應包含 {SKILL_MELEE}"
    );
    assert_eq!(
        game_data.unit_type_map.len(),
        2,
        "unit_type_map 應包含 2 個單位類型"
    );
    assert!(
        game_data.unit_type_map.contains_key(UNIT_TYPE_WARRIOR),
        "unit_type_map 應包含 {UNIT_TYPE_WARRIOR}"
    );
    assert!(
        game_data.unit_type_map.contains_key(UNIT_TYPE_MAGE),
        "unit_type_map 應包含 {UNIT_TYPE_MAGE}"
    );
    assert_eq!(
        game_data.object_type_map.len(),
        2,
        "object_type_map 應包含 2 個物件類型"
    );
    assert!(
        game_data.object_type_map.contains_key(OBJECT_TYPE_WALL),
        "object_type_map 應包含 {OBJECT_TYPE_WALL}"
    );
    assert!(
        game_data.object_type_map.contains_key(OBJECT_TYPE_PIT),
        "object_type_map 應包含 {OBJECT_TYPE_PIT}"
    );
}

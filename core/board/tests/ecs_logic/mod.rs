mod constants;
mod test_deployment;
mod test_loader;
mod test_movement;
mod test_query;
mod test_reaction;
mod test_skill;
mod test_skill_list;
mod test_skill_targeting;
mod test_spawner;
mod test_turn;

use bevy_ecs::prelude::{Entity, World};
use board::domain::constants::PLAYER_FACTION_ID;
use board::ecs_logic::loader::parse_and_insert_game_data;
use board::ecs_logic::spawner::spawn_level;
use board::ecs_types::components::{CurrentMp, Initiative, Occupant, Position};
use board::test_helpers::level_builder::{LevelBuilder, load_from_ascii};
use constants::{
    OBJECT_TYPE_SWAMP, OBJECT_TYPE_WALL, OBJECTS_TOML, SKILLS_TOML, UNIT_TYPE_MAGE,
    UNIT_TYPE_WARRIOR, UNITS_TOML,
};
use std::collections::HashMap;

const ALLY_FACTION_ID: u32 = 1;
const ENEMY_FACTION_ID: u32 = 2;

fn setup_world_with_level(level_toml: &str) -> World {
    let mut world = World::new();
    parse_and_insert_game_data(&mut world, UNITS_TOML, SKILLS_TOML, OBJECTS_TOML)
        .expect("parse_and_insert_game_data 應成功");
    spawn_level(&mut world, level_toml, "test-level").expect("spawn_level 應成功");
    world
}

fn build_warrior_world(ascii: &str) -> (World, Occupant, HashMap<String, Vec<Position>>) {
    let (_, markers) = load_from_ascii(ascii).expect("load_from_ascii 應成功");

    let level_toml = LevelBuilder::from_ascii(ascii)
        .unit("P", UNIT_TYPE_WARRIOR, PLAYER_FACTION_ID)
        .unit("A", UNIT_TYPE_WARRIOR, ALLY_FACTION_ID)
        .unit("E", UNIT_TYPE_WARRIOR, ENEMY_FACTION_ID)
        .object("w", OBJECT_TYPE_WALL)
        .object("p", OBJECT_TYPE_SWAMP)
        .to_toml()
        .expect("LevelBuilder::to_toml 應成功");
    let mut world = setup_world_with_level(&level_toml);

    // 從 markers 直接取得玩家位置
    let player_pos = markers["P"][0];
    let (player_entity, occupant) = {
        let mut query = world.query::<(Entity, &Occupant, &Position)>();
        query
            .iter(&world)
            .find(|(_, _, p)| **p == player_pos)
            .map(|(entity, occ, _)| (entity, *occ))
            .expect("應找到玩家單位")
    };
    // 覆蓋 init
    world.entity_mut(player_entity).insert(Initiative(100));
    (world, occupant, markers)
}

/// 建立以 mage 為玩家單位的 World
fn build_mage_world(ascii: &str) -> (World, Occupant, HashMap<String, Vec<Position>>) {
    let (_, markers) = load_from_ascii(ascii).expect("load_from_ascii 應成功");

    let level_toml = LevelBuilder::from_ascii(ascii)
        .unit("P", UNIT_TYPE_MAGE, PLAYER_FACTION_ID)
        .unit("A", UNIT_TYPE_WARRIOR, ALLY_FACTION_ID)
        .unit("E", UNIT_TYPE_WARRIOR, ENEMY_FACTION_ID)
        .object("w", OBJECT_TYPE_WALL)
        .object("p", OBJECT_TYPE_SWAMP)
        .to_toml()
        .expect("LevelBuilder::to_toml 應成功");
    let mut world = setup_world_with_level(&level_toml);

    let player_pos = markers["P"][0];
    let (player_entity, occupant) = {
        let mut query = world.query::<(Entity, &Occupant, &Position)>();
        query
            .iter(&world)
            .find(|(_, _, p)| **p == player_pos)
            .map(|(entity, occ, _)| (entity, *occ))
            .expect("應找到玩家單位")
    };
    world.entity_mut(player_entity).insert(Initiative(100));
    world.entity_mut(player_entity).insert(CurrentMp(100));
    (world, occupant, markers)
}

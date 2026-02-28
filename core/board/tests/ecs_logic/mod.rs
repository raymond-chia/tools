mod constants;
mod deployment;
mod loader;
mod query;
mod spawner;

use bevy_ecs::prelude::World;
use board::ecs_logic::loader::parse_and_insert_game_data;
use board::ecs_logic::spawner::spawn_level;
use constants::{OBJECTS_TOML, SKILLS_TOML, UNITS_TOML};

fn setup_world_with_level(level_toml: &str) -> World {
    let mut world = World::new();
    parse_and_insert_game_data(&mut world, UNITS_TOML, SKILLS_TOML, OBJECTS_TOML)
        .expect("parse_and_insert_game_data 應成功");
    spawn_level(&mut world, level_toml, "test-level").expect("spawn_level 應成功");
    world
}

use crate::domain::alias::ID;
use crate::ecs_types::components::{
    BlocksSight, BlocksSound, ContactEffects, MovementUsed, Object, ObjectBundle, Occupant,
    OccupantTypeName, Skills, TerrainMovementCost, Unit, UnitBundle, UnitFaction,
};
use crate::ecs_types::resources::{Board, DeploymentConfig, GameData, LevelConfig};
use crate::error::{DataError, LoadError, Result};
use crate::loader_schema::LevelType;
use crate::logic::debug::short_type_name;
use crate::logic::id_generator::generate_unique_id;
use crate::logic::unit_attributes;
use bevy_ecs::prelude::World;
use std::collections::HashSet;

/// 反序列化並生成關卡的所有 Entity（棋盤、單位、物件）
pub fn spawn_level(world: &mut World, level_toml: &str, level_name: &str) -> Result<()> {
    let level: LevelType = toml::from_str(level_toml).map_err(|e| LoadError::DeserializeError {
        format: level_name.to_string(),
        reason: e.to_string(),
    })?;

    // 第一階段：借用 GameData，預先收集所有需要 spawn 的資料
    let (unit_bundles, object_spawn_data) = {
        let game_data = world
            .get_resource::<GameData>()
            .ok_or(DataError::MissingResource {
                name: short_type_name::<GameData>(),
                note: "請先呼叫 parse_and_insert_game_data".to_string(),
            })?;

        let mut used_ids: HashSet<ID> = HashSet::new();
        let mut unit_bundles: Vec<UnitBundle> = Vec::new();
        for placement in &level.unit_placements {
            let id = generate_unique_id(&mut used_ids)?;
            let unit_type = game_data
                .unit_type_map
                .get(&placement.unit_type_name)
                .ok_or_else(|| DataError::UnitTypeNotFound {
                    type_name: placement.unit_type_name.clone(),
                })?;
            let no_buffs = &vec![];
            let effects = unit_attributes::filter_continuous_effect(
                &unit_type.skills,
                no_buffs,
                &game_data.skill_map,
            )?;
            let attributes = unit_attributes::calculate_attributes(effects);

            unit_bundles.push(UnitBundle {
                unit: Unit,
                position: placement.position,
                occupant: Occupant::Unit(id),
                occupant_type_name: OccupantTypeName(unit_type.name.clone()),
                unit_faction: UnitFaction(placement.faction_id),
                skills: Skills(unit_type.skills.clone()),
                attributes,
                movement_used: MovementUsed(0),
            });
        }

        let mut object_spawn_data: Vec<(ObjectBundle, Option<BlocksSight>, Option<BlocksSound>)> =
            Vec::new();
        for placement in &level.object_placements {
            let id = generate_unique_id(&mut used_ids)?;
            let object_type = game_data
                .object_type_map
                .get(&placement.object_type_name)
                .ok_or_else(|| DataError::ObjectTypeNotFound {
                    type_name: placement.object_type_name.clone(),
                })?;

            object_spawn_data.push((
                ObjectBundle {
                    object: Object,
                    position: placement.position,
                    occupant: Occupant::Object(id),
                    occupant_type_name: OccupantTypeName(object_type.name.clone()),
                    terrain_movement_cost: TerrainMovementCost(object_type.movement_cost),
                    contact_effects: ContactEffects(Vec::new()),
                },
                object_type.blocks_sight.then_some(BlocksSight),
                object_type.blocks_sound.then_some(BlocksSound),
            ));
        }

        (unit_bundles, object_spawn_data)
    };

    // 第二階段：GameData 借用已結束，可以可變借用 world 進行 spawn

    // 插入 Board resource
    world.insert_resource(Board {
        width: level.board_width,
        height: level.board_height,
    });

    // 插入 LevelConfig resource
    world.insert_resource(LevelConfig {
        name: level.name,
        factions: level.factions.into_iter().map(|f| (f.id, f)).collect(),
    });

    // 插入 DeploymentConfig resource
    world.insert_resource(DeploymentConfig {
        max_player_units: level.max_player_units,
        deployment_positions: level.deployment_positions.into_iter().collect(),
    });

    // Spawn Unit entities
    for bundle in unit_bundles {
        world.spawn(bundle);
    }

    // Spawn Object entities
    for (bundle, blocks_sight, blocks_sound) in object_spawn_data {
        let mut entity = world.spawn(bundle);

        if let Some(tag) = blocks_sight {
            entity.insert(tag);
        }

        if let Some(tag) = blocks_sound {
            entity.insert(tag);
        }
    }

    Ok(())
}

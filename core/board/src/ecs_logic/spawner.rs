use crate::domain::alias::ID;
use crate::ecs_types::components::{
    AttributeBundle, Block, BlockProtection, BlocksSight, BlocksSound, CurrentHp, CurrentMp,
    Evasion, Faction, Fortitude, Hit, HpModify, Initiative, MagicalAttack, MagicalDc, MaxHp, MaxMp,
    Movement, Object, ObjectBundle, Occupant, OccupantTypeName, PhysicalAttack, Position, Reaction,
    Reflex, Skills, TerrainMovementCost, Unit, UnitBundle, Will,
};
use crate::ecs_types::resources::{Board, DeploymentConfig, GameData, LevelConfig};
use crate::error::{DataError, LoadError, Result};
use crate::loader_schema::{BuffEffect, LevelType};
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
            .ok_or(DataError::GameDataNotFound)?;

        let mut used_ids: HashSet<ID> = HashSet::new();

        let mut unit_bundles: Vec<UnitBundle> = Vec::new();
        for placement in &level.unit_placements {
            let unit_type = game_data
                .unit_type_map
                .get(&placement.unit_type_name)
                .ok_or_else(|| DataError::UnitTypeNotFound {
                    type_name: placement.unit_type_name.clone(),
                })?;

            let no_buffs: &[BuffEffect] = &[];
            let attributes = unit_attributes::calculate_attributes(
                &unit_type.skills,
                no_buffs,
                &game_data.skill_map,
            )?;

            unit_bundles.push(UnitBundle {
                unit: Unit,
                position: Position {
                    x: placement.position.x,
                    y: placement.position.y,
                },
                occupant: Occupant::Unit(generate_unique_id(&mut used_ids)),
                occupant_type_name: OccupantTypeName(unit_type.name.clone()),
                faction: Faction(placement.faction_id),
                skills: Skills(unit_type.skills.clone()),
                attributes: AttributeBundle {
                    max_hp: MaxHp(attributes.hp),
                    current_hp: CurrentHp(attributes.hp),
                    max_mp: MaxMp(attributes.mp),
                    current_mp: CurrentMp(attributes.mp),
                    initiative: Initiative(attributes.initiative),
                    hit: Hit(attributes.hit),
                    evasion: Evasion(attributes.evasion),
                    block: Block(attributes.block),
                    block_protection: BlockProtection(attributes.block_protection),
                    physical_attack: PhysicalAttack(attributes.physical_attack),
                    magical_attack: MagicalAttack(attributes.magical_attack),
                    magical_dc: MagicalDc(attributes.magical_dc),
                    fortitude: Fortitude(attributes.fortitude),
                    reflex: Reflex(attributes.reflex),
                    will: Will(attributes.will),
                    movement: Movement(attributes.movement),
                    reaction: Reaction(attributes.reaction),
                },
            });
        }

        let mut object_spawn_data: Vec<(ObjectBundle, Option<BlocksSight>, Option<BlocksSound>)> =
            Vec::new();
        for placement in &level.object_placements {
            let object_type = game_data
                .object_type_map
                .get(&placement.object_type_name)
                .ok_or_else(|| DataError::ObjectTypeNotFound {
                    type_name: placement.object_type_name.clone(),
                })?;

            object_spawn_data.push((
                ObjectBundle {
                    object: Object,
                    position: Position {
                        x: placement.position.x,
                        y: placement.position.y,
                    },
                    occupant: Occupant::Object(generate_unique_id(&mut used_ids)),
                    occupant_type_name: OccupantTypeName(object_type.name.clone()),
                    terrain_movement_cost: TerrainMovementCost(object_type.movement_cost),
                    hp_modify: HpModify(object_type.hp_modify),
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
        name: level.name.clone(),
        factions: level.factions.clone(),
    });

    // 插入 DeploymentConfig resource
    world.insert_resource(DeploymentConfig {
        max_player_units: level.max_player_units,
        deployment_positions: level.deployment_positions.clone(),
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

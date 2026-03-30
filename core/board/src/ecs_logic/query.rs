use super::clone_component;
use crate::ecs_types::components::{
    Accuracy, ActionState, AttributeBundle, Block, BlockProtection, BlocksSight, BlocksSound,
    ContactEffects, CurrentHp, CurrentMp, Evasion, Fortitude, Initiative, MagicalAttack, MagicalDc,
    MaxHp, MaxMp, MovementPoint, Object, ObjectBundle, Occupant, OccupantTypeName, PhysicalAttack,
    Position, ReactionPoint, Reflex, Skills, TerrainMovementCost, Unit, UnitBundle, UnitFaction,
    Will,
};
use crate::ecs_types::resources::{Board, DeploymentConfig, LevelConfig};
use crate::error::{DataError, Result};
use crate::logic::debug::short_type_name;
use bevy_ecs::prelude::{Entity, With, World};
use std::collections::HashMap;

/// 取得棋盤尺寸
pub fn get_board(world: &World) -> Result<Board> {
    world.get_resource::<Board>().copied().ok_or_else(|| {
        DataError::MissingResource {
            name: short_type_name::<Board>(),
            note: "請先呼叫 spawn_level".to_string(),
        }
        .into()
    })
}

/// 取得部署設定
pub fn get_deployment_config(world: &World) -> Result<DeploymentConfig> {
    world
        .get_resource::<DeploymentConfig>()
        .cloned()
        .ok_or_else(|| {
            DataError::MissingResource {
                name: short_type_name::<DeploymentConfig>(),
                note: "請先呼叫 spawn_level".to_string(),
            }
            .into()
        })
}

/// 取得關卡設定
pub fn get_level_config(world: &World) -> Result<LevelConfig> {
    world.get_resource::<LevelConfig>().cloned().ok_or_else(|| {
        DataError::MissingResource {
            name: short_type_name::<LevelConfig>(),
            note: "請先呼叫 spawn_level".to_string(),
        }
        .into()
    })
}

/// 查詢所有單位，以位置為 key
pub fn get_all_units(world: &mut World) -> Result<HashMap<Position, UnitBundle>> {
    let entities: Vec<(Entity, Position)> = world
        .query_filtered::<(Entity, &Position), With<Unit>>()
        .iter(world)
        .map(|(entity, pos)| (entity, *pos))
        .collect();

    let mut result = HashMap::new();
    for (entity, position) in entities {
        let entity_ref = world.entity(entity);
        let bundle = UnitBundle {
            unit: Unit,
            position,
            occupant: clone_component!(entity_ref, Occupant),
            occupant_type_name: clone_component!(entity_ref, OccupantTypeName),
            unit_faction: clone_component!(entity_ref, UnitFaction),
            skills: clone_component!(entity_ref, Skills),
            attributes: AttributeBundle {
                max_hp: clone_component!(entity_ref, MaxHp),
                current_hp: clone_component!(entity_ref, CurrentHp),
                max_mp: clone_component!(entity_ref, MaxMp),
                current_mp: clone_component!(entity_ref, CurrentMp),
                initiative: clone_component!(entity_ref, Initiative),
                accuracy: clone_component!(entity_ref, Accuracy),
                evasion: clone_component!(entity_ref, Evasion),
                block: clone_component!(entity_ref, Block),
                block_protection: clone_component!(entity_ref, BlockProtection),
                physical_attack: clone_component!(entity_ref, PhysicalAttack),
                magical_attack: clone_component!(entity_ref, MagicalAttack),
                magical_dc: clone_component!(entity_ref, MagicalDc),
                fortitude: clone_component!(entity_ref, Fortitude),
                reflex: clone_component!(entity_ref, Reflex),
                will: clone_component!(entity_ref, Will),
                movement_point: clone_component!(entity_ref, MovementPoint),
                reaction_point: clone_component!(entity_ref, ReactionPoint),
            },
            action_state: clone_component!(entity_ref, ActionState),
        };
        result.insert(position, bundle);
    }
    Ok(result)
}

/// 物件查詢結果（包含 Bundle 資料 + 可選 tag）
#[derive(Debug)]
pub struct ObjectQueryResult {
    pub bundle: ObjectBundle,
    pub blocks_sight: bool,
    pub blocks_sound: bool,
}

/// 查詢所有物件，以位置為 key
pub fn get_all_objects(world: &mut World) -> Result<HashMap<Position, ObjectQueryResult>> {
    let entities: Vec<(Entity, Position)> = world
        .query_filtered::<(Entity, &Position), With<Object>>()
        .iter(world)
        .map(|(entity, pos)| (entity, *pos))
        .collect();

    let mut result = HashMap::new();
    for (entity, position) in entities {
        let entity_ref = world.entity(entity);
        let bundle = ObjectBundle {
            object: Object,
            position,
            occupant: clone_component!(entity_ref, Occupant),
            occupant_type_name: clone_component!(entity_ref, OccupantTypeName),
            terrain_movement_cost: clone_component!(entity_ref, TerrainMovementCost),
            contact_effects: clone_component!(entity_ref, ContactEffects),
        };
        let blocks_sight = entity_ref.get::<BlocksSight>().is_some();
        let blocks_sound = entity_ref.get::<BlocksSound>().is_some();
        result.insert(
            position,
            ObjectQueryResult {
                bundle,
                blocks_sight,
                blocks_sound,
            },
        );
    }
    Ok(result)
}

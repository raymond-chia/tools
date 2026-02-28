use super::get_component;
use crate::ecs_types::components::{
    AttributeBundle, Block, BlockProtection, BlocksSight, BlocksSound, CurrentHp, CurrentMp,
    Evasion, Faction, Fortitude, Hit, HpModify, Initiative, MagicalAttack, MagicalDc, MaxHp, MaxMp,
    Movement, Object, ObjectBundle, Occupant, OccupantTypeName, PhysicalAttack, Position, Reaction,
    Reflex, Skills, TerrainMovementCost, Unit, UnitBundle, Will,
};
use crate::ecs_types::resources::{Board, DeploymentConfig, LevelConfig};
use crate::error::{DataError, Result};
use bevy_ecs::prelude::{Entity, With, World};
use std::collections::HashMap;

/// 取得棋盤尺寸
pub fn get_board(world: &World) -> Result<Board> {
    world.get_resource::<Board>().copied().ok_or_else(|| {
        DataError::BoardConfigNotFound {
            config_name: "Board".to_string(),
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
            DataError::BoardConfigNotFound {
                config_name: "DeploymentConfig".to_string(),
            }
            .into()
        })
}

/// 取得關卡設定
pub fn get_level_config(world: &World) -> Result<LevelConfig> {
    world.get_resource::<LevelConfig>().cloned().ok_or_else(|| {
        DataError::BoardConfigNotFound {
            config_name: "LevelConfig".to_string(),
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
            occupant: get_component!(entity_ref, Occupant),
            occupant_type_name: get_component!(entity_ref, OccupantTypeName),
            faction: get_component!(entity_ref, Faction),
            skills: get_component!(entity_ref, Skills),
            attributes: AttributeBundle {
                max_hp: get_component!(entity_ref, MaxHp),
                current_hp: get_component!(entity_ref, CurrentHp),
                max_mp: get_component!(entity_ref, MaxMp),
                current_mp: get_component!(entity_ref, CurrentMp),
                initiative: get_component!(entity_ref, Initiative),
                hit: get_component!(entity_ref, Hit),
                evasion: get_component!(entity_ref, Evasion),
                block: get_component!(entity_ref, Block),
                block_protection: get_component!(entity_ref, BlockProtection),
                physical_attack: get_component!(entity_ref, PhysicalAttack),
                magical_attack: get_component!(entity_ref, MagicalAttack),
                magical_dc: get_component!(entity_ref, MagicalDc),
                fortitude: get_component!(entity_ref, Fortitude),
                reflex: get_component!(entity_ref, Reflex),
                will: get_component!(entity_ref, Will),
                movement: get_component!(entity_ref, Movement),
                reaction: get_component!(entity_ref, Reaction),
            },
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
            occupant: get_component!(entity_ref, Occupant),
            occupant_type_name: get_component!(entity_ref, OccupantTypeName),
            terrain_movement_cost: get_component!(entity_ref, TerrainMovementCost),
            hp_modify: get_component!(entity_ref, HpModify),
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

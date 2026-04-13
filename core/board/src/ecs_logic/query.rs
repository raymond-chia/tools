use crate::ecs_logic::get_component;
use crate::ecs_types::components::{
    ActionState, Agility, AttributeBundle, Block, BlockProtection, BlocksSight, BlocksSound,
    ContactEffects, CurrentHp, CurrentMp, Fortitude, Initiative, MagicalAccuracy, MagicalAttack,
    MaxHp, MaxMp, MovementPoint, Object, ObjectBundle, ObjectMovementCost, Occupant,
    OccupantTypeName, PhysicalAccuracy, PhysicalAttack, Position, ReactionPoint, Skills, Unit,
    UnitBundle, UnitFaction, Will,
};
use crate::ecs_types::resources::OccupantIndex;
use crate::error::{BoardError, DataError, Result};
use crate::logic::debug::short_type_name;
use bevy_ecs::change_detection::Mut;
use bevy_ecs::event::EntityEvent;
use bevy_ecs::lifecycle::{Add, Remove};
use bevy_ecs::prelude::{Entity, On, Query, ResMut, Resource, With, World};
use std::collections::HashMap;

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
            occupant: *get_component!(entity_ref, Occupant)?,
            occupant_type_name: get_component!(entity_ref, OccupantTypeName)?.clone(),
            unit_faction: *get_component!(entity_ref, UnitFaction)?,
            skills: get_component!(entity_ref, Skills)?.clone(),
            attributes: AttributeBundle {
                max_hp: get_component!(entity_ref, MaxHp)?.clone(),
                current_hp: get_component!(entity_ref, CurrentHp)?.clone(),
                max_mp: get_component!(entity_ref, MaxMp)?.clone(),
                current_mp: get_component!(entity_ref, CurrentMp)?.clone(),
                initiative: get_component!(entity_ref, Initiative)?.clone(),
                physical_attack: get_component!(entity_ref, PhysicalAttack)?.clone(),
                magical_attack: get_component!(entity_ref, MagicalAttack)?.clone(),
                physical_accuracy: get_component!(entity_ref, PhysicalAccuracy)?.clone(),
                magical_accuracy: get_component!(entity_ref, MagicalAccuracy)?.clone(),
                fortitude: get_component!(entity_ref, Fortitude)?.clone(),
                agility: get_component!(entity_ref, Agility)?.clone(),
                block: get_component!(entity_ref, Block)?.clone(),
                block_protection: get_component!(entity_ref, BlockProtection)?.clone(),
                will: get_component!(entity_ref, Will)?.clone(),
                movement_point: get_component!(entity_ref, MovementPoint)?.clone(),
                reaction_point: get_component!(entity_ref, ReactionPoint)?.clone(),
            },
            action_state: get_component!(entity_ref, ActionState)?.clone(),
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
            occupant: *get_component!(entity_ref, Occupant)?,
            occupant_type_name: get_component!(entity_ref, OccupantTypeName)?.clone(),
            terrain_movement_cost: get_component!(entity_ref, ObjectMovementCost)?.clone(),
            contact_effects: get_component!(entity_ref, ContactEffects)?.clone(),
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

// ============================================================================
// ECS 查詢工具函式
// ============================================================================

/// 初始化 OccupantIndex resource 與 observer（spawn/despawn 時自動同步）
pub(crate) fn setup_occupant_index(world: &mut World) {
    if world.contains_resource::<OccupantIndex>() {
        world.insert_resource(OccupantIndex::default());
        return;
    }
    world.insert_resource(OccupantIndex::default());
    world.add_observer(
        |trigger: On<Add, Occupant>, query: Query<&Occupant>, mut index: ResMut<OccupantIndex>| {
            let entity = trigger.event_target();
            if let Ok(occupant) = query.get(entity) {
                index.0.insert(*occupant, entity);
            }
        },
    );
    world.add_observer(
        |trigger: On<Remove, Occupant>,
         query: Query<&Occupant>,
         mut index: ResMut<OccupantIndex>| {
            let entity = trigger.event_target();
            if let Ok(occupant) = query.get(entity) {
                index.0.remove(occupant);
            }
        },
    );
}

/// 透過 OccupantIndex 查詢 Occupant 對應的 Entity
pub(crate) fn find_entity_by_occupant(world: &World, occupant: Occupant) -> Result<Entity> {
    let index = get_resource::<OccupantIndex>(world, "請先呼叫 setup_occupant_index")?;
    index
        .0
        .get(&occupant)
        .copied()
        .ok_or_else(|| BoardError::OccupantNotFound { occupant }.into())
}

// ============================================================================
// 特定查詢函式（封裝常用查詢邏輯，提供更友善的錯誤訊息）
// ============================================================================

/// 取得資源，若不存在則回傳適當的錯誤訊息
pub fn get_resource<'a, T: Resource>(world: &'a World, note: &str) -> Result<&'a T> {
    world.get_resource::<T>().ok_or_else(|| {
        DataError::MissingResource {
            name: short_type_name::<T>(),
            note: note.to_string(),
        }
        .into()
    })
}

/// 取得可變資源，若不存在則回傳適當的錯誤訊息
pub(crate) fn get_resource_mut<'a, T: Resource>(
    world: &'a mut World,
    note: &str,
) -> Result<Mut<'a, T>> {
    world.get_resource_mut::<T>().ok_or_else(|| {
        DataError::MissingResource {
            name: short_type_name::<T>(),
            note: note.to_string(),
        }
        .into()
    })
}

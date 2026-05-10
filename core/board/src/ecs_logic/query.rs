use crate::domain::alias::{ID, SkillName};
use crate::domain::constants::IMPASSABLE_MOVEMENT_COST;
use crate::domain::core_types::{EffectNode, SkillTag, SkillType, Target, TriggeringSource};
use crate::ecs_logic::get_component;
use crate::ecs_types::components::{
    ActionState, Agility, AttributeBundle, Block, BlockProtection, BlocksSight, BlocksSound,
    ContactEffects, CurrentHp, CurrentMp, FlankingAccuracyBonus, Fortitude, Initiative,
    MagicalAccuracy, MagicalAttack, MaxHp, MaxMp, MaxReactionPoint, MovementPoint, Object,
    ObjectBundle, ObjectMovementCost, Occupant, OccupantTypeName, PhysicalAccuracy, PhysicalAttack,
    Position, ReactionPoint, Skills, Unit, UnitBundle, UnitFaction, Will,
};
use crate::ecs_types::resources::{GameData, LevelConfig, OccupantIndex, SkillTargeting};
use crate::error::{BoardError, DataError, Result, UnitError};
use crate::logic::debug::short_type_name;
use crate::logic::skill::UnitInfo;
use crate::logic::skill::skill_execution::{CombatStats, ObjectOnBoard};
use bevy_ecs::change_detection::Mut;
use bevy_ecs::event::EntityEvent;
use bevy_ecs::lifecycle::{Add, Remove};
use bevy_ecs::prelude::{Entity, On, Query, ResMut, Resource, With, World};
use bevy_ecs::world::EntityRef;
use std::collections::HashMap;
use std::sync::Arc;

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
            attributes: read_attribute_bundle(&entity_ref)?,
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

/// 建立 faction_id → alliance_id 的對應表
pub(crate) fn build_faction_alliance_map(world: &World) -> Result<HashMap<ID, ID>> {
    let level_config = get_resource::<LevelConfig>(world, "請先呼叫 spawn_level")?;
    Ok(level_config
        .factions
        .iter()
        .map(|(id, f)| (*id, f.alliance))
        .collect())
}

/// 從 faction_alliance_map 查詢 alliance_id，找不到時回傳豐富錯誤訊息
pub(crate) fn resolve_alliance(map: &HashMap<ID, ID>, faction_id: ID) -> Result<ID> {
    map.get(&faction_id).copied().ok_or_else(|| {
        DataError::InvalidComponent {
            name: short_type_name::<UnitFaction>(),
            note: format!(
                "faction_id {} 在 faction_to_alliance 中找不到對應",
                faction_id
            ),
        }
        .into()
    })
}

/// 取得指定技能名稱對應的 Active 技能欄位；若非 Active 則視為 SkillNotFound
pub(crate) fn get_active_skill_data(
    game_data: &GameData,
    skill_name: &SkillName,
) -> Result<(Target, Arc<[EffectNode]>, u32, Vec<SkillTag>)> {
    let skill_type =
        game_data
            .skill_map
            .get(skill_name)
            .ok_or_else(|| UnitError::SkillNotFound {
                skill_name: skill_name.clone(),
            })?;
    match skill_type {
        SkillType::Active {
            target,
            effects,
            cost,
            tags,
            name: _,
        } => Ok((target.clone(), effects.clone(), *cost, tags.clone())),
        SkillType::Reaction { .. } | SkillType::Passive { .. } => Err(UnitError::SkillNotFound {
            skill_name: skill_name.clone(),
        }
        .into()),
    }
}

/// 取得指定技能名稱對應的 Reaction 技能欄位；若非 Reaction 則視為 SkillNotFound
pub(crate) fn get_reaction_skill_data(
    game_data: &GameData,
    skill_name: &SkillName,
) -> Result<(TriggeringSource, Arc<[EffectNode]>, u32, Vec<SkillTag>)> {
    let skill_type =
        game_data
            .skill_map
            .get(skill_name)
            .ok_or_else(|| UnitError::SkillNotFound {
                skill_name: skill_name.clone(),
            })?;
    match skill_type {
        SkillType::Reaction {
            triggering_unit,
            effects,
            cost,
            tags,
            name: _,
        } => Ok((
            triggering_unit.clone(),
            effects.clone(),
            *cost,
            tags.clone(),
        )),
        SkillType::Active { .. } | SkillType::Passive { .. } => Err(UnitError::SkillNotFound {
            skill_name: skill_name.clone(),
        }
        .into()),
    }
}

/// 從 EntityRef 逐一讀取屬性 component，組裝成 AttributeBundle
pub(crate) fn read_attribute_bundle(entity_ref: &EntityRef) -> Result<AttributeBundle> {
    Ok(AttributeBundle {
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
        max_reaction_point: get_component!(entity_ref, MaxReactionPoint)?.clone(),
        reaction_point: get_component!(entity_ref, ReactionPoint)?.clone(),
        flanking_accuracy_bonus: get_component!(entity_ref, FlankingAccuracyBonus)?.clone(),
    })
}

/// 查詢當前技能選目標狀態供 UI 渲染與確認施放
pub fn get_skill_targeting(world: &World) -> Result<&SkillTargeting> {
    get_resource::<SkillTargeting>(world, "請先呼叫 start_skill_targeting")
}

/// 建構棋盤上所有物件的位置對應表
pub(crate) fn build_objects_on_board(world: &mut World) -> HashMap<Position, ObjectOnBoard> {
    world
        .query_filtered::<(&Position, &Occupant, &ObjectMovementCost), With<Object>>()
        .iter(world)
        .map(|(pos, occ, mc)| {
            (
                *pos,
                ObjectOnBoard {
                    occupant: *occ,
                    occupies_tile: mc.0 >= IMPASSABLE_MOVEMENT_COST,
                },
            )
        })
        .collect()
}

/// 建構棋盤上所有單位的戰鬥屬性位置對應表
pub(crate) fn build_unit_stats_on_board(
    world: &mut World,
    faction_to_alliance: &HashMap<ID, ID>,
) -> Result<HashMap<Position, CombatStats>> {
    let unit_entities: Vec<Entity> = world
        .query_filtered::<Entity, With<Unit>>()
        .iter(world)
        .collect();
    let mut result = HashMap::new();
    for unit_entity in unit_entities {
        let entity_ref = world.entity(unit_entity);
        let pos = *get_component!(entity_ref, Position)?;
        let occupant = *get_component!(entity_ref, Occupant)?;
        let faction_id = get_component!(entity_ref, UnitFaction)?.0;
        let attributes = read_attribute_bundle(&entity_ref)?;
        let alliance_id = resolve_alliance(faction_to_alliance, faction_id)?;
        result.insert(
            pos,
            CombatStats {
                unit_info: UnitInfo {
                    occupant,
                    faction_id,
                    alliance_id,
                },
                attribute: attributes,
            },
        );
    }
    Ok(result)
}

use crate::domain::alias::{ID, TypeName};
use crate::domain::constants::PLAYER_FACTION_ID;
use crate::ecs_types::components::{
    AttributeBundle, Block, BlockProtection, CurrentHp, CurrentMp, Evasion, Faction, Fortitude,
    Hit, Initiative, MagicalAttack, MagicalDc, MaxHp, MaxMp, Movement, Occupant, OccupantTypeName,
    PhysicalAttack, Position, Reaction, Reflex, Skills, Unit, UnitBundle, Will,
};
use crate::ecs_types::resources::{DeploymentConfig, GameData};
use crate::error::{DataError, DeploymentError, Result};
use crate::loader_schema::BuffEffect;
use crate::logic::debug::short_type_name;
use crate::logic::id_generator::generate_unique_id;
use crate::logic::unit_attributes;
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::World;
use std::collections::HashSet;

/// 將玩家單位部署到指定位置
///
/// - 位置必須在 DeploymentConfig 的合法部署區域內
/// - 若同一格已有玩家單位，先移除再部署（替換）
/// - 若無替換，已部署數量不得超過 max_player_units
/// - 陣營固定為 PLAYER_FACTION_ID
pub fn deploy_unit(world: &mut World, unit_type_name: &TypeName, position: Position) -> Result<()> {
    // 第一階段：借用 resources 並收集所有需要的資料
    let (entity_to_remove, current_player_unit_count, deployment_config, game_data, mut used_ids) = {
        let used_ids: HashSet<ID> = world
            .query::<&Occupant>()
            .iter(world)
            .map(|occupant| match occupant {
                Occupant::Unit(id) => *id,
                Occupant::Object(id) => *id,
            })
            .collect();

        // 找出同格的玩家單位（準備替換）
        let entity_to_remove = world
            .query::<(bevy_ecs::entity::Entity, &Position, &Faction, &Unit)>()
            .iter(world)
            .find(|(_, pos, faction, _)| **pos == position && faction.0 == PLAYER_FACTION_ID)
            .map(|(entity, _, _, _)| entity);

        let deployment_config = world
            .get_resource::<DeploymentConfig>()
            .ok_or(DataError::BoardConfigNotFound {
                config_name: short_type_name::<DeploymentConfig>(),
            })?
            .clone();

        // resource 借用已結束，可再次查詢 world
        // 計算站在部署點上的玩家單位數（即已部署的單位，不含關卡預設單位）
        let current_player_unit_count = world
            .query::<(&Unit, &Position)>()
            .iter(world)
            .filter(|(_, pos)| deployment_config.deployment_positions.contains(pos))
            .count();

        let game_data = world
            .get_resource::<GameData>()
            .ok_or(DataError::GameDataNotFound)?;
        (
            entity_to_remove,
            current_player_unit_count,
            deployment_config,
            game_data,
            used_ids,
        )
    };

    // Fail fast：驗證位置在合法部署區域內
    if !deployment_config.deployment_positions.contains(&position) {
        return Err(DeploymentError::PositionNotDeployable {
            x: position.x,
            y: position.y,
        }
        .into());
    }

    let new_id = generate_unique_id(&mut used_ids);
    let unit_type =
        game_data
            .unit_type_map
            .get(unit_type_name)
            .ok_or_else(|| DataError::UnitTypeNotFound {
                type_name: unit_type_name.clone(),
            })?;
    let no_buffs: &[BuffEffect] = &[];
    let attributes =
        unit_attributes::calculate_attributes(&unit_type.skills, no_buffs, &game_data.skill_map)?;
    let bundle = UnitBundle {
        unit: Unit,
        position,
        occupant: Occupant::Unit(new_id),
        occupant_type_name: OccupantTypeName(unit_type.name.clone()),
        faction: Faction(PLAYER_FACTION_ID),
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
    };

    // 第二階段：resource 借用已結束，可以可變借用 world

    match entity_to_remove {
        Some(entity) => {
            // 同格有玩家單位：替換（先移除）
            world.despawn(entity);
        }
        None => {
            // 無替換：檢查是否已達上限
            if current_player_unit_count >= deployment_config.max_player_units {
                return Err(DeploymentError::MaxPlayerUnitsReached {
                    max: deployment_config.max_player_units,
                }
                .into());
            }
        }
    }

    world.spawn(bundle);

    Ok(())
}

/// 取消指定部署點上的玩家單位部署
///
/// - 位置必須在 DeploymentConfig 的合法部署區域內
/// - 若位置上沒有玩家單位，回傳 NothingToUndeploy 錯誤
pub fn undeploy_unit(world: &mut World, position: Position) -> Result<()> {
    // 第一階段：讀取所有需要的資料
    let (deployment_positions, entity_to_remove) = {
        let deployment_config = world
            .get_resource::<DeploymentConfig>()
            .ok_or(DataError::BoardConfigNotFound {
                config_name: short_type_name::<DeploymentConfig>(),
            })?
            .clone();

        let entity_to_remove: Option<Entity> = world
            .query::<(Entity, &Position, &Unit)>()
            .iter(world)
            .find(|(_, pos, _)| **pos == position)
            .map(|(entity, _, _)| entity);

        (deployment_config.deployment_positions, entity_to_remove)
    };

    // Fail fast 驗證
    if !deployment_positions.contains(&position) {
        return Err(DeploymentError::PositionNotDeployable {
            x: position.x,
            y: position.y,
        }
        .into());
    }

    // 第二階段：寫入 World
    match entity_to_remove {
        Some(entity) => {
            world.despawn(entity);
            Ok(())
        }
        None => Err(DeploymentError::NothingToUndeploy {
            x: position.x,
            y: position.y,
        }
        .into()),
    }
}

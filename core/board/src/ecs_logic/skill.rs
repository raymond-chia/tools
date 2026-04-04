//! 技能系統 ECS 操作函數

use super::get_component;
use crate::domain::alias::{ID, SkillName};
use crate::domain::core_types::{SkillType, TargetSelection};
use crate::ecs_logic::query::{find_entity_by_occupant, get_resource};
use crate::ecs_types::components::{
    ActionState, CurrentMp, MovementPoint, Occupant, Position, Skills, Unit, UnitFaction,
};
use crate::ecs_types::resources::{Board, GameData, LevelConfig, TurnOrder};
use crate::error::{BoardError, DataError, Result, UnitError};
use crate::logic::debug::short_type_name;
use crate::logic::skill::{self as skill_logic, UnitInfo};
use crate::logic::turn_order::get_active_unit;
use bevy_ecs::prelude::{With, World};
use std::collections::HashMap;

/// 可用技能資訊
pub struct AvailableSkill {
    pub name: SkillName,
    pub usable: bool,
}

/// 取得當前行動單位的所有主動技能及其可用狀態
pub fn get_available_skills(world: &mut World) -> Result<Vec<AvailableSkill>> {
    // 讀取：TurnOrder → active unit
    let turn_order = get_resource::<TurnOrder>(world, "請先呼叫 start_new_round")?;
    let active_occupant = get_active_unit(&turn_order.entries).ok_or(BoardError::NoActiveUnit)?;

    // 讀取：當前單位的 Skills、CurrentMp、ActionState、MovementPoint
    let entity = find_entity_by_occupant(world, active_occupant)?;
    let entity_ref = world.entity(entity);
    let skills = get_component!(entity_ref, Skills)?;
    let current_mp = get_component!(entity_ref, CurrentMp)?.0;
    let action_state = get_component!(entity_ref, ActionState)?;
    let movement_point = get_component!(entity_ref, MovementPoint)?.0;

    // 讀取：GameData
    let game_data = get_resource::<GameData>(world, "請先呼叫 parse_and_insert_game_data")?;

    // 純邏輯：篩選 Active 技能，判定 usable
    let can_act = match action_state {
        ActionState::Done => false,
        ActionState::Moved { cost } => (*cost as i32) <= movement_point,
    };

    let mut result = Vec::new();
    for skill_name in &skills.0 {
        let skill_type =
            game_data
                .skill_map
                .get(skill_name)
                .ok_or_else(|| UnitError::SkillNotFound {
                    skill_name: skill_name.clone(),
                })?;
        match skill_type {
            SkillType::Active { name, cost, .. } => {
                let usable = can_act && current_mp >= *cost as i32;
                result.push(AvailableSkill {
                    name: name.clone(),
                    usable,
                });
            }
            SkillType::Reaction { .. } | SkillType::Passive { .. } => {}
        }
    }

    Ok(result)
}

/// 查詢指定技能的射程內所有可選格子
///
/// 根據技能的 Target.range 與施放者位置，回傳曼哈頓距離在射程內的所有格子
pub fn get_skill_targetable_positions(
    world: &mut World,
    skill_name: &SkillName,
) -> Result<Vec<Position>> {
    // 讀取：TurnOrder → active unit
    let turn_order = get_resource::<TurnOrder>(world, "請先呼叫 start_new_round")?;
    let active_occupant = get_active_unit(&turn_order.entries).ok_or(BoardError::NoActiveUnit)?;

    // 讀取：當前單位的位置
    let caster_pos = {
        let entity = find_entity_by_occupant(world, active_occupant)?;
        *get_component!(world.entity(entity), Position)?
    };

    // 讀取：GameData
    let game_data = get_resource::<GameData>(world, "請先呼叫 parse_and_insert_game_data")?;

    // 讀取：Board
    let board = get_resource::<Board>(world, "請先呼叫 spawn_level")?;

    // 純邏輯：取得技能的 range，計算射程內格子
    let skill_type =
        game_data
            .skill_map
            .get(skill_name)
            .ok_or_else(|| UnitError::SkillNotFound {
                skill_name: skill_name.clone(),
            })?;

    match skill_type {
        SkillType::Active { target, .. } => Ok(skill_logic::compute_range_positions(
            caster_pos,
            target.range,
            *board,
        )),
        SkillType::Reaction { .. } | SkillType::Passive { .. } => Err(UnitError::SkillNotFound {
            skill_name: skill_name.clone(),
        }
        .into()),
    }
}

/// 預覽技能 AOE 影響範圍結果
pub struct PreviewAffectedPositions {
    /// AOE 範圍內所有格子（不管 filter）
    pub all_positions: Vec<Position>,
    /// 過濾後的格子
    pub filtered_positions: Vec<Position>,
}

/// 預覽技能 AOE 影響範圍
///
/// 根據技能的 Area、施放者位置與目標位置，計算影響的所有格子。
/// 同時驗證目標位置在技能射程內。
/// 回傳未過濾和過濾後兩組結果。
pub fn get_skill_affected_positions(
    world: &mut World,
    skill_name: &SkillName,
    target_pos: Position,
) -> Result<PreviewAffectedPositions> {
    // 讀取
    let board = *get_resource::<Board>(world, "請先呼叫 spawn_level")?;

    let target = {
        let game_data = get_resource::<GameData>(world, "請先呼叫 parse_and_insert_game_data")?;
        let skill_type =
            game_data
                .skill_map
                .get(skill_name)
                .ok_or_else(|| UnitError::SkillNotFound {
                    skill_name: skill_name.clone(),
                })?;
        match skill_type {
            SkillType::Active { target, .. } => target.clone(),
            SkillType::Reaction { .. } | SkillType::Passive { .. } => {
                return Err(UnitError::SkillNotFound {
                    skill_name: skill_name.clone(),
                }
                .into());
            }
        }
    };

    let faction_to_alliance: HashMap<ID, ID> = {
        let level_config = get_resource::<LevelConfig>(world, "請先呼叫 spawn_level")?;
        level_config
            .factions
            .iter()
            .map(|(id, f)| (*id, f.alliance))
            .collect()
    };

    // 建立 caster unit info
    let turn_order = get_resource::<TurnOrder>(world, "請先呼叫 start_new_round")?;
    let active_occupant = get_active_unit(&turn_order.entries).ok_or(BoardError::NoActiveUnit)?;
    let (caster_pos, caster_occupant, caster_faction) = {
        let entity = find_entity_by_occupant(world, active_occupant)?;
        let entity_ref = world.entity(entity);
        let pos = *get_component!(entity_ref, Position)?;
        let occupant = *get_component!(entity_ref, Occupant)?;
        let faction = get_component!(entity_ref, UnitFaction)?.0;
        (pos, occupant, faction)
    };
    let caster_alliance = faction_to_alliance
        .get(&caster_faction)
        .copied()
        .ok_or_else(|| DataError::InvalidComponent {
            name: short_type_name::<UnitFaction>(),
            note: format!(
                "faction_id {} 在 faction_to_alliance 中找不到對應",
                caster_faction
            ),
        })?;
    let caster_info = UnitInfo {
        occupant: caster_occupant,
        faction_id: caster_faction,
        alliance_id: caster_alliance,
    };

    // 建立場上單位 position → UnitInfo
    let units_on_board = {
        let mut units_on_board: HashMap<Position, UnitInfo> = HashMap::new();
        for (pos, occupant, faction_id) in world
            .query_filtered::<(&Position, &Occupant, &UnitFaction), With<Unit>>()
            .iter(world)
            .map(|(pos, occ, fac)| (*pos, *occ, fac.0))
        {
            let alliance_id = faction_to_alliance
                .get(&faction_id)
                .copied()
                .ok_or_else(|| DataError::InvalidComponent {
                    name: short_type_name::<UnitFaction>(),
                    note: format!(
                        "faction_id {} 在 faction_to_alliance 中找不到對應",
                        faction_id
                    ),
                })?;
            units_on_board.insert(
                pos,
                UnitInfo {
                    occupant,
                    faction_id,
                    alliance_id,
                },
            );
        }
        units_on_board
    };

    // ============================================================================

    // 計算 AOE 範圍
    let (min_range, max_range) = target.range;
    let distance = skill_logic::manhattan_distance(caster_pos, target_pos);
    if distance < min_range || distance > max_range {
        return Err(BoardError::OutOfRange {
            distance,
            min_range,
            max_range,
        }
        .into());
    }
    let all_positions =
        skill_logic::compute_affected_positions(&target.area, caster_pos, target_pos, board)?;

    let can_target_ground = matches!(target.selection, TargetSelection::Ground);
    let filter = &target.selectable_filter;
    let filtered_positions = all_positions
        .iter()
        .filter(|pos| match units_on_board.get(pos) {
            None => can_target_ground,
            Some(unit) => skill_logic::is_in_filter(&caster_info, unit, filter),
        })
        .copied()
        .collect();

    Ok(PreviewAffectedPositions {
        all_positions,
        filtered_positions,
    })
}

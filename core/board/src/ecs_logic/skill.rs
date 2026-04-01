//! 技能系統 ECS 操作函數

use crate::domain::alias::SkillName;
use crate::domain::core_types::SkillType;
use crate::ecs_types::components::{ActionState, CurrentMp, MovementPoint, Occupant, Skills, Unit};
use crate::ecs_types::resources::{GameData, TurnOrder};
use crate::error::{BoardError, DataError, Result, UnitError};
use crate::logic::debug::short_type_name;
use crate::logic::turn_order::get_active_unit;
use bevy_ecs::prelude::{With, World};

/// 可用技能資訊
pub struct AvailableSkill {
    pub name: SkillName,
    pub usable: bool,
}

/// 取得當前行動單位的所有主動技能及其可用狀態
pub fn get_available_skills(world: &mut World) -> Result<Vec<AvailableSkill>> {
    // 讀取：TurnOrder → active unit
    let turn_order =
        world
            .get_resource::<TurnOrder>()
            .ok_or_else(|| DataError::MissingResource {
                name: short_type_name::<TurnOrder>(),
                note: "請先呼叫 start_new_round".to_string(),
            })?;
    let active_occupant = get_active_unit(&turn_order.entries).ok_or(BoardError::NoActiveUnit)?;

    // 讀取：當前單位的 Skills、CurrentMp、ActionState、MovementPoint
    let (skill_names, current_mp, action_state, movement_point) = {
        let mut query = world.query_filtered::<(
            &Occupant,
            &Skills,
            &CurrentMp,
            &ActionState,
            &MovementPoint,
        ), With<Unit>>();
        let (_, skills, mp, state, mv) = query
            .iter(world)
            .find(|(occ, _, _, _, _)| **occ == active_occupant)
            .ok_or_else(|| DataError::MissingComponent {
                name: format!(
                    "Skills/CurrentMp/ActionState/MovementPoint for {:?}",
                    active_occupant
                ),
            })?;
        (skills.0.clone(), mp.0, state.clone(), mv.0)
    };

    // 讀取：GameData
    let game_data = world
        .get_resource::<GameData>()
        .ok_or_else(|| DataError::MissingResource {
            name: short_type_name::<GameData>(),
            note: "請先呼叫 parse_and_insert_game_data".to_string(),
        })?;

    // 純邏輯：篩選 Active 技能，判定 usable
    let can_act = match &action_state {
        ActionState::Done => false,
        ActionState::Moved { cost } => (*cost as i32) <= movement_point,
    };

    let mut result = Vec::new();
    for skill_name in &skill_names {
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

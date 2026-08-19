use super::check_action_point;
use crate::domain::alias::SkillName;
use crate::domain::core_types::SkillType;
use crate::ecs_logic::get_component;
use crate::ecs_logic::query::{find_entity_by_occupant, get_resource};
use crate::ecs_logic::turn::get_current_unit;
use crate::ecs_types::components::{ActionState, CurrentMp, MovementPoint, Skills};
use crate::ecs_types::resources::{GameData, TurnOrder};
use crate::error::{Result, UnitError};
use bevy_ecs::prelude::World;

/// 可用技能資訊。
pub struct AvailableSkill {
    pub name: SkillName,
    pub cost: u32,
    pub usable: bool,
}

/// 查詢當前單位是否可使用技能（行動點足夠才可使用）。
pub fn can_use_skill_current_unit(world: &mut World) -> Result<bool> {
    let turn_order = get_resource::<TurnOrder>(world, "請先呼叫 start_new_round")?;
    let active_occupant = get_current_unit(turn_order)?;

    let entity = find_entity_by_occupant(world, active_occupant)?;
    let entity_ref = world.entity(entity);
    let action_state = get_component!(entity_ref, ActionState)?;
    let movement_point = get_component!(entity_ref, MovementPoint)?.0;

    Ok(check_action_point(action_state, movement_point).is_ok())
}

/// 取得當前行動單位的所有主動技能及其可用狀態。
pub fn get_available_skills(world: &mut World) -> Result<Vec<AvailableSkill>> {
    // 讀取：TurnOrder → active unit
    let turn_order = get_resource::<TurnOrder>(world, "請先呼叫 start_new_round")?;
    let active_occupant = get_current_unit(turn_order)?;

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
                    cost: *cost,
                    usable,
                });
            }
            SkillType::Reaction { .. } | SkillType::Passive { .. } => {}
        }
    }

    Ok(result)
}

//! ECS 移動操作函數

use super::{get_component, get_component_mut};
use crate::domain::alias::{ID, MovementCost, SkillName};
use crate::domain::constants::BASIC_MOVEMENT_COST;
use crate::domain::core_types::{PendingReaction, ReactionTrigger, SkillType};
use crate::ecs_logic::query::{
    build_faction_alliance_map, find_entity_by_occupant, get_resource, get_resource_mut,
    resolve_alliance,
};
use crate::ecs_logic::turn::get_current_unit;
use crate::ecs_types::components::{
    ActionState, MovementPoint, Object, ObjectMovementCost, Occupant, Position, ReactionPoint,
    Skills, Unit, UnitFaction,
};
use crate::ecs_types::resources::{Board, GameData, MovementPlan, ReactionState, TurnOrder};
use crate::error::{BoardError, DataError, Result};
use crate::logic::movement::{Mover, ReachableInfo, reachable_positions, reconstruct_path};
use crate::logic::skill::UnitInfo;
use crate::logic::skill::skill_reaction::{ReactionUnitInfo, collect_move_reactions};
use bevy_ecs::prelude::{With, World};
use std::collections::HashMap;

/// advance_move 的回傳值
#[derive(Debug, Clone)]
pub enum AdvanceMoveResult {
    /// 走完整段路徑，移動結束
    Completed {
        path_walked: Vec<Position>,
        cost: MovementCost,
    },
    /// 走到某格觸發反應，停在該格等待反應處理
    Interrupted {
        path_walked: Vec<Position>,
        cost: MovementCost,
    },
}

/// 計算單位可到達的所有位置
///
/// 返回 position -> ReachableInfo 的對應表
pub fn get_reachable_positions(
    world: &mut World,
    occupant: Occupant,
) -> Result<HashMap<Position, ReachableInfo>> {
    let entity = find_entity_by_occupant(world, occupant)?;
    let entity_ref = world.entity(entity);
    let unit_pos = *get_component!(entity_ref, Position)?;
    let faction = get_component!(entity_ref, UnitFaction)?.0;
    let movement_point = get_component!(entity_ref, MovementPoint)?.0 as MovementCost;
    let movement_used = match get_component!(entity_ref, ActionState)? {
        ActionState::Moved { cost } => *cost,
        ActionState::Done => movement_point * 2,
    };

    let board = *get_resource::<Board>(world, "請先呼叫 spawn_level")?;
    let units_faction = get_units_faction_map(world)?;
    let objects_movement_cost = get_objects_movement_cost_map(world)?;
    let faction_to_alliance = build_faction_alliance_map(world)?;

    // 計算可用預算（2 倍移動力 - 已使用的）
    let budget = movement_point * 2 - movement_used;

    let get_occupant_alliance = |pos: Position| -> Option<ID> {
        units_faction.get(&pos).and_then(|faction_id| {
            faction_to_alliance
                .get(faction_id)
                .copied()
                .or_else(|| unreachable!("faction_id {} 不存在於 faction_to_alliance", faction_id))
        })
    };

    let get_terrain_cost = |pos: Position| -> MovementCost {
        BASIC_MOVEMENT_COST + objects_movement_cost.get(&pos).copied().unwrap_or(0)
    };

    let mover_alliance = resolve_alliance(&faction_to_alliance, faction)?;

    let mover = Mover {
        pos: unit_pos,
        faction_alliance: mover_alliance,
    };

    reachable_positions(
        board,
        mover,
        budget,
        get_occupant_alliance,
        get_terrain_cost,
    )
}

/// 規劃移動路徑並存入 MovementPlan resource
///
/// 驗證目標位置可到達後，將完整路徑存入 resource。
/// 後續呼叫 advance_move 或 force_advance_move 才真正移動單位。
pub fn plan_move(world: &mut World, target: Position) -> Result<()> {
    // 從 TurnOrder 取得當前行動單位
    let turn_order = get_resource::<TurnOrder>(world, "請先呼叫 start_new_round")?;
    let occupant = get_current_unit(turn_order)?;

    let entity = find_entity_by_occupant(world, occupant)?;
    let start_pos = *get_component!(world.entity(entity), Position)?;

    let reachable = get_reachable_positions(world, occupant)?;

    // 驗證目標位置是否可到達且不是僅穿越位置
    let reach_info = reachable
        .get(&target)
        .ok_or_else(|| BoardError::Unreachable {
            x: target.x,
            y: target.y,
        })?;

    // 目標位置不能是友軍佔據的位置（僅穿越）
    if reach_info.passthrough_only {
        return Err(BoardError::Unreachable {
            x: target.x,
            y: target.y,
        }
        .into());
    }

    // path 含起點：[start, step1, ..., target]
    let path = reconstruct_path(&reachable, start_pos, target);

    // step_costs[0] = 0（起點），step_costs[i] = 走到 path[i] 的移動消耗
    let mut step_costs = Vec::with_capacity(path.len());
    step_costs.push(0);
    for i in 1..path.len() {
        let prev_cumulative = if i == 1 {
            0
        } else {
            reachable
                .get(&path[i - 1])
                .expect("path 中的位置必定在 reachable 內")
                .cost
        };
        let curr_cumulative = reachable
            .get(&path[i])
            .expect("path 中的位置必定在 reachable 內")
            .cost;
        step_costs.push(curr_cumulative - prev_cumulative);
    }

    world.insert_resource(MovementPlan {
        path,
        step_costs,
        next_step_index: 0,
    });

    Ok(())
}

/// 沿著 MovementPlan 的路徑移動，每格檢查反應
///
/// 遇到反應時停在觸發格，寫入 ReactionState 並回傳 Interrupted。
/// 反應處理完後再次呼叫此函數繼續走剩餘路徑。
/// 走完整段路徑後移除 MovementPlan 並回傳 Completed。
pub fn advance_move(world: &mut World) -> Result<AdvanceMoveResult> {
    // === 讀取階段 ===
    let turn_order = get_resource::<TurnOrder>(world, "請先呼叫 start_new_round")?;
    let occupant = get_current_unit(turn_order)?;

    let entity = find_entity_by_occupant(world, occupant)?;
    let mover_faction = get_component!(world.entity(entity), UnitFaction)?.0;

    let MovementPlan {
        path,
        step_costs,
        next_step_index,
    } = get_resource::<MovementPlan>(world, "請先呼叫 plan_move")?.clone();

    let faction_to_alliance = build_faction_alliance_map(world)?;
    let mover_alliance = resolve_alliance(&faction_to_alliance, mover_faction)?;
    let mover_info = UnitInfo {
        occupant,
        faction_id: mover_faction,
        alliance_id: mover_alliance,
    };

    let reaction_unit_map = build_reaction_unit_map(world, &faction_to_alliance)?;

    // === 純邏輯 ===
    // path[next_step_index..] 從當前位置開始，含當前格作為 from
    let reaction_result =
        collect_move_reactions(&mover_info, &path[next_step_index..], &reaction_unit_map)?;
    let stop_pos = reaction_result.stop_position;
    let has_reactions = !reaction_result.reactions.is_empty();

    // steps_to_stop：在 path[next_step_index..] 中的 index，即走了幾步
    let steps_to_stop = path[next_step_index..]
        .iter()
        .position(|p| *p == stop_pos)
        .ok_or_else(|| DataError::InternalError {
            message: format!(
                "collect_move_reactions 回傳的 stop_position {:?} 不在 path[{}..] 中",
                stop_pos, next_step_index
            ),
        })?;

    let stop_index = next_step_index + steps_to_stop;
    let walked_path: Vec<Position> = path[next_step_index..=stop_index].to_vec();
    let walked_cost: MovementCost = step_costs[next_step_index + 1..=stop_index].iter().sum();
    let reached_end = stop_index == path.len() - 1;

    // === 寫入階段 ===
    let entity = find_entity_by_occupant(world, occupant)?;
    let mut entity_mut = world.entity_mut(entity);
    {
        let mut pos = get_component_mut!(entity_mut, Position)?;
        *pos = stop_pos;
    }
    {
        let mut action_state = get_component_mut!(entity_mut, ActionState)?;
        match action_state.as_ref() {
            ActionState::Moved { cost } => {
                *action_state = ActionState::Moved {
                    cost: cost + walked_cost,
                };
            }
            ActionState::Done => {
                unreachable!("ActionState::Done 時不應有 reachable");
            }
        }
    }

    if reached_end {
        world.remove_resource::<MovementPlan>();
    } else {
        let mut plan_mut = get_resource_mut::<MovementPlan>(world, "請先呼叫 plan_move")?;
        plan_mut.next_step_index = stop_index;
    }

    if has_reactions {
        let pending: Vec<PendingReaction> = reaction_result
            .reactions
            .into_iter()
            .map(|r| PendingReaction {
                reactor: r.occupant,
                trigger: occupant,
                trigger_event: ReactionTrigger::AttackOfOpportunity,
                available_skills: r.skill_names,
            })
            .collect();
        world.insert_resource(ReactionState {
            pending,
            decided: vec![],
        });
        return Ok(AdvanceMoveResult::Interrupted {
            path_walked: walked_path,
            cost: walked_cost,
        });
    }

    Ok(AdvanceMoveResult::Completed {
        path_walked: walked_path,
        cost: walked_cost,
    })
}

/// 強制沿著 MovementPlan 走下一格，不檢查反應
///
/// 用於反應觸發後強制離開觸發格，避免重複觸發。
/// 走完整段路徑時移除 MovementPlan 並回傳 Completed。
pub fn force_advance_move(world: &mut World) -> Result<AdvanceMoveResult> {
    let turn_order = get_resource::<TurnOrder>(world, "請先呼叫 start_new_round")?;
    let occupant = get_current_unit(turn_order)?;

    let MovementPlan {
        path,
        step_costs,
        next_step_index,
    } = get_resource::<MovementPlan>(world, "請先呼叫 plan_move")?.clone();

    let new_next_step_index = next_step_index + 1;
    let next_pos = path[new_next_step_index];
    let this_step_cost = step_costs[new_next_step_index];

    let entity = find_entity_by_occupant(world, occupant)?;
    let mut entity_mut = world.entity_mut(entity);
    {
        let mut pos = get_component_mut!(entity_mut, Position)?;
        *pos = next_pos;
    }
    {
        let mut action_state = get_component_mut!(entity_mut, ActionState)?;
        match action_state.as_ref() {
            ActionState::Moved { cost } => {
                *action_state = ActionState::Moved {
                    cost: cost + this_step_cost,
                };
            }
            ActionState::Done => {
                unreachable!("ActionState::Done 時不應有 reachable");
            }
        }
    }

    world.remove_resource::<MovementPlan>();
    return Ok(AdvanceMoveResult::Completed {
        path_walked: vec![next_pos],
        cost: this_step_cost,
    });
}

// ============================================================================
// 私有輔助函數
// ============================================================================

fn build_reaction_unit_map<'a>(
    world: &'a mut World,
    faction_to_alliance: &'a HashMap<ID, ID>,
) -> Result<HashMap<Position, ReactionUnitInfo<'a>>> {
    let mut query = world.query_filtered::<(
        &Position,
        &Occupant,
        &UnitFaction,
        &ReactionPoint,
        &Skills,
    ), With<Unit>>();

    // 先快照原始資料釋放 world borrow，再借用 game_data 查詢 SkillType
    let snapshots: Vec<(Position, Occupant, ID, i32, Vec<SkillName>)> = query
        .iter(world)
        .filter(|(_, _, _, reaction_point, _)| reaction_point.0 > 0)
        .map(|(pos, occupant, faction, reaction_point, skills)| {
            (
                *pos,
                *occupant,
                faction.0,
                reaction_point.0,
                skills.0.clone(),
            )
        })
        .collect();

    let game_data = get_resource::<GameData>(world, "請先呼叫 parse_and_insert_game_data")?;

    let mut result = HashMap::new();
    for (pos, occupant, faction_id, remaining_reactions, skill_names) in snapshots {
        let alliance_id = resolve_alliance(faction_to_alliance, faction_id)?;

        let skills: Vec<&'a SkillType> = skill_names
            .iter()
            .filter_map(|name| game_data.skill_map.get(name))
            .filter(|s| matches!(s, SkillType::Reaction { .. }))
            .collect();

        result.insert(
            pos,
            ReactionUnitInfo {
                unit_info: UnitInfo {
                    occupant,
                    faction_id,
                    alliance_id,
                },
                remaining_reactions,
                skills,
            },
        );
    }

    Ok(result)
}

/// 取得單位 faction 對應表
fn get_units_faction_map(world: &mut World) -> Result<HashMap<Position, ID>> {
    let mut result = HashMap::new();
    let mut query = world.query_filtered::<(&Position, &UnitFaction), With<Unit>>();

    for (position, unit_faction) in query.iter(world) {
        result.insert(*position, unit_faction.0);
    }
    Ok(result)
}

/// 取得物件地形消耗對應表
fn get_objects_movement_cost_map(world: &mut World) -> Result<HashMap<Position, MovementCost>> {
    let mut result = HashMap::new();
    let mut query = world.query_filtered::<(&Position, &ObjectMovementCost), With<Object>>();

    for (position, movement_cost) in query.iter(world) {
        result.insert(*position, movement_cost.0);
    }
    Ok(result)
}

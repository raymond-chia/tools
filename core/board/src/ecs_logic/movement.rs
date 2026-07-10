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
    ActionState, BlocksSight, Hazardous, MovementPoint, Object, ObjectMovementCost, Occupant,
    Position, ReactionPoint, Skills, Unit, UnitFaction,
};
use crate::ecs_types::resources::{Board, GameData, MovementPlan, ReactionState, TurnOrder};
use crate::error::{BoardError, DataError, Result};
use crate::logic::movement::{Mover, ReachableInfo, reachable_positions, reconstruct_path};
use crate::logic::skill::UnitInfo;
use crate::logic::skill::skill_reaction::{
    CollectMoveReactionsResult, MoveReaction, ReactionUnitInfo, collect_move_reactions,
};
use bevy_ecs::prelude::{With, World};
use std::collections::{HashMap, HashSet};

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

/// 預覽當前行動單位移動到目標格會觸發的藉機攻擊
///
/// 唯讀操作：計算路徑並收集反應者，不改變 World、不產生 pending 反應。
/// 供前端在玩家確認移動前顯示藉機攻擊警示。
/// 目標不可達時回傳空路徑，收集不到任何反應。
pub fn preview_move_reactions(
    world: &mut World,
    target: Position,
) -> Result<CollectMoveReactionsResult> {
    let turn_order = get_resource::<TurnOrder>(world, "請先呼叫 start_new_round")?;
    let occupant = get_current_unit(turn_order)?;

    let entity = find_entity_by_occupant(world, occupant)?;
    let start_pos = *get_component!(world.entity(entity), Position)?;
    let mover_faction = get_component!(world.entity(entity), UnitFaction)?.0;

    let reachable = get_reachable_positions(world, occupant)?;

    let faction_to_alliance = build_faction_alliance_map(world)?;
    let mover_alliance = resolve_alliance(&faction_to_alliance, mover_faction)?;
    let mover_info = UnitInfo {
        occupant,
        faction_id: mover_faction,
        alliance_id: mover_alliance,
    };

    let blocks_sight: HashSet<Position> = world
        .query_filtered::<&Position, With<BlocksSight>>()
        .iter(world)
        .copied()
        .collect();

    let reaction_unit_map = build_reaction_unit_map(world, &faction_to_alliance)?;

    // path 含起點：[start, step1, ..., target]；目標不可達時為空
    let path = reconstruct_path(&reachable, start_pos, target);

    collect_move_reactions(&mover_info, &path, &reaction_unit_map, &blocks_sight)
}

/// 移動路徑規劃期預覽的回傳值
///
/// 對應 BG3 的移動預覽：整條路徑的藉機攻擊警示與危險地面警示。
/// 唯讀，不改變 World、不產生 pending 反應。
#[derive(Debug)]
pub struct MovePathPreview {
    /// 整條路徑上所有會觸發藉機攻擊的反應者（沿途每處脫離都提示）
    pub reactions: Vec<MoveReaction>,
    /// 路徑上經過的危險地面格
    pub hazard_positions: Vec<Position>,
}

/// 預覽當前行動單位移動到目標格的整條路徑警示
///
/// 唯讀操作：計算路徑，收集整條路徑的藉機攻擊反應者與危險地面格，
/// 不改變 World、不產生 pending 反應。供前端在玩家確認移動前顯示完整風險。
/// 目標不可達時回傳空路徑，無任何警示。
///
/// 與 advance_move 不同：預覽回傳整條路徑的反應者，實際移動仍走到第一個觸發就停。
pub fn preview_move_path(world: &mut World, target: Position) -> Result<MovePathPreview> {
    let turn_order = get_resource::<TurnOrder>(world, "請先呼叫 start_new_round")?;
    let occupant = get_current_unit(turn_order)?;

    let entity = find_entity_by_occupant(world, occupant)?;
    let start_pos = *get_component!(world.entity(entity), Position)?;
    let mover_faction = get_component!(world.entity(entity), UnitFaction)?.0;

    let reachable = get_reachable_positions(world, occupant)?;

    let faction_to_alliance = build_faction_alliance_map(world)?;
    let mover_alliance = resolve_alliance(&faction_to_alliance, mover_faction)?;
    let mover_info = UnitInfo {
        occupant,
        faction_id: mover_faction,
        alliance_id: mover_alliance,
    };

    let blocks_sight: HashSet<Position> = world
        .query_filtered::<&Position, With<BlocksSight>>()
        .iter(world)
        .copied()
        .collect();

    let hazard_cells: HashSet<Position> = world
        .query_filtered::<&Position, With<Hazardous>>()
        .iter(world)
        .copied()
        .collect();

    let reaction_unit_map = build_reaction_unit_map(world, &faction_to_alliance)?;

    // path 含起點：[start, step1, ..., target]；目標不可達時為空
    let path = reconstruct_path(&reachable, start_pos, target);

    if path.is_empty() {
        return Err(BoardError::Unreachable {
            x: target.x,
            y: target.y,
        }
        .into());
    }

    let reaction_result =
        collect_move_reactions(&mover_info, &path, &reaction_unit_map, &blocks_sight)?;

    // 危險地面：路徑上（不含起點）經過的危險格
    let hazard_positions: Vec<Position> = path[1..]
        .iter()
        .copied()
        .filter(|pos| hazard_cells.contains(pos))
        .collect();

    Ok(MovePathPreview {
        reactions: reaction_result.reactions,
        hazard_positions,
    })
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
        occupant,
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
        occupant: _,
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

    let blocks_sight: HashSet<Position> = world
        .query_filtered::<&Position, With<BlocksSight>>()
        .iter(world)
        .copied()
        .collect();

    let reaction_unit_map = build_reaction_unit_map(world, &faction_to_alliance)?;

    // === 純邏輯 ===
    // path[next_step_index..] 從當前位置開始，含當前格作為 from
    let reaction_result = collect_move_reactions(
        &mover_info,
        &path[next_step_index..],
        &reaction_unit_map,
        &blocks_sight,
    )?;
    let stop_pos = reaction_result.stop_position;
    // 只取最早觸發步驟的那批反應者（實際移動走到第一個觸發就停）
    let earliest_from_index = reaction_result.earliest_from_index;
    let earliest_reactions: Vec<_> = reaction_result
        .reactions
        .into_iter()
        .filter(|reaction| reaction.from_index == earliest_from_index)
        .collect();
    let has_reactions = !earliest_reactions.is_empty();

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

    // 有反應時停在觸發格的前一格（stop_index - 1），無反應時走到 stop_index
    let stop_index = next_step_index + steps_to_stop;
    let stop_index = if has_reactions {
        stop_index - 1
    } else {
        stop_index
    };
    let actual_stop_pos = path[stop_index];
    let walked_path: Vec<Position> = path[next_step_index..=stop_index].to_vec();
    let walked_cost: MovementCost = step_costs[next_step_index + 1..=stop_index].iter().sum();
    let reached_end = stop_index == path.len() - 1;

    // === 寫入階段 ===
    let entity = find_entity_by_occupant(world, occupant)?;
    let mut entity_mut = world.entity_mut(entity);
    {
        let mut pos = get_component_mut!(entity_mut, Position)?;
        *pos = actual_stop_pos;
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
        let pending: Vec<PendingReaction> = earliest_reactions
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
///
/// 若原移動者已在反應鏈中死亡（當前單位已遞補為他人），此計畫作廢：
/// 移除 MovementPlan、不移動任何單位，回傳走了 0 步的 Completed。
/// 避免把死者遺留的計畫誤套用到遞補的當前單位身上。
pub fn force_advance_move(world: &mut World) -> Result<AdvanceMoveResult> {
    let turn_order = get_resource::<TurnOrder>(world, "請先呼叫 start_new_round")?;
    let occupant = get_current_unit(turn_order)?;

    let MovementPlan {
        occupant: plan_occupant,
        path,
        step_costs,
        next_step_index,
    } = get_resource::<MovementPlan>(world, "請先呼叫 plan_move")?.clone();

    // 移動者已死、當前單位已遞補為他人：計畫作廢，不動任何單位
    if plan_occupant != occupant {
        world.remove_resource::<MovementPlan>();
        return Ok(AdvanceMoveResult::Completed {
            path_walked: vec![],
            cost: 0,
        });
    }

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

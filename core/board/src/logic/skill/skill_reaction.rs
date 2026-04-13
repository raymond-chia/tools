//! 移動反應收集邏輯

use crate::domain::alias::SkillName;
use crate::domain::core_types::{ReactionTrigger, SkillType};
use crate::ecs_types::components::{Occupant, Position};
use crate::error::Result;
use crate::logic::skill::{UnitInfo, is_in_filter, manhattan_distance};
use std::collections::HashMap;

/// 反應者的場上資訊
#[derive(Debug)]
pub struct ReactionUnitInfo<'a> {
    pub unit_info: UnitInfo,
    pub remaining_reactions: i32,
    pub skills: &'a [SkillType],
}

/// 單一反應者的反應結果
#[derive(Debug)]
pub struct MoveReaction {
    pub occupant: Occupant,
    pub skill_names: Vec<SkillName>,
}

/// collect_move_reactions 的回傳值
#[derive(Debug)]
pub struct CollectMoveReactionsResult {
    pub stop_position: Position,
    pub reactions: Vec<MoveReaction>,
}

/// 收集移動路徑上最早觸發反應的步驟的所有反應者
///
/// 外層遍歷每個反應者，內層掃描路徑找到該反應者最早觸發的步驟。
/// 維護 earliest_from_idx 逐步收縮搜尋範圍，後續 reactor 只需掃到該步驟為止。
pub(crate) fn collect_move_reactions(
    mover: &UnitInfo,
    path: &[Position],
    units_on_board: &HashMap<Position, ReactionUnitInfo<'_>>,
) -> Result<CollectMoveReactionsResult> {
    // earliest_from_idx: 目前已知最早觸發的步驟索引（exclusive upper bound）
    // from -> to. 最後一個 to idx = len-1, 最後一個 from idx = len-2
    let mut earliest_from_idx = path.len() - 2;
    let mut reactions: Vec<MoveReaction> = Vec::new();

    for (reactor_pos, reactor) in units_on_board {
        if reactor.unit_info.occupant == mover.occupant {
            continue;
        }
        if reactor.remaining_reactions <= 0 {
            continue;
        }

        // 預過濾：只保留 Reaction 且 trigger 為 AttackOfOpportunity 且 filter 符合的技能
        let reaction_skills: Vec<_> = reactor
            .skills
            .iter()
            .filter_map(|skill| match skill {
                SkillType::Reaction {
                    name,
                    triggering_unit,
                    ..
                } => {
                    if matches!(
                        triggering_unit.trigger,
                        ReactionTrigger::AttackOfOpportunity
                    ) && is_in_filter(&reactor.unit_info, mover, triggering_unit.source_filter)
                    {
                        Some((name, triggering_unit))
                    } else {
                        None
                    }
                }
                SkillType::Active { .. } | SkillType::Passive { .. } => None,
            })
            .collect();

        if reaction_skills.is_empty() {
            continue;
        }

        // 掃描路徑，只掃到 earliest_from_idx 為止（含）
        // take(n) = take(idx + 1)
        for (step_index, from) in path.iter().enumerate().take(earliest_from_idx + 1) {
            let distance = manhattan_distance(*reactor_pos, *from);

            let is_in_range = |range: (usize, usize)| distance >= range.0 && distance <= range.1;

            let matching_skills: Vec<SkillName> = reaction_skills
                .iter()
                .filter(|(_, t)| is_in_range(t.source_range))
                .map(|(name, _)| (*name).clone())
                .collect();

            if matching_skills.is_empty() {
                continue;
            }

            if step_index < earliest_from_idx {
                // 發現更早的步驟，丟掉之前的結果
                earliest_from_idx = step_index;
                reactions.clear();
            }

            reactions.push(MoveReaction {
                occupant: reactor.unit_info.occupant,
                skill_names: matching_skills,
            });
            // 這個 reactor 已找到最早觸發步驟，不需繼續掃
            break;
        }
    }

    Ok(CollectMoveReactionsResult {
        stop_position: path[earliest_from_idx + 1],
        reactions,
    })
}

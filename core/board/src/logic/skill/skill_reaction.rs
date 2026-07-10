//! 反應收集邏輯

use crate::domain::alias::SkillName;
use crate::domain::core_types::{PendingReaction, ReactionTrigger, SkillType, TriggeringSource};
use crate::ecs_types::components::{Occupant, Position};
use crate::ecs_types::resources::GameData;
use crate::error::Result;
use crate::logic::skill::line_of_sight::has_line_of_sight;
use crate::logic::skill::skill_execution::{CheckTarget, CombatStats, EffectEntry, ResolvedEffect};
use crate::logic::skill::{UnitInfo, is_in_filter, manhattan_distance};
use std::collections::{HashMap, HashSet};

/// 反應者的場上資訊
#[derive(Debug)]
pub struct ReactionUnitInfo<'a> {
    pub unit_info: UnitInfo,
    pub remaining_reactions: i32,
    pub skills: Vec<&'a SkillType>,
}

/// 單一反應者的反應結果
#[derive(Debug)]
pub struct MoveReaction {
    pub occupant: Occupant,
    pub skill_names: Vec<SkillName>,
    /// 該反應者觸發的步驟索引：移動者離開的那格（path 的 from 索引）
    pub from_index: usize,
}

/// collect_move_reactions 的回傳值
///
/// `reactions` 涵蓋整條路徑上所有會觸發藉機攻擊的反應者（每個反應者記其最早觸發的步驟），
/// 不收斂到單一步驟。呼叫端可自行取用：
/// - 預覽：使用整條路徑的全部反應者。
/// - 實際移動：過濾出最早觸發步驟（`earliest_from_index`）的那批。
#[derive(Debug)]
pub struct CollectMoveReactionsResult {
    /// 最早觸發步驟的下一格（移動者實際會停下的位置）；無反應時為路徑終點
    pub stop_position: Position,
    /// 最早觸發的步驟索引；無反應時為 path.len() - 2
    pub earliest_from_index: usize,
    /// 整條路徑上所有反應者，每個記其最早觸發步驟
    pub reactions: Vec<MoveReaction>,
}

/// 收集移動路徑上所有會觸發藉機攻擊的反應者
///
/// 外層遍歷每個反應者，內層掃描路徑找到該反應者最早觸發的步驟。
/// 不收斂：每個反應者各自記錄其最早觸發步驟，全部保留供呼叫端自行取用。
///
/// `blocks_sight`：阻擋視線的格子集合。反應者對觸發格（移動者離開的那格）無視線時，
/// 該步驟不觸發反應，比照 execute_skill 的 caster ↔ target 判定。
pub(crate) fn collect_move_reactions(
    mover: &UnitInfo,
    path: &[Position],
    units_on_board: &HashMap<Position, ReactionUnitInfo<'_>>,
    blocks_sight: &HashSet<Position>,
) -> Result<CollectMoveReactionsResult> {
    // from -> to. 最後一個 to idx = len-1, 最後一個 from idx = len-2
    let last_from_idx = path.len() - 2;
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

        // 掃描整條路徑，找該反應者最早觸發的步驟
        for (step_index, from) in path.iter().enumerate().take(last_from_idx + 1) {
            // 視線基準：反應者 ↔ 觸發格（移動者離開的那格），被阻擋則此步驟不觸發
            if !has_line_of_sight(*reactor_pos, *from, blocks_sight) {
                continue;
            }

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

            reactions.push(MoveReaction {
                occupant: reactor.unit_info.occupant,
                skill_names: matching_skills,
                from_index: step_index,
            });
            // 這個 reactor 已找到最早觸發步驟，不需繼續掃
            break;
        }
    }

    // 最早觸發步驟：所有反應者中最小的 from_index；無反應時為 last_from_idx
    let earliest_from_index = reactions
        .iter()
        .map(|reaction| reaction.from_index)
        .min()
        .unwrap_or(last_from_idx);

    Ok(CollectMoveReactionsResult {
        stop_position: path[earliest_from_index + 1],
        reactions,
        earliest_from_index,
    })
}

/// 受傷單位的反應資訊（供 collect_takes_damage_reactions 使用）
pub struct TakesDamageUnitInfo {
    pub pos: Position,
    pub remaining_reaction_point: i32,
    pub skill_names: Vec<SkillName>,
}

/// 從效果結果中收集受傷單位可觸發的 TakesDamage 反應
///
/// - `entries`：技能執行後產生的效果條目
/// - `attacker`：施放者（成為 trigger）
/// - `unit_reaction_info`：場上單位的反應資訊，key 為 Occupant
/// - `game_data`：用於過濾 TakesDamage 技能
/// - `unit_stats_on_board`：場上單位的戰鬥數值，用於 filter 判斷
pub(crate) fn collect_takes_damage_reactions(
    entries: &[EffectEntry],
    attacker: Occupant,
    attacker_pos: Position,
    game_data: &GameData,
    unit_reaction_info: &HashMap<Occupant, TakesDamageUnitInfo>,
    unit_stats_on_board: &HashMap<Position, CombatStats>,
) -> Vec<PendingReaction> {
    let takes_damage_skills: HashMap<&SkillName, _> = game_data
        .skill_map
        .iter()
        .filter_map(|(name, skill_type)| match skill_type {
            SkillType::Reaction {
                triggering_unit, ..
            } if matches!(triggering_unit.trigger, ReactionTrigger::TakesDamage) => {
                Some((name, triggering_unit))
            }
            _ => None,
        })
        .collect();
    entries
        .iter()
        .filter_map(|entry| match (&entry.effect, &entry.target) {
            (ResolvedEffect::HpChange { final_amount, .. }, CheckTarget::Unit(damaged_id))
                if *final_amount < 0 =>
            {
                Some(Occupant::Unit(*damaged_id))
            }
            _ => None,
        })
        .filter_map(|damaged_occupant| {
            let info = unit_reaction_info.get(&damaged_occupant)?;
            if info.remaining_reaction_point <= 0 {
                return None;
            }
            let available_skills = filter_takes_damage_skills(
                info.pos,
                &info.skill_names,
                attacker_pos,
                &takes_damage_skills,
                unit_stats_on_board,
            );
            if available_skills.is_empty() {
                return None;
            }
            Some(PendingReaction {
                reactor: damaged_occupant,
                trigger: attacker,
                trigger_event: ReactionTrigger::TakesDamage,
                available_skills,
            })
        })
        .collect()
}

fn filter_takes_damage_skills<'a>(
    damaged_pos: Position,
    damaged_skills: &[SkillName],
    attacker_pos: Position,
    takes_damage_skills: &HashMap<&'a SkillName, &'a TriggeringSource>,
    unit_stats_on_board: &HashMap<Position, CombatStats>,
) -> Vec<SkillName> {
    damaged_skills
        .iter()
        .filter_map(|skill_name| {
            let triggering_unit = takes_damage_skills.get(skill_name)?;
            let distance = manhattan_distance(damaged_pos, attacker_pos);
            let (min_range, max_range) = triggering_unit.source_range;
            if distance < min_range || distance > max_range {
                return None;
            }
            let damaged_info = unit_stats_on_board.get(&damaged_pos)?;
            let attacker_info = unit_stats_on_board.get(&attacker_pos)?;
            if !is_in_filter(
                &damaged_info.unit_info,
                &attacker_info.unit_info,
                triggering_unit.source_filter,
            ) {
                return None;
            }
            Some(skill_name.clone())
        })
        .collect()
}

//! 技能效果樹執行邏輯

use crate::domain::alias::ID;
use crate::domain::core_types::{
    Attribute, CasterOrTarget, Effect, EffectCondition, EffectNode, Scaling,
};
use crate::ecs_types::components::{AttributeBundle, Occupant, Position};
use crate::ecs_types::resources::Board;
use crate::logic::skill::skill_check::{HitResult, resolve_hit};
use crate::logic::skill::skill_range::compute_affected_positions;
use crate::logic::skill::{UnitInfo, is_in_filter};
use std::collections::HashMap;

/// 戰鬥屬性（傳入 resolve_effect_tree 的單位資料）
#[derive(Debug, Clone)]
pub struct CombatStats {
    pub unit_info: UnitInfo,
    pub attribute: AttributeBundle,
    pub crit_rate: i32,
}

/// 效果作用目標
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheckTarget {
    Unit(ID),
    TODO,
}

/// 判定結果
#[derive(Debug, Clone, PartialEq)]
pub enum CheckResult {
    /// 無判定，必定生效
    Auto,
    Hit {
        crit: bool,
    },
    Block {
        crit: bool,
    },
    Evade,
}

/// 解析後的效果
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedEffect {
    NoEffect,
    HpChange { raw_amount: i32, final_amount: i32 },
}

/// 單筆效果條目
#[derive(Debug, Clone)]
pub struct EffectEntry {
    pub target: CheckTarget,
    pub check: CheckResult,
    pub effect: ResolvedEffect,
}

/// 解析效果樹，產生效果條目列表
pub(crate) fn resolve_effect_tree(
    nodes: &[EffectNode],
    caster: &CombatStats,
    caster_pos: Position,
    target_pos: Position,
    units_on_board: &HashMap<Position, CombatStats>,
    board: Board,
    rng: &mut impl FnMut() -> i32,
) -> Vec<EffectEntry> {
    let mut entries = Vec::new();

    for node in nodes {
        match node {
            EffectNode::Area {
                area,
                filter,
                nodes: inner_nodes,
            } => {
                let affected = compute_affected_positions(area, caster_pos, target_pos, board)
                    .expect("compute_affected_positions 應成功計算 AOE 範圍");

                for pos in affected {
                    let target_stats = match units_on_board.get(&pos) {
                        Some(stats) => stats,
                        // TODO
                        None => continue,
                    };
                    if !is_in_filter(&caster.unit_info, &target_stats.unit_info, filter) {
                        continue;
                    }
                    resolve_nodes_for_target(inner_nodes, caster, target_stats, rng, &mut entries);
                }
            }
            EffectNode::Leaf { .. } => {
                let target_stats = match units_on_board.get(&target_pos) {
                    Some(stats) => stats,
                    // TODO
                    None => continue,
                };
                resolve_nodes_for_target(
                    std::slice::from_ref(node),
                    caster,
                    target_stats,
                    rng,
                    &mut entries,
                );
            }
            EffectNode::Branch { .. } => {
                let target_stats = match units_on_board.get(&target_pos) {
                    Some(stats) => stats,
                    // TODO
                    None => continue,
                };
                resolve_nodes_for_target(
                    std::slice::from_ref(node),
                    caster,
                    target_stats,
                    rng,
                    &mut entries,
                );
            }
        }
    }

    entries
}

/// 將 HitResult 轉換為 CheckResult
fn hit_result_to_check(hit: HitResult) -> CheckResult {
    match hit {
        HitResult::Hit { crit } => CheckResult::Hit { crit },
        HitResult::Block { crit } => CheckResult::Block { crit },
        HitResult::Evade => CheckResult::Evade,
    }
}

/// 從 Attribute enum 取得 AttributeBundle 中對應的值
fn get_attribute_value(bundle: &AttributeBundle, attr: Attribute) -> i32 {
    match attr {
        Attribute::Hp => bundle.current_hp.0,
        Attribute::Mp => bundle.current_mp.0,
        Attribute::Initiative => bundle.initiative.0,
        Attribute::Accuracy => bundle.accuracy.0,
        Attribute::Evasion => bundle.evasion.0,
        Attribute::Block => bundle.block.0,
        Attribute::BlockProtection => bundle.block_protection.0,
        Attribute::PhysicalAttack => bundle.physical_attack.0,
        Attribute::MagicalAttack => bundle.magical_attack.0,
        Attribute::MagicalDc => bundle.magical_dc.0,
        Attribute::Fortitude => bundle.fortitude.0,
        Attribute::Reflex => bundle.reflex.0,
        Attribute::Will => bundle.will.0,
        Attribute::MovementPoint => bundle.movement_point.0,
        Attribute::ReactionPoint => bundle.reaction_point.0,
    }
}

/// 計算 Scaling 的數值
fn compute_scaling(scaling: &Scaling, caster: &CombatStats, target: &CombatStats) -> i32 {
    let source_stats = match scaling.source {
        CasterOrTarget::Caster => caster,
        CasterOrTarget::Target => target,
    };
    let base = get_attribute_value(&source_stats.attribute, scaling.source_attribute.clone());
    base * scaling.value_percent / 100
}

/// 將 Occupant 轉換為 CheckTarget
fn occupant_to_check_target(occupant: Occupant) -> CheckTarget {
    match occupant {
        Occupant::Unit(id) => CheckTarget::Unit(id),
        // TODO
        Occupant::Object(id) => CheckTarget::Unit(id),
    }
}

/// 對單一目標解析效果節點列表
fn resolve_nodes_for_target(
    nodes: &[EffectNode],
    caster: &CombatStats,
    target: &CombatStats,
    rng: &mut impl FnMut() -> i32,
    entries: &mut Vec<EffectEntry>,
) {
    for node in nodes {
        match node {
            EffectNode::Leaf { who, effect } => {
                let resolved_target = match who {
                    CasterOrTarget::Caster => &caster,
                    CasterOrTarget::Target => &target,
                };
                let check_target = occupant_to_check_target(resolved_target.unit_info.occupant);
                match effect {
                    Effect::HpEffect { scaling } => {
                        let raw_amount = compute_scaling(scaling, caster, target);
                        entries.push(EffectEntry {
                            target: check_target,
                            check: CheckResult::Auto,
                            effect: ResolvedEffect::HpChange {
                                raw_amount,
                                final_amount: raw_amount,
                            },
                        });
                    }
                    _ => {}
                }
            }
            EffectNode::Branch {
                who: _,
                condition,
                on_success,
                on_failure,
            } => {
                let check = match condition {
                    EffectCondition::HitCheck {
                        accuracy_bonus,
                        crit_bonus,
                    } => {
                        let attacker_hit = caster.attribute.accuracy.0 + accuracy_bonus;
                        let defender_evasion = target.attribute.evasion.0;
                        let defender_block = target.attribute.block.0;
                        let crit_rate = caster.crit_rate + crit_bonus;
                        let hit = resolve_hit(
                            attacker_hit,
                            defender_evasion,
                            defender_block,
                            crit_rate,
                            rng,
                        );
                        hit_result_to_check(hit)
                    }
                    EffectCondition::DcCheck { .. } => {
                        // DC 判定暫不實作
                        CheckResult::Auto
                    }
                };

                let branch_nodes = match check {
                    CheckResult::Auto | CheckResult::Hit { .. } | CheckResult::Block { .. } => {
                        on_success
                    }
                    CheckResult::Evade => on_failure,
                };

                if branch_nodes.is_empty() {
                    let check_target = occupant_to_check_target(target.unit_info.occupant);
                    entries.push(EffectEntry {
                        target: check_target,
                        check,
                        effect: ResolvedEffect::NoEffect,
                    });
                } else {
                    let start = entries.len();
                    resolve_nodes_for_target(branch_nodes, caster, target, rng, entries);
                    for entry in &mut entries[start..] {
                        entry.check = check.clone();
                    }
                }
            }
            EffectNode::Area { .. } => {
                // Area 在頂層處理，巢狀 Area 不支援
            }
        }
    }
}

//! 技能效果樹執行邏輯

use crate::domain::alias::{ID, TypeName};
use crate::domain::core_types::{
    AccuracySource, Attribute, CasterOrTarget, DefenseType, Effect, EffectCondition, EffectNode,
    Scaling, TargetFilter,
};
use crate::ecs_types::components::{AttributeBundle, Occupant, Position};
use crate::ecs_types::resources::Board;
use crate::error::Result;
use crate::logic::skill::skill_check::{HitResult, resolve_hit};
use crate::logic::skill::skill_range::compute_affected_positions;
use crate::logic::skill::{UnitInfo, is_in_filter};
use std::collections::HashMap;

/// 戰鬥屬性（傳入 resolve_effect_tree 的單位資料）
#[derive(Debug, Clone)]
pub struct CombatStats {
    pub unit_info: UnitInfo,
    pub attribute: AttributeBundle,
}

/// 棋盤上的物件資訊
#[derive(Debug, Clone)]
pub struct ObjectOnBoard {
    pub occupant: Occupant,
    pub occupies_tile: bool,
}

/// 效果作用目標
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheckTarget {
    Unit(ID),
    Position(Position),
}

/// 判定結果
#[derive(Debug, Clone, Copy, PartialEq)]
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
    Resisted, // 對應 evade
    Affected, // 對應 hit
}

/// 解析後的效果
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedEffect {
    NoEffect,
    HpChange { raw_amount: i32, final_amount: i32 },
    SpawnObject { object_type: TypeName },
    ApplyBuff(String),
}

/// 單筆效果條目
#[derive(Debug, Clone, PartialEq)]
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
    objects_on_board: &HashMap<Position, ObjectOnBoard>,
    board: Board,
    rng: &mut impl FnMut() -> i32,
) -> Result<Vec<EffectEntry>> {
    let mut entries = Vec::new();

    for node in nodes {
        match node {
            EffectNode::Area {
                area,
                filter,
                nodes: inner_nodes,
            } => {
                let affected_positions =
                    compute_affected_positions(area, caster_pos, target_pos, board)?;

                for target_pos in affected_positions {
                    resolve_at_position(
                        inner_nodes,
                        caster,
                        target_pos,
                        *filter,
                        units_on_board,
                        objects_on_board,
                        rng,
                        &mut entries,
                    );
                }
            }
            EffectNode::Branch { .. } | EffectNode::Leaf { .. } => {
                resolve_at_position(
                    std::slice::from_ref(node),
                    caster,
                    target_pos,
                    TargetFilter::Any,
                    units_on_board,
                    objects_on_board,
                    rng,
                    &mut entries,
                );
            }
        }
    }

    Ok(entries)
}

/// 在指定位置解析效果節點
fn resolve_at_position(
    nodes: &[EffectNode],
    caster: &CombatStats,
    target_pos: Position,
    filter: TargetFilter,
    units_on_board: &HashMap<Position, CombatStats>,
    objects_on_board: &HashMap<Position, ObjectOnBoard>,
    rng: &mut impl FnMut() -> i32,
    entries: &mut Vec<EffectEntry>,
) {
    match units_on_board.get(&target_pos) {
        Some(target_stats) => {
            if !is_in_filter(&caster.unit_info, &target_stats.unit_info, filter) {
                return;
            }
            resolve_nodes_for_unit(nodes, caster, target_stats, CheckResult::Auto, rng, entries);
        }
        None => {
            resolve_nodes_for_position(
                nodes,
                target_pos,
                units_on_board,
                objects_on_board,
                entries,
            );
        }
    }
}

/// 帶判定結果的效果節點解析
fn resolve_nodes_for_unit(
    nodes: &[EffectNode],
    caster: &CombatStats,
    target: &CombatStats,
    parent_check: CheckResult,
    rng: &mut impl FnMut() -> i32,
    entries: &mut Vec<EffectEntry>,
) {
    for node in nodes {
        match node {
            EffectNode::Leaf { who, effect } => {
                let resolved_target = match who {
                    CasterOrTarget::Caster => caster,
                    CasterOrTarget::Target => target,
                };
                let check_target = occupant_to_check_target(resolved_target.unit_info.occupant);
                match effect {
                    Effect::HpEffect { scaling } => {
                        let raw_amount = compute_scaling(scaling, caster, target);
                        let final_amount = match parent_check {
                            CheckResult::Block { .. } => apply_block_protection(
                                raw_amount,
                                target.attribute.block_protection.0,
                            ),
                            CheckResult::Auto
                            | CheckResult::Hit { .. }
                            | CheckResult::Evade
                            | CheckResult::Resisted
                            | CheckResult::Affected => raw_amount,
                        };
                        entries.push(EffectEntry {
                            target: check_target,
                            check: parent_check,
                            effect: ResolvedEffect::HpChange {
                                raw_amount,
                                final_amount,
                            },
                        });
                    }
                    Effect::ApplyBuff { buff } => {
                        entries.push(EffectEntry {
                            target: check_target,
                            check: parent_check,
                            effect: ResolvedEffect::ApplyBuff(buff.name.clone()),
                        });
                    }
                    Effect::SpawnObject { .. } => {
                        // TODO 新增格子著火的測試
                    }
                    _ => unimplemented!("Effect type not supported yet: {:?}", effect),
                }
            }
            EffectNode::Branch {
                condition,
                on_success,
                on_failure,
            } => {
                let check = resolve_branch_check(caster, target, condition, rng);

                let branch_nodes = match check {
                    CheckResult::Auto
                    | CheckResult::Hit { .. }
                    | CheckResult::Block { .. }
                    | CheckResult::Affected => on_success,
                    CheckResult::Evade | CheckResult::Resisted => on_failure,
                };

                if branch_nodes.is_empty() {
                    let check_target = occupant_to_check_target(target.unit_info.occupant);
                    entries.push(EffectEntry {
                        target: check_target,
                        check,
                        effect: ResolvedEffect::NoEffect,
                    });
                } else {
                    resolve_nodes_for_unit(branch_nodes, caster, target, check, rng, entries);
                }
            }
            EffectNode::Area { .. } => {
                unreachable!("Nested Area nodes are not supported");
            }
        }
    }
}

/// 對無單位位置解析效果節點（僅處理 SpawnObject 等位置效果）
fn resolve_nodes_for_position(
    nodes: &[EffectNode],
    pos: Position,
    units_on_board: &HashMap<Position, CombatStats>,
    objects_on_board: &HashMap<Position, ObjectOnBoard>,
    entries: &mut Vec<EffectEntry>,
) {
    for node in nodes {
        if let EffectNode::Leaf { effect, who: _who } = node {
            match effect {
                Effect::SpawnObject { object_type, .. } => {
                    if !is_tile_occupied(pos, units_on_board, objects_on_board) {
                        entries.push(EffectEntry {
                            target: CheckTarget::Position(pos),
                            check: CheckResult::Auto,
                            effect: ResolvedEffect::SpawnObject {
                                object_type: object_type.clone(),
                            },
                        });
                    }
                }
                Effect::HpEffect { .. } | Effect::ApplyBuff { .. } => {}
                _ => unimplemented!(
                    "Effect type not supported for position target yet: {:?}",
                    effect
                ),
            }
        }
    }
}

/// 解析 Branch 節點的判定結果
fn resolve_branch_check(
    caster: &CombatStats,
    target: &CombatStats,
    condition: &EffectCondition,
    rng: &mut impl FnMut() -> i32,
) -> CheckResult {
    let attacker_acc = match condition.accuracy_source {
        AccuracySource::Physical => caster.attribute.physical_accuracy.0,
        AccuracySource::Magical => caster.attribute.magical_accuracy.0,
    } + condition.accuracy_bonus;

    let (defender_evasion, defender_block) =
        get_defense_values(&target.attribute, condition.defense_type);
    let crit_rate = condition.crit_bonus;

    let hit = resolve_hit(
        attacker_acc,
        defender_evasion,
        defender_block,
        crit_rate,
        rng,
    );
    hit_to_check(hit, condition.defense_type)
}

/// 取得防禦值（閃避值、格擋值）
fn get_defense_values(target: &AttributeBundle, defense_type: DefenseType) -> (i32, i32) {
    match defense_type {
        DefenseType::Fortitude => (target.fortitude.0, 0),
        DefenseType::Agility => (target.agility.0, 0),
        DefenseType::AgilityAndBlock => (target.agility.0, target.block.0),
        DefenseType::Will => (target.will.0, 0),
    }
}

/// 將 HitResult 轉換為 CheckResult
fn hit_to_check(hit: HitResult, defense_type: DefenseType) -> CheckResult {
    match defense_type {
        DefenseType::AgilityAndBlock => match hit {
            HitResult::Hit { crit } => CheckResult::Hit { crit },
            HitResult::Block { crit } => CheckResult::Block { crit },
            HitResult::Evade => CheckResult::Evade,
        },
        DefenseType::Fortitude | DefenseType::Agility | DefenseType::Will => match hit {
            HitResult::Hit { .. } | HitResult::Block { .. } => CheckResult::Affected,
            HitResult::Evade => CheckResult::Resisted,
        },
    }
}

/// 計算 Scaling 的數值
fn compute_scaling(scaling: &Scaling, caster: &CombatStats, target: &CombatStats) -> i32 {
    let source_stats = match scaling.source {
        CasterOrTarget::Caster => caster,
        CasterOrTarget::Target => target,
    };
    let base = get_attribute_value(&source_stats.attribute, scaling.source_attribute);
    base * scaling.value_percent / 100
}

/// 從 Attribute enum 取得 AttributeBundle 中對應的值
fn get_attribute_value(bundle: &AttributeBundle, attr: Attribute) -> i32 {
    match attr {
        Attribute::Hp => bundle.current_hp.0,
        Attribute::Mp => bundle.current_mp.0,
        Attribute::Initiative => bundle.initiative.0,
        Attribute::PhysicalAttack => bundle.physical_attack.0,
        Attribute::MagicalAttack => bundle.magical_attack.0,
        Attribute::PhysicalAccuracy => bundle.physical_accuracy.0,
        Attribute::MagicalAccuracy => bundle.magical_accuracy.0,
        Attribute::Fortitude => bundle.fortitude.0,
        Attribute::Agility => bundle.agility.0,
        Attribute::Block => bundle.block.0,
        Attribute::BlockProtection => bundle.block_protection.0,
        Attribute::Will => bundle.will.0,
        Attribute::MovementPoint => bundle.movement_point.0,
        Attribute::ReactionPoint => bundle.reaction_point.0,
    }
}

/// 將 Occupant 轉換為 CheckTarget
fn occupant_to_check_target(occupant: Occupant) -> CheckTarget {
    match occupant {
        Occupant::Unit(id) => CheckTarget::Unit(id),
        Occupant::Object(id) => {
            unimplemented!("Object occupant not supported yet: {}", id)
        }
    }
}

/// 計算格擋後的最終傷害
fn apply_block_protection(raw_amount: i32, block_protection: i32) -> i32 {
    // block_protection 為減傷百分比，只對負數（傷害）生效
    if raw_amount >= 0 {
        return raw_amount;
    }
    raw_amount * (100 - block_protection) / 100
}

/// 判斷位置是否被佔據（有單位或不可通過物件）
fn is_tile_occupied(
    pos: Position,
    units_on_board: &HashMap<Position, CombatStats>,
    objects_on_board: &HashMap<Position, ObjectOnBoard>,
) -> bool {
    if units_on_board.contains_key(&pos) {
        return true;
    }
    match objects_on_board.get(&pos) {
        Some(obj) => obj.occupies_tile,
        None => false,
    }
}

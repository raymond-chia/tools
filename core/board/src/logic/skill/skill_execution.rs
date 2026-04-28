//! 技能效果樹執行邏輯

use crate::domain::alias::{ID, SkillName, TypeName};
use crate::domain::constants::{CRIT_DAMAGE_MULTIPLIER, FLANKING_REQUIRED_ALLIES};
use crate::domain::core_types::{
    AccuracySource, Attribute, CasterOrTarget, DefenseType, Effect, EffectCondition, EffectNode,
    Scaling, SkillTag, TargetFilter,
};
use crate::ecs_types::components::{AttributeBundle, Occupant, Position};
use crate::ecs_types::resources::Board;
use crate::error::Result;
use crate::logic::board::try_position;
use crate::logic::skill::skill_check::{HitCheckResult, resolve_hit};
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

/// 命中判定的詳細數值（用於 log 顯示）
#[derive(Debug, Clone, PartialEq)]
pub struct CheckDetail {
    pub accuracy_source: AccuracySource,
    pub defense_type: DefenseType,
    pub attacker_accuracy: i32,
    pub defender_evasion: i32,
    pub defender_block: i32,
    pub crit_rate: i32,
    pub roll: i32,
}

/// 單筆效果條目
#[derive(Debug, Clone, PartialEq)]
pub struct EffectEntry {
    pub caster: ID,
    pub skill_name: SkillName,
    pub target: CheckTarget,
    pub check: CheckResult,
    pub check_detail: Option<CheckDetail>,
    pub effect: ResolvedEffect,
}

/// 解析效果樹，產生效果條目列表
pub(crate) fn resolve_effect_tree(
    caster_id: ID,
    skill_name: &str,
    skill_tags: &[SkillTag],
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
                        caster_id,
                        skill_name,
                        skill_tags,
                        inner_nodes,
                        caster,
                        target_pos,
                        *filter,
                        units_on_board,
                        objects_on_board,
                        board,
                        rng,
                        &mut entries,
                    );
                }
            }
            EffectNode::Branch { .. } | EffectNode::Leaf { .. } => {
                resolve_at_position(
                    caster_id,
                    skill_name,
                    skill_tags,
                    std::slice::from_ref(node),
                    caster,
                    target_pos,
                    TargetFilter::Any,
                    units_on_board,
                    objects_on_board,
                    board,
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
    caster_id: ID,
    skill_name: &str,
    skill_tags: &[SkillTag],
    nodes: &[EffectNode],
    caster: &CombatStats,
    target_pos: Position,
    filter: TargetFilter,
    units_on_board: &HashMap<Position, CombatStats>,
    objects_on_board: &HashMap<Position, ObjectOnBoard>,
    board: Board,
    rng: &mut impl FnMut() -> i32,
    entries: &mut Vec<EffectEntry>,
) {
    match units_on_board.get(&target_pos) {
        Some(target_stats) => {
            if !is_in_filter(&caster.unit_info, &target_stats.unit_info, filter) {
                return;
            }
            let flanking_bonus =
                compute_flanking_bonus(skill_tags, caster, target_pos, units_on_board, board);
            resolve_nodes_for_unit(
                caster_id,
                skill_name,
                nodes,
                caster,
                target_stats,
                flanking_bonus,
                CheckResult::Auto,
                None,
                rng,
                entries,
            );
        }
        None => {
            resolve_nodes_for_position(
                caster_id,
                skill_name,
                nodes,
                target_pos,
                units_on_board,
                objects_on_board,
                entries,
            );
        }
    }
}

/// 根據技能 Flankable tag、夾擊狀態與 caster 屬性計算命中加成
fn compute_flanking_bonus(
    skill_tags: &[SkillTag],
    caster: &CombatStats,
    target_pos: Position,
    units_on_board: &HashMap<Position, CombatStats>,
    board: Board,
) -> i32 {
    let is_flankable = skill_tags.iter().any(|t| matches!(t, SkillTag::Flankable));
    if !is_flankable || !is_flanked(&caster.unit_info, target_pos, units_on_board, board) {
        return 0;
    }
    caster.attribute.flanking_accuracy_bonus.0
}

/// 帶判定結果的效果節點解析
fn resolve_nodes_for_unit(
    caster_id: ID,
    skill_name: &str,
    nodes: &[EffectNode],
    caster: &CombatStats,
    target: &CombatStats,
    flanking_bonus: i32,
    parent_check: CheckResult,
    parent_check_detail: Option<CheckDetail>,
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
                        let crit_multiplier = match parent_check {
                            CheckResult::Hit { crit: true } | CheckResult::Block { crit: true } => {
                                CRIT_DAMAGE_MULTIPLIER
                            }
                            _ => 1,
                        };
                        let final_amount = raw_amount * crit_multiplier;
                        let final_amount = match parent_check {
                            CheckResult::Block { .. } => apply_block_protection(
                                final_amount,
                                target.attribute.block_protection.0,
                            ),
                            CheckResult::Auto
                            | CheckResult::Hit { .. }
                            | CheckResult::Evade
                            | CheckResult::Resisted
                            | CheckResult::Affected => final_amount,
                        };
                        entries.push(EffectEntry {
                            caster: caster_id,
                            skill_name: skill_name.to_string(),
                            target: check_target,
                            check: parent_check,
                            check_detail: parent_check_detail.clone(),
                            effect: ResolvedEffect::HpChange {
                                raw_amount,
                                final_amount,
                            },
                        });
                    }
                    Effect::ApplyBuff { buff } => {
                        entries.push(EffectEntry {
                            caster: caster_id,
                            skill_name: skill_name.to_string(),
                            target: check_target,
                            check: parent_check,
                            check_detail: parent_check_detail.clone(),
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
                let (check, detail) =
                    resolve_branch_check(caster, target, condition, flanking_bonus, rng);

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
                        caster: caster_id,
                        skill_name: skill_name.to_string(),
                        target: check_target,
                        check,
                        check_detail: Some(detail),
                        effect: ResolvedEffect::NoEffect,
                    });
                } else {
                    resolve_nodes_for_unit(
                        caster_id,
                        skill_name,
                        branch_nodes,
                        caster,
                        target,
                        flanking_bonus,
                        check,
                        Some(detail),
                        rng,
                        entries,
                    );
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
    caster_id: ID,
    skill_name: &str,
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
                            caster: caster_id,
                            skill_name: skill_name.to_string(),
                            target: CheckTarget::Position(pos),
                            check: CheckResult::Auto,
                            check_detail: None,
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
    flanking_bonus: i32,
    rng: &mut impl FnMut() -> i32,
) -> (CheckResult, CheckDetail) {
    let attacker_acc = match condition.accuracy_source {
        AccuracySource::Physical => caster.attribute.physical_accuracy.0,
        AccuracySource::Magical => caster.attribute.magical_accuracy.0,
    } + condition.accuracy_bonus
        + flanking_bonus;

    let (defender_evasion, defender_block) =
        get_defense_values(&target.attribute, condition.defense_type);
    let crit_rate = condition.crit_bonus;

    let outcome = resolve_hit(
        attacker_acc,
        defender_evasion,
        defender_block,
        crit_rate,
        rng,
    );
    let check = hit_to_check(outcome.check, condition.defense_type);
    let detail = CheckDetail {
        accuracy_source: condition.accuracy_source.clone(),
        defense_type: condition.defense_type,
        attacker_accuracy: attacker_acc,
        defender_evasion,
        defender_block,
        crit_rate,
        roll: outcome.roll,
    };
    (check, detail)
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

/// 將 HitCheckResult 轉換為 CheckResult
fn hit_to_check(hit: HitCheckResult, defense_type: DefenseType) -> CheckResult {
    match defense_type {
        DefenseType::AgilityAndBlock => match hit {
            HitCheckResult::Hit { crit } => CheckResult::Hit { crit },
            HitCheckResult::Block { crit } => CheckResult::Block { crit },
            HitCheckResult::Evade => CheckResult::Evade,
        },
        DefenseType::Fortitude | DefenseType::Agility | DefenseType::Will => match hit {
            HitCheckResult::Hit { .. } | HitCheckResult::Block { .. } => CheckResult::Affected,
            HitCheckResult::Evade => CheckResult::Resisted,
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
        Attribute::FlankingAccuracyBonus => bundle.flanking_accuracy_bonus.0,
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

/// 判定 target 是否被 caster alliance 夾擊
///
/// 條件：target 的 4 個曼哈頓相鄰格中，被 caster alliance 友軍佔據的格子數
/// 達到 FLANKING_REQUIRED_ALLIES。units_on_board 須包含 caster 本身。
fn is_flanked(
    caster: &UnitInfo,
    target_pos: Position,
    units_on_board: &HashMap<Position, CombatStats>,
    board: Board,
) -> bool {
    let neighbors = [(1_i32, 0_i32), (-1, 0), (0, 1), (0, -1)];
    let ally_count = neighbors
        .iter()
        .filter_map(|(dx, dy)| {
            try_position(board, target_pos.x as i32 + dx, target_pos.y as i32 + dy)
        })
        .filter(|neighbor| {
            units_on_board
                .get(neighbor)
                .is_some_and(|stats| stats.unit_info.alliance_id == caster.alliance_id)
        })
        .count();
    ally_count >= FLANKING_REQUIRED_ALLIES
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

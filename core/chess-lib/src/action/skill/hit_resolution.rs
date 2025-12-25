//! 命中判定與豁免檢定模組
//!
//! 負責技能命中判定、閃避、格擋、爆擊、豁免檢定等戰鬥計算邏輯

use crate::*;
use rand::Rng;
use skills_lib::*;
use std::collections::BTreeMap;

use super::{
    CRITICAL_FAILURE_THRESHOLD, CRITICAL_SUCCESS_THRESHOLD, MAX_BLOCK_REDUCTION_PERCENT,
    MIN_BLOCK_REDUCTION_PERCENT,
};
use crate::action::skill::effect_application::{
    apply_all_effects, apply_effects_to_empty_tile, apply_effects_with_block,
};

/// 攻擊結果（用於判斷是否爆擊）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackResult {
    NoAttack, // 無攻擊（非傷害技能）
    Normal,   // 普通攻擊
    Critical, // 爆擊
}

/// 豁免檢定結果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveResult {
    NoSave,  // 不需要豁免
    Success, // 豁免成功
    Failure, // 豁免失敗
}

/// 計算技能命中結果（包含閃避、格擋、爆擊判定）
/// 對影響區域內的每個目標進行命中判定，並應用相應效果
pub fn calc_hit_result(
    board: &mut Board,
    caster: (UnitID, Pos),
    skills: &BTreeMap<SkillID, Skill>,
    skill: &Skill,
    affect_area: Vec<Pos>,
    accuracy: i32,
) -> Result<Vec<String>, Error> {
    let func = "calc_hit_result";
    let (caster_id, caster_pos) = caster;

    // 有命中數值，進行命中機制（命中只算一次，閃避/格擋每目標）
    let mut rng = rand::rng();
    let hit_random = rng.random_range(1..=100);
    let hit_score = accuracy + hit_random;

    let mut msgs = vec![];

    for pos in affect_area {
        let unit_id = match board.pos_to_unit(pos) {
            // 無單位，直接套用效果（不需要豁免判定）
            None => {
                msgs.extend(apply_effects_to_empty_tile(board, skill, caster_pos, pos));
                continue;
            }
            Some(unit_id) => unit_id,
        };
        let unit = board
            .units
            .get(&unit_id)
            .ok_or_else(|| Error::InvalidImplementation {
                func,
                detail: "unit not found".to_string(),
            })?;
        let unit_type = unit.unit_template_type.clone();
        let unit_skills = &unit.skills;

        if hit_random <= CRITICAL_FAILURE_THRESHOLD {
            // 完全閃避
            msgs.push(format!(
                "亂數={hit_random} <= {CRITICAL_FAILURE_THRESHOLD}%，單位 {unit_type} 完全閃避了攻擊！"
            ));
            continue;
        }

        // 爆擊判定（決定傷害倍率）
        let attack_result = skill.crit_rate.map_or(AttackResult::NoAttack, |crit_rate| {
            if hit_random > (100 - crit_rate as i32) {
                AttackResult::Critical
            } else {
                AttackResult::Normal
            }
        });
        let crit_msg = if matches!(attack_result, AttackResult::Critical) {
            "爆擊"
        } else {
            "攻擊"
        };

        if hit_random > CRITICAL_SUCCESS_THRESHOLD {
            // 完全命中（套用倍率）
            let effect_msgs = apply_all_effects(
                board,
                skills,
                caster_id,
                unit_id,
                caster_pos,
                pos,
                skill,
                attack_result,
            )
            .wrap_context(func)?;
            for msg in effect_msgs {
                msgs.push(format!(
                    "亂數={hit_random} > {CRITICAL_SUCCESS_THRESHOLD}%，單位 {unit_type} 被完全命中{crit_msg}了：{msg}"
                ));
            }
            continue;
        }

        // 計算閃避值
        let evasion = unit::skills_to_evasion(unit_skills.iter(), skills).wrap_context(func)?;
        // 閃避
        let evade_score = hit_score - evasion;
        if evade_score <= 0 {
            msgs.push(format!(
                "單位 {unit_type} 閃避了{crit_msg}！(accuracy={accuracy}, random={hit_random}, evade={evasion})",
            ));
            continue;
        }

        // 計算格擋值（用於命中判定）
        let block = unit::skills_to_block(unit_skills.iter(), skills).wrap_context(func)?;
        let block_score = hit_score - block - evasion;

        // 計算格擋減傷百分比（用於傷害計算）
        // 格擋減傷百分比：基礎值與最大值由常量定義
        let block_reduction = unit::skills_to_block_reduction(unit_skills.iter(), skills)
            .wrap_context(func)?
            .clamp(MIN_BLOCK_REDUCTION_PERCENT, MAX_BLOCK_REDUCTION_PERCENT);

        // 格擋（百分比減傷）
        if block_score <= 0 {
            let effect_results = apply_effects_with_block(
                board,
                skills,
                caster_id,
                unit_id,
                caster_pos,
                pos,
                skill,
                attack_result,
                block_reduction,
            )
            .wrap_context(func)?;

            for msg in effect_results {
                msgs.push(format!("單位 {unit_type} 格擋{crit_msg}！：{msg}"));
            }
            continue;
        }

        // 完全命中（普通路徑，套用倍率）
        let effect_msgs = apply_all_effects(
            board,
            skills,
            caster_id,
            unit_id,
            caster_pos,
            pos,
            skill,
            attack_result,
        )
        .wrap_context(func)?;
        for msg in effect_msgs {
            msgs.push(format!("單位 {unit_type} 被{crit_msg}了：{msg}"));
        }
    }
    Ok(msgs)
}

/// 計算豁免檢定結果
/// - 只對有單位的目標進行豁免判定
/// - 調用前必須確保目標單位存在
pub fn calc_save_result(
    board: &Board,
    skills: &BTreeMap<SkillID, Skill>,
    caster_id: UnitID,
    target_id: UnitID,
    skill: &Skill,
    effect: &Effect,
) -> Result<SaveResult, Error> {
    let func = "calc_save_result";

    // 檢查效果是否需要豁免判定
    let save_type = match effect.save_type() {
        Some(st) => st,
        None => return Ok(SaveResult::NoSave),
    };

    // 計算施法者的 potency（累加所有 skill tags 的 potency）
    let caster_unit = board
        .units
        .get(&caster_id)
        .ok_or(Error::NoActingUnit {
            func,
            unit_id: caster_id,
        })?;

    let mut caster_potency = 0;
    for tag in &skill.tags {
        caster_potency +=
            unit::skills_to_potency(caster_unit.skills.iter(), skills, tag).wrap_context(func)?;
    }

    // 計算目標的 resistance
    let target = board
        .units
        .get(&target_id)
        .ok_or_else(|| Error::InvalidImplementation {
            func,
            detail: format!(
                "target_id {} from pos_to_unit not found in units",
                target_id
            ),
        })?;

    let target_resistance =
        unit::skills_to_resistance(target.skills.iter(), skills, save_type).wrap_context(func)?;

    // 豁免檢定
    let mut rng = rand::rng();
    let save_score = target_resistance + rng.random_range(1..=100);

    if save_score <= caster_potency {
        Ok(SaveResult::Failure)
    } else {
        Ok(SaveResult::Success)
    }
}

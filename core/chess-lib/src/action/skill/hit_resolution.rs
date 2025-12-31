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

        msgs.extend(process_target_hit(
            board, skills, caster_id, caster_pos, unit_id, pos, skill, hit_random, hit_score,
        )?);
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
    let caster_unit = board.units.get(&caster_id).ok_or(Error::NoActingUnit {
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

use inner::*;
mod inner {
    use super::*;

    /// 判定攻擊結果（普通或爆擊）與訊息文字
    pub(super) fn determine_attack_result(
        skill: &Skill,
        hit_random: i32,
    ) -> (AttackResult, &'static str) {
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
        (attack_result, crit_msg)
    }

    /// 計算夾擊加成（對角夾擊）
    ///
    /// 判定目標是否被夾擊：計算攻擊者相對於目標的方向，檢查目標對面是否有友軍
    /// - 如果目標對面有友軍，視為被夾擊（前後夾攻）
    /// - 回傳施法者的 Flanking 技能加成總和
    pub(super) fn calc_flanking_bonus(
        board: &Board,
        skills: &BTreeMap<SkillID, Skill>,
        caster_id: UnitID,
        caster_pos: Pos,
        target_pos: Pos,
    ) -> Result<i32, Error> {
        let func = "calc_flanking_bonus";

        // 取得施法者資料
        let caster = board.units.get(&caster_id).ok_or(Error::NoActingUnit {
            func,
            unit_id: caster_id,
        })?;
        let caster_team = &caster.team;

        // 計算方向向量（目標相對於攻擊者）
        let dx = target_pos.x as isize - caster_pos.x as isize;
        let dy = target_pos.y as isize - caster_pos.y as isize;

        // 計算目標對面的位置
        let opposite_x = target_pos.x as isize + dx;
        let opposite_y = target_pos.y as isize + dy;

        // 檢查對面位置是否在棋盤範圍內且有友軍
        let opposite_pos = if opposite_x >= 0 && opposite_y >= 0 {
            Pos {
                x: opposite_x as usize,
                y: opposite_y as usize,
            }
        } else {
            return Ok(0);
        };

        // 檢查對面位置是否有友軍
        let opposite_unit = match board.pos_to_unit(opposite_pos) {
            None => return Ok(0),
            Some(unit_id) => board.units.get(&unit_id),
        };
        let opposite_unit = match opposite_unit {
            None => return Ok(0),
            Some(unit) => unit,
        };
        if opposite_unit.team != *caster_team {
            return Ok(0);
        }

        // 對面有友軍，構成夾擊
        unit::skills_to_flanking(caster.skills.iter(), skills).wrap_context(func)
    }

    /// 處理單一目標的命中判定與效果應用
    pub(super) fn process_target_hit(
        board: &mut Board,
        skills: &BTreeMap<SkillID, Skill>,
        caster_id: UnitID,
        caster_pos: Pos,
        unit_id: UnitID,
        pos: Pos,
        skill: &Skill,
        hit_random: i32,
        hit_score: i32,
    ) -> Result<Vec<String>, Error> {
        let func = "process_target_hit";

        let unit = board
            .units
            .get(&unit_id)
            .ok_or(Error::InvalidImplementation {
                func,
                detail: "unit not found".to_string(),
            })?;
        let unit_type = unit.unit_template_type.clone();
        let unit_skills = &unit.skills;

        let mut msgs = vec![];

        // 完全閃避
        if hit_random <= CRITICAL_FAILURE_THRESHOLD {
            msgs.push(format!(
                "亂數={hit_random} <= {CRITICAL_FAILURE_THRESHOLD}%，單位 {unit_type} 完全閃避了攻擊！"
            ));
            return Ok(msgs);
        }

        let (attack_result, crit_msg) = determine_attack_result(skill, hit_random);

        // 完全命中
        if hit_random > CRITICAL_SUCCESS_THRESHOLD {
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
            return Ok(msgs);
        }

        // 計算夾擊加成
        let flanking_bonus =
            calc_flanking_bonus(board, skills, caster_id, caster_pos, pos).wrap_context(func)?;
        let flanking_msg = if flanking_bonus > 0 {
            format!("+夾擊{flanking_bonus}")
        } else {
            String::new()
        };
        let total_hit_score = hit_score + flanking_bonus;

        // 閃避判定（考慮夾擊加成）
        let evasion = unit::skills_to_evasion(unit_skills.iter(), skills).wrap_context(func)?;
        let evade_score = total_hit_score - evasion;
        if evade_score <= 0 {
            msgs.push(format!(
                "單位 {unit_type} 閃避了{crit_msg}！(命中={hit_score}{flanking_msg}, 閃避={evasion})",
            ));
            return Ok(msgs);
        }

        // 格擋判定
        let block = unit::skills_to_block(unit_skills.iter(), skills).wrap_context(func)?;
        let block_score = total_hit_score - block - evasion;

        let block_reduction = unit::skills_to_block_reduction(unit_skills.iter(), skills)
            .wrap_context(func)?
            .clamp(MIN_BLOCK_REDUCTION_PERCENT, MAX_BLOCK_REDUCTION_PERCENT);

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
            return Ok(msgs);
        }

        // 完全命中
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

        Ok(msgs)
    }
}

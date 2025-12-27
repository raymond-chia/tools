//! action/reaction.rs：
//! - Reaction 系統核心邏輯
//! - 負責查找符合條件的 reaction 技能
//! - 檢查觸發條件與次數限制
use crate::*;
use skills_lib::*;
use std::collections::{BTreeMap, BTreeSet};

/// 根據 TriggeredSkill 查找符合條件的技能
///
/// # 參數
/// - `triggered_skill`: 被觸發的技能（SkillId 或 Tag）
/// - `unit_skills`: 單位擁有的技能 ID 集合
/// - `all_skills`: 全局技能表
///
/// # 前提條件
/// - `unit_skills` 中的所有技能都必須存在於 `all_skills` 中（在 Unit::from_template 時已驗證）
///
/// # 返回值
/// - Ok(Vec<SkillID>): 符合條件的技能 ID 列表
/// - Err(Error): 單位不擁有指定的技能
pub fn find_reaction_skills(
    triggered_skill: &TriggeredSkill,
    unit_skills: &BTreeSet<String>,
    all_skills: &BTreeMap<SkillID, Skill>,
) -> Result<Vec<SkillID>, Error> {
    let func = "find_reaction_skills";

    match triggered_skill {
        TriggeredSkill::SkillId { id } => {
            // 檢查單位是否擁有該技能
            if !unit_skills.contains(id) {
                return Err(Error::SkillNotFound {
                    func,
                    skill_id: id.clone(),
                });
            }
            Ok(vec![id.clone()])
        }
        TriggeredSkill::Tag { tag } => {
            // 查找單位擁有的所有具有該 tag 的技能
            let mut matching_skills = Vec::new();
            for skill_id in unit_skills {
                let skill =
                    all_skills
                        .get(skill_id.as_str())
                        .ok_or_else(|| Error::SkillNotFound {
                            func,
                            skill_id: skill_id.clone(),
                        })?;

                if skill.tags.contains(tag) {
                    matching_skills.push(skill_id.clone());
                }
            }

            // 如果沒有找到任何符合 tag 的技能，返回錯誤
            if matching_skills.is_empty() {
                return Err(Error::SkillNotFound {
                    func,
                    skill_id: format!("tag:{:?}", tag),
                });
            }

            Ok(matching_skills)
        }
    }
}

/// 檢查單位是否可以觸發 reaction
///
/// # 參數
/// - `unit`: 要檢查的單位
///
/// # 返回值
/// - Ok(()): 可以觸發（次數未用盡）
/// - Err(Error): 不可觸發（次數已用盡）
pub fn is_able_to_react(unit: &Unit) -> Result<(), Error> {
    let func = "is_able_to_react";

    if unit.reactions_used_this_turn >= unit.max_reactions_per_turn {
        return Err(Error::NotEnoughAP { func });
    }
    Ok(())
}

/// 消耗一次 reaction 次數
///
/// # 參數
/// - `unit`: 要消耗次數的單位
///
/// # 返回值
/// - Ok(()): 成功消耗
/// - Err(Error): 沒有可用的 reaction 次數
///
/// # 使用場景
/// 在執行 reaction 技能後調用此函數來消耗次數
pub fn consume_reaction(unit: &mut Unit) -> Result<(), Error> {
    let func = "consume_reaction";

    is_able_to_react(unit).wrap_context(func)?;

    unit.reactions_used_this_turn += 1;
    Ok(())
}

/// 簡化的 reaction 資訊（單一單位）
#[derive(Debug, Clone)]
pub struct ReactionInfo {
    pub triggered_skill: TriggeredSkill, // 被觸發的技能來源
    pub available_skills: Vec<SkillID>,  // 可用的技能列表
}

/// 完整的 pending reaction（包含觸發者資訊）
#[derive(Debug, Clone)]
pub struct PendingReaction {
    pub reactor_id: UnitID,       // 觸發 reaction 的單位 ID
    pub reactor_pos: Pos,         // 觸發 reaction 的單位位置
    pub trigger: ReactionTrigger, // 觸發條件類型
    pub info: ReactionInfo,       // reaction 詳細資訊
}

/// 檢查單一單位的 reactions（公開供測試使用）
///
/// # 參數
/// - `unit`: 要檢測的單位
/// - `trigger_type`: 觸發條件類型（OnMove 或 OnAttacked）
/// - `all_skills`: 全局技能表
///
/// # 返回值
/// - 可觸發的 reactions 列表（可能為空）
///
/// # 實作說明
/// - 直接從 unit.skills 檢查被動技能中的 Effect::Reaction
/// - 暫不支援從 status_effects 檢查（buff 產生的臨時 Reaction）
pub fn check_unit_reactions(
    unit: &Unit,
    trigger_type: ReactionTrigger,
    all_skills: &BTreeMap<SkillID, Skill>,
) -> Result<Vec<ReactionInfo>, Error> {
    let func = "check_unit_reactions";

    // 檢查是否還有可用的 reaction 次數
    if is_able_to_react(unit).is_err() {
        return Ok(Vec::new());
    }

    let mut reactions = Vec::new();

    // 遍歷單位擁有的所有技能，檢查是否有匹配的 Reaction effect
    for skill_id in &unit.skills {
        let skill = all_skills
            .get(skill_id.as_str())
            .ok_or_else(|| Error::SkillNotFound {
                func,
                skill_id: skill_id.clone(),
            })?;

        // 檢查技能的所有效果
        for effect in &skill.effects {
            if let Effect::Reaction {
                trigger,
                triggered_skill,
                ..
            } = effect
            {
                // 只處理匹配的觸發類型
                if *trigger != trigger_type {
                    continue;
                }

                // 查找可用的技能
                let available_skills =
                    find_reaction_skills(triggered_skill, &unit.skills, all_skills)
                        .map_err(|e| match e {
                            Error::SkillNotFound { skill_id, func } => Error::SkillNotFoundInUnit {
                                func, // 保留原始 func name
                                unit_id: unit.id,
                                unit_type: unit.unit_template_type.clone(),
                                skill_id,
                            },
                            other => other,
                        })
                        .wrap_context(func)?;

                reactions.push(ReactionInfo {
                    triggered_skill: triggered_skill.clone(),
                    available_skills,
                });
            }
        }
    }

    Ok(reactions)
}

/// 檢查多個單位的 reactions（公開 API）
///
/// # 參數
/// - `units`: 要檢查的單位列表（單位引用 + 位置）
/// - `trigger_type`: 觸發條件類型（OnMove 或 OnAttacked）
/// - `all_skills`: 全局技能表
///
/// # 返回值
/// - 所有可觸發的 reactions 列表（可能為空）
pub fn check_reactions(
    units: &[(&Unit, Pos)],
    trigger_type: ReactionTrigger,
    all_skills: &BTreeMap<SkillID, Skill>,
) -> Result<Vec<PendingReaction>, Error> {
    let mut all_pending = Vec::new();

    for &(unit, pos) in units {
        // 檢查該單位的 reactions
        let reactions = check_unit_reactions(unit, trigger_type.clone(), all_skills)?;

        // 將 ReactionInfo 包裝成 PendingReaction
        for info in reactions {
            all_pending.push(PendingReaction {
                reactor_id: unit.id,
                reactor_pos: pos,
                trigger: trigger_type.clone(),
                info,
            });
        }
    }

    Ok(all_pending)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_reaction_skills_by_id() {
        let mut all_skills = BTreeMap::new();
        let skill = Skill::default();
        all_skills.insert("basic_attack".to_string(), skill);

        let mut unit_skills = BTreeSet::new();
        unit_skills.insert("basic_attack".to_string());

        let source = TriggeredSkill::SkillId {
            id: "basic_attack".to_string(),
        };

        let result = find_reaction_skills(&source, &unit_skills, &all_skills).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "basic_attack");
    }

    #[test]
    fn test_find_reaction_skills_by_id_not_owned() {
        let mut all_skills = BTreeMap::new();
        let skill = Skill::default();
        all_skills.insert("basic_attack".to_string(), skill);

        let unit_skills = BTreeSet::new(); // 單位沒有該技能

        let source = TriggeredSkill::SkillId {
            id: "basic_attack".to_string(),
        };

        let result = find_reaction_skills(&source, &unit_skills, &all_skills);
        match result {
            Err(Error::SkillNotFound { skill_id, .. }) => {
                assert_eq!(skill_id, "basic_attack");
            }
            _ => panic!("應該返回 SkillNotFound 錯誤"),
        }
    }

    #[test]
    fn test_find_reaction_skills_by_tag() {
        let mut all_skills = BTreeMap::new();

        let mut skill1 = Skill::default();
        skill1.tags = vec![Tag::Physical].into_iter().collect();
        all_skills.insert("basic_attack".to_string(), skill1);

        let mut skill2 = Skill::default();
        skill2.tags = vec![Tag::Physical].into_iter().collect();
        all_skills.insert("heavy_strike".to_string(), skill2);

        let mut skill3 = Skill::default();
        skill3.tags = vec![Tag::Fire].into_iter().collect();
        all_skills.insert("fireball".to_string(), skill3);

        let mut unit_skills = BTreeSet::new();
        unit_skills.insert("basic_attack".to_string());
        unit_skills.insert("heavy_strike".to_string());
        unit_skills.insert("fireball".to_string());

        let source = TriggeredSkill::Tag { tag: Tag::Physical };

        let result = find_reaction_skills(&source, &unit_skills, &all_skills).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.contains(&"basic_attack".to_string()));
        assert!(result.contains(&"heavy_strike".to_string()));
    }

    #[test]
    fn test_find_reaction_skills_by_tag_no_match() {
        let mut all_skills = BTreeMap::new();

        let mut skill1 = Skill::default();
        skill1.tags = vec![Tag::Fire].into_iter().collect();
        all_skills.insert("fireball".to_string(), skill1);

        let mut unit_skills = BTreeSet::new();
        unit_skills.insert("fireball".to_string());

        let source = TriggeredSkill::Tag { tag: Tag::Physical };

        let result = find_reaction_skills(&source, &unit_skills, &all_skills);
        match result {
            Err(Error::SkillNotFound { skill_id, .. }) => {
                assert!(skill_id.contains("Physical"));
            }
            _ => panic!("應該返回 SkillNotFound 錯誤"),
        }
    }

    #[test]
    fn test_is_able_to_react() {
        let unit = Unit {
            id: 1,
            unit_template_type: "test".to_string(),
            team: "t1".to_string(),
            moved: 0,
            move_points: 10,
            has_cast_skill_this_turn: false,
            hp: 100,
            max_hp: 100,
            mp: 50,
            max_mp: 50,
            skills: BTreeSet::new(),
            status_effects: Vec::new(),
            max_reactions_per_turn: 1,
            reactions_used_this_turn: 0,
        };

        assert!(is_able_to_react(&unit).is_ok());
    }

    #[test]
    fn test_is_able_to_react_exhausted() {
        let unit = Unit {
            id: 1,
            unit_template_type: "test".to_string(),
            team: "t1".to_string(),
            moved: 0,
            move_points: 10,
            has_cast_skill_this_turn: false,
            hp: 100,
            max_hp: 100,
            mp: 50,
            max_mp: 50,
            skills: BTreeSet::new(),
            status_effects: Vec::new(),
            max_reactions_per_turn: 1,
            reactions_used_this_turn: 1, // 已用盡
        };

        assert!(is_able_to_react(&unit).is_err());
    }

    #[test]
    fn test_consume_reaction_success() {
        let mut unit = Unit {
            id: 1,
            unit_template_type: "test".to_string(),
            team: "t1".to_string(),
            moved: 0,
            move_points: 10,
            has_cast_skill_this_turn: false,
            hp: 100,
            max_hp: 100,
            mp: 50,
            max_mp: 50,
            skills: BTreeSet::new(),
            status_effects: Vec::new(),
            max_reactions_per_turn: 2,
            reactions_used_this_turn: 0,
        };

        // 第一次消耗
        assert!(consume_reaction(&mut unit).is_ok());
        assert_eq!(unit.reactions_used_this_turn, 1);

        // 第二次消耗
        assert!(consume_reaction(&mut unit).is_ok());
        assert_eq!(unit.reactions_used_this_turn, 2);
    }

    #[test]
    fn test_consume_reaction_exhausted() {
        let mut unit = Unit {
            id: 1,
            unit_template_type: "test".to_string(),
            team: "t1".to_string(),
            moved: 0,
            move_points: 10,
            has_cast_skill_this_turn: false,
            hp: 100,
            max_hp: 100,
            mp: 50,
            max_mp: 50,
            skills: BTreeSet::new(),
            status_effects: Vec::new(),
            max_reactions_per_turn: 1,
            reactions_used_this_turn: 1, // 已用盡
        };

        // 應該返回錯誤
        let result = consume_reaction(&mut unit);
        assert!(result.is_err());
        // 次數不應該增加
        assert_eq!(unit.reactions_used_this_turn, 1);
    }

    #[test]
    fn test_consume_reaction_until_exhausted() {
        let mut unit = Unit {
            id: 1,
            unit_template_type: "test".to_string(),
            team: "t1".to_string(),
            moved: 0,
            move_points: 10,
            has_cast_skill_this_turn: false,
            hp: 100,
            max_hp: 100,
            mp: 50,
            max_mp: 50,
            skills: BTreeSet::new(),
            status_effects: Vec::new(),
            max_reactions_per_turn: 3,
            reactions_used_this_turn: 0,
        };

        // 消耗 3 次應該成功
        for i in 0..3 {
            assert!(consume_reaction(&mut unit).is_ok());
            assert_eq!(unit.reactions_used_this_turn, i + 1);
        }

        // 第 4 次應該失敗
        assert!(consume_reaction(&mut unit).is_err());
        assert_eq!(unit.reactions_used_this_turn, 3);
    }
}

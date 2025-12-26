//! action/reaction.rs：
//! - Reaction 系統核心邏輯
//! - 負責查找符合條件的 reaction 技能
//! - 檢查觸發條件與次數限制
use crate::*;
use skills_lib::*;
use std::collections::{BTreeMap,BTreeSet};

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
                let skill = all_skills.get(skill_id.as_str()).ok_or_else(|| {
                    Error::SkillNotFound {
                        func,
                        skill_id: skill_id.clone(),
                    }
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
/// - true: 可以觸發（次數未用盡）
/// - false: 不可觸發（次數已用盡）
pub fn can_trigger_reaction(unit: &Unit) -> bool {
    unit.reactions_used_this_turn < unit.max_reactions_per_turn
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
    fn test_can_trigger_reaction() {
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

        assert!(can_trigger_reaction(&unit));
    }

    #[test]
    fn test_can_trigger_reaction_exhausted() {
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

        assert!(!can_trigger_reaction(&unit));
    }
}

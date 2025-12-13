//! 技能系統
//!
//! 實作 PF2e 技能檢定與熟練度

use crate::{abilities::Ability, dice::roll_d20, ProficiencyRank};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// PF2e 的技能
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Skill {
    // 力量為基礎
    Athletics,

    // 敏捷為基礎
    Acrobatics,
    Stealth,
    Thievery,

    // 智力為基礎
    Arcana,
    Crafting,
    Nature,
    Occultism,
    Religion,
    Society,

    // 感知為基礎
    Medicine,
    Perception,
    Survival,

    // 魅力為基礎
    Deception,
    Diplomacy,
    Intimidation,
    Performance,

    // 學識技能可以動態定義
    Lore(u32), // 暫時使用 u32 作為佔位符
}

impl Skill {
    /// 取得此技能的關鍵屬性
    pub fn key_ability(&self) -> Ability {
        match self {
            Self::Athletics => Ability::Strength,

            Self::Acrobatics | Self::Stealth | Self::Thievery => Ability::Dexterity,

            Self::Arcana
            | Self::Crafting
            | Self::Nature
            | Self::Occultism
            | Self::Religion
            | Self::Society
            | Self::Lore(_) => Ability::Intelligence,

            Self::Medicine | Self::Perception | Self::Survival => Ability::Wisdom,

            Self::Deception | Self::Diplomacy | Self::Intimidation | Self::Performance => {
                Ability::Charisma
            }
        }
    }

    /// 取得技能名稱
    pub fn name(&self) -> String {
        match self {
            Self::Athletics => "Athletics".to_string(),
            Self::Acrobatics => "Acrobatics".to_string(),
            Self::Stealth => "Stealth".to_string(),
            Self::Thievery => "Thievery".to_string(),
            Self::Arcana => "Arcana".to_string(),
            Self::Crafting => "Crafting".to_string(),
            Self::Nature => "Nature".to_string(),
            Self::Occultism => "Occultism".to_string(),
            Self::Religion => "Religion".to_string(),
            Self::Society => "Society".to_string(),
            Self::Medicine => "Medicine".to_string(),
            Self::Perception => "Perception".to_string(),
            Self::Survival => "Survival".to_string(),
            Self::Deception => "Deception".to_string(),
            Self::Diplomacy => "Diplomacy".to_string(),
            Self::Intimidation => "Intimidation".to_string(),
            Self::Performance => "Performance".to_string(),
            Self::Lore(id) => format!("Lore ({})", id),
        }
    }

    /// 取得所有標準技能（不包括學識）
    pub fn standard_skills() -> Vec<Skill> {
        vec![
            Self::Athletics,
            Self::Acrobatics,
            Self::Stealth,
            Self::Thievery,
            Self::Arcana,
            Self::Crafting,
            Self::Nature,
            Self::Occultism,
            Self::Religion,
            Self::Society,
            Self::Medicine,
            Self::Perception,
            Self::Survival,
            Self::Deception,
            Self::Diplomacy,
            Self::Intimidation,
            Self::Performance,
        ]
    }
}

/// 技能熟練度（技能 + 熟練等級）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillProficiency {
    pub skill: Skill,
    pub rank: ProficiencyRank,
    /// 物品加值
    pub item_bonus: i32,
    /// 狀態加值
    pub status_bonus: i32,
    /// 環境加值
    pub circumstance_bonus: i32,
}

impl SkillProficiency {
    pub fn new(skill: Skill, rank: ProficiencyRank) -> Self {
        Self {
            skill,
            rank,
            item_bonus: 0,
            status_bonus: 0,
            circumstance_bonus: 0,
        }
    }

    /// 計算技能檢定的總加值
    ///
    /// 公式：屬性調整值 + 等級（如果受訓）+ 熟練加值 + 物品加值 + 狀態加值 + 環境加值
    pub fn total_bonus(&self, ability_modifier: i32, level: i32) -> i32 {
        let proficiency_bonus = if self.rank == ProficiencyRank::Untrained {
            0
        } else {
            level + self.rank.bonus()
        };

        ability_modifier
            + proficiency_bonus
            + self.item_bonus
            + self.status_bonus
            + self.circumstance_bonus
    }
}

/// 技能集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skills {
    proficiencies: HashMap<String, SkillProficiency>,
}

impl Default for Skills {
    fn default() -> Self {
        Self {
            proficiencies: HashMap::new(),
        }
    }
}

impl Skills {
    pub fn new() -> Self {
        Self::default()
    }

    /// 設定技能熟練度
    pub fn set_proficiency(&mut self, skill: Skill, rank: ProficiencyRank) {
        let key = format!("{:?}", skill);
        self.proficiencies
            .insert(key, SkillProficiency::new(skill, rank));
    }

    /// 取得技能熟練等級
    pub fn get_proficiency(&self, skill: &Skill) -> ProficiencyRank {
        let key = format!("{:?}", skill);
        self.proficiencies
            .get(&key)
            .map(|p| p.rank)
            .unwrap_or(ProficiencyRank::Untrained)
    }

    /// 進行技能檢定
    pub fn make_check(
        &self,
        skill: Skill,
        ability_modifier: i32,
        level: i32,
    ) -> crate::dice::DiceRoll {
        let key = format!("{:?}", skill);
        let bonus = if let Some(prof) = self.proficiencies.get(&key) {
            prof.total_bonus(ability_modifier, level)
        } else {
            ability_modifier // 未受訓僅使用屬性調整值
        };

        roll_d20(bonus)
    }

    /// 取得所有熟練度
    pub fn all_proficiencies(&self) -> Vec<&SkillProficiency> {
        self.proficiencies.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_key_ability() {
        assert_eq!(Skill::Athletics.key_ability(), Ability::Strength);
        assert_eq!(Skill::Acrobatics.key_ability(), Ability::Dexterity);
        assert_eq!(Skill::Arcana.key_ability(), Ability::Intelligence);
        assert_eq!(Skill::Medicine.key_ability(), Ability::Wisdom);
        assert_eq!(Skill::Deception.key_ability(), Ability::Charisma);
    }

    #[test]
    fn test_skill_proficiency_bonus() {
        let prof = SkillProficiency::new(Skill::Athletics, ProficiencyRank::Trained);
        // 1 級角色，力量調整值 +3
        assert_eq!(prof.total_bonus(3, 1), 6); // 3（屬性）+ 1（等級）+ 2（受訓）= 6

        let prof = SkillProficiency::new(Skill::Athletics, ProficiencyRank::Expert);
        assert_eq!(prof.total_bonus(3, 5), 12); // 3 + 5 + 4 = 12
    }

    #[test]
    fn test_skills_collection() {
        let mut skills = Skills::new();
        skills.set_proficiency(Skill::Athletics, ProficiencyRank::Expert);
        skills.set_proficiency(Skill::Stealth, ProficiencyRank::Trained);

        assert_eq!(
            skills.get_proficiency(&Skill::Athletics),
            ProficiencyRank::Expert
        );
        assert_eq!(
            skills.get_proficiency(&Skill::Stealth),
            ProficiencyRank::Trained
        );
        assert_eq!(
            skills.get_proficiency(&Skill::Arcana),
            ProficiencyRank::Untrained
        );
    }
}

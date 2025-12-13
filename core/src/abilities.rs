//! 屬性值系統
//!
//! 實作 PF2e 的六項屬性值(力量、敏捷、體質、智力、睿智、魅力)

use crate::GameError;
use serde::{Deserialize, Serialize};

/// PF2e 的六項屬性值
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Ability {
    /// 力量 - 影響近戰攻擊和負重能力
    Strength,
    /// 敏捷 - 影響 AC、反射豁免和遠程攻擊
    Dexterity,
    /// 體質 - 影響 HP 和堅韌豁免
    Constitution,
    /// 智力 - 影響法術 DC 和攻擊加成
    Intelligence,
    /// 睿智 - 影響察覺和意志豁免
    Wisdom,
    /// 魅力 - 影響法術 DC 和攻擊加成
    Charisma,
}

impl Ability {
    /// 獲取屬性的縮寫名稱
    pub fn abbreviation(&self) -> String {
        // 使用 enum 名稱自動生成鍵：Strength -> "abilities.Strength_abbr"
        let key = format!("abilities.{:?}_abbr", self);
        crate::i18n::t(&key)
    }

    /// 獲取屬性的完整名稱
    pub fn full_name(&self) -> String {
        // 使用 enum 名稱自動生成鍵：Strength -> "abilities.Strength"
        let key = format!("abilities.{:?}", self);
        crate::i18n::t(&key)
    }

    /// 獲取所有屬性
    pub fn all() -> [Ability; 6] {
        [
            Self::Strength,
            Self::Dexterity,
            Self::Constitution,
            Self::Intelligence,
            Self::Wisdom,
            Self::Charisma,
        ]
    }
}

/// 角色的屬性值
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbilityScores {
    pub strength: i32,
    pub dexterity: i32,
    pub constitution: i32,
    pub intelligence: i32,
    pub wisdom: i32,
    pub charisma: i32,
}

impl Default for AbilityScores {
    fn default() -> Self {
        Self {
            strength: 10,
            dexterity: 10,
            constitution: 10,
            intelligence: 10,
            wisdom: 10,
            charisma: 10,
        }
    }
}

impl AbilityScores {
    /// 創建新的屬性值
    pub fn new(
        str: i32,
        dex: i32,
        con: i32,
        int: i32,
        wis: i32,
        cha: i32,
    ) -> Result<Self, GameError> {
        // 在 PF2e 中，屬性值通常在 1 到 30 之間
        for &score in &[str, dex, con, int, wis, cha] {
            if !(1..=30).contains(&score) {
                return Err(GameError::InvalidAbilityScore(score));
            }
        }

        Ok(Self {
            strength: str,
            dexterity: dex,
            constitution: con,
            intelligence: int,
            wisdom: wis,
            charisma: cha,
        })
    }

    /// 獲取特定屬性值
    pub fn get(&self, ability: Ability) -> i32 {
        match ability {
            Ability::Strength => self.strength,
            Ability::Dexterity => self.dexterity,
            Ability::Constitution => self.constitution,
            Ability::Intelligence => self.intelligence,
            Ability::Wisdom => self.wisdom,
            Ability::Charisma => self.charisma,
        }
    }

    /// 設定特定屬性值
    pub fn set(&mut self, ability: Ability, value: i32) -> Result<(), GameError> {
        if !(1..=30).contains(&value) {
            return Err(GameError::InvalidAbilityScore(value));
        }

        match ability {
            Ability::Strength => self.strength = value,
            Ability::Dexterity => self.dexterity = value,
            Ability::Constitution => self.constitution = value,
            Ability::Intelligence => self.intelligence = value,
            Ability::Wisdom => self.wisdom = value,
            Ability::Charisma => self.charisma = value,
        }

        Ok(())
    }

    /// 計算屬性調整值
    ///
    /// PF2e 規則：調整值 = (屬性分數 - 10) / 2（無條件捨去）
    pub fn modifier(&self, ability: Ability) -> i32 {
        let score = self.get(ability);
        (score - 10) / 2
    }

    /// 套用屬性提升
    ///
    /// PF2e 規則：
    /// - 如果屬性分數 < 18，增加 2
    /// - 如果屬性分數 >= 18，增加 1
    pub fn apply_boost(&mut self, ability: Ability) -> Result<(), GameError> {
        let current = self.get(ability);
        let new_value = if current < 18 {
            current + 2
        } else {
            current + 1
        };

        self.set(ability, new_value)
    }

    /// 應用屬性缺陷（降低）
    pub fn apply_flaw(&mut self, ability: Ability) -> Result<(), GameError> {
        let current = self.get(ability);
        let new_value = current - 2;
        self.set(ability, new_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ability_modifier() {
        let scores = AbilityScores::default();
        // 屬性 10 的調整值應該是 0
        assert_eq!(scores.modifier(Ability::Strength), 0);

        let scores = AbilityScores::new(18, 14, 12, 10, 8, 16).unwrap();
        assert_eq!(scores.modifier(Ability::Strength), 4); // (18-10)/2 = 4
        assert_eq!(scores.modifier(Ability::Dexterity), 2); // (14-10)/2 = 2
        assert_eq!(scores.modifier(Ability::Constitution), 1); // (12-10)/2 = 1
        assert_eq!(scores.modifier(Ability::Intelligence), 0); // (10-10)/2 = 0
        assert_eq!(scores.modifier(Ability::Wisdom), -1); // (8-10)/2 = -1
        assert_eq!(scores.modifier(Ability::Charisma), 3); // (16-10)/2 = 3
    }

    #[test]
    fn test_ability_boost() {
        let mut scores = AbilityScores::default();
        scores.apply_boost(Ability::Strength).unwrap();
        assert_eq!(scores.strength, 12); // 10 -> 12

        scores.strength = 18;
        scores.apply_boost(Ability::Strength).unwrap();
        assert_eq!(scores.strength, 19); // 18 -> 19 (>=18 only adds 1)
    }

    #[test]
    fn test_ability_flaw() {
        let mut scores = AbilityScores::default();
        scores.apply_flaw(Ability::Strength).unwrap();
        assert_eq!(scores.strength, 8); // 10 -> 8
    }

    #[test]
    fn test_invalid_ability_score() {
        let result = AbilityScores::new(0, 10, 10, 10, 10, 10);
        assert!(result.is_err());

        let result = AbilityScores::new(31, 10, 10, 10, 10, 10);
        assert!(result.is_err());
    }
}

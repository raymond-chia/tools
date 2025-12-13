//! 角色系統
//!
//! 實作 PF2e 角色、血統和職業

use crate::{
    abilities::{Ability, AbilityScores},
    skills::Skills,
    traits::TraitSet,
    GameError, ProficiencyRank,
};
use serde::{Deserialize, Serialize};

/// 血統
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ancestry {
    Human,
    Elf,
    Dwarf,
    Gnome,
    Goblin,
    Halfling,
    Orc,
    Custom(String),
}

impl Ancestry {
    /// 獲取血統名稱
    pub fn name(&self) -> String {
        match self {
            Self::Human => "Human".to_string(),
            Self::Elf => "Elf".to_string(),
            Self::Dwarf => "Dwarf".to_string(),
            Self::Gnome => "Gnome".to_string(),
            Self::Goblin => "Goblin".to_string(),
            Self::Halfling => "Halfling".to_string(),
            Self::Orc => "Orc".to_string(),
            Self::Custom(name) => name.clone(),
        }
    }

    /// 獲取血統基礎 HP
    pub fn base_hp(&self) -> i32 {
        match self {
            Self::Human => 8,
            Self::Elf => 6,
            Self::Dwarf => 10,
            Self::Gnome => 8,
            Self::Goblin => 6,
            Self::Halfling => 6,
            Self::Orc => 10,
            Self::Custom(_) => 8,
        }
    }

    /// 獲取速度（英尺）
    pub fn speed(&self) -> i32 {
        match self {
            Self::Human => 25,
            Self::Elf => 30,
            Self::Dwarf => 20,
            Self::Gnome => 25,
            Self::Goblin => 25,
            Self::Halfling => 25,
            Self::Orc => 25,
            Self::Custom(_) => 25,
        }
    }

    /// 獲取體型
    pub fn size(&self) -> Size {
        match self {
            Self::Human | Self::Elf | Self::Orc => Size::Medium,
            Self::Dwarf | Self::Gnome | Self::Goblin | Self::Halfling => Size::Small,
            Self::Custom(_) => Size::Medium,
        }
    }
}

/// 角色職業
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CharacterClass {
    Fighter,
    Wizard,
    Cleric,
    Rogue,
    Ranger,
    Barbarian,
    Bard,
    Champion,
    Druid,
    Monk,
    Sorcerer,
    Alchemist,
    Custom(String),
}

impl CharacterClass {
    /// 獲取職業名稱
    pub fn name(&self) -> String {
        match self {
            Self::Fighter => "Fighter".to_string(),
            Self::Wizard => "Wizard".to_string(),
            Self::Cleric => "Cleric".to_string(),
            Self::Rogue => "Rogue".to_string(),
            Self::Ranger => "Ranger".to_string(),
            Self::Barbarian => "Barbarian".to_string(),
            Self::Bard => "Bard".to_string(),
            Self::Champion => "Champion".to_string(),
            Self::Druid => "Druid".to_string(),
            Self::Monk => "Monk".to_string(),
            Self::Sorcerer => "Sorcerer".to_string(),
            Self::Alchemist => "Alchemist".to_string(),
            Self::Custom(name) => name.clone(),
        }
    }

    /// 獲取職業關鍵屬性
    pub fn key_ability(&self) -> Ability {
        match self {
            Self::Fighter | Self::Barbarian | Self::Champion => Ability::Strength,
            Self::Rogue | Self::Ranger | Self::Monk => Ability::Dexterity,
            Self::Wizard | Self::Alchemist => Ability::Intelligence,
            Self::Cleric | Self::Druid => Ability::Wisdom,
            Self::Bard | Self::Sorcerer => Ability::Charisma,
            Self::Custom(_) => Ability::Strength,
        }
    }

    /// 獲取職業每級 HP
    pub fn hp_per_level(&self) -> i32 {
        match self {
            Self::Barbarian => 12,
            Self::Fighter | Self::Champion | Self::Ranger => 10,
            Self::Monk | Self::Rogue | Self::Cleric | Self::Druid => 8,
            Self::Bard | Self::Alchemist => 8,
            Self::Wizard | Self::Sorcerer => 6,
            Self::Custom(_) => 8,
        }
    }

    /// 獲取初始察覺熟練度
    pub fn perception_proficiency(&self) -> ProficiencyRank {
        match self {
            Self::Ranger | Self::Rogue => ProficiencyRank::Expert,
            _ => ProficiencyRank::Trained,
        }
    }
}

/// 體型類別
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Size {
    Tiny,
    Small,
    Medium,
    Large,
    Huge,
    Gargantuan,
}

impl Size {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Tiny => "Tiny",
            Self::Small => "Small",
            Self::Medium => "Medium",
            Self::Large => "Large",
            Self::Huge => "Huge",
            Self::Gargantuan => "Gargantuan",
        }
    }
}

/// 角色
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub name: String,
    pub ancestry: Ancestry,
    pub class: CharacterClass,
    pub level: i32,
    pub experience_points: i32,

    /// 屬性值
    pub ability_scores: AbilityScores,

    /// 技能
    pub skills: Skills,

    /// 當前 HP
    pub current_hp: i32,
    /// 最大 HP
    pub max_hp: i32,
    /// 臨時 HP
    pub temp_hp: i32,

    /// 防禦等級 (AC)
    pub armor_class: i32,

    /// 豁免熟練度
    pub fortitude_proficiency: ProficiencyRank,
    pub reflex_proficiency: ProficiencyRank,
    pub will_proficiency: ProficiencyRank,

    /// 察覺熟練度
    pub perception_proficiency: ProficiencyRank,

    /// 特性
    pub traits: TraitSet,
}

impl Character {
    /// 創建新角色
    pub fn new(
        name: String,
        ancestry: Ancestry,
        class: CharacterClass,
        ability_scores: AbilityScores,
    ) -> Result<Self, GameError> {
        let level = 1;

        // 計算最大 HP
        let ancestry_hp = ancestry.base_hp();
        let class_hp = class.hp_per_level();
        let con_modifier = ability_scores.modifier(Ability::Constitution);
        let max_hp = ancestry_hp + class_hp + con_modifier;

        Ok(Self {
            name,
            ancestry,
            class,
            level,
            experience_points: 0,
            ability_scores,
            skills: Skills::new(),
            current_hp: max_hp,
            max_hp,
            temp_hp: 0,
            armor_class: 10,
            fortitude_proficiency: ProficiencyRank::Trained,
            reflex_proficiency: ProficiencyRank::Trained,
            will_proficiency: ProficiencyRank::Trained,
            perception_proficiency: ProficiencyRank::Trained,
            traits: TraitSet::new(),
        })
    }

    /// 升級
    pub fn level_up(&mut self) -> Result<(), GameError> {
        if self.level >= 20 {
            return Err(GameError::InvalidLevel(self.level + 1));
        }

        self.level += 1;

        // 增加 HP
        let class_hp = self.class.hp_per_level();
        let con_modifier = self.ability_scores.modifier(Ability::Constitution);
        let hp_gain = class_hp + con_modifier;

        self.max_hp += hp_gain;
        self.current_hp += hp_gain;

        Ok(())
    }

    /// 承受傷害
    pub fn take_damage(&mut self, damage: i32) {
        // 先扣除臨時 HP
        if self.temp_hp > 0 {
            if damage <= self.temp_hp {
                self.temp_hp -= damage;
                return;
            } else {
                let remaining_damage = damage - self.temp_hp;
                self.temp_hp = 0;
                self.current_hp = (self.current_hp - remaining_damage).max(0);
            }
        } else {
            self.current_hp = (self.current_hp - damage).max(0);
        }
    }

    /// 治療
    pub fn heal(&mut self, amount: i32) {
        self.current_hp = (self.current_hp + amount).min(self.max_hp);
    }

    /// 檢查是否存活
    pub fn is_alive(&self) -> bool {
        self.current_hp > 0
    }

    /// 檢查是否瀕死
    pub fn is_dying(&self) -> bool {
        self.current_hp == 0
    }

    /// 計算察覺檢定加成
    pub fn perception_bonus(&self) -> i32 {
        let wis_mod = self.ability_scores.modifier(Ability::Wisdom);
        let prof_bonus = if self.perception_proficiency == ProficiencyRank::Untrained {
            0
        } else {
            self.level + self.perception_proficiency.bonus()
        };

        wis_mod + prof_bonus
    }

    /// 計算豁免加成
    pub fn save_bonus(&self, save_type: SaveType) -> i32 {
        let (ability, proficiency) = match save_type {
            SaveType::Fortitude => (Ability::Constitution, self.fortitude_proficiency),
            SaveType::Reflex => (Ability::Dexterity, self.reflex_proficiency),
            SaveType::Will => (Ability::Wisdom, self.will_proficiency),
        };

        let ability_mod = self.ability_scores.modifier(ability);
        let prof_bonus = if proficiency == ProficiencyRank::Untrained {
            0
        } else {
            self.level + proficiency.bonus()
        };

        ability_mod + prof_bonus
    }
}

/// 豁免類型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SaveType {
    Fortitude,
    Reflex,
    Will,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_character_creation() {
        let abilities = AbilityScores::new(16, 14, 12, 10, 13, 8).unwrap();
        let character = Character::new(
            "Test Fighter".to_string(),
            Ancestry::Human,
            CharacterClass::Fighter,
            abilities,
        )
        .unwrap();

        assert_eq!(character.level, 1);
        assert_eq!(character.name, "Test Fighter");
        // Human 8 + Fighter 10 + Con modifier 1 = 19
        assert_eq!(character.max_hp, 19);
    }

    #[test]
    fn test_level_up() {
        let abilities = AbilityScores::default();
        let mut character = Character::new(
            "Test".to_string(),
            Ancestry::Human,
            CharacterClass::Fighter,
            abilities,
        )
        .unwrap();

        let old_max_hp = character.max_hp;
        character.level_up().unwrap();

        assert_eq!(character.level, 2);
        assert!(character.max_hp > old_max_hp);
    }

    #[test]
    fn test_damage_and_healing() {
        let abilities = AbilityScores::default();
        let mut character = Character::new(
            "Test".to_string(),
            Ancestry::Human,
            CharacterClass::Fighter,
            abilities,
        )
        .unwrap();

        let max_hp = character.max_hp;
        character.take_damage(5);
        assert_eq!(character.current_hp, max_hp - 5);

        character.heal(3);
        assert_eq!(character.current_hp, max_hp - 2);

        character.heal(10);
        assert_eq!(character.current_hp, max_hp); // Cannot exceed max
    }
}

//! 特性系統
//!
//! PF2e 中的特性是用於能力、法術、生物等的描述性標籤

use serde::{Deserialize, Serialize};

/// 遊戲特性標籤
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Trait {
    // 動作特性
    Attack,
    Concentrate,
    Manipulate,
    Move,

    // 傷害類型
    Fire,
    Cold,
    Electricity,
    Acid,
    Sonic,
    Positive,
    Negative,
    Force,
    Mental,
    Poison,

    // 魔法學派
    Abjuration,
    Conjuration,
    Divination,
    Enchantment,
    Evocation,
    Illusion,
    Necromancy,
    Transmutation,

    // 種族特性
    Human,
    Elf,
    Dwarf,
    Gnome,
    Goblin,
    Halfling,
    Orc,

    // 武器特性
    Deadly,
    Fatal,
    Finesse,
    Agile,
    Reach,
    Thrown,
    Versatile,

    // 稀有度
    Common,
    Uncommon,
    Rare,
    Unique,

    // 陣營
    Lawful,
    Chaotic,
    Good,
    Evil,

    // 生物類型
    Aberration,
    Animal,
    Beast,
    Celestial,
    Construct,
    Dragon,
    Elemental,
    Fey,
    Fiend,
    Fungus,
    Humanoid,
    Ooze,
    Plant,
    Spirit,
    Undead,

    // 自訂特性
    Custom(String),
}

impl Trait {
    /// 取得特性的顯示名稱
    pub fn display_name(&self) -> String {
        match self {
            Self::Attack => "Attack".to_string(),
            Self::Concentrate => "Concentrate".to_string(),
            Self::Manipulate => "Manipulate".to_string(),
            Self::Move => "Move".to_string(),

            Self::Fire => "Fire".to_string(),
            Self::Cold => "Cold".to_string(),
            Self::Electricity => "Electricity".to_string(),
            Self::Acid => "Acid".to_string(),
            Self::Sonic => "Sonic".to_string(),
            Self::Positive => "Positive".to_string(),
            Self::Negative => "Negative".to_string(),
            Self::Force => "Force".to_string(),
            Self::Mental => "Mental".to_string(),
            Self::Poison => "Poison".to_string(),

            Self::Abjuration => "Abjuration".to_string(),
            Self::Conjuration => "Conjuration".to_string(),
            Self::Divination => "Divination".to_string(),
            Self::Enchantment => "Enchantment".to_string(),
            Self::Evocation => "Evocation".to_string(),
            Self::Illusion => "Illusion".to_string(),
            Self::Necromancy => "Necromancy".to_string(),
            Self::Transmutation => "Transmutation".to_string(),

            Self::Human => "Human".to_string(),
            Self::Elf => "Elf".to_string(),
            Self::Dwarf => "Dwarf".to_string(),
            Self::Gnome => "Gnome".to_string(),
            Self::Goblin => "Goblin".to_string(),
            Self::Halfling => "Halfling".to_string(),
            Self::Orc => "Orc".to_string(),

            Self::Deadly => "Deadly".to_string(),
            Self::Fatal => "Fatal".to_string(),
            Self::Finesse => "Finesse".to_string(),
            Self::Agile => "Agile".to_string(),
            Self::Reach => "Reach".to_string(),
            Self::Thrown => "Thrown".to_string(),
            Self::Versatile => "Versatile".to_string(),

            Self::Common => "Common".to_string(),
            Self::Uncommon => "Uncommon".to_string(),
            Self::Rare => "Rare".to_string(),
            Self::Unique => "Unique".to_string(),

            Self::Lawful => "Lawful".to_string(),
            Self::Chaotic => "Chaotic".to_string(),
            Self::Good => "Good".to_string(),
            Self::Evil => "Evil".to_string(),

            Self::Aberration => "Aberration".to_string(),
            Self::Animal => "Animal".to_string(),
            Self::Beast => "Beast".to_string(),
            Self::Celestial => "Celestial".to_string(),
            Self::Construct => "Construct".to_string(),
            Self::Dragon => "Dragon".to_string(),
            Self::Elemental => "Elemental".to_string(),
            Self::Fey => "Fey".to_string(),
            Self::Fiend => "Fiend".to_string(),
            Self::Fungus => "Fungus".to_string(),
            Self::Humanoid => "Humanoid".to_string(),
            Self::Ooze => "Ooze".to_string(),
            Self::Plant => "Plant".to_string(),
            Self::Spirit => "Spirit".to_string(),
            Self::Undead => "Undead".to_string(),

            Self::Custom(name) => name.clone(),
        }
    }
}

/// 特性集合
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TraitSet {
    pub traits: Vec<Trait>,
}

impl TraitSet {
    pub fn new() -> Self {
        Self::default()
    }

    /// 加入一個特性
    pub fn add(&mut self, trait_: Trait) {
        if !self.traits.contains(&trait_) {
            self.traits.push(trait_);
        }
    }

    /// 移除一個特性
    pub fn remove(&mut self, trait_: &Trait) {
        self.traits.retain(|t| t != trait_);
    }

    /// 檢查是否包含特定特性
    pub fn contains(&self, trait_: &Trait) -> bool {
        self.traits.contains(trait_)
    }

    /// 檢查是否包含任一給定的特性
    pub fn contains_any(&self, traits: &[Trait]) -> bool {
        traits.iter().any(|t| self.contains(t))
    }

    /// 檢查是否包含所有給定的特性
    pub fn contains_all(&self, traits: &[Trait]) -> bool {
        traits.iter().all(|t| self.contains(t))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trait_display_name() {
        assert_eq!(Trait::Fire.display_name(), "Fire");
        assert_eq!(Trait::Human.display_name(), "Human");
        assert_eq!(Trait::Attack.display_name(), "Attack");
    }

    #[test]
    fn test_trait_set() {
        let mut traits = TraitSet::new();
        traits.add(Trait::Fire);
        traits.add(Trait::Attack);

        assert!(traits.contains(&Trait::Fire));
        assert!(traits.contains(&Trait::Attack));
        assert!(!traits.contains(&Trait::Cold));

        traits.remove(&Trait::Fire);
        assert!(!traits.contains(&Trait::Fire));
    }
}

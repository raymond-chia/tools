//! 法術系統
//!
//! 實作 PF2e 法術與施法

use crate::{
    abilities::Ability,
    combat::{ActionCost, DamageDice},
    traits::TraitSet,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 法術等級（0 = 戲法，1-10 = 法術等級）
pub type SpellLevel = u8;

/// 魔法傳統
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MagicTradition {
    Arcane,
    Divine,
    Occult,
    Primal,
}

impl MagicTradition {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Arcane => "Arcane",
            Self::Divine => "Divine",
            Self::Occult => "Occult",
            Self::Primal => "Primal",
        }
    }
}

/// 法術學派
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpellSchool {
    Abjuration,
    Conjuration,
    Divination,
    Enchantment,
    Evocation,
    Illusion,
    Necromancy,
    Transmutation,
}

impl SpellSchool {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Abjuration => "Abjuration",
            Self::Conjuration => "Conjuration",
            Self::Divination => "Divination",
            Self::Enchantment => "Enchantment",
            Self::Evocation => "Evocation",
            Self::Illusion => "Illusion",
            Self::Necromancy => "Necromancy",
            Self::Transmutation => "Transmutation",
        }
    }
}

/// 法術範圍
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpellRange {
    Touch,
    Feet(i32),
    Unlimited,
}

impl SpellRange {
    pub fn description(&self) -> String {
        match self {
            Self::Touch => "Touch".to_string(),
            Self::Feet(ft) => format!("{} feet", ft),
            Self::Unlimited => "Unlimited".to_string(),
        }
    }
}

/// 法術區域
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpellArea {
    Burst { radius: i32 },
    Cone { length: i32 },
    Line { length: i32 },
    Emanation { radius: i32 },
}

/// 法術目標類型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpellTarget {
    Self_,
    Creature(u8), // 生物數量
    Area(SpellArea),
}

/// 法術持續時間
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpellDuration {
    Instantaneous,
    Rounds(i32),
    Minutes(i32),
    Hours(i32),
    UntilNextDailyPreparations,
    Sustained,
}

impl SpellDuration {
    pub fn description(&self) -> String {
        match self {
            Self::Instantaneous => "Instantaneous".to_string(),
            Self::Rounds(n) => format!("{} rounds", n),
            Self::Minutes(n) => format!("{} minutes", n),
            Self::Hours(n) => format!("{} hours", n),
            Self::UntilNextDailyPreparations => "Until next daily preparations".to_string(),
            Self::Sustained => "Sustained".to_string(),
        }
    }
}

/// 一個法術
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spell {
    pub name: String,
    pub level: SpellLevel,
    pub tradition: MagicTradition,
    pub school: SpellSchool,
    pub cast_time: ActionCost,
    pub range: SpellRange,
    pub target: SpellTarget,
    pub duration: SpellDuration,
    pub description: String,
    pub damage: Option<DamageDice>,
    pub traits: TraitSet,
    /// 法術是否需要豁免檢定
    pub saving_throw: Option<crate::character::SaveType>,
    /// 法術是否需要攻擊擲骰
    pub spell_attack: bool,
}

impl Spell {
    pub fn new(name: String, level: SpellLevel, tradition: MagicTradition) -> Self {
        Self {
            name,
            level,
            tradition,
            school: SpellSchool::Evocation,
            cast_time: ActionCost::TwoActions,
            range: SpellRange::Feet(30),
            target: SpellTarget::Creature(1),
            duration: SpellDuration::Instantaneous,
            description: String::new(),
            damage: None,
            traits: TraitSet::new(),
            saving_throw: None,
            spell_attack: false,
        }
    }

    /// 建立魔法飛彈法術
    pub fn magic_missile() -> Self {
        let mut spell = Self::new("Magic Missile".to_string(), 1, MagicTradition::Arcane);
        spell.school = SpellSchool::Evocation;
        spell.target = SpellTarget::Creature(1);
        spell.description =
            "A missile of magical energy darts forth and unerringly strikes one foe".to_string();
        spell.damage = Some(DamageDice::new(1, 4, crate::combat::DamageType::Force));
        spell.traits.add(crate::traits::Trait::Force);
        spell.traits.add(crate::traits::Trait::Evocation);
        spell
    }

    /// 建立火球術法術
    pub fn fireball() -> Self {
        let mut spell = Self::new("Fireball".to_string(), 3, MagicTradition::Arcane);
        spell.school = SpellSchool::Evocation;
        spell.range = SpellRange::Feet(500);
        spell.target = SpellTarget::Area(SpellArea::Burst { radius: 20 });
        spell.description = "A roaring blast of fire appears".to_string();
        spell.damage = Some(DamageDice::new(6, 6, crate::combat::DamageType::Fire));
        spell.saving_throw = Some(crate::character::SaveType::Reflex);
        spell.traits.add(crate::traits::Trait::Fire);
        spell.traits.add(crate::traits::Trait::Evocation);
        spell
    }

    /// 檢查是否為戲法
    pub fn is_cantrip(&self) -> bool {
        self.level == 0
    }
}

/// 施法職業能力
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellcastingAbility {
    pub tradition: MagicTradition,
    pub key_ability: Ability,
    pub spell_dc: i32,
    pub spell_attack_bonus: i32,
}

impl SpellcastingAbility {
    pub fn new(
        tradition: MagicTradition,
        key_ability: Ability,
        level: i32,
        proficiency_bonus: i32,
        ability_modifier: i32,
    ) -> Self {
        let spell_dc = 10 + level + proficiency_bonus + ability_modifier;
        let spell_attack_bonus = level + proficiency_bonus + ability_modifier;

        Self {
            tradition,
            key_ability,
            spell_dc,
            spell_attack_bonus,
        }
    }
}

/// 每個等級的法術位
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpellSlots {
    slots: HashMap<SpellLevel, (u8, u8)>, // (目前, 最大)
}

impl SpellSlots {
    pub fn new() -> Self {
        Self::default()
    }

    /// 設定某等級的最大法術位
    pub fn set_max_slots(&mut self, level: SpellLevel, max: u8) {
        self.slots.insert(level, (max, max));
    }

    /// 取得某等級的目前法術位
    pub fn current_slots(&self, level: SpellLevel) -> u8 {
        self.slots
            .get(&level)
            .map(|(current, _)| *current)
            .unwrap_or(0)
    }

    /// 取得某等級的最大法術位
    pub fn max_slots(&self, level: SpellLevel) -> u8 {
        self.slots.get(&level).map(|(_, max)| *max).unwrap_or(0)
    }

    /// 使用一個法術位
    pub fn use_slot(&mut self, level: SpellLevel) -> bool {
        if let Some((current, _max)) = self.slots.get_mut(&level) {
            if *current > 0 {
                *current -= 1;
                return true;
            }
        }
        false
    }

    /// 恢復所有法術位
    pub fn restore_all(&mut self) {
        for (current, max) in self.slots.values_mut() {
            *current = *max;
        }
    }
}

/// 角色的法術書/已知法術
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Spellbook {
    pub spells: Vec<Spell>,
    pub prepared_spells: Vec<String>, // 法術名稱
}

impl Spellbook {
    pub fn new() -> Self {
        Self::default()
    }

    /// 加入法術到法術書
    pub fn add_spell(&mut self, spell: Spell) {
        if !self.spells.iter().any(|s| s.name == spell.name) {
            self.spells.push(spell);
        }
    }

    /// 準備一個法術
    pub fn prepare_spell(&mut self, spell_name: &str) -> bool {
        if self.spells.iter().any(|s| s.name == spell_name) {
            self.prepared_spells.push(spell_name.to_string());
            true
        } else {
            false
        }
    }

    /// 檢查法術是否已準備
    pub fn is_prepared(&self, spell_name: &str) -> bool {
        self.prepared_spells.contains(&spell_name.to_string())
    }

    /// 清除所有已準備的法術
    pub fn clear_prepared(&mut self) {
        self.prepared_spells.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spell_creation() {
        let magic_missile = Spell::magic_missile();
        assert_eq!(magic_missile.name, "Magic Missile");
        assert_eq!(magic_missile.level, 1);
        assert!(magic_missile.damage.is_some());
    }

    #[test]
    fn test_spell_slots() {
        let mut slots = SpellSlots::new();
        slots.set_max_slots(1, 3);

        assert_eq!(slots.current_slots(1), 3);
        assert!(slots.use_slot(1));
        assert_eq!(slots.current_slots(1), 2);

        slots.restore_all();
        assert_eq!(slots.current_slots(1), 3);
    }

    #[test]
    fn test_spellbook() {
        let mut book = Spellbook::new();
        let spell = Spell::magic_missile();

        book.add_spell(spell);
        assert_eq!(book.spells.len(), 1);

        assert!(book.prepare_spell("Magic Missile"));
        assert!(book.is_prepared("Magic Missile"));

        book.clear_prepared();
        assert!(!book.is_prepared("Magic Missile"));
    }
}

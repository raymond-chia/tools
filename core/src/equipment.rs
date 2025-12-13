//! 裝備與物品系統
//!
//! 實作 PF2e 裝備、武器和護甲

use crate::{
    combat::{Attack, DamageDice, DamageType},
    traits::TraitSet,
    ProficiencyRank,
};
use serde::{Deserialize, Serialize};

/// 裝備欄位
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipmentSlot {
    Head,
    Neck,
    Body,
    Hands,
    Ring1,
    Ring2,
    Feet,
    MainHand,
    OffHand,
}

/// 物品稀有度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Unique,
}

/// 基礎物品
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub name: String,
    pub description: String,
    pub level: i32,
    pub rarity: Rarity,
    pub bulk: f32,
    pub price_in_gp: i32,
    pub traits: TraitSet,
}

impl Item {
    pub fn new(name: String, level: i32) -> Self {
        Self {
            name,
            description: String::new(),
            level,
            rarity: Rarity::Common,
            bulk: 0.0,
            price_in_gp: 0,
            traits: TraitSet::new(),
        }
    }
}

/// 武器群組
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeaponGroup {
    Sword,
    Axe,
    Club,
    Bow,
    Knife,
    Hammer,
    Spear,
    Pick,
    Flail,
}

/// 武器類別
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeaponCategory {
    Unarmed,
    Simple,
    Martial,
    Advanced,
}

/// 武器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Weapon {
    pub item: Item,
    pub category: WeaponCategory,
    pub group: WeaponGroup,
    pub damage_dice: DamageDice,
    pub hands: u8,
    pub range: Option<i32>,
}

impl Weapon {
    pub fn new(
        name: String,
        category: WeaponCategory,
        group: WeaponGroup,
        damage_dice: DamageDice,
    ) -> Self {
        Self {
            item: Item::new(name, 0),
            category,
            group,
            damage_dice,
            hands: 1,
            range: None,
        }
    }

    /// 建立長劍
    pub fn longsword() -> Self {
        Self::new(
            "Longsword".to_string(),
            WeaponCategory::Martial,
            WeaponGroup::Sword,
            DamageDice::new(1, 8, DamageType::Slashing),
        )
    }

    /// 建立短弓
    pub fn shortbow() -> Self {
        let mut weapon = Self::new(
            "Shortbow".to_string(),
            WeaponCategory::Martial,
            WeaponGroup::Bow,
            DamageDice::new(1, 6, DamageType::Piercing),
        );
        weapon.hands = 2;
        weapon.range = Some(60);
        weapon
    }

    /// 檢查是否為近戰武器
    pub fn is_melee(&self) -> bool {
        self.range.is_none()
    }

    /// 檢查是否為遠程武器
    pub fn is_ranged(&self) -> bool {
        self.range.is_some()
    }

    /// 取得此武器所需的熟練等級
    pub fn required_proficiency(&self) -> ProficiencyRank {
        match self.category {
            WeaponCategory::Unarmed | WeaponCategory::Simple => ProficiencyRank::Trained,
            WeaponCategory::Martial => ProficiencyRank::Expert,
            WeaponCategory::Advanced => ProficiencyRank::Master,
        }
    }

    /// 用此武器建立一次攻擊
    pub fn create_attack(&self, attack_bonus: i32, damage_bonus: i32) -> Attack {
        let mut attack = Attack::new(
            self.item.name.clone(),
            attack_bonus,
            self.damage_dice.clone(),
            damage_bonus,
        );
        attack.traits = self.item.traits.clone();
        attack
    }
}

/// 護甲類別
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArmorCategory {
    Unarmored,
    Light,
    Medium,
    Heavy,
}

/// 護甲
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Armor {
    pub item: Item,
    pub category: ArmorCategory,
    pub ac_bonus: i32,
    pub dex_cap: Option<i32>,
    pub check_penalty: i32,
    pub speed_penalty: i32,
    pub strength_requirement: i32,
}

impl Armor {
    pub fn new(name: String, category: ArmorCategory, ac_bonus: i32) -> Self {
        Self {
            item: Item::new(name, 0),
            category,
            ac_bonus,
            dex_cap: None,
            check_penalty: 0,
            speed_penalty: 0,
            strength_requirement: 0,
        }
    }

    /// 建立皮甲
    pub fn leather() -> Self {
        let mut armor = Self::new("Leather Armor".to_string(), ArmorCategory::Light, 1);
        armor.dex_cap = Some(4);
        armor
    }

    /// 建立鎖甲
    pub fn chain_mail() -> Self {
        let mut armor = Self::new("Chain Mail".to_string(), ArmorCategory::Medium, 4);
        armor.dex_cap = Some(1);
        armor.check_penalty = -2;
        armor.speed_penalty = -5;
        armor.strength_requirement = 16;
        armor
    }

    /// 建立全身板甲
    pub fn full_plate() -> Self {
        let mut armor = Self::new("Full Plate".to_string(), ArmorCategory::Heavy, 6);
        armor.dex_cap = Some(0);
        armor.check_penalty = -3;
        armor.speed_penalty = -10;
        armor.strength_requirement = 18;
        armor
    }
}

/// 角色的物品欄
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Inventory {
    pub items: Vec<Item>,
    pub weapons: Vec<Weapon>,
    pub armor: Option<Armor>,
    pub gold: i32,
}

impl Inventory {
    pub fn new() -> Self {
        Self::default()
    }

    /// 加入一個物品
    pub fn add_item(&mut self, item: Item) {
        self.items.push(item);
    }

    /// 加入一把武器
    pub fn add_weapon(&mut self, weapon: Weapon) {
        self.weapons.push(weapon);
    }

    /// 裝備護甲
    pub fn equip_armor(&mut self, armor: Armor) {
        self.armor = Some(armor);
    }

    /// 計算總負重
    pub fn total_bulk(&self) -> f32 {
        let item_bulk: f32 = self.items.iter().map(|i| i.bulk).sum();
        let weapon_bulk: f32 = self.weapons.iter().map(|w| w.item.bulk).sum();
        let armor_bulk = self.armor.as_ref().map(|a| a.item.bulk).unwrap_or(0.0);

        item_bulk + weapon_bulk + armor_bulk
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weapon_creation() {
        let longsword = Weapon::longsword();
        assert_eq!(longsword.item.name, "Longsword");
        assert_eq!(longsword.category, WeaponCategory::Martial);
        assert!(longsword.is_melee());
        assert!(!longsword.is_ranged());
    }

    #[test]
    fn test_armor_creation() {
        let leather = Armor::leather();
        assert_eq!(leather.ac_bonus, 1);
        assert_eq!(leather.dex_cap, Some(4));

        let full_plate = Armor::full_plate();
        assert_eq!(full_plate.ac_bonus, 6);
        assert_eq!(full_plate.dex_cap, Some(0));
    }

    #[test]
    fn test_inventory() {
        let mut inventory = Inventory::new();
        inventory.gold = 100;

        let weapon = Weapon::longsword();
        inventory.add_weapon(weapon);

        let armor = Armor::leather();
        inventory.equip_armor(armor);

        assert_eq!(inventory.weapons.len(), 1);
        assert!(inventory.armor.is_some());
    }
}

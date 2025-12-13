//! 戰鬥系統
//!
//! 實作 PF2e 戰鬥機制，包括動作經濟、攻擊、傷害與定位

use crate::{
    character::Character,
    dice::{roll_d20, roll_dice, CheckResult, DiceRoll},
    traits::TraitSet,
    GameError,
};
use serde::{Deserialize, Serialize};

/// 戰術網格上的位置（以 5 呎方格為單位）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// 使用 PF2e 網格規則計算距離（對角線算作 1 格）
    pub fn distance_to(&self, other: &Position) -> i32 {
        let dx = (self.x - other.x).abs();
        let dy = (self.y - other.y).abs();
        dx.max(dy) * 5 // 每格為 5 呎
    }

    /// 檢查是否相鄰（5 呎內，用於近戰）
    pub fn is_adjacent(&self, other: &Position) -> bool {
        self.distance_to(other) <= 5
    }
}

/// 戰術戰鬥中的單位
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatUnit {
    pub character: Character,
    pub position: Position,
    pub actions_remaining: u8,
    pub has_reaction: bool,
    pub map: MultipleAttackPenalty,
}

impl CombatUnit {
    pub fn new(character: Character, position: Position) -> Self {
        Self {
            character,
            position,
            actions_remaining: 3,
            has_reaction: true,
            map: MultipleAttackPenalty::new(),
        }
    }

    /// 回合開始
    pub fn start_turn(&mut self) {
        self.actions_remaining = 3;
        self.has_reaction = true;
        self.map.reset();
    }

    /// 使用動作
    pub fn use_action(&mut self, cost: u8) -> Result<(), GameError> {
        if self.actions_remaining >= cost {
            self.actions_remaining -= cost;
            Ok(())
        } else {
            Err(GameError::InvalidAction("Not enough actions".to_string()))
        }
    }

    /// 跨步：移動至你的速度（1 個動作）
    pub fn stride(&mut self, target: Position) -> Result<(), GameError> {
        let distance = self.position.distance_to(&target);
        let speed = self.character.ancestry.speed();

        if distance > speed {
            return Err(GameError::InvalidAction(format!(
                "Cannot move {} feet (Speed is {})",
                distance, speed
            )));
        }

        self.use_action(1)?;
        self.position = target;
        Ok(())
    }

    /// 踏步：移動 5 呎而不觸發反應（1 個動作）
    pub fn step(&mut self, target: Position) -> Result<(), GameError> {
        let distance = self.position.distance_to(&target);

        if distance > 5 {
            return Err(GameError::InvalidAction(
                "Step can only move 5 feet".to_string(),
            ));
        }

        self.use_action(1)?;
        self.position = target;
        Ok(())
    }

    /// 取得顯示符號（名稱的第一個字母）
    pub fn symbol(&self) -> String {
        self.character
            .name
            .chars()
            .next()
            .unwrap_or('?')
            .to_string()
    }
}

/// PF2e 三動作經濟中的動作成本
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionCost {
    /// 自由動作
    Free,
    /// 反應
    Reaction,
    /// 一個動作
    OneAction,
    /// 兩個動作
    TwoActions,
    /// 三個動作
    ThreeActions,
    /// 整輪活動
    Activity,
}

impl ActionCost {
    pub fn num_actions(&self) -> u8 {
        match self {
            Self::Free => 0,
            Self::Reaction => 0,
            Self::OneAction => 1,
            Self::TwoActions => 2,
            Self::ThreeActions => 3,
            Self::Activity => 3,
        }
    }
}

/// 可在戰鬥中執行的動作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub name: String,
    pub cost: ActionCost,
    pub description: String,
    pub traits: TraitSet,
}

impl Action {
    pub fn new(name: String, cost: ActionCost, description: String) -> Self {
        Self {
            name,
            cost,
            description,
            traits: TraitSet::new(),
        }
    }

    /// 打擊動作（基本攻擊）
    pub fn strike() -> Self {
        let mut action = Self::new(
            "Strike".to_string(),
            ActionCost::OneAction,
            "Make a melee or ranged attack".to_string(),
        );
        action.traits.add(crate::traits::Trait::Attack);
        action
    }

    /// 跨步動作（移動）
    pub fn stride() -> Self {
        let mut action = Self::new(
            "Stride".to_string(),
            ActionCost::OneAction,
            "Move up to your Speed".to_string(),
        );
        action.traits.add(crate::traits::Trait::Move);
        action
    }

    /// 施放法術
    pub fn cast_spell() -> Self {
        Self::new(
            "Cast a Spell".to_string(),
            ActionCost::TwoActions,
            "Cast a spell".to_string(),
        )
    }
}

/// 傷害骰規格
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageDice {
    pub num_dice: u32,
    pub die_size: u32,
    pub damage_type: DamageType,
}

impl DamageDice {
    pub fn new(num_dice: u32, die_size: u32, damage_type: DamageType) -> Self {
        Self {
            num_dice,
            die_size,
            damage_type,
        }
    }

    pub fn roll(&self, modifier: i32) -> Damage {
        let roll = roll_dice(self.num_dice, self.die_size, modifier);
        Damage {
            amount: roll.total,
            damage_type: self.damage_type,
        }
    }
}

/// 傷害類型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageType {
    Bludgeoning,
    Piercing,
    Slashing,
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
    Untyped,
}

impl DamageType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Bludgeoning => "Bludgeoning",
            Self::Piercing => "Piercing",
            Self::Slashing => "Slashing",
            Self::Fire => "Fire",
            Self::Cold => "Cold",
            Self::Electricity => "Electricity",
            Self::Acid => "Acid",
            Self::Sonic => "Sonic",
            Self::Positive => "Positive",
            Self::Negative => "Negative",
            Self::Force => "Force",
            Self::Mental => "Mental",
            Self::Poison => "Poison",
            Self::Untyped => "Untyped",
        }
    }

    /// 檢查是否為物理傷害
    pub fn is_physical(&self) -> bool {
        matches!(self, Self::Bludgeoning | Self::Piercing | Self::Slashing)
    }

    /// 檢查是否為能量傷害
    pub fn is_energy(&self) -> bool {
        matches!(
            self,
            Self::Fire | Self::Cold | Self::Electricity | Self::Acid | Self::Sonic
        )
    }
}

/// 造成的傷害
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Damage {
    pub amount: i32,
    pub damage_type: DamageType,
}

impl Damage {
    pub fn new(amount: i32, damage_type: DamageType) -> Self {
        Self {
            amount,
            damage_type,
        }
    }
}

/// 一次攻擊
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attack {
    pub name: String,
    pub attack_bonus: i32,
    pub damage_dice: DamageDice,
    pub damage_bonus: i32,
    pub traits: TraitSet,
}

impl Attack {
    pub fn new(
        name: String,
        attack_bonus: i32,
        damage_dice: DamageDice,
        damage_bonus: i32,
    ) -> Self {
        Self {
            name,
            attack_bonus,
            damage_dice,
            damage_bonus,
            traits: TraitSet::new(),
        }
    }

    /// 進行攻擊擲骰
    pub fn roll_attack(&self) -> DiceRoll {
        roll_d20(self.attack_bonus)
    }

    /// 擲傷害骰
    pub fn roll_damage(&self) -> Damage {
        self.damage_dice.roll(self.damage_bonus)
    }

    /// 對目標 AC 進行完整攻擊
    pub fn attack(&self, target_ac: i32) -> AttackResult {
        let attack_roll = self.roll_attack();
        let check_result = CheckResult::from_roll(&attack_roll, target_ac);

        let damage = match check_result {
            CheckResult::CriticalSuccess => {
                // 重擊：傷害加倍
                let dmg = self.roll_damage();
                Some(Damage::new(dmg.amount * 2, dmg.damage_type))
            }
            CheckResult::Success => Some(self.roll_damage()),
            CheckResult::Failure | CheckResult::CriticalFailure => None,
        };

        AttackResult {
            attack_roll,
            check_result,
            damage,
        }
    }
}

/// 攻擊結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackResult {
    pub attack_roll: DiceRoll,
    pub check_result: CheckResult,
    pub damage: Option<Damage>,
}

impl AttackResult {
    pub fn hit(&self) -> bool {
        self.check_result.is_success()
    }

    pub fn critical_hit(&self) -> bool {
        self.check_result.is_critical_success()
    }
}

/// 多重攻擊減值（MAP）
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MultipleAttackPenalty {
    pub attacks_made: u8,
    pub agile: bool,
}

impl MultipleAttackPenalty {
    pub fn new() -> Self {
        Self {
            attacks_made: 0,
            agile: false,
        }
    }

    pub fn with_agile(mut self, agile: bool) -> Self {
        self.agile = agile;
        self
    }

    /// 取得目前的減值
    pub fn penalty(&self) -> i32 {
        match self.attacks_made {
            0 => 0,
            1 => {
                if self.agile {
                    -4
                } else {
                    -5
                }
            }
            _ => {
                if self.agile {
                    -8
                } else {
                    -10
                }
            }
        }
    }

    /// 記錄一次攻擊
    pub fn record_attack(&mut self) {
        self.attacks_made += 1;
    }

    /// 回合開始時重置
    pub fn reset(&mut self) {
        self.attacks_made = 0;
    }
}

impl Default for MultipleAttackPenalty {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_cost() {
        assert_eq!(ActionCost::OneAction.num_actions(), 1);
        assert_eq!(ActionCost::TwoActions.num_actions(), 2);
        assert_eq!(ActionCost::ThreeActions.num_actions(), 3);
    }

    #[test]
    fn test_damage_type() {
        assert!(DamageType::Bludgeoning.is_physical());
        assert!(DamageType::Fire.is_energy());
        assert!(!DamageType::Mental.is_physical());
    }

    #[test]
    fn test_multiple_attack_penalty() {
        let mut map = MultipleAttackPenalty::new();
        assert_eq!(map.penalty(), 0);

        map.record_attack();
        assert_eq!(map.penalty(), -5);

        map.record_attack();
        assert_eq!(map.penalty(), -10);

        // 測試敏捷武器
        let mut map_agile = MultipleAttackPenalty::new().with_agile(true);
        map_agile.record_attack();
        assert_eq!(map_agile.penalty(), -4);

        map_agile.record_attack();
        assert_eq!(map_agile.penalty(), -8);
    }

    #[test]
    fn test_attack() {
        let damage_dice = DamageDice::new(1, 8, DamageType::Slashing);
        let attack = Attack::new("Longsword".to_string(), 5, damage_dice, 3);

        let result = attack.attack(15);
        // 結果應該有攻擊擲骰和可能的傷害
        assert!(result.attack_roll.total >= 6 && result.attack_roll.total <= 25);
    }
}

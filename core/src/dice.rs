//! 擲骰系統
//!
//! 實作 PF2e 擲骰機制

use rand::Rng;
use serde::{Deserialize, Serialize};

/// 擲骰結果
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiceRoll {
    /// 骰子數量
    pub num_dice: u32,
    /// 骰子面數（面數）
    pub die_size: u32,
    /// 要加到擲骰的調整值
    pub modifier: i32,
    /// 個別骰子的結果
    pub rolls: Vec<u32>,
    /// 總計（包含調整值）
    pub total: i32,
}

impl DiceRoll {
    /// 建立一個新的擲骰
    pub fn new(num_dice: u32, die_size: u32, modifier: i32) -> Self {
        let mut rng = rand::rng();
        let rolls: Vec<u32> = (0..num_dice)
            .map(|_| rng.random_range(1..=die_size))
            .collect();

        let sum: u32 = rolls.iter().sum();
        let total = sum as i32 + modifier;

        Self {
            num_dice,
            die_size,
            modifier,
            rolls,
            total,
        }
    }

    /// 格式化擲骰以供顯示（例如："3d6+2 = [4, 5, 3] + 2 = 14"）
    pub fn format(&self) -> String {
        format!(
            "{}d{}+{} = {:?} + {} = {}",
            self.num_dice, self.die_size, self.modifier, self.rolls, self.modifier, self.total
        )
    }
}

/// 擲 d20（最常見的檢定骰）
pub fn roll_d20(modifier: i32) -> DiceRoll {
    DiceRoll::new(1, 20, modifier)
}

/// 擲骰（通用）
///
/// # 範例
/// ```
/// use pf2e_core::dice::roll_dice;
///
/// // 擲 3d6+2
/// let result = roll_dice(3, 6, 2);
/// assert!(result.total >= 5 && result.total <= 20);
/// ```
pub fn roll_dice(num_dice: u32, die_size: u32, modifier: i32) -> DiceRoll {
    DiceRoll::new(num_dice, die_size, modifier)
}

/// 檢定結果等級
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckResult {
    /// 大失敗（低於 DC 10 以上，或自然骰 1）
    CriticalFailure,
    /// 失敗
    Failure,
    /// 成功
    Success,
    /// 大成功（高於 DC 10 以上，或自然骰 20）
    CriticalSuccess,
}

impl CheckResult {
    /// 從擲骰和 DC 判定成功等級
    pub fn from_roll(roll: &DiceRoll, dc: i32) -> Self {
        let natural_roll = roll.rolls[0];
        let total = roll.total;
        let diff = total - dc;

        // 自然骰 20 總是將成功等級提高一級
        if natural_roll == 20 {
            if diff >= 0 {
                return Self::CriticalSuccess;
            } else if diff >= -9 {
                return Self::Success;
            } else {
                return Self::Failure;
            }
        }

        // 自然骰 1 總是將成功等級降低一級
        if natural_roll == 1 {
            if diff >= 10 {
                return Self::Success;
            } else if diff >= 0 {
                return Self::Failure;
            } else {
                return Self::CriticalFailure;
            }
        }

        // 正常判定
        if diff >= 10 {
            Self::CriticalSuccess
        } else if diff >= 0 {
            Self::Success
        } else if diff >= -9 {
            Self::Failure
        } else {
            Self::CriticalFailure
        }
    }

    /// 檢查是否成功（包括大成功）
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success | Self::CriticalSuccess)
    }

    /// 檢查是否為大成功
    pub fn is_critical_success(&self) -> bool {
        matches!(self, Self::CriticalSuccess)
    }

    /// 檢查是否失敗（包括大失敗）
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Failure | Self::CriticalFailure)
    }

    /// 檢查是否為大失敗
    pub fn is_critical_failure(&self) -> bool {
        matches!(self, Self::CriticalFailure)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dice_roll() {
        let roll = roll_dice(3, 6, 2);
        assert_eq!(roll.num_dice, 3);
        assert_eq!(roll.die_size, 6);
        assert_eq!(roll.modifier, 2);
        assert_eq!(roll.rolls.len(), 3);
        assert!(roll.total >= 5 && roll.total <= 20);
    }

    #[test]
    fn test_d20_roll() {
        let roll = roll_d20(5);
        assert_eq!(roll.num_dice, 1);
        assert_eq!(roll.die_size, 20);
        assert_eq!(roll.modifier, 5);
        assert!(roll.total >= 6 && roll.total <= 25);
    }

    #[test]
    fn test_check_result() {
        // 模擬自然骰 20
        let roll = DiceRoll {
            num_dice: 1,
            die_size: 20,
            modifier: 0,
            rolls: vec![20],
            total: 20,
        };
        assert!(CheckResult::from_roll(&roll, 25).is_success());

        // 模擬自然骰 1
        let roll = DiceRoll {
            num_dice: 1,
            die_size: 20,
            modifier: 10,
            rolls: vec![1],
            total: 11,
        };
        assert!(CheckResult::from_roll(&roll, 10).is_failure());
    }
}

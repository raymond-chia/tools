//! Pathfinder 2e 核心遊戲邏輯函式庫
//!
//! 此函式庫實作 Pathfinder 2e 的核心遊戲規則，包括：
//! - 角色屬性與技能系統
//! - 戰鬥系統（動作經濟、攻擊擲骰、傷害計算）
//! - 裝備與物品系統
//! - 法術系統
//! - 狀態效果系統

pub mod abilities;
pub mod character;
pub mod combat;
pub mod dice;
pub mod equipment;
pub mod i18n;
pub mod skills;
pub mod spells;
pub mod traits;

// 重新導出常用類型
pub use abilities::{Ability, AbilityScores};
pub use character::{Ancestry, Character, CharacterClass};
pub use combat::{Action, ActionCost, Attack, CombatUnit, Damage, Position};
pub use dice::{roll_d20, roll_dice, DiceRoll};
pub use skills::{Skill, SkillProficiency};

/// PF2e 的熟練等級
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ProficiencyRank {
    /// 未受訓
    Untrained = 0,
    /// 受訓
    Trained = 2,
    /// 專家
    Expert = 4,
    /// 大師
    Master = 6,
    /// 傳奇
    Legendary = 8,
}

impl ProficiencyRank {
    /// 取得熟練加值
    pub fn bonus(&self) -> i32 {
        *self as i32
    }
}

/// 遊戲操作的結果類型
pub type GameResult<T> = Result<T, GameError>;

/// 遊戲錯誤類型
#[derive(Debug, thiserror::Error)]
pub enum GameError {
    #[error("Invalid ability score: {0}")]
    InvalidAbilityScore(i32),

    #[error("Invalid level: {0}")]
    InvalidLevel(i32),

    #[error("Invalid action: {0}")]
    InvalidAction(String),

    #[error("Equipment error: {0}")]
    EquipmentError(String),

    #[error("Spell error: {0}")]
    SpellError(String),

    #[error("Other error: {0}")]
    Other(String),
}

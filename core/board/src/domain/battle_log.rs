//! 戰鬥 Log 事件型別定義（純資料）
//!
//! 這是一套**獨立、人類可讀、自帶名稱快照**的 log 事件型別，與動畫用的
//! `EffectEntry`（見 `logic::skill::skill_execution`）職責完全分開：
//! - log 給玩家閱讀，前端依結構化欄位 + 語系產生顯示字串。
//! - `EffectEntry` 給未來動畫驅動，不在此重用。
//!
//! 設計原則：
//! - **扁平結構**：一筆 log = 一個事件，多目標散成多筆。
//! - **名稱快照**：事件產生當下就寫入單位/物件的 type name，
//!   死者被 despawn 後 log 仍能正確顯示，前端不需事後反查。
//! - **不寫死文案**：只存結構化欄位（名稱、技能名、判定數值、效果摘要），
//!   供多國語言渲染。
//!
//! 持有事件序列的 `BattleLog` Resource 定義在 `ecs_types::resources`。

use crate::domain::alias::{SkillName, TypeName};
use crate::domain::core_types::{AccuracySource, DefenseType, HitCheckBreakdowns};

/// log 事件的目標（自帶名稱快照，不依賴事後反查、不暴露座標給玩家）
#[derive(Debug, Clone, PartialEq)]
pub enum LogTarget {
    /// 目標是單位
    Unit { name: TypeName },
    /// 目標格上有物件（取其一）
    Object { name: TypeName },
    /// 目標格為空地（無單位也無物件）
    EmptyGround,
}

/// log 的命中判定結果
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogCheck {
    /// 無判定，必定生效
    Auto,
    Hit {
        crit: bool,
    },
    Block {
        crit: bool,
    },
    Evade,
    Resisted,
    Affected,
}

/// log 的命中判定明細數值（供逐項展開顯示）
///
/// 與動畫用的 `CheckDetail` 維持分離（各自獨立結構），但內部共用
/// `HitCheckBreakdowns` 數值明細，避免重複維護同一組 breakdown 欄位。
#[derive(Debug, Clone, PartialEq)]
pub struct LogCheckDetail {
    pub accuracy_source: AccuracySource,
    pub defense_type: DefenseType,
    pub breakdowns: HitCheckBreakdowns,
    pub roll: i32,
}

/// log 的效果摘要
#[derive(Debug, Clone, PartialEq)]
pub enum LogEffect {
    None,
    HpChange { amount: i32 },
    SpawnObject { object_type: TypeName },
    ApplyBuff { buff_name: String },
}

/// 單筆戰鬥 log 事件（扁平結構，照發生順序 append）
#[derive(Debug, Clone, PartialEq)]
pub enum LogEvent {
    /// 主動技能對單一目標的判定結果
    Skill {
        caster: TypeName,
        skill_name: SkillName,
        target: LogTarget,
        check: LogCheck,
        check_detail: Option<LogCheckDetail>,
        effect: LogEffect,
    },
    /// 反應技能對單一目標的判定結果
    Reaction {
        reactor: TypeName,
        trigger: TypeName,
        skill_name: SkillName,
        target: LogTarget,
        check: LogCheck,
        check_detail: Option<LogCheckDetail>,
        effect: LogEffect,
    },
    /// 單位死亡（只記身分名稱快照）
    Death { unit: TypeName },
}

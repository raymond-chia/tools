//! 基本資料類型定義

use crate::domain::alias::{Coord, SkillName, TypeName};
use crate::ecs_types::components::Occupant;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter};

// ============================================================================
// 回合系統
// ============================================================================

/// 單位在回合表中的資訊
#[derive(Debug, Clone)]
pub struct TurnEntry {
    pub occupant: Occupant,
    pub initiative: i32, // 原始 INI
    pub roll: i32,
    pub total: i32,      // INI + roll（主排序，顯示用）
    pub tiebreaker: f64, // INI*10 + 1 if player + 0.xxx（次排序，隱藏）
    pub has_acted: bool,
}

// ============================================================================
// 屬性系統
// ============================================================================

/// 定義屬性列表的 macro（單一來源）
///
/// 格式：(欄位名, Attribute enum variant)
/// - `Attribute` enum
macro_rules! define_attributes {
    ($(($field:ident, $variant:ident)),* $(,)?) => {
        /// 角色屬性類型
        #[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, Display, EnumIter)]
        pub enum Attribute {
            #[default]
            $($variant,)*
        }
    };
}

define_attributes!(
    (hp, Hp),
    (mp, Mp),
    (initiative, Initiative),
    (accuracy, Accuracy),
    (evasion, Evasion),
    (block, Block),
    (block_protection, BlockProtection),
    (physical_attack, PhysicalAttack),
    (magical_attack, MagicalAttack),
    (magical_dc, MagicalDc),
    (fortitude, Fortitude),
    (reflex, Reflex),
    (will, Will),
    (movement_point, MovementPoint),
    (reaction_point, ReactionPoint),
);

// ============================================================================
// 技能系統 - enum
// ============================================================================

/// 技能標籤
#[derive(Debug, Clone, Default, Serialize, Deserialize, Display, EnumIter)]
pub enum SkillTag {
    #[default]
    UsableAfterDoubleMove,
    AllowedDuringGrabbing,
}

/// DC 檢定類型
#[derive(Debug, Clone, Default, Serialize, Deserialize, Display, EnumIter)]
pub enum DcType {
    #[default]
    Fortitude,
    Reflex,
    Will,
}

/// 效果目標
#[derive(Debug, Clone, Default, Serialize, Deserialize, Display, EnumIter)]
pub enum CasterOrTarget {
    Caster,
    #[default]
    Target,
}

/// 目標過濾條件
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Display, EnumIter)]
pub enum TargetFilter {
    #[default]
    Any,
    AnyExceptCaster,
    Ally,
    AllyExceptCaster,
    Enemy,
    CasterOnly,
}

/// 強制位移方向
#[derive(Debug, Clone, Default, Serialize, Deserialize, Display, EnumIter)]
pub enum MoveDirection {
    #[default]
    AwayFromCaster,
    TowardCaster,
    AwayFromTarget,
    TowardTarget,
}

/// 範圍形狀
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Display, EnumIter)]
pub enum Area {
    #[default]
    Single,
    Diamond {
        radius: Coord,
    },
    Cross {
        length: Coord,
    },
    Line {
        length: Coord,
    },
}

/// 目標選擇方式
#[derive(Debug, Clone, Default, Serialize, Deserialize, Display, EnumIter)]
pub enum TargetSelection {
    #[default]
    Unit,
    Ground,
}

/// 反應觸發類型
#[derive(Debug, Clone, Default, Serialize, Deserialize, Display, EnumIter)]
pub enum ReactionTrigger {
    #[default]
    AttackOfOpportunity,
    TakesDamage,
}

/// 效果條件
#[derive(Debug, Clone, Serialize, Deserialize, Display, EnumIter)]
pub enum EffectCondition {
    HitCheck {
        accuracy_bonus: i32,
        crit_bonus: i32,
    },
    DcCheck {
        dc_type: DcType,
        dc_bonus: i32,
    },
}

/// Buff 結束條件（多個條件之間為 OR 關係）
#[derive(Debug, Clone, Serialize, Deserialize, Display, EnumIter)]
pub enum EndCondition {
    Duration(u32),
    TargetSavesPerTurn,
    CasterUsesSkillWithoutTag(SkillTag),
    EitherDies,
    EitherMoves,
    TargetMoves,
}

/// 感知類型
#[derive(Debug, Clone, Default, Serialize, Deserialize, Display, EnumIter)]
pub enum PerceptionType {
    #[default]
    Auditory,
}

/// 光源類型
#[derive(Debug, Clone, Default, Serialize, Deserialize, Display, EnumIter)]
pub enum LightType {
    #[default]
    Bright,
    Dim,
    Darkness,
}

// ============================================================================
// 技能系統 - struct
// ============================================================================

/// 主動技能的目標定義
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Target {
    pub range: (Coord, Coord),
    pub selection: TargetSelection,
    pub selectable_filter: TargetFilter,
    pub count: usize,
    pub allow_same_target: bool,
    pub area: Area,
}

/// 反應技能的觸發來源定義
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TriggeringSource {
    pub source_range: (Coord, Coord),
    pub source_filter: TargetFilter,
    pub trigger: ReactionTrigger,
}

/// 屬性倍率
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Scaling {
    pub source: CasterOrTarget,
    pub source_attribute: Attribute,
    pub value_percent: i32,
}

// ============================================================================
// 技能系統 - 效果層
// ============================================================================

/// 效果節點（遞迴巢狀結構）
#[derive(Debug, Clone, Serialize, Deserialize, Display, EnumIter)]
pub enum EffectNode {
    Area {
        area: Area,
        filter: TargetFilter,
        nodes: Vec<EffectNode>,
    },
    Branch {
        who: CasterOrTarget,
        condition: EffectCondition,
        on_success: Vec<EffectNode>,
        on_failure: Vec<EffectNode>,
    },
    Leaf {
        who: CasterOrTarget,
        effect: Effect,
    },
}

/// 技能效果
#[derive(Debug, Clone, Serialize, Deserialize, Display, EnumIter)]
pub enum Effect {
    HpEffect {
        scaling: Scaling,
    },
    MpEffect {
        value: i32,
    },
    ApplyBuff {
        buff: BuffType,
    },
    ForcedMove {
        direction: MoveDirection,
        distance: Coord,
    },
    AllowRemainingMovement,
    SwapPosition,
    Trample {
        distance: Coord,
        scaling: Scaling,
    },
    SpawnObject {
        object_type: TypeName,
        duration: Option<u32>,
        contact_effects: Vec<EffectNode>,
    },
}

/// 持續性效果（被動技能與 Buff 共用）
#[derive(Debug, Clone, Serialize, Deserialize, Display, EnumIter)]
pub enum ContinuousEffect {
    AttributeFlat {
        attribute: Attribute,
        value: i32,
    },
    AttributeScaling {
        target_attribute: Attribute,
        source: CasterOrTarget,
        source_attribute: Attribute,
        value_percent: i32,
    },
    NearbyAllyScaling {
        range: Coord,
        attribute: Attribute,
        per_ally_percent: i32,
        base_percent: i32,
    },
    HpRatioScaling {
        attribute: Attribute,
        min_bonus_percent: i32,
        step_percent: u32,
        bonus_per_step: i32,
        max_bonus_percent: i32,
    },
    Perception {
        perception_type: PerceptionType,
        range: Coord,
    },
    DamageToMp {
        ratio_percent: i32,
    },
    EmitLight {
        light_type: LightType,
        range: Coord,
    },
    Blinded,
}

// ============================================================================
// 技能系統 - 主結構
// ============================================================================

/// 技能類型定義
#[derive(Debug, Clone, Serialize, Deserialize, Display, EnumIter)]
pub enum SkillType {
    Active {
        name: SkillName,
        tags: Vec<SkillTag>,
        cost: u32,
        target: Target,
        effects: Vec<EffectNode>,
    },
    Reaction {
        name: SkillName,
        tags: Vec<SkillTag>,
        cost: u32,
        triggering_unit: TriggeringSource,
        effects: Vec<EffectNode>,
    },
    Passive {
        name: SkillName,
        tags: Vec<SkillTag>,
        effects: Vec<ContinuousEffect>,
    },
}

/// Buff 定義（內嵌在技能 TOML 中）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BuffType {
    pub stackable: bool,
    pub while_active: Vec<ContinuousEffect>,
    pub per_turn_effects: Vec<EffectNode>,
    pub end_conditions: Vec<EndCondition>,
}

// ============================================================================
// 手動實作 Default（EnumIter 需要 Default，但 #[default] 只能用在 unit variant）
// ============================================================================

impl Default for EffectCondition {
    fn default() -> Self {
        Self::HitCheck {
            accuracy_bonus: 0,
            crit_bonus: 0,
        }
    }
}

impl Default for EffectNode {
    fn default() -> Self {
        Self::Leaf {
            who: CasterOrTarget::Target,
            effect: Effect::default(),
        }
    }
}

impl Default for Effect {
    fn default() -> Self {
        Self::HpEffect {
            scaling: Scaling::default(),
        }
    }
}

impl Default for ContinuousEffect {
    fn default() -> Self {
        Self::AttributeFlat {
            attribute: Attribute::default(),
            value: 0,
        }
    }
}

impl Default for EndCondition {
    fn default() -> Self {
        Self::Duration(0)
    }
}

impl Default for SkillType {
    fn default() -> Self {
        Self::Active {
            name: SkillName::default(),
            tags: Vec::default(),
            cost: 0,
            target: Target::default(),
            effects: Vec::default(),
        }
    }
}

impl SkillType {
    /// 獲取技能名稱
    pub fn name(&self) -> &SkillName {
        match self {
            Self::Active { name, .. } => name,
            Self::Reaction { name, .. } => name,
            Self::Passive { name, .. } => name,
        }
    }
}

//! ECS Component 定義

use crate::domain::alias::{Coord, ID, MovementCost, SkillName, TypeName};
use crate::domain::core_types::{BuffType, DefenseType, EffectNode};
use bevy_ecs::bundle::Bundle;
use bevy_ecs::component::Component;
use serde::{Deserialize, Serialize};

/// 生成 tag components 的 macro
macro_rules! define_tag_components {
    ($($name:ident),* $(,)?) => {
        $(
            #[doc = concat!("tag component: ", stringify!($name))]
            #[derive(Debug, Component)]
            pub struct $name;
        )*
    };
}

// ============================================================================
// 棋盤與位置 Components
// ============================================================================

/// 棋盤位置（座標）
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Component,
    Serialize,
    Deserialize,
)]
pub struct Position {
    pub x: Coord,
    pub y: Coord,
}

// ============================================================================
// 佔據者 Components
// ============================================================================

// ============================================================================
// 身份識別 Components
// ============================================================================

/// 佔據者的類型名稱（對應 loader_schema 中的 TypeName）
#[derive(Debug, Clone, Component)]
pub struct OccupantTypeName(pub TypeName);

define_tag_components!(Unit, Object);

/// 位置上的佔據者（單位或物件）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub enum Occupant {
    Unit(ID),
    Object(ID),
}

// ============================================================================
// 單位專用 Components
// ============================================================================

/// 單位持有的技能列表
#[derive(Debug, Clone, Component)]
pub struct Skills(pub Vec<SkillName>);

/// 單位所屬的陣營 ID（用於區分友軍/敵軍）
#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct UnitFaction(pub ID);

/// 生成屬性 components 和 AttributeBundle 的 macro
macro_rules! define_attribute_components {
    ($(($field:ident, $Type:ident)),* $(,)?) => {
        $(
            #[doc = concat!("角色屬性 component: ", stringify!($Type))]
            #[derive(Debug, Clone, Default, Component)]
            pub struct $Type(pub i32);
        )*

        /// 所有屬性 Component 的 Bundle
        #[derive(Debug, Clone, Default, Bundle)]
        pub struct AttributeBundle {
            $(pub $field: $Type,)*
        }
    };
}

define_attribute_components!(
    (max_hp, MaxHp),
    (current_hp, CurrentHp),
    (max_mp, MaxMp),
    (current_mp, CurrentMp),
    (initiative, Initiative),
    (physical_attack, PhysicalAttack),
    (magical_attack, MagicalAttack),
    (physical_accuracy, PhysicalAccuracy),
    (magical_accuracy, MagicalAccuracy),
    (fortitude, Fortitude),
    (agility, Agility),
    (block, Block),
    (block_protection, BlockProtection),
    (will, Will),
    (movement_point, MovementPoint),
    (reaction_point, ReactionPoint),
    (flanking_accuracy_bonus, FlankingAccuracyBonus),
);

/// 單位的行動狀態
///
/// 初始值為 `Moved { cost: 0 }`，使用技能後變為 `Done`
#[derive(Debug, Clone, Component)]
pub enum ActionState {
    /// 尚未使用技能，記錄已消耗的移動力
    Moved { cost: MovementCost },
    /// 已使用技能，回合結束
    Done,
}

// ============================================================================
// 物件專用 Components
// ============================================================================

/// 地形移動花費
#[derive(Debug, Clone, Component)]
pub struct ObjectMovementCost(pub MovementCost);

define_tag_components!(BlocksSight, BlocksSound);

/// 接觸效果（單位接觸物件時觸發的效果鏈）
// by claude
// - 每幀/熱迴圈：即使幾百 bytes 也值得避免
// - 每回合操作（像你這裡）：幾 KB 完全沒問題，甚至幾十 KB 都不會有感
// - 真正該擔心的：大型集合（上千元素的 Vec/HashMap）在熱路徑中反覆 clone
#[derive(Debug, Clone, Component)]
pub struct ContactEffects(pub Vec<EffectNode>);

/// 施加在單位上的 Buff
/// Buff 施加後的運行時狀態
#[derive(Debug, Component)]
pub struct AppliedBuff {
    pub def: BuffType,
    pub caster: Occupant,
    pub target: Occupant,
    pub remaining_duration: Option<u32>,
    pub inherited_defense: Option<DefenseType>,
}

// ============================================================================
// Bundles
// ============================================================================

/// 單位 Entity 的完整 Bundle
#[derive(Debug, Bundle)]
pub struct UnitBundle {
    pub unit: Unit,
    pub position: Position,
    pub occupant: Occupant,
    pub occupant_type_name: OccupantTypeName,
    pub unit_faction: UnitFaction,
    pub skills: Skills,
    pub attributes: AttributeBundle,
    pub action_state: ActionState,
}

/// 物件 Entity 的完整 Bundle
#[derive(Debug, Bundle)]
pub struct ObjectBundle {
    pub object: Object,
    pub position: Position,
    pub occupant: Occupant,
    pub occupant_type_name: OccupantTypeName,
    pub terrain_movement_cost: ObjectMovementCost,
    pub contact_effects: ContactEffects,
    // block sight
    // block sound
}

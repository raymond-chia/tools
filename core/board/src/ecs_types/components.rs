//! ECS Component 定義

use crate::domain::alias::{Coord, ID, MovementCost, SkillName, TypeName};
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

/// 陣營（用於區分友軍/敵軍）
#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Faction(pub ID);

/// 生成屬性 components 和 AttributeBundle 的 macro
macro_rules! define_attribute_components {
    ($(($field:ident, $Type:ident)),* $(,)?) => {
        $(
            #[doc = concat!("角色屬性 component: ", stringify!($Type))]
            #[derive(Debug, Clone, Component)]
            pub struct $Type(pub i32);
        )*

        /// 所有屬性 Component 的 Bundle
        #[derive(Debug, Bundle, Clone)]
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
    (hit, Hit),
    (evasion, Evasion),
    (block, Block),
    (block_protection, BlockProtection),
    (physical_attack, PhysicalAttack),
    (magical_attack, MagicalAttack),
    (magical_dc, MagicalDc),
    (fortitude, Fortitude),
    (reflex, Reflex),
    (will, Will),
    (movement, Movement),
    (reaction, Reaction),
);

// ============================================================================
// 物件專用 Components
// ============================================================================

/// 地形移動花費
#[derive(Debug, Clone, Component)]
pub struct TerrainMovementCost(pub MovementCost);

/// HP 修正量（正數為增益，負數為減益）
#[derive(Debug, Clone, Component)]
pub struct HpModify(pub i32);

define_tag_components!(BlocksSight, BlocksSound);

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
    pub faction: Faction,
    pub skills: Skills,
    pub attributes: AttributeBundle,
}

/// 物件 Entity 的完整 Bundle
#[derive(Debug, Bundle)]
pub struct ObjectBundle {
    pub object: Object,
    pub position: Position,
    pub occupant: Occupant,
    pub occupant_type_name: OccupantTypeName,
    pub terrain_movement_cost: TerrainMovementCost,
    pub hp_modify: HpModify,
    // block sight
    // block sound
}

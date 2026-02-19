//! ECS Component 定義

use crate::alias::{Coord, ID};
use bevy_ecs::component::Component;
use serde::{Deserialize, Serialize};

/// 棋盤
#[derive(Debug, Clone, Copy, Component)]
pub struct Board {
    pub width: Coord,
    pub height: Coord,
}

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

/// 位置上的佔據者（單位或物件）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub enum Occupant {
    Unit(ID),
    Object(ID),
}

/// 陣營（用於區分友軍/敵軍）
#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Faction(pub ID);

// ============================================================================
// 屬性 Components（Attribute Components）
// ============================================================================

/// 生成屬性 components 的 macro
macro_rules! define_attribute_components {
    ($($name:ident),* $(,)?) => {
        $(
            #[doc = concat!("角色屬性 component: ", stringify!($name))]
            #[derive(Debug, Component)]
            pub struct $name(pub i32);
        )*
    };
}

/// 最大 HP
#[derive(Debug, Component)]
pub struct MaxHp(pub i32);

/// 當前 HP
#[derive(Debug, Component)]
pub struct CurrentHp(pub i32);

/// 最大 MP
#[derive(Debug, Component)]
pub struct MaxMp(pub i32);

/// 當前 MP
#[derive(Debug, Component)]
pub struct CurrentMp(pub i32);

// 使用 macro 定義其他 13 個屬性
define_attribute_components!(
    Initiative,
    Hit,
    Evasion,
    Block,
    BlockProtection,
    PhysicalAttack,
    MagicalAttack,
    MagicalDc,
    Fortitude,
    Reflex,
    Will,
    Movement,
    OpportunityAttacks
);

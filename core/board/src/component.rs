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

/// 標記為物件的 Component
#[derive(Debug, Component)]
pub struct Object;

/// 標記為單位的 Component
#[derive(Debug, Component)]
pub struct Unit;

/// 陣營（用於區分友軍/敵軍）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub struct Faction(pub ID);

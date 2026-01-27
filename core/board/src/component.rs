//! ECS Component 定義

use crate::alias::{Coord, ID};
use bevy_ecs::component::Component;

/// 標記地圖所有者
#[derive(Debug, Clone, Copy, Component)]
pub struct MapOwner(pub ID);

/// 棋盤
#[derive(Debug, Clone, Copy, Component)]
pub struct Board {
    pub width: Coord,
    pub height: Coord,
}

/// 棋盤位置（座標）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct Position {
    pub x: Coord,
    pub y: Coord,
}

/// 標記為單位的 Component
#[derive(Debug, Component)]
pub struct Unit;

/// 標記為物件的 Component
#[derive(Debug, Component)]
pub struct Object;

/// 陣營（用於區分友軍/敵軍）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub struct Faction(pub ID);

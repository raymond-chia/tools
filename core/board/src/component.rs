//! ECS Component 定義

use bevy_ecs::component::Component;

pub type Coord = usize;
pub type ID = u32;

/// 棋盤位置（座標）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
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

/// 標記地圖所有者
#[derive(Debug, Clone, Copy, Component)]
pub struct MapOwner(pub ID);

/// 棋盤
#[derive(Debug, Clone, Copy, Component)]
pub struct Board {
    pub width: Coord,
    pub height: Coord,
}

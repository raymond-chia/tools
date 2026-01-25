//! 共用類型定義

pub type Coord = usize;
pub type ID = u32;

/// 移動方向（四方向）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

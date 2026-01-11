use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter};

/// 物件方向
#[derive(Debug, Deserialize, Serialize, Clone, Copy, Default, Display, EnumIter, PartialEq)]
pub enum Orientation {
    #[default]
    Up,
    Down,
    Left,
    Right,
}

/// 物件類型
#[derive(Debug, Deserialize, Serialize, Clone, Display, EnumIter, PartialEq)]
pub enum ObjectType {
    Tree,
    Wall,
    Cliff { orientation: Orientation },
    Pit,
    Tent2 { orientation: Orientation },
    Tent15 { orientation: Orientation },
    Torch { lit: bool },
    Campfire { lit: bool },
}

impl Default for ObjectType {
    fn default() -> Self {
        ObjectType::Tree
    }
}

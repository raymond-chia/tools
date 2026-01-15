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

impl ObjectType {
    /// 檢查物件是否可被點燃
    pub fn is_ignitable(&self) -> bool {
        match self {
            ObjectType::Torch { lit } => !lit,
            ObjectType::Campfire { lit } => !lit,
            _ => false,
        }
    }

    /// 檢查物件是否可被熄滅
    pub fn is_extinguishable(&self) -> bool {
        match self {
            ObjectType::Torch { lit } => *lit,
            ObjectType::Campfire { lit } => *lit,
            _ => false,
        }
    }

    /// 嘗試點燃物件，回傳是否成功
    pub fn try_ignite(&mut self) -> bool {
        match self {
            ObjectType::Torch { lit } if !*lit => {
                *lit = true;
                true
            }
            ObjectType::Campfire { lit } if !*lit => {
                *lit = true;
                true
            }
            _ => false,
        }
    }

    /// 嘗試熄滅物件，回傳是否成功
    pub fn try_extinguish(&mut self) -> bool {
        match self {
            ObjectType::Torch { lit } if *lit => {
                *lit = false;
                true
            }
            ObjectType::Campfire { lit } if *lit => {
                *lit = false;
                true
            }
            _ => false,
        }
    }
}

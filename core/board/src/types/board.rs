use crate::types::{BoardError, Coord, Pos, Result, UnitMap};

/// 戰棋板
#[derive(Debug)]
pub struct Board {
    pub width: Coord,
    pub height: Coord,
    pub units: UnitMap,
}

impl Board {
    /// 創建新的棋盤
    pub fn new(width: Coord, height: Coord) -> Result<Self> {
        match (width, height) {
            (0, _) | (_, 0) => Err(BoardError::InvalidDimensions(width, height).into()),
            _ => Ok(Board {
                width,
                height,
                units: UnitMap::new(),
            }),
        }
    }

    /// 檢查位置是否在棋盤範圍內
    pub fn is_valid_position(&self, pos: Pos) -> bool {
        pos.x < self.width && pos.y < self.height
    }
}

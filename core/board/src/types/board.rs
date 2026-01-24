use crate::types::{Pos, Result, SceneError};

/// 戰棋板
#[derive(Debug)]
pub struct Board {
    width: usize,
    height: usize,
}

impl Board {
    /// 創建新的棋盤
    pub fn new(width: usize, height: usize) -> Result<Self> {
        match (width, height) {
            (0, _) | (_, 0) => Err(SceneError::InvalidDimensions(
                format!("棋盤維度必須大於 0，收到 width={}, height={}", width, height),
            )
            .into()),
            _ => Ok(Board { width, height }),
        }
    }

    /// 檢查位置是否在棋盤範圍內
    pub fn is_valid_position(&self, pos: &Pos) -> bool {
        pos.x < self.width && pos.y < self.height
    }
}
//! 棋盤邏輯

use crate::ecs_types::components::Position;
use crate::ecs_types::resources::Board;

/// 驗證位置是否在棋盤邊界內
pub fn is_valid_position(board: Board, pos: Position) -> bool {
    pos.x < board.width && pos.y < board.height
}

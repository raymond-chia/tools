use crate::types::{Board, BoardError, Pos, Result, UnitId};

/// 在棋盤上添加單位
pub fn add_unit(board: &mut Board, unit_id: UnitId, pos: Pos) -> Result<()> {
    if !board.is_valid_position(pos) {
        return Err(
            BoardError::PositionOutOfBounds(pos.x, pos.y, board.width, board.height).into(),
        );
    }

    board.units.insert(unit_id, pos);
    Ok(())
}

/// 移動單位到新位置
pub fn move_unit(board: &mut Board, unit_id: UnitId, new_pos: Pos) -> Result<Option<(Pos, Pos)>> {
    // 檢查新位置是否有效
    if !board.is_valid_position(new_pos) {
        return Err(BoardError::PositionOutOfBounds(
            new_pos.x,
            new_pos.y,
            board.width,
            board.height,
        )
        .into());
    }

    // 檢查單位是否存在
    let old_pos = match board.units.get_position(unit_id) {
        Some(pos) => pos,
        None => return Err(BoardError::UnitNotFound(unit_id).into()),
    };

    // 移除舊位置，插入新位置
    board.units.remove(unit_id);
    board.units.insert(unit_id, new_pos);

    Ok(Some((old_pos, new_pos)))
}

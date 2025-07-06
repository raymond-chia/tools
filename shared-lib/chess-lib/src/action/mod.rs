use crate::*;

/// 將 from 位置的單位移動到 to 位置
pub fn move_unit(board: &mut Board, from: Pos, to: Pos) -> Result<(), String> {
    // 檢查 from 位置有無單位
    let unit_id = match board.pos_to_unit.get(&from) {
        Some(id) => *id,
        None => return Err(format!("from 位置 {:?} 沒有單位", from)),
    };
    // 檢查 to 位置是否已有單位
    if board.pos_to_unit.contains_key(&to) {
        return Err(format!("to 位置 {:?} 已有單位", to));
    }
    // 更新 pos_to_unit 映射
    board.pos_to_unit.remove(&from);
    board.pos_to_unit.insert(to, unit_id);
    Ok(())
}

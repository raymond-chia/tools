//! 基本資料類型定義

use crate::component::{Occupant, Position};
use crate::error::{BoardError, Result};
use std::collections::HashMap;

/// 雙向 occupant 位置索引
///
/// 同時維護兩個方向的 mapping，確保兩者永遠同步：
/// - `Position → Vec<Occupant>`：查詢某位置有哪些佔據者
/// - `Occupant → Position`：查詢某佔據者在哪個位置
#[derive(Debug, Default)]
pub struct OccupantMap {
    pos_to_occupants: HashMap<Position, Vec<Occupant>>,
    occupant_to_pos: HashMap<Occupant, Position>,
}

impl OccupantMap {
    /// 查詢指定位置的所有佔據者，空格子回傳空切片
    pub fn get_occupants_at(&self, pos: Position) -> &[Occupant] {
        match self.pos_to_occupants.get(&pos) {
            Some(occupants) => occupants.as_slice(),
            None => &[],
        }
    }

    /// 查詢指定佔據者的位置
    pub fn get_position_of(&self, occupant: Occupant) -> Option<Position> {
        self.occupant_to_pos.get(&occupant).copied()
    }

    /// 插入佔據者到指定位置
    ///
    /// 若該佔據者已存在於任何位置，回傳錯誤。
    pub fn insert(&mut self, pos: Position, occupant: Occupant) -> Result<()> {
        if let Some(existing_pos) = self.occupant_to_pos.get(&occupant).copied() {
            return Err(BoardError::OccupantAlreadyExists {
                occupant: format!("{occupant:?}"),
                x: existing_pos.x,
                y: existing_pos.y,
            }
            .into());
        }

        self.pos_to_occupants.entry(pos).or_default().push(occupant);
        self.occupant_to_pos.insert(occupant, pos);
        Ok(())
    }

    /// 移除佔據者（自動從其所在位置移除）
    ///
    /// 若佔據者不存在，不做任何事。
    pub fn remove(&mut self, occupant: Occupant) {
        match self.occupant_to_pos.remove(&occupant) {
            Some(pos) => self.remove_from_pos(pos, occupant),
            None => {}
        }
    }

    // 從指定位置移除佔據者（內部使用，假設 occupant 確實存在於該位置）
    fn remove_from_pos(&mut self, pos: Position, occupant: Occupant) {
        if let Some(occupants) = self.pos_to_occupants.get_mut(&pos) {
            occupants.retain(|o| *o != occupant);
            if occupants.is_empty() {
                self.pos_to_occupants.remove(&pos);
            }
        }
    }
}

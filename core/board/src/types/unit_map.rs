use crate::types::{Pos, UnitId};
use std::collections::HashMap;

/// 雙向單位位置映射
#[derive(Debug)]
pub struct UnitMap {
    unit_to_pos: HashMap<UnitId, Pos>,
    pos_to_unit: HashMap<Pos, UnitId>,
}

impl UnitMap {
    /// 創建新的單位映射
    pub fn new() -> Self {
        Self {
            unit_to_pos: HashMap::new(),
            pos_to_unit: HashMap::new(),
        }
    }

    /// 添加單位到指定位置
    pub fn insert(&mut self, unit_id: UnitId, pos: Pos) {
        self.unit_to_pos.insert(unit_id, pos);
        self.pos_to_unit.insert(pos, unit_id);
    }

    /// 取得單位的位置
    pub fn get_position(&self, unit_id: UnitId) -> Option<Pos> {
        self.unit_to_pos.get(&unit_id).copied()
    }

    /// 取得指定位置的單位
    pub fn get_unit(&self, pos: Pos) -> Option<UnitId> {
        self.pos_to_unit.get(&pos).copied()
    }

    /// 移除單位
    pub fn remove(&mut self, unit_id: UnitId) -> Option<Pos> {
        match self.unit_to_pos.remove(&unit_id) {
            Some(pos) => {
                self.pos_to_unit.remove(&pos);
                Some(pos)
            }
            None => None,
        }
    }
}

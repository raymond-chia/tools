//! battle.rs：
//! - 負責戰鬥流程、回合管理，以及所有專案自訂的戰鬥判定邏輯（如命中、閃避、格擋、傷害計算等）。
//! - 命中機制（HitContext、HitResult、resolve_hit 等）應放於此，並由呼叫端負責所有修正來源的加總與 context 組裝。
//! - 不負責單位屬性衍生值（如先攻、移動力等）的計算。
use crate::*;
use skills_lib::Effect;

/// Burn 效果每回合造成的固定傷害
const BURN_DAMAGE_PER_TURN: i32 = 5;

/// 回合實體：可以是單位或物件
#[derive(Debug, Clone, PartialEq)]
pub enum TurnEntity {
    Unit(UnitID),
    Object(ObjectID),
}

#[derive(Debug, Clone, Default)]
pub struct Battle {
    pub turn_order: Vec<TurnEntity>,
    pub current_turn_index: Option<usize>,
    pub next_turn_index: usize,
}

impl Battle {
    pub fn new(turn_order: Vec<TurnEntity>) -> Self {
        let current_turn_index = if turn_order.len() > 0 { Some(0) } else { None };
        let next_turn_index = if turn_order.len() > 1 { 1 } else { 0 };

        Self {
            turn_order,
            current_turn_index,
            next_turn_index,
        }
    }

    /// 取得當前回合的實體
    pub fn get_current_entity(&self) -> Option<&TurnEntity> {
        self.current_turn_index
            .and_then(|idx| self.turn_order.get(idx))
    }

    /// 從回合順序中移除實體（單位或物件）
    /// 會自動調整 current_turn_index 和 next_turn_index
    pub fn remove_entity_from_turn_order(&mut self, entity: &TurnEntity) {
        let index = match self.turn_order.iter().position(|e| e == entity) {
            Some(index) => index,
            None => return,
        };

        self.turn_order.remove(index);

        if self.turn_order.is_empty() {
            self.current_turn_index = None;
            self.next_turn_index = 0;
            return;
        }

        // 調整 current_turn_index
        match self.current_turn_index {
            Some(current) => {
                if index < current {
                    self.current_turn_index = Some(current - 1);
                } else if index == current {
                    self.current_turn_index = None;
                }
                // index > current: 不需調整
            }
            None => {
                // current 已經是 None，不需調整
            }
        }

        // 調整 next_turn_index
        if index < self.next_turn_index {
            self.next_turn_index -= 1;
        } else if index == self.next_turn_index {
            // next 被移除，保持 index 不變（會指向原本的下一個）
            // 但需要確保不超出範圍
            if self.next_turn_index >= self.turn_order.len() {
                self.next_turn_index = 0;
            }
        }
        // index > next_turn_index: 不需調整
    }

    /// 在 next_turn_index 前插入物件到回合順序
    /// 插入後自動調整 next_turn_index（+1）
    pub fn insert_object_before_next_turn(&mut self, object_id: ObjectID) {
        let insert_idx = self.next_turn_index;
        self.turn_order
            .insert(insert_idx, TurnEntity::Object(object_id));

        // 插入後 next_turn_index 需要 +1（因為在它前面插入了一個元素）
        self.next_turn_index += 1;
    }

    pub fn next_turn(&mut self, board: &mut Board, skill_selection: &mut SkillSelection) {
        if self.turn_order.is_empty() {
            return;
        }

        // 1. 處理當前實體的回合結束效果
        if let Some(current_entity) = self.get_current_entity().cloned() {
            match current_entity {
                TurnEntity::Unit(unit_id) => {
                    if let Some(unit) = board.units.get_mut(&unit_id) {
                        process_status_effects_at_turn_end(unit);
                    }
                }
                TurnEntity::Object(object_id) => {
                    board.object_map.decrease_object_duration(object_id);
                }
            }
        }

        // 2. 切換到下一個實體
        self.current_turn_index = Some(self.next_turn_index);
        self.next_turn_index = (self.next_turn_index + 1) % self.turn_order.len();

        // 3. 移除新實體的過期效果並重置狀態
        //    如果連續多個物件過期，持續檢查下一個
        loop {
            match self.get_current_entity().cloned() {
                None => break, // 沒有實體了
                Some(TurnEntity::Unit(unit_id)) => {
                    if let Some(unit) = board.units.get_mut(&unit_id) {
                        remove_expired_status_effects(unit);
                        unit.moved = 0;
                        unit.has_cast_skill_this_turn = false;
                        unit.reactions_used_this_turn = 0;
                    }
                    break; // 單位處理完成
                }
                Some(TurnEntity::Object(object_id)) => {
                    if remove_object_if_expired(board, object_id) {
                        // 物件已過期並被移除，從 turn_order 中移除
                        self.remove_entity_from_turn_order(&TurnEntity::Object(object_id));
                        // current_turn_index 已被設為 None
                        // 切換到 next
                        self.current_turn_index = Some(self.next_turn_index);
                        self.next_turn_index = (self.next_turn_index + 1) % self.turn_order.len();
                        // 繼續迴圈，檢查新的 current_entity
                    } else {
                        break; // 物件未過期，處理完成
                    }
                }
            }
        }
        skill_selection.select_skill(None);
    }
}

/// 在回合開始時移除過期狀態效果
pub fn remove_expired_status_effects(unit: &mut Unit) {
    // 移除 duration = 0 的效果（保留 -1 永久效果和 duration > 0 的效果）
    unit.status_effects.retain(|effect| {
        let d = effect.duration();
        d != 0 // 保留 d > 0 和 d == -1
    });
}

/// 在回合結束時處理狀態效果
pub fn process_status_effects_at_turn_end(unit: &mut Unit) {
    // 1. 處理持續傷害效果（Burn）
    for effect in &unit.status_effects {
        if let Effect::Burn { .. } = effect {
            unit.hp -= BURN_DAMAGE_PER_TURN;
            if unit.hp < 0 {
                unit.hp = 0;
            }
        }
    }

    // 2. 減少所有狀態效果的 duration（永久效果 -1 不減少）
    for effect in unit.status_effects.iter_mut() {
        effect.decrease_duration();
    }
}

/// 在回合開始時檢查並移除過期物件（duration = 0）
/// 返回是否移除了該物件
pub fn remove_object_if_expired(board: &mut Board, object_id: ObjectID) -> bool {
    // 檢查物件是否存在且過期
    let should_remove = board
        .object_map
        .get(object_id)
        .map(|obj| obj.duration == 0)
        .unwrap_or(false);

    if !should_remove {
        return false;
    }

    // 移除物件
    board.object_map.remove(object_id).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet, HashMap};

    #[test]
    fn test_battle_empty() {
        let battle = Battle::new(vec![]);
        assert_eq!(battle.current_turn_index, None);
        assert_eq!(battle.next_turn_index, 0);
        assert_eq!(battle.get_current_entity(), None);
    }

    #[test]
    fn test_battle_basic() {
        let ids = vec![123, 223, 323];
        let turn_order: Vec<TurnEntity> = ids.iter().map(|&id| TurnEntity::Unit(id)).collect();
        let units = {
            let data = include_str!("../tests/unit.json");
            let v: serde_json::Value = serde_json::from_str(data).unwrap();
            let marker: UnitMarker = serde_json::from_value(v["UnitMarker"].clone()).unwrap();
            let mut template: UnitTemplate =
                serde_json::from_value(v["UnitTemplate"].clone()).unwrap();
            template.skills = BTreeSet::new();
            HashMap::from([
                (123, {
                    let mut unit =
                        Unit::from_template(&marker, &template, &BTreeMap::new()).unwrap();
                    unit.moved = 3;
                    unit.has_cast_skill_this_turn = true;
                    unit.reactions_used_this_turn = 2; // 已使用 2 次 reaction
                    unit
                }),
                (223, {
                    let mut unit =
                        Unit::from_template(&marker, &template, &BTreeMap::new()).unwrap();
                    unit.id = 223;
                    unit
                }),
                (323, {
                    let mut unit =
                        Unit::from_template(&marker, &template, &BTreeMap::new()).unwrap();
                    unit.id = 323;
                    unit
                }),
            ])
        };

        let mut battle = Battle::new(turn_order);
        let mut board = Board {
            units,
            ..Default::default()
        };
        let mut skill_selection = SkillSelection::default();
        skill_selection.select_skill(Some("skill".to_string()));

        // 初始狀態：current = 0 (ids[0]), next = 1 (ids[1])
        assert_eq!(battle.current_turn_index, Some(0));
        assert_eq!(battle.next_turn_index, 1);
        assert_eq!(battle.get_current_entity(), Some(&TurnEntity::Unit(ids[0])));
        assert_eq!(skill_selection.selected_skill, Some("skill".to_string()));
        assert_eq!(board.units.get(&ids[0]).unwrap().moved, 3);
        assert_eq!(
            board.units.get(&ids[0]).unwrap().has_cast_skill_this_turn,
            true
        );

        for i in [1, 2, 0] {
            battle.next_turn(&mut board, &mut skill_selection);
            assert_eq!(battle.get_current_entity(), Some(&TurnEntity::Unit(ids[i])));
            assert_eq!(skill_selection.selected_skill, None);
            assert_eq!(
                board.units.get(&ids[i]).unwrap().moved,
                0,
                "{i}th turn moved distance check"
            );
            assert_eq!(
                board.units.get(&ids[i]).unwrap().has_cast_skill_this_turn,
                false,
                "{i}th turn has cast skill this turn check"
            );
            assert_eq!(
                board.units.get(&ids[i]).unwrap().reactions_used_this_turn,
                0,
                "{i}th turn reactions used check"
            );
        }
    }
}

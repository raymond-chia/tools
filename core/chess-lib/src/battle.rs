//! battle.rs：
//! - 負責戰鬥流程、回合管理，以及所有專案自訂的戰鬥判定邏輯（如命中、閃避、格擋、傷害計算等）。
//! - 命中機制（HitContext、HitResult、resolve_hit 等）應放於此，並由呼叫端負責所有修正來源的加總與 context 組裝。
//! - 不負責單位屬性衍生值（如先攻、移動力等）的計算。
use crate::*;
use skills_lib::Effect;

/// Burn 效果每回合造成的固定傷害
const BURN_DAMAGE_PER_TURN: i32 = 5;

#[derive(Debug, Clone, Default)]
pub struct Battle {
    pub turn_order: Vec<UnitID>,
    pub current_turn_index: usize,
}

impl Battle {
    pub fn new(turn_order: Vec<UnitID>) -> Self {
        Self {
            turn_order,
            current_turn_index: 0,
        }
    }

    pub fn get_current_unit_id(&self) -> Option<&UnitID> {
        self.turn_order.get(self.current_turn_index)
    }

    pub fn next_turn(&mut self, board: &mut Board, skill_selection: &mut SkillSelection) {
        if self.turn_order.is_empty() {
            return;
        }

        // 1. 處理當前單位的回合結束效果（Burn 傷害、duration 減少）
        if let Some(current_unit_id) = self.get_current_unit_id().copied() {
            if let Some(unit) = board.units.get_mut(&current_unit_id) {
                process_status_effects_at_turn_end(unit);
            }
        }

        // 2. 切換到下一個單位
        self.current_turn_index = (self.current_turn_index + 1) % self.turn_order.len();

        // 3. 移除新單位的過期效果並重置狀態
        if let Some(current_unit_id) = self.get_current_unit_id().copied() {
            if let Some(unit) = board.units.get_mut(&current_unit_id) {
                remove_expired_status_effects(unit);
                unit.moved = 0;
                unit.has_cast_skill_this_turn = false;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet, HashMap};

    #[test]
    fn test_battle_empty() {
        let mut battle = Battle::new(vec![]);
        let mut board = Board {
            units: HashMap::new(),
            ..Default::default()
        };
        let mut skill_selection = SkillSelection::default();
        assert_eq!(battle.get_current_unit_id(), None);
        battle.next_turn(&mut board, &mut skill_selection);
        assert_eq!(battle.get_current_unit_id(), None);
    }

    #[test]
    fn test_battle_basic() {
        let ids = vec![123, 223, 323];
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

        let mut battle = Battle::new(ids.clone());
        let mut board = Board {
            units,
            ..Default::default()
        };
        let mut skill_selection = SkillSelection::default();
        skill_selection.select_skill(Some("skill".to_string()));
        assert_eq!(battle.get_current_unit_id(), Some(&ids[0]));
        assert_eq!(skill_selection.selected_skill, Some("skill".to_string()));
        assert_eq!(board.units.get(&ids[0]).unwrap().moved, 3);
        assert_eq!(
            board.units.get(&ids[0]).unwrap().has_cast_skill_this_turn,
            true
        );
        for i in [1, 2, 0] {
            battle.next_turn(&mut board, &mut skill_selection);
            assert_eq!(battle.get_current_unit_id(), Some(&ids[i]));
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
        }
    }
}

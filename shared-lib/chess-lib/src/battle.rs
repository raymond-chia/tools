use crate::*;

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

    pub fn next_turn(&mut self) {
        if self.turn_order.is_empty() {
            return;
        }
        self.current_turn_index = (self.current_turn_index + 1) % self.turn_order.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_battle_empty() {
        let mut battle = Battle::new(vec![]);
        assert_eq!(battle.get_current_unit_id(), None);
        battle.next_turn();
        assert_eq!(battle.get_current_unit_id(), None);
    }

    #[test]
    fn test_battle_basic() {
        let ids = vec![123, 223, 323];
        let mut battle = Battle::new(ids.clone());
        assert_eq!(battle.get_current_unit_id(), Some(&ids[0]));
        battle.next_turn();
        assert_eq!(battle.get_current_unit_id(), Some(&ids[1]));
        battle.next_turn();
        assert_eq!(battle.get_current_unit_id(), Some(&ids[2]));
        battle.next_turn();
        assert_eq!(battle.get_current_unit_id(), Some(&ids[0]));
    }
}

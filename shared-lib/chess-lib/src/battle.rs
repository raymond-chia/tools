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

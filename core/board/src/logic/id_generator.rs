//! 唯一 ID 產生邏輯

use crate::domain::alias::ID;
use rand::random;
use std::collections::HashSet;

/// 從集合中產生唯一的隨機 ID
pub fn generate_unique_id(used_ids: &mut HashSet<ID>) -> ID {
    loop {
        let new_id: ID = random();
        if used_ids.insert(new_id) {
            return new_id;
        }
    }
}

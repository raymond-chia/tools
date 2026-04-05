//! 唯一 ID 產生邏輯

use crate::domain::alias::ID;
use crate::error::{DataError, Result};
use rand::random;
use std::collections::HashSet;

/// 從集合中產生唯一的隨機 ID
pub(crate) fn generate_unique_id(used_ids: &mut HashSet<ID>) -> Result<ID> {
    for _ in 0..1000 {
        let new_id: ID = random();
        if used_ids.insert(new_id) {
            return Ok(new_id);
        }
    }
    Err(DataError::IDGenerationFailed.into())
}

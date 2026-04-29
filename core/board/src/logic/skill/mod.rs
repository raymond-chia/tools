//! 技能邏輯

pub mod line_of_sight;
pub mod skill_check;
pub mod skill_execution;
pub mod skill_range;
pub mod skill_reaction;
pub mod skill_target;
pub mod unit_attributes;

use crate::domain::alias::{Coord, ID};
use crate::domain::core_types::TargetFilter;
use crate::ecs_types::components::{Occupant, Position};
use crate::error::{BoardError, Result};

/// 場上單位資訊（純資料，不依賴 ECS）
#[derive(Debug, Clone)]
pub struct UnitInfo {
    pub occupant: Occupant,
    pub faction_id: ID,
    pub alliance_id: ID,
}

/// 施放者資訊
#[derive(Debug, Clone)]
pub struct CasterInfo {
    pub position: Position,
    pub unit_info: UnitInfo,
}

/// 曼哈頓距離
pub(crate) fn manhattan_distance(a: Position, b: Position) -> Coord {
    let dx = (a.x as i32 - b.x as i32).unsigned_abs() as Coord;
    let dy = (a.y as i32 - b.y as i32).unsigned_abs() as Coord;
    dx + dy
}

/// 檢查單位是否通過篩選條件
pub(crate) fn is_in_filter(caster: &UnitInfo, target: &UnitInfo, filter: TargetFilter) -> bool {
    let is_caster = target.occupant == caster.occupant;
    let is_same_alliance = target.alliance_id == caster.alliance_id;

    match filter {
        TargetFilter::Any => true,
        TargetFilter::AnyExceptCaster => !is_caster,
        TargetFilter::Enemy => !is_same_alliance,
        TargetFilter::Ally => is_same_alliance,
        TargetFilter::AllyExceptCaster => is_same_alliance && !is_caster,
        TargetFilter::CasterOnly => is_caster,
    }
}

// ============================================================================
// 工具函數
// ============================================================================

/// 將方向向量正規化為單位步進（僅支援正交方向）
/// signum 將任意正負值壓縮為 1 或 -1
pub(crate) fn normalize_direction(caster: Position, target: Position) -> Result<(i32, i32)> {
    let dx = (target.x as i32) - (caster.x as i32);
    let dy = (target.y as i32) - (caster.y as i32);

    match (dx != 0, dy != 0) {
        (true, false) => Ok((dx.signum(), 0)),
        (false, true) => Ok((0, dy.signum())),
        _ => Err(BoardError::InvalidSkillTarget {
            shape: "Line".to_string(),
            caster_x: caster.x,
            caster_y: caster.y,
            target_x: target.x,
            target_y: target.y,
        }
        .into()),
    }
}

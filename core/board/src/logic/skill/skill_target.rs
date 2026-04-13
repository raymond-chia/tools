use crate::domain::alias::Coord;
use crate::domain::core_types::{Area, Target, TargetFilter, TargetSelection};
use crate::ecs_types::components::Position;
use crate::ecs_types::resources::Board;
use crate::error::{BoardError, Result};
use crate::logic::board;
use crate::logic::skill::{
    CasterInfo, UnitInfo, is_in_filter, manhattan_distance, normalize_direction,
};
use std::collections::{HashMap, HashSet};

/// 驗證技能目標是否合法
/// targets: 玩家在 UI 上選擇的目標位置
///   - 最多 count 個目標
pub(crate) fn validate_skill_targets(
    caster: &CasterInfo,
    target: &Target,
    target_positions: &[Position],
    units_on_board: &HashMap<Position, UnitInfo>,
    board: Board,
) -> Result<()> {
    let (min_range, max_range) = target.range;
    let filter = target.selectable_filter;
    let is_targeting_unit = match target.selection {
        TargetSelection::Unit => true,
        TargetSelection::Ground => false,
    };

    if target_positions.len() == 0 || target_positions.len() > target.count {
        return Err(BoardError::WrongTargetCount {
            expected: target.count,
            actual: target_positions.len(),
        }
        .into());
    }

    if !target.allow_same_target {
        validate_no_duplicates(target_positions)?;
    }

    for &target_pos in target_positions {
        validate_range(caster.position, target_pos, min_range, max_range, board)?;
        if let Area::Line { .. } = target.area {
            normalize_direction(caster.position, target_pos)?;
        }
        if is_targeting_unit {
            let target_unit = get_unit_at(target_pos, units_on_board)?;
            validate_filter(caster, target_unit, target_pos, filter)?;
        }
    }

    Ok(())
}

/// 檢查目標位置無重複
fn validate_no_duplicates(targets: &[Position]) -> Result<()> {
    let mut seen = HashSet::new();
    let has_duplicate = targets.iter().any(|pos| !seen.insert(pos));
    if has_duplicate {
        let targets: Vec<(Coord, Coord)> = targets.iter().map(|pos| (pos.x, pos.y)).collect();
        return Err(BoardError::DuplicateTarget { targets }.into());
    }
    Ok(())
}

/// 驗證射程（曼哈頓距離）
fn validate_range(
    caster_pos: Position,
    target_pos: Position,
    min_range: Coord,
    max_range: Coord,
    board: Board,
) -> Result<()> {
    if !board::is_valid_position(board, target_pos) {
        return Err(BoardError::OutOfBoard {
            x: target_pos.x,
            y: target_pos.y,
            width: board.width,
            height: board.height,
        }
        .into());
    }
    let distance = manhattan_distance(caster_pos, target_pos);
    if distance < min_range || distance > max_range {
        return Err(BoardError::OutOfRange {
            distance,
            min_range,
            max_range,
        }
        .into());
    }
    Ok(())
}

/// 要求目標位置有單位
fn get_unit_at<'a>(
    pos: Position,
    units_on_board: &'a HashMap<Position, UnitInfo>,
) -> Result<&'a UnitInfo> {
    units_on_board
        .get(&pos)
        .ok_or_else(|| BoardError::NoUnitAtTarget { x: pos.x, y: pos.y }.into())
}

/// 驗證篩選條件（回傳 Result）
fn validate_filter(
    caster: &CasterInfo,
    target: &UnitInfo,
    target_pos: Position,
    filter: TargetFilter,
) -> Result<()> {
    if is_in_filter(&caster.unit_info, target, filter) {
        Ok(())
    } else {
        Err(BoardError::TargetFilterMismatch {
            x: target_pos.x,
            y: target_pos.y,
            filter: format!("{:?}", filter),
        }
        .into())
    }
}

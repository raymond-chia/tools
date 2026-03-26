//! 技能邏輯

use crate::domain::alias::{Coord, ID};
use crate::domain::core_types::{Area, Target, TargetFilter, TargetSelection};
use crate::ecs_types::components::{Occupant, Position};
use crate::ecs_types::resources::Board;
use crate::error::{BoardError, Result};
use crate::logic::board;
use std::collections::{HashMap, HashSet};

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

/// 驗證並解析技能目標
/// targets: 玩家在 UI 上選擇的目標位置
///   - count=1 + Single: 單一目標
///   - count>1 + Single: 多目標
///   - area != Single: AOE
pub fn select_skill_targets(
    caster: &CasterInfo,
    target_def: &Target,
    targets: &[Position],
    units_on_board: &HashMap<Position, UnitInfo>,
    board_size: Board,
) -> Result<Vec<Occupant>> {
    let (min_range, max_range) = target_def.range;
    let filter = &target_def.selectable_filter;
    let targets_unit = match target_def.selection {
        TargetSelection::Unit => true,
        TargetSelection::Ground => false,
    };

    match target_def.area {
        Area::Single => {
            let count = target_def.count;
            if count == 1 {
                validate_target_count(targets, 1)?;
                let target_pos = targets[0];
                validate_range(caster.position, target_pos, min_range, max_range)?;
                if targets_unit {
                    let target_unit = get_unit_at(target_pos, units_on_board)?;
                    validate_filter(caster, target_unit, target_pos, filter)?;
                    Ok(vec![target_unit.occupant])
                } else {
                    Ok(Vec::new())
                }
            } else {
                if targets.len() > count {
                    return Err(BoardError::WrongTargetCount {
                        expected: count,
                        actual: targets.len(),
                    }
                    .into());
                }
                if !target_def.allow_same_target {
                    validate_no_duplicates(targets)?;
                }
                let mut result = Vec::new();
                if targets_unit {
                    for &target_pos in targets {
                        validate_range(caster.position, target_pos, min_range, max_range)?;
                        let target_unit = get_unit_at(target_pos, units_on_board)?;
                        validate_filter(caster, target_unit, target_pos, filter)?;
                        result.push(target_unit.occupant);
                    }
                }
                Ok(result)
            }
        }
        Area::Diamond { .. } | Area::Cross { .. } | Area::Line { .. } => {
            validate_target_count(targets, 1)?;
            let target_pos = targets[0];
            validate_range(caster.position, target_pos, min_range, max_range)?;
            resolve_area_targets(
                caster,
                target_pos,
                units_on_board,
                board_size,
                &target_def.area,
                targets_unit,
                filter,
            )
        }
    }
}

/// 計算 AOE 影響的所有位置
/// - Single: 回傳該格
/// - Diamond/Cross: 以 target 為中心，忽略 caster
/// - Line: 以 caster→target 方向延伸
pub fn compute_affected_positions(
    area: &Area,
    caster: Position,
    target: Position,
    board_size: Board,
) -> Result<Vec<Position>> {
    match area {
        Area::Single => Ok(vec![target]),
        Area::Diamond { radius } => Ok(compute_diamond(target, *radius, board_size)),
        Area::Cross { length } => Ok(compute_cross(target, *length, board_size)),
        Area::Line { length } => compute_line(caster, target, *length, board_size),
    }
}

// ============================================================================
// 挑選目標
// ============================================================================

/// 處理 Area 類型的目標選擇
fn resolve_area_targets(
    caster: &CasterInfo,
    target_pos: Position,
    units_on_board: &HashMap<Position, UnitInfo>,
    board_size: Board,
    area: &Area,
    targets_unit: bool,
    filter: &TargetFilter,
) -> Result<Vec<Occupant>> {
    // 根據 targets_unit 檢查目標位置，並驗證 filter
    let target_unit = get_unit_at(target_pos, units_on_board);
    if targets_unit {
        match target_unit {
            Ok(target) => validate_filter(caster, target, target_pos, filter)?,
            Err(_) => {
                return Err(BoardError::NoUnitAtTarget {
                    x: target_pos.x,
                    y: target_pos.y,
                }
                .into());
            }
        }
    }

    let aoe_positions = compute_affected_positions(area, caster.position, target_pos, board_size)?;

    // 收集 AOE 範圍內通過篩選的單位
    let filtered: Vec<Occupant> = aoe_positions
        .iter()
        .filter_map(|pos| {
            let unit = units_on_board.get(pos)?;
            if is_in_filter(caster, unit, filter) {
                Some(unit.occupant)
            } else {
                None
            }
        })
        .collect();

    // targets_unit=true 時，至少需要一個單位通過篩選
    if targets_unit && filtered.is_empty() {
        return Err(BoardError::TargetFilterMismatch {
            x: target_pos.x,
            y: target_pos.y,
            filter: format!("{:?}", filter),
        }
        .into());
    }

    Ok(filtered)
}

/// 驗證目標數量
fn validate_target_count(targets: &[Position], expected: usize) -> Result<()> {
    if targets.len() != expected {
        return Err(BoardError::WrongTargetCount {
            expected,
            actual: targets.len(),
        }
        .into());
    }
    Ok(())
}

/// 驗證射程（曼哈頓距離）
fn validate_range(
    caster_pos: Position,
    target_pos: Position,
    min_range: Coord,
    max_range: Coord,
) -> Result<()> {
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

/// 曼哈頓距離
pub(crate) fn manhattan_distance(a: Position, b: Position) -> Coord {
    let dx = (a.x as i32 - b.x as i32).unsigned_abs() as Coord;
    let dy = (a.y as i32 - b.y as i32).unsigned_abs() as Coord;
    dx + dy
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
    filter: &TargetFilter,
) -> Result<()> {
    if is_in_filter(caster, target, filter) {
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

/// 檢查單位是否通過篩選條件
fn is_in_filter(caster: &CasterInfo, target: &UnitInfo, filter: &TargetFilter) -> bool {
    let is_caster = target.occupant == caster.unit_info.occupant;
    let is_same_alliance = target.alliance_id == caster.unit_info.alliance_id;

    match filter {
        TargetFilter::Any => true,
        TargetFilter::AnyExceptCaster => !is_caster,
        TargetFilter::Enemy => !is_same_alliance,
        TargetFilter::Ally => is_same_alliance,
        TargetFilter::AllyExceptCaster => is_same_alliance && !is_caster,
        TargetFilter::CasterOnly => is_caster,
    }
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

// ============================================================================
// AOE 計算
// ============================================================================

/// 計算菱形 AOE（曼哈頓距離）
fn compute_diamond(target: Position, radius: Coord, board_size: Board) -> Vec<Position> {
    let mut positions = Vec::new();
    let target_x = target.x as i32;
    let target_y = target.y as i32;
    let radius = radius as i32;

    for dy in -(radius)..=radius {
        for dx in -(radius)..=radius {
            if (dx.abs() + dy.abs()) > radius {
                continue;
            }
            let x = target_x + dx;
            let y = target_y + dy;

            if let Some(pos) = board::try_position(board_size, x, y) {
                positions.push(pos);
            }
        }
    }

    positions
}

/// 計算十字形 AOE
fn compute_cross(target: Position, length: Coord, board_size: Board) -> Vec<Position> {
    let mut positions = vec![target];
    let target_x = target.x as i32;
    let target_y = target.y as i32;
    let length = length as i32;

    const DIRECTIONS: [(i32, i32); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];

    for (step_x, step_y) in DIRECTIONS {
        for i in 1..=length {
            let x = target_x + step_x * i;
            let y = target_y + step_y * i;
            match board::try_position(board_size, x, y) {
                Some(pos) => positions.push(pos),
                // 邊界裁切：遇到無效位置就停止該方向的延伸
                None => break,
            }
        }
    }

    positions
}

/// 計算直線 AOE（從施放者到目標方向，長度為 length 格）
fn compute_line(
    caster: Position,
    target: Position,
    length: Coord,
    board_size: Board,
) -> Result<Vec<Position>> {
    let dx = (target.x as i32) - (caster.x as i32);
    let dy = (target.y as i32) - (caster.y as i32);

    let (step_x, step_y) =
        normalize_direction(dx, dy).ok_or_else(|| BoardError::InvalidSkillTarget {
            shape: "Line".to_string(),
            caster_x: caster.x,
            caster_y: caster.y,
            target_x: target.x,
            target_y: target.y,
        })?;

    let mut positions = Vec::new();
    let caster_x = caster.x as i32;
    let caster_y = caster.y as i32;
    let length = length as i32;

    for i in 0..=length {
        let x = caster_x + step_x * i;
        let y = caster_y + step_y * i;
        match board::try_position(board_size, x, y) {
            Some(pos) => positions.push(pos),
            // 邊界裁切：遇到無效位置就停止延伸
            None => break,
        }
    }

    Ok(positions)
}

/// 將方向向量正規化為單位步進（僅支援正交方向）
/// signum 將任意正負值壓縮為 1 或 -1
fn normalize_direction(dx: i32, dy: i32) -> Option<(i32, i32)> {
    match (dx != 0, dy != 0) {
        (true, false) => Some((dx.signum(), 0)),
        (false, true) => Some((0, dy.signum())),
        _ => None,
    }
}

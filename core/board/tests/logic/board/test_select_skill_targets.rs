//! select_skill_targets 測試

use crate::helpers::level_builder::{LevelBuilder, MarkerEntry};
use board::domain::alias::Coord;
use board::domain::alias::ID;
use board::domain::core_types::{Area, Target, TargetFilter, TargetSelection};
use board::ecs_types::components::{Occupant, Position};
use board::ecs_types::resources::Board;
use board::error::Result;
use board::logic::skill::{CasterInfo, UnitInfo, select_skill_targets};
use std::collections::{HashMap, HashSet};
use strum::IntoEnumIterator;

const PLAYER_FACTION: ID = 0;
const ALLY_FACTION: ID = 1;
const ENEMY_FACTION: ID = 2;

/// 標準棋盤建構：C=施放者(player), Pt/Pa/Pn=玩家, At/Aa/An=友軍, Et/Ea/En=敵軍
fn standard_board(
    ascii: &str,
) -> Result<(
    Board,
    HashMap<String, Vec<Position>>,
    HashMap<String, Vec<MarkerEntry>>,
)> {
    LevelBuilder::from_ascii(ascii)
        .unit("C", "caster", PLAYER_FACTION)
        .unit("Pa", "player", PLAYER_FACTION)
        .unit("Pb", "player", PLAYER_FACTION)
        .unit("Aa", "ally", ALLY_FACTION)
        .unit("Ab", "ally", ALLY_FACTION)
        .unit("Ea", "enemy", ENEMY_FACTION)
        .unit("Eb", "enemy", ENEMY_FACTION)
        .to_unit_map()
}

fn skill_with(
    range: (Coord, Coord),
    selection: TargetSelection,
    filter: TargetFilter,
    count: usize,
    allow_same_target: bool,
    area: Area,
) -> Target {
    Target {
        range,
        selection,
        selectable_filter: filter,
        count,
        allow_same_target,
        area,
    }
}

fn skill_with_unit_target(
    filter: TargetFilter,
    count: usize,
    allow_same_target: bool,
    area: Area,
) -> Target {
    skill_with(
        (0, 2),
        TargetSelection::Unit,
        filter,
        count,
        allow_same_target,
        area,
    )
}

fn skill_with_ground_target(
    filter: TargetFilter,
    count: usize,
    allow_same_target: bool,
    area: Area,
) -> Target {
    skill_with(
        (0, 2),
        TargetSelection::Ground,
        filter,
        count,
        allow_same_target,
        area,
    )
}

/// 從 marker map 建立 select_skill_targets 需要的 HashMap<Position, UnitInfo>
fn to_position_map(marker_map: &HashMap<String, Vec<MarkerEntry>>) -> HashMap<Position, UnitInfo> {
    marker_map
        .values()
        .flatten()
        .map(|entry| (entry.position, entry.unit_info.clone()))
        .collect()
}

/// 從 marker map 取得指定 marker 的所有位置
fn all_positions_of(marker_map: &HashMap<String, Vec<MarkerEntry>>, marker: &str) -> Vec<Position> {
    marker_map[marker]
        .iter()
        .map(|entry| entry.position)
        .collect()
}

/// 從 marker map 取得指定 marker 的所有 Occupant
fn all_occupants_of(marker_map: &HashMap<String, Vec<MarkerEntry>>, marker: &str) -> Vec<Occupant> {
    marker_map[marker]
        .iter()
        .map(|entry| entry.unit_info.occupant)
        .collect()
}

#[test]
fn test_select_skill_targets_normal_case() {
    let skill_with_fixed_range =
        |filter: TargetFilter, area: Area| skill_with_unit_target(filter, 1, false, area);

    let test_data = [
        (
            "--- SingleTarget",
            vec![
                r#"
                    C  Ea Eb
                    Pa Aa Ab
                    Pb . .
                    "#,
                r#"
                    C  Pa Eb
                    Aa Pb .
                    Ea Ab .
                    "#,
            ],
            vec![
                // 方便起見，只在這邊測試技能距離
                (
                    skill_with(
                        (1, 1),
                        TargetSelection::Unit,
                        TargetFilter::Any,
                        1,
                        false,
                        Area::Single,
                    ),
                    vec![
                        // 太近
                        (vec!["C"], None),
                        (vec!["Pa"], Some(vec!["Pa"])),
                        // 太遠
                        (vec!["Pb"], None),
                        // 太遠
                        (vec!["Ab"], None),
                    ],
                ),
                (
                    skill_with_fixed_range(TargetFilter::Any, Area::Single),
                    vec![
                        (vec!["C"], Some(vec!["C"])),
                        (vec!["Pa"], Some(vec!["Pa"])),
                        (vec!["Pb"], Some(vec!["Pb"])),
                        (vec!["Aa"], Some(vec!["Aa"])),
                        (vec!["Ab"], None),
                        (vec!["Ea"], Some(vec!["Ea"])),
                        (vec!["Eb"], Some(vec!["Eb"])),
                    ],
                ),
                (
                    skill_with_fixed_range(TargetFilter::AnyExceptCaster, Area::Single),
                    vec![
                        (vec!["C"], None),
                        (vec!["Pa"], Some(vec!["Pa"])),
                        (vec!["Pb"], Some(vec!["Pb"])),
                        (vec!["Aa"], Some(vec!["Aa"])),
                        (vec!["Ab"], None),
                        (vec!["Ea"], Some(vec!["Ea"])),
                        (vec!["Eb"], Some(vec!["Eb"])),
                    ],
                ),
                (
                    skill_with_fixed_range(TargetFilter::Enemy, Area::Single),
                    vec![
                        (vec!["C"], None),
                        (vec!["Pa"], None),
                        (vec!["Pb"], None),
                        (vec!["Aa"], None),
                        (vec!["Ab"], None),
                        (vec!["Ea"], Some(vec!["Ea"])),
                        (vec!["Eb"], Some(vec!["Eb"])),
                    ],
                ),
                (
                    skill_with_fixed_range(TargetFilter::Ally, Area::Single),
                    vec![
                        (vec!["C"], Some(vec!["C"])),
                        (vec!["Pa"], Some(vec!["Pa"])),
                        (vec!["Pb"], Some(vec!["Pb"])),
                        (vec!["Aa"], Some(vec!["Aa"])),
                        (vec!["Ab"], None),
                        (vec!["Ea"], None),
                        (vec!["Eb"], None),
                    ],
                ),
                (
                    skill_with_fixed_range(TargetFilter::AllyExceptCaster, Area::Single),
                    vec![
                        (vec!["C"], None),
                        (vec!["Pa"], Some(vec!["Pa"])),
                        (vec!["Pb"], Some(vec!["Pb"])),
                        (vec!["Aa"], Some(vec!["Aa"])),
                        (vec!["Ab"], None),
                        (vec!["Ea"], None),
                        (vec!["Eb"], None),
                    ],
                ),
                (
                    skill_with_fixed_range(TargetFilter::CasterOnly, Area::Single),
                    vec![
                        (vec!["C"], Some(vec!["C"])),
                        (vec!["Pa"], None),
                        (vec!["Pb"], None),
                        (vec!["Aa"], None),
                        (vec!["Ab"], None),
                        (vec!["Ea"], None),
                        (vec!["Eb"], None),
                    ],
                ),
            ],
        ),
        (
            "--- MultiTarget",
            vec![
                r#"
                    C  Ea Eb
                    Pa Aa Ab
                    Pb . .
                    "#,
                r#"
                    C  Pa Eb
                    Aa Pb .
                    Ea Ab .
                    "#,
            ],
            vec![
                (
                    skill_with(
                        (0, 2),
                        TargetSelection::Unit,
                        TargetFilter::Any,
                        2,
                        false,
                        Area::Single,
                    ),
                    vec![
                        (vec!["C"], Some(vec!["C"])),
                        (vec!["Pa"], Some(vec!["Pa"])),
                        (vec!["Pb"], Some(vec!["Pb"])),
                        (vec!["Aa"], Some(vec!["Aa"])),
                        (vec!["Ab"], None),
                        (vec!["Ea"], Some(vec!["Ea"])),
                        (vec!["Eb"], Some(vec!["Eb"])),
                        (vec!["C", "Pa"], Some(vec!["C", "Pa"])),
                        (vec!["C", "Aa"], Some(vec!["C", "Aa"])),
                        (vec!["C", "Ea"], Some(vec!["C", "Ea"])),
                        (vec!["Pa", "Aa"], Some(vec!["Pa", "Aa"])),
                        (vec!["Pa", "Ea"], Some(vec!["Pa", "Ea"])),
                        (vec!["Aa", "Ea"], Some(vec!["Aa", "Ea"])),
                        (vec!["Ea", "Eb"], Some(vec!["Ea", "Eb"])),
                        (vec!["C", "Pa", "Aa"], None),
                    ],
                ),
                (
                    skill_with(
                        (0, 2),
                        TargetSelection::Unit,
                        TargetFilter::Any,
                        3,
                        false,
                        Area::Single,
                    ),
                    vec![
                        (vec!["C"], Some(vec!["C"])),
                        (vec!["Pa"], Some(vec!["Pa"])),
                        (vec!["Pb"], Some(vec!["Pb"])),
                        (vec!["Aa"], Some(vec!["Aa"])),
                        (vec!["Ab"], None),
                        (vec!["Ea"], Some(vec!["Ea"])),
                        (vec!["Eb"], Some(vec!["Eb"])),
                        (vec!["C", "Pa"], Some(vec!["C", "Pa"])),
                        (vec!["C", "Aa"], Some(vec!["C", "Aa"])),
                        (vec!["C", "Ea"], Some(vec!["C", "Ea"])),
                        (vec!["Pa", "Aa"], Some(vec!["Pa", "Aa"])),
                        (vec!["Pa", "Ea"], Some(vec!["Pa", "Ea"])),
                        (vec!["Aa", "Ea"], Some(vec!["Aa", "Ea"])),
                        (vec!["Ea", "Eb"], Some(vec!["Ea", "Eb"])),
                        (vec!["C", "Pa", "Aa"], Some(vec!["C", "Pa", "Aa"])),
                    ],
                ),
                (
                    skill_with(
                        (0, 2),
                        TargetSelection::Unit,
                        TargetFilter::AnyExceptCaster,
                        2,
                        false,
                        Area::Single,
                    ),
                    vec![
                        (vec!["C"], None),
                        (vec!["Pa"], Some(vec!["Pa"])),
                        (vec!["Pb"], Some(vec!["Pb"])),
                        (vec!["Aa"], Some(vec!["Aa"])),
                        (vec!["Ab"], None),
                        (vec!["Ea"], Some(vec!["Ea"])),
                        (vec!["Eb"], Some(vec!["Eb"])),
                        (vec!["C", "Pa"], None),
                        (vec!["C", "Aa"], None),
                        (vec!["C", "Ea"], None),
                        (vec!["Pa", "Aa"], Some(vec!["Pa", "Aa"])),
                        (vec!["Pa", "Ea"], Some(vec!["Pa", "Ea"])),
                        (vec!["Aa", "Ea"], Some(vec!["Aa", "Ea"])),
                        (vec!["Ea", "Eb"], Some(vec!["Ea", "Eb"])),
                        (vec!["C", "Pa", "Aa"], None),
                    ],
                ),
                (
                    skill_with(
                        (0, 2),
                        TargetSelection::Unit,
                        TargetFilter::AnyExceptCaster,
                        3,
                        false,
                        Area::Single,
                    ),
                    vec![
                        (vec!["C"], None),
                        (vec!["Pa"], Some(vec!["Pa"])),
                        (vec!["Pb"], Some(vec!["Pb"])),
                        (vec!["Aa"], Some(vec!["Aa"])),
                        (vec!["Ab"], None),
                        (vec!["Ea"], Some(vec!["Ea"])),
                        (vec!["Eb"], Some(vec!["Eb"])),
                        (vec!["C", "Pa"], None),
                        (vec!["C", "Aa"], None),
                        (vec!["C", "Ea"], None),
                        (vec!["Pa", "Aa"], Some(vec!["Pa", "Aa"])),
                        (vec!["Pa", "Ea"], Some(vec!["Pa", "Ea"])),
                        (vec!["Aa", "Ea"], Some(vec!["Aa", "Ea"])),
                        (vec!["Ea", "Eb"], Some(vec!["Ea", "Eb"])),
                        (vec!["C", "Pa", "Aa"], None),
                    ],
                ),
                (
                    skill_with(
                        (0, 2),
                        TargetSelection::Unit,
                        TargetFilter::Enemy,
                        2,
                        false,
                        Area::Single,
                    ),
                    vec![
                        (vec!["C"], None),
                        (vec!["Pa"], None),
                        (vec!["Pb"], None),
                        (vec!["Aa"], None),
                        (vec!["Ab"], None),
                        (vec!["Ea"], Some(vec!["Ea"])),
                        (vec!["Eb"], Some(vec!["Eb"])),
                        (vec!["C", "Pa"], None),
                        (vec!["C", "Aa"], None),
                        (vec!["C", "Ea"], None),
                        (vec!["Pa", "Aa"], None),
                        (vec!["Pa", "Ea"], None),
                        (vec!["Aa", "Ea"], None),
                        (vec!["Ea", "Eb"], Some(vec!["Ea", "Eb"])),
                        (vec!["C", "Pa", "Aa"], None),
                    ],
                ),
                (
                    skill_with(
                        (0, 2),
                        TargetSelection::Unit,
                        TargetFilter::Ally,
                        2,
                        false,
                        Area::Single,
                    ),
                    vec![
                        (vec!["C"], Some(vec!["C"])),
                        (vec!["Pa"], Some(vec!["Pa"])),
                        (vec!["Pb"], Some(vec!["Pb"])),
                        (vec!["Aa"], Some(vec!["Aa"])),
                        (vec!["Ab"], None),
                        (vec!["Ea"], None),
                        (vec!["Eb"], None),
                        (vec!["C", "Pa"], Some(vec!["C", "Pa"])),
                        (vec!["C", "Aa"], Some(vec!["C", "Aa"])),
                        (vec!["C", "Ea"], None),
                        (vec!["Pa", "Aa"], Some(vec!["Pa", "Aa"])),
                        (vec!["Pa", "Ea"], None),
                        (vec!["Aa", "Ea"], None),
                        (vec!["Ea", "Eb"], None),
                        (vec!["C", "Pa", "Aa"], None),
                    ],
                ),
                (
                    skill_with(
                        (0, 2),
                        TargetSelection::Unit,
                        TargetFilter::Ally,
                        3,
                        false,
                        Area::Single,
                    ),
                    vec![
                        (vec!["C"], Some(vec!["C"])),
                        (vec!["Pa"], Some(vec!["Pa"])),
                        (vec!["Pb"], Some(vec!["Pb"])),
                        (vec!["Aa"], Some(vec!["Aa"])),
                        (vec!["Ab"], None),
                        (vec!["Ea"], None),
                        (vec!["Eb"], None),
                        (vec!["C", "Pa"], Some(vec!["C", "Pa"])),
                        (vec!["C", "Aa"], Some(vec!["C", "Aa"])),
                        (vec!["C", "Ea"], None),
                        (vec!["Pa", "Aa"], Some(vec!["Pa", "Aa"])),
                        (vec!["Pa", "Ea"], None),
                        (vec!["Aa", "Ea"], None),
                        (vec!["Ea", "Eb"], None),
                        (vec!["C", "Pa", "Aa"], Some(vec!["C", "Pa", "Aa"])),
                    ],
                ),
                (
                    skill_with(
                        (0, 2),
                        TargetSelection::Unit,
                        TargetFilter::AllyExceptCaster,
                        2,
                        false,
                        Area::Single,
                    ),
                    vec![
                        (vec!["C"], None),
                        (vec!["Pa"], Some(vec!["Pa"])),
                        (vec!["Pb"], Some(vec!["Pb"])),
                        (vec!["Aa"], Some(vec!["Aa"])),
                        (vec!["Ab"], None),
                        (vec!["Ea"], None),
                        (vec!["Eb"], None),
                        (vec!["C", "Pa"], None),
                        (vec!["C", "Aa"], None),
                        (vec!["C", "Ea"], None),
                        (vec!["Pa", "Aa"], Some(vec!["Pa", "Aa"])),
                        (vec!["Pa", "Ea"], None),
                        (vec!["Aa", "Ea"], None),
                        (vec!["Ea", "Eb"], None),
                        (vec!["C", "Pa", "Aa"], None),
                    ],
                ),
                (
                    skill_with(
                        (0, 2),
                        TargetSelection::Unit,
                        TargetFilter::AllyExceptCaster,
                        3,
                        false,
                        Area::Single,
                    ),
                    vec![
                        (vec!["C"], None),
                        (vec!["Pa"], Some(vec!["Pa"])),
                        (vec!["Pb"], Some(vec!["Pb"])),
                        (vec!["Aa"], Some(vec!["Aa"])),
                        (vec!["Ab"], None),
                        (vec!["Ea"], None),
                        (vec!["Eb"], None),
                        (vec!["C", "Pa"], None),
                        (vec!["C", "Aa"], None),
                        (vec!["C", "Ea"], None),
                        (vec!["Pa", "Aa"], Some(vec!["Pa", "Aa"])),
                        (vec!["Pa", "Ea"], None),
                        (vec!["Aa", "Ea"], None),
                        (vec!["Ea", "Eb"], None),
                        (vec!["C", "Pa", "Aa"], None),
                    ],
                ),
            ],
        ),
        (
            "--- Diamond",
            vec![
                r#"
                    C  Ea Eb
                    Pa Aa Ab
                    Pb . .
                    "#,
            ],
            vec![
                // radius 0 不合法，不測試
                (
                    skill_with_fixed_range(TargetFilter::Any, Area::Diamond { radius: 1 }),
                    vec![
                        (vec!["C"], Some(vec!["C", "Ea", "Pa"])),
                        (vec!["Pa"], Some(vec!["Pa", "C", "Aa", "Pb"])),
                        (vec!["Pb"], Some(vec!["Pb", "Pa"])),
                        (vec!["Aa"], Some(vec!["Aa", "Ea", "Pa", "Ab"])),
                        (vec!["Ab"], None),
                        (vec!["Ea"], Some(vec!["Ea", "C", "Eb", "Aa"])),
                        (vec!["Eb"], Some(vec!["Eb", "Ea", "Ab"])),
                    ],
                ),
                // 方便起見，只測試這個 diamond radius 2
                (
                    skill_with_fixed_range(TargetFilter::Any, Area::Diamond { radius: 2 }),
                    vec![
                        (vec!["C"], Some(vec!["C", "Ea", "Eb", "Pa", "Aa", "Pb"])),
                        (vec!["Pa"], Some(vec!["Pa", "C", "Ea", "Aa", "Ab", "Pb"])),
                        (vec!["Pb"], Some(vec!["Pb", "C", "Pa", "Aa"])),
                        (
                            vec!["Aa"],
                            Some(vec!["Aa", "C", "Ea", "Eb", "Pa", "Ab", "Pb"]),
                        ),
                        (vec!["Ab"], None),
                        (vec!["Ea"], Some(vec!["Ea", "C", "Eb", "Pa", "Aa", "Ab"])),
                        (vec!["Eb"], Some(vec!["Eb", "C", "Ea", "Aa", "Ab"])),
                    ],
                ),
                (
                    skill_with_fixed_range(
                        TargetFilter::AnyExceptCaster,
                        Area::Diamond { radius: 1 },
                    ),
                    vec![
                        (vec!["C"], None),
                        (vec!["Pa"], Some(vec!["Pa", "Aa", "Pb"])),
                        (vec!["Pb"], Some(vec!["Pb", "Pa"])),
                        (vec!["Aa"], Some(vec!["Aa", "Ea", "Pa", "Ab"])),
                        (vec!["Ab"], None),
                        (vec!["Ea"], Some(vec!["Ea", "Eb", "Aa"])),
                        (vec!["Eb"], Some(vec!["Eb", "Ea", "Ab"])),
                    ],
                ),
                (
                    skill_with_fixed_range(TargetFilter::Enemy, Area::Diamond { radius: 1 }),
                    vec![
                        (vec!["C"], None),
                        (vec!["Pa"], None),
                        (vec!["Pb"], None),
                        (vec!["Aa"], None),
                        (vec!["Ab"], None),
                        (vec!["Ea"], Some(vec!["Ea", "Eb"])),
                        (vec!["Eb"], Some(vec!["Eb", "Ea"])),
                    ],
                ),
                (
                    skill_with_fixed_range(TargetFilter::Ally, Area::Diamond { radius: 1 }),
                    vec![
                        (vec!["C"], Some(vec!["C", "Pa"])),
                        (vec!["Pa"], Some(vec!["Pa", "C", "Aa", "Pb"])),
                        (vec!["Pb"], Some(vec!["Pb", "Pa"])),
                        (vec!["Aa"], Some(vec!["Aa", "Pa", "Ab"])),
                        (vec!["Ab"], None),
                        (vec!["Ea"], None),
                        (vec!["Eb"], None),
                    ],
                ),
                (
                    skill_with_fixed_range(
                        TargetFilter::AllyExceptCaster,
                        Area::Diamond { radius: 1 },
                    ),
                    vec![
                        (vec!["C"], None),
                        (vec!["Pa"], Some(vec!["Pa", "Aa", "Pb"])),
                        (vec!["Pb"], Some(vec!["Pb", "Pa"])),
                        (vec!["Aa"], Some(vec!["Aa", "Pa", "Ab"])),
                        (vec!["Ab"], None),
                        (vec!["Ea"], None),
                        (vec!["Eb"], None),
                    ],
                ),
                (
                    skill_with_fixed_range(TargetFilter::CasterOnly, Area::Diamond { radius: 1 }),
                    vec![
                        (vec!["C"], Some(vec!["C"])),
                        (vec!["Pa"], None),
                        (vec!["Pb"], None),
                        (vec!["Aa"], None),
                        (vec!["Ab"], None),
                        (vec!["Ea"], None),
                        (vec!["Eb"], None),
                    ],
                ),
            ],
        ),
        (
            "--- Cross",
            vec![
                r#"
                    C  Ea Eb
                    Pa Aa Ab
                    Pb . .
                    "#,
            ],
            vec![
                (
                    skill_with_fixed_range(TargetFilter::Any, Area::Cross { length: 1 }),
                    vec![
                        (vec!["C"], Some(vec!["C", "Ea", "Pa"])),
                        (vec!["Pa"], Some(vec!["Pa", "C", "Aa", "Pb"])),
                        (vec!["Pb"], Some(vec!["Pb", "Pa"])),
                        (vec!["Aa"], Some(vec!["Aa", "Ea", "Pa", "Ab"])),
                        (vec!["Ab"], None),
                        (vec!["Ea"], Some(vec!["Ea", "C", "Eb", "Aa"])),
                        (vec!["Eb"], Some(vec!["Eb", "Ea", "Ab"])),
                    ],
                ),
                // 方便起見，只測試這個 cross length 2
                (
                    skill_with_fixed_range(TargetFilter::Any, Area::Cross { length: 2 }),
                    vec![
                        (vec!["C"], Some(vec!["C", "Ea", "Eb", "Pa", "Pb"])),
                        (vec!["Pa"], Some(vec!["Pa", "C", "Aa", "Ab", "Pb"])),
                        (vec!["Pb"], Some(vec!["Pb", "C", "Pa"])),
                        (vec!["Aa"], Some(vec!["Aa", "Ea", "Pa", "Ab"])),
                        (vec!["Ab"], None),
                        (vec!["Ea"], Some(vec!["Ea", "C", "Eb", "Aa"])),
                        (vec!["Eb"], Some(vec!["Eb", "C", "Ea", "Ab"])),
                    ],
                ),
                (
                    skill_with_fixed_range(
                        TargetFilter::AnyExceptCaster,
                        Area::Cross { length: 1 },
                    ),
                    vec![
                        (vec!["C"], None),
                        (vec!["Pa"], Some(vec!["Pa", "Aa", "Pb"])),
                        (vec!["Pb"], Some(vec!["Pb", "Pa"])),
                        (vec!["Aa"], Some(vec!["Aa", "Ea", "Pa", "Ab"])),
                        (vec!["Ab"], None),
                        (vec!["Ea"], Some(vec!["Ea", "Eb", "Aa"])),
                        (vec!["Eb"], Some(vec!["Eb", "Ea", "Ab"])),
                    ],
                ),
                (
                    skill_with_fixed_range(TargetFilter::Enemy, Area::Cross { length: 1 }),
                    vec![
                        (vec!["C"], None),
                        (vec!["Pa"], None),
                        (vec!["Pb"], None),
                        (vec!["Aa"], None),
                        (vec!["Ab"], None),
                        (vec!["Ea"], Some(vec!["Ea", "Eb"])),
                        (vec!["Eb"], Some(vec!["Eb", "Ea"])),
                    ],
                ),
                (
                    skill_with_fixed_range(TargetFilter::Ally, Area::Cross { length: 1 }),
                    vec![
                        (vec!["C"], Some(vec!["C", "Pa"])),
                        (vec!["Pa"], Some(vec!["Pa", "C", "Aa", "Pb"])),
                        (vec!["Pb"], Some(vec!["Pb", "Pa"])),
                        (vec!["Aa"], Some(vec!["Aa", "Pa", "Ab"])),
                        (vec!["Ab"], None),
                        (vec!["Ea"], None),
                        (vec!["Eb"], None),
                    ],
                ),
                (
                    skill_with_fixed_range(
                        TargetFilter::AllyExceptCaster,
                        Area::Cross { length: 1 },
                    ),
                    vec![
                        (vec!["C"], None),
                        (vec!["Pa"], Some(vec!["Pa", "Aa", "Pb"])),
                        (vec!["Pb"], Some(vec!["Pb", "Pa"])),
                        (vec!["Aa"], Some(vec!["Aa", "Pa", "Ab"])),
                        (vec!["Ab"], None),
                        (vec!["Ea"], None),
                        (vec!["Eb"], None),
                    ],
                ),
                (
                    skill_with_fixed_range(TargetFilter::CasterOnly, Area::Cross { length: 1 }),
                    vec![
                        (vec!["C"], Some(vec!["C"])),
                        (vec!["Pa"], None),
                        (vec!["Pb"], None),
                        (vec!["Aa"], None),
                        (vec!["Ab"], None),
                        (vec!["Ea"], None),
                        (vec!["Eb"], None),
                    ],
                ),
            ],
        ),
        (
            "--- Line",
            vec![
                r#"
                    C  Ea Eb
                    Pa Aa Ab
                    Pb . .
                    "#,
            ],
            vec![
                (
                    skill_with(
                        (1, 1),
                        TargetSelection::Unit,
                        TargetFilter::Any,
                        1,
                        false,
                        Area::Line { length: 1 },
                    ),
                    vec![
                        (vec!["Pa"], Some(vec!["C", "Pa"])),
                        (vec!["Pb"], None),
                        (vec!["Aa"], None),
                        (vec!["Ab"], None),
                        (vec!["Ea"], Some(vec!["C", "Ea"])),
                        (vec!["Eb"], None),
                    ],
                ),
                (
                    skill_with_fixed_range(TargetFilter::Any, Area::Line { length: 2 }),
                    vec![
                        (vec!["Pa"], Some(vec!["C", "Pa", "Pb"])),
                        (vec!["Pb"], Some(vec!["C", "Pa", "Pb"])),
                        (vec!["Aa"], None),
                        (vec!["Ab"], None),
                        (vec!["Ea"], Some(vec!["C", "Ea", "Eb"])),
                        (vec!["Eb"], Some(vec!["C", "Ea", "Eb"])),
                    ],
                ),
                (
                    skill_with_fixed_range(TargetFilter::AnyExceptCaster, Area::Line { length: 2 }),
                    vec![
                        (vec!["Pa"], Some(vec!["Pa", "Pb"])),
                        (vec!["Pb"], Some(vec!["Pa", "Pb"])),
                        (vec!["Aa"], None),
                        (vec!["Ab"], None),
                        (vec!["Ea"], Some(vec!["Ea", "Eb"])),
                        (vec!["Eb"], Some(vec!["Ea", "Eb"])),
                    ],
                ),
                (
                    skill_with_fixed_range(TargetFilter::Enemy, Area::Line { length: 2 }),
                    vec![
                        (vec!["Pa"], None),
                        (vec!["Pb"], None),
                        (vec!["Aa"], None),
                        (vec!["Ab"], None),
                        (vec!["Ea"], Some(vec!["Ea", "Eb"])),
                        (vec!["Eb"], Some(vec!["Ea", "Eb"])),
                    ],
                ),
                (
                    skill_with_fixed_range(TargetFilter::Ally, Area::Line { length: 2 }),
                    vec![
                        (vec!["Pa"], Some(vec!["C", "Pa", "Pb"])),
                        (vec!["Pb"], Some(vec!["C", "Pa", "Pb"])),
                        (vec!["Aa"], None),
                        (vec!["Ab"], None),
                        (vec!["Ea"], None),
                        (vec!["Eb"], None),
                    ],
                ),
                (
                    skill_with_fixed_range(
                        TargetFilter::AllyExceptCaster,
                        Area::Line { length: 2 },
                    ),
                    vec![
                        (vec!["Pa"], Some(vec!["Pa", "Pb"])),
                        (vec!["Pb"], Some(vec!["Pa", "Pb"])),
                        (vec!["Aa"], None),
                        (vec!["Ab"], None),
                        (vec!["Ea"], None),
                        (vec!["Eb"], None),
                    ],
                ),
            ],
        ),
    ];

    for (description, levels, skill_cases) in test_data.iter() {
        for level in levels.iter() {
            let (board, _, marker_map) =
                standard_board(level).expect(&format!("建立測試棋盤失敗：{}", description));
            let position_map = to_position_map(&marker_map);
            let caster = CasterInfo {
                position: marker_map["C"][0].position,
                unit_info: marker_map["C"][0].unit_info.clone(),
            };

            for (skill, target_cases) in skill_cases.iter() {
                for (target_markers, expected_markers) in target_cases.iter() {
                    let targets: Vec<Position> = target_markers
                        .iter()
                        .flat_map(|marker| all_positions_of(&marker_map, marker))
                        .collect();

                    match expected_markers {
                        Some(expected_markers) => {
                            let expected: HashSet<Occupant> = expected_markers
                                .iter()
                                .flat_map(|marker| all_occupants_of(&marker_map, marker))
                                .collect();

                            let result = select_skill_targets(
                                &caster,
                                skill,
                                &targets,
                                &position_map,
                                board,
                            )
                            .expect(&format!("應成功：{}", description));
                            let result_set: HashSet<_> = result.into_iter().collect();
                            assert_eq!(
                                result_set, expected,
                                "測試失敗：{description}, targets={target_markers:?}, skill={skill:?}"
                            );
                        }
                        None => {
                            let result = select_skill_targets(
                                &caster,
                                skill,
                                &targets,
                                &position_map,
                                board,
                            );
                            assert!(
                                result.is_err(),
                                "應失敗但成功了：{description}, targets={target_markers:?}, skill={skill:?}, result={result:?}"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn test_select_skill_targets_singletarget() {
    let level = r#"
        C  Ea Eb
        Pa T .
        Aa . .
        "#;
    let (board, markers, marker_map) = standard_board(level).expect("建立測試棋盤失敗");
    let position_map = to_position_map(&marker_map);
    let caster = CasterInfo {
        position: marker_map["C"][0].position,
        unit_info: marker_map["C"][0].unit_info.clone(),
    };

    for filter in TargetFilter::iter() {
        let msg = format!("filter={filter:?} 不可以瞄準空地");
        let skill = skill_with_unit_target(filter, 1, false, Area::Single);
        let m = "T";
        let targets = vec![markers[m][0]];
        let result = select_skill_targets(&caster, &skill, &targets, &position_map, board);
        assert!(result.is_err(), "{}", msg);
    }

    for filter in TargetFilter::iter() {
        let msg = format!("filter={filter:?} Ground 可以瞄準空地");
        let skill = skill_with_ground_target(filter, 1, false, Area::Single);
        let m = "T";
        let targets = vec![markers[m][0]];
        let result = select_skill_targets(&caster, &skill, &targets, &position_map, board);
        assert!(result.is_ok(), "{}", msg);
        assert!(result.expect(&msg).is_empty(), "{}", msg);
    }
}

#[test]
fn test_select_skill_targets_multitarget() {
    let level = r#"
        C  Ea Eb
        Pa T .
        Aa . .
        "#;
    let (board, markers, marker_map) = standard_board(level).expect("建立測試棋盤失敗");
    let position_map = to_position_map(&marker_map);
    let caster = CasterInfo {
        position: marker_map["C"][0].position,
        unit_info: marker_map["C"][0].unit_info.clone(),
    };

    for allow_duplicate in [false, true] {
        for filter in TargetFilter::iter() {
            let msg = format!(
                "filter={filter:?} 不可以瞄準空地 - 重複瞄準同一個目標 {allow_duplicate:?}"
            );
            let skill = skill_with_unit_target(filter, 2, allow_duplicate, Area::Single);
            let m = "T";
            let targets = vec![markers[m][0]];
            let result = select_skill_targets(&caster, &skill, &targets, &position_map, board);
            assert!(result.is_err(), "{}", msg);
        }

        for filter in TargetFilter::iter() {
            let msg =
                format!("filter={filter:?} 可以瞄準空地 - 重複瞄準同一個目標 {allow_duplicate:?}");
            let skill = skill_with_ground_target(filter, 2, allow_duplicate, Area::Single);
            let m = "T";
            let targets = vec![markers[m][0]];
            let result = select_skill_targets(&caster, &skill, &targets, &position_map, board);
            assert!(result.is_ok(), "{}", msg);
            assert!(result.expect(&msg).is_empty(), "{}", msg);
        }

        for filter in TargetFilter::iter() {
            let msg = format!("filter={filter:?} 重複瞄準同一個目標 {allow_duplicate:?}");
            let skill = skill_with_unit_target(filter, 2, allow_duplicate, Area::Single);
            let m = match &filter {
                TargetFilter::Any | TargetFilter::AnyExceptCaster | TargetFilter::Enemy => "Ea",
                TargetFilter::Ally | TargetFilter::AllyExceptCaster => "Aa",
                TargetFilter::CasterOnly => "C",
            };
            let targets = vec![markers[m][0], markers[m][0]];
            let result = select_skill_targets(&caster, &skill, &targets, &position_map, board);
            if allow_duplicate {
                assert!(result.is_ok(), "{}", msg);
                let occupants = all_occupants_of(&marker_map, m);
                assert_eq!(
                    result.expect(&msg),
                    vec![occupants[0], occupants[0]],
                    "{}",
                    msg
                );
            } else {
                assert!(result.is_err(), "{}", msg);
            }
        }
    }
}

#[test]
fn test_select_skill_targets_area_diamond_and_cross() {
    let level = r#"
        C  Ea Eb
        Pa T .
        Aa . .
        "#;
    let (board, markers, marker_map) = standard_board(level).expect("建立測試棋盤失敗");
    let position_map = to_position_map(&marker_map);
    let caster = CasterInfo {
        position: marker_map["C"][0].position,
        unit_info: marker_map["C"][0].unit_info.clone(),
    };

    for area in [Area::Diamond { radius: 1 }, Area::Cross { length: 1 }] {
        for filter in TargetFilter::iter() {
            // Ground target：瞄準空地必須成功
            let test_case = match &filter {
                TargetFilter::Any => {
                    vec![
                        ("T", vec!["Pa", "Ea"]),
                        ("Ea", vec!["C", "Ea", "Eb"]),
                        ("Pa", vec!["C", "Pa", "Aa"]),
                    ]
                }
                TargetFilter::AnyExceptCaster => {
                    vec![
                        ("T", vec!["Pa", "Ea"]),
                        ("Ea", vec!["Ea", "Eb"]),
                        ("Pa", vec!["Pa", "Aa"]),
                    ]
                }
                TargetFilter::Enemy => {
                    vec![("T", vec!["Ea"]), ("Ea", vec!["Ea", "Eb"]), ("Pa", vec![])]
                }
                TargetFilter::Ally => {
                    vec![
                        ("T", vec!["Pa"]),
                        ("Ea", vec!["C"]),
                        ("Pa", vec!["C", "Pa", "Aa"]),
                    ]
                }
                TargetFilter::AllyExceptCaster => {
                    vec![("T", vec!["Pa"]), ("Ea", vec![]), ("Pa", vec!["Pa", "Aa"])]
                }
                TargetFilter::CasterOnly => {
                    vec![("T", vec![]), ("Ea", vec!["C"]), ("Pa", vec!["C"])]
                }
            };
            for (m, expected) in test_case {
                let msg = format!("{area:?} - {filter:?} - 不用瞄準單位:{m}");
                let skill = skill_with_ground_target(filter, 1, false, area);
                let targets = vec![markers[m][0]];
                let result = select_skill_targets(&caster, &skill, &targets, &position_map, board);
                assert!(result.is_ok(), "{}", msg);
                let result_set: HashSet<_> = result.expect(&msg).into_iter().collect();
                let occupants = HashSet::from_iter(
                    expected
                        .into_iter()
                        .map(|m| all_occupants_of(&marker_map, m)[0]),
                );
                assert_eq!(result_set, occupants, "{}", msg);
            }

            // Unit target：瞄準空地必須失敗
            {
                let msg = format!("{area:?} - {filter:?} - 瞄準單位:空地必須失敗");
                let skill = skill_with_unit_target(filter, 1, false, area);
                let targets = vec![markers["T"][0]];
                let result = select_skill_targets(&caster, &skill, &targets, &position_map, board);
                assert!(result.is_err(), "{}", msg);
            }
            // Unit target：瞄準有單位的位置
            let unit_test_case: Vec<(&str, std::result::Result<Vec<&str>, ()>)> = match &filter {
                TargetFilter::Any => vec![
                    ("Ea", Ok(vec!["C", "Ea", "Eb"])),
                    ("Pa", Ok(vec!["C", "Pa", "Aa"])),
                ],
                TargetFilter::AnyExceptCaster => {
                    vec![("Ea", Ok(vec!["Ea", "Eb"])), ("Pa", Ok(vec!["Pa", "Aa"]))]
                }
                TargetFilter::Enemy => vec![("Ea", Ok(vec!["Ea", "Eb"])), ("Pa", Err(()))],
                TargetFilter::Ally => vec![("Ea", Err(())), ("Pa", Ok(vec!["C", "Pa", "Aa"]))],
                TargetFilter::AllyExceptCaster => {
                    vec![("Ea", Err(())), ("Pa", Ok(vec!["Pa", "Aa"]))]
                }
                TargetFilter::CasterOnly => vec![("Ea", Err(())), ("Pa", Err(()))],
            };
            for (m, expected) in unit_test_case {
                let msg = format!("{area:?} - {filter:?} - 瞄準單位:{m}");
                let skill = skill_with_unit_target(filter, 1, false, area);
                let targets = vec![markers[m][0]];
                let result = select_skill_targets(&caster, &skill, &targets, &position_map, board);
                match expected {
                    Ok(expected_markers) => {
                        assert!(result.is_ok(), "{}", msg);
                        let result_set: HashSet<_> = result.expect(&msg).into_iter().collect();
                        let occupants = HashSet::from_iter(
                            expected_markers
                                .into_iter()
                                .map(|m| all_occupants_of(&marker_map, m)[0]),
                        );
                        assert_eq!(result_set, occupants, "{}", msg);
                    }
                    Err(()) => {
                        assert!(result.is_err(), "{}", msg);
                    }
                }
            }
        }
    }
}

#[test]
fn test_select_skill_targets_area_line() {
    let level = r#"
        C  T Eb
        Pa . .
        Aa . .
        "#;
    let (board, markers, marker_map) = standard_board(level).expect("建立測試棋盤失敗");
    let position_map = to_position_map(&marker_map);
    let caster = CasterInfo {
        position: marker_map["C"][0].position,
        unit_info: marker_map["C"][0].unit_info.clone(),
    };

    for filter in TargetFilter::iter() {
        let test_case = match &filter {
            TargetFilter::Any => {
                vec![
                    ("T", vec!["C", "Eb"]),
                    ("Eb", vec!["C", "Eb"]),
                    ("Pa", vec!["C", "Pa", "Aa"]),
                ]
            }
            TargetFilter::AnyExceptCaster => {
                vec![
                    ("T", vec!["Eb"]),
                    ("Eb", vec!["Eb"]),
                    ("Pa", vec!["Pa", "Aa"]),
                ]
            }
            TargetFilter::Enemy => {
                vec![("T", vec!["Eb"]), ("Eb", vec!["Eb"]), ("Pa", vec![])]
            }
            TargetFilter::Ally => {
                vec![
                    ("T", vec!["C"]),
                    ("Eb", vec!["C"]),
                    ("Pa", vec!["C", "Pa", "Aa"]),
                ]
            }
            TargetFilter::AllyExceptCaster => {
                vec![("T", vec![]), ("Eb", vec![]), ("Pa", vec!["Pa", "Aa"])]
            }
            TargetFilter::CasterOnly => {
                vec![("T", vec!["C"]), ("Eb", vec!["C"]), ("Pa", vec!["C"])]
            }
        };
        for (m, expected) in test_case {
            let msg = format!("line - {filter:?} - 不用瞄準單位:{m}");
            let skill = skill_with_ground_target(filter, 1, false, Area::Line { length: 2 });
            let targets = vec![markers[m][0]];
            let result = select_skill_targets(&caster, &skill, &targets, &position_map, board);
            assert!(result.is_ok(), "{}", msg);
            let result_set: HashSet<_> = result.expect(&msg).into_iter().collect();
            let occupants = HashSet::from_iter(
                expected
                    .into_iter()
                    .map(|m| all_occupants_of(&marker_map, m)[0]),
            );
            assert_eq!(result_set, occupants, "{}", msg);
        }
    }
}

//! validate_skill_targets 測試

use crate::domain::alias::{Coord, ID};
use crate::domain::constants::PLAYER_FACTION_ID;
use crate::domain::core_types::*;
use crate::ecs_types::components::*;
use crate::ecs_types::resources::Board;
use crate::error::Result;
use crate::logic::skill::skill_execution::{
    CheckResult, CheckTarget, CombatStats, EffectEntry, ResolvedEffect, resolve_effect_tree,
};
use crate::logic::skill::skill_target::validate_skill_targets;
use crate::logic::skill::{CasterInfo, UnitInfo};
use crate::test_helpers::level_builder::{LevelBuilder, MarkerEntry};
use std::collections::HashMap;
use strum::IntoEnumIterator;

const ALLY_FACTION_ID: ID = 1;
const ENEMY_FACTION_ID: ID = 2;
const TEST_CASTER_ID: ID = 9999;
const TEST_SKILL_NAME: &str = "test_skill";

/// 標準棋盤建構：C=施放者(player), Pt/Pa/Pn=玩家, At/Aa/An=友軍, Et/Ea/En=敵軍
fn standard_board(
    ascii: &str,
) -> Result<(
    Board,
    HashMap<String, Vec<Position>>,
    HashMap<String, Vec<MarkerEntry>>,
)> {
    LevelBuilder::from_ascii(ascii)
        .unit("C", "caster", PLAYER_FACTION_ID)
        .unit("Pa", "player", PLAYER_FACTION_ID)
        .unit("Pb", "player", PLAYER_FACTION_ID)
        .unit("Aa", "ally", ALLY_FACTION_ID)
        .unit("Ab", "ally", ALLY_FACTION_ID)
        .unit("Ea", "enemy", ENEMY_FACTION_ID)
        .unit("Eb", "enemy", ENEMY_FACTION_ID)
        .to_unit_map()
}

// ============================================================================
// 檢查目標相關工具
// ============================================================================

fn target_with(
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

fn target_with_unit_target(
    filter: TargetFilter,
    count: usize,
    allow_same_target: bool,
    area: Area,
) -> Target {
    target_with(
        (0, 2),
        TargetSelection::Unit,
        filter,
        count,
        allow_same_target,
        area,
    )
}

fn target_with_ground_target(
    filter: TargetFilter,
    count: usize,
    allow_same_target: bool,
    area: Area,
) -> Target {
    target_with(
        (0, 2),
        TargetSelection::Ground,
        filter,
        count,
        allow_same_target,
        area,
    )
}

/// 從 marker map 建立 validate_skill_targets 需要的 HashMap<Position, UnitInfo>
fn to_position_map(
    unit_markers: &HashMap<String, Vec<MarkerEntry>>,
) -> HashMap<Position, UnitInfo> {
    unit_markers
        .values()
        .flatten()
        .map(|entry| (entry.position, entry.unit_info.clone()))
        .collect()
}

/// 從 marker map 取得指定 marker 的所有位置
fn all_positions_of(
    unit_markers: &HashMap<String, Vec<MarkerEntry>>,
    marker: &str,
) -> Vec<Position> {
    unit_markers[marker]
        .iter()
        .map(|entry| entry.position)
        .collect()
}

/// 從 marker map 取得指定 marker 的所有 Occupant
fn all_occupants_of(
    unit_markers: &HashMap<String, Vec<MarkerEntry>>,
    marker: &str,
) -> Vec<Occupant> {
    unit_markers[marker]
        .iter()
        .map(|entry| entry.unit_info.occupant)
        .collect()
}

// ============================================================================
// 套用技能相關工具
// ============================================================================

fn hp_leaf(source_attribute: Attribute, value_percent: i32) -> EffectNode {
    EffectNode::Leaf {
        who: CasterOrTarget::Target,
        effect: Effect::HpEffect {
            scaling: Scaling {
                source: CasterOrTarget::Caster,
                source_attribute,
                value_percent,
            },
        },
    }
}

/// 將節點包入 Area，若技能為 Single 則直接使用
fn wrap_area(node: EffectNode, skill_target: &Target) -> Vec<EffectNode> {
    match skill_target.area {
        Area::Single => vec![node],
        Area::Diamond { .. } | Area::Cross { .. } | Area::Line { .. } => {
            vec![EffectNode::Area {
                area: skill_target.area,
                filter: skill_target.selectable_filter.clone(),
                nodes: vec![node],
            }]
        }
    }
}

/// 判定模式：Auto（無判定）或 (命中來源 × 防禦類型) 組合
#[derive(Debug, Clone)]
enum CheckMode {
    Auto,
    Check {
        defense_type: DefenseType,
        accuracy_source: AccuracySource,
    },
}

/// 產生此專案所有需要測試的判定模式
fn all_check_modes() -> Vec<CheckMode> {
    let mut modes = vec![CheckMode::Auto];
    for accuracy_source in AccuracySource::iter() {
        for defense_type in DefenseType::iter() {
            modes.push(CheckMode::Check {
                defense_type: defense_type.clone(),
                accuracy_source: accuracy_source.clone(),
            });
        }
    }
    modes
}

/// 依判定模式將葉節點包裝成對應的 EffectNode
fn build_check_node(mode: &CheckMode, leaf: EffectNode) -> EffectNode {
    match mode {
        CheckMode::Auto => leaf,
        CheckMode::Check {
            defense_type,
            accuracy_source,
        } => EffectNode::Branch {
            condition: EffectCondition {
                defense_type: defense_type.clone(),
                accuracy_source: accuracy_source.clone(),
                accuracy_bonus: 0,
                crit_bonus: 0,
            },
            on_success: vec![leaf],
            on_failure: vec![],
        },
    }
}

/// 預期的判定結果（依 resolve_hit 與 skill_execution 的對應規則）
#[derive(Debug, Clone, Copy, PartialEq)]
enum ExpectedCheck {
    Auto,
    Hit,
    Evade,
    Affected,
    Resisted,
}

/// 依據 mode 與投骰結果預測 CheckResult
///
/// 測試前提：attacker accuracy = 0、block = 0，defender 防禦屬性 = stat_value
fn expected_check(mode: &CheckMode, pass: bool) -> ExpectedCheck {
    match mode {
        CheckMode::Auto => ExpectedCheck::Auto,
        CheckMode::Check {
            defense_type: DefenseType::AgilityAndBlock,
            ..
        } if pass => ExpectedCheck::Hit,
        CheckMode::Check {
            defense_type: DefenseType::AgilityAndBlock,
            ..
        } => ExpectedCheck::Evade,
        CheckMode::Check { .. } if pass => ExpectedCheck::Affected,
        CheckMode::Check { .. } => ExpectedCheck::Resisted,
    }
}

/// 建立固定回傳值的 rng
fn fixed_rng(value: i32) -> impl FnMut() -> i32 {
    move || value
}

/// 對所有目標位置執行效果樹並收集所有條目
fn run_for_targets(
    nodes: &[EffectNode],
    caster_stats: &CombatStats,
    caster_position: Position,
    target_positions: &[Position],
    units_on_board: &HashMap<Position, CombatStats>,
    board: Board,
    rng: &mut impl FnMut() -> i32,
) -> Vec<EffectEntry> {
    target_positions
        .iter()
        .flat_map(|pos| {
            resolve_effect_tree(
                TEST_CASTER_ID,
                TEST_SKILL_NAME,
                &[],
                nodes,
                caster_stats,
                caster_position,
                *pos,
                units_on_board,
                &HashMap::new(),
                board,
                rng,
            )
            .expect("resolve_effect_tree 應成功執行")
        })
        .collect()
}

/// 在條目中尋找對應 Occupant 的效果條目
fn find_entry_for<'a>(entries: &'a [EffectEntry], expected: &Occupant) -> Option<&'a EffectEntry> {
    entries.iter().find(
        |entry| matches!(entry.target, CheckTarget::Unit(id) if Occupant::Unit(id) == *expected),
    )
}

/// 建立帶有指定屬性的 CombatStats
fn build_stats(unit_info: UnitInfo) -> CombatStats {
    CombatStats {
        unit_info,
        attribute: AttributeBundle::default(),
    }
}

fn build_stats_with_defenses(
    unit_info: UnitInfo,
    mode: &CheckMode,
    stat_value: i32,
) -> CombatStats {
    let mut stats = build_stats(unit_info);
    match mode {
        CheckMode::Auto => {
            stats.attribute.fortitude = Fortitude(stat_value);
            stats.attribute.agility = Agility(stat_value);
            stats.attribute.will = Will(stat_value);
        }
        CheckMode::Check { defense_type, .. } => match defense_type {
            DefenseType::Fortitude => stats.attribute.fortitude = Fortitude(stat_value),
            DefenseType::Agility => stats.attribute.agility = Agility(stat_value),
            DefenseType::AgilityAndBlock => {
                stats.attribute.agility = Agility(stat_value);
            }
            DefenseType::Will => stats.attribute.will = Will(stat_value),
        },
    }
    stats
}

fn assert_hp(
    caster_position: Position,
    target_positions: &[Position],
    skill_target: &Target,
    board: Board,
    unit_markers: &HashMap<String, Vec<MarkerEntry>>,
    expected: Vec<Occupant>,
    msg: String,
) {
    /// (atk, value_percent, expected_amount)
    const SKILL_TEST_DATA: &[(i32, i32, i32)] = &[
        (100, -100, -100),
        (100, -50, -50),
        (100, -120, -120),
        (200, -75, -150),
        (80, -100, -80),
        (0, -100, 0),
        (100, 0, 0),
        (50, -200, -100),
    ];

    /// (stat_value, rng_roll, expected_hit)
    const STAT_TEST_DATA: &[(i32, i32, bool)] = &[
        (1, 1, false),
        (1, 5, false),
        (1, 6, true),
        (1, 9, true),
        (1, 10, true),
        (1, 90, true),
        (1, 95, true),
        (1, 96, true),
        (1, 100, true),
        (10, 1, false),
        (10, 5, false),
        (10, 6, false),
        (10, 9, false),
        (10, 10, true),
        (10, 90, true),
        (10, 95, true),
        (10, 96, true),
        (10, 100, true),
        (100, 1, false),
        (100, 5, false),
        (100, 6, false),
        (100, 9, false),
        (100, 10, false),
        (100, 90, false),
        (100, 95, false),
        (100, 96, true),
        (100, 100, true),
        (1000, 1, false),
        (1000, 5, false),
        (1000, 6, false),
        (1000, 9, false),
        (1000, 10, false),
        (1000, 90, false),
        (1000, 95, false),
        (1000, 96, true),
        (1000, 100, true),
    ];

    let expected_count = expected.len();
    let modes = all_check_modes();

    // Precompute：nodes 只依賴 (mode, value_percent)
    let unique_value_percents: Vec<i32> = {
        let mut v: Vec<i32> = SKILL_TEST_DATA.iter().map(|&(_, vp, _)| vp).collect();
        v.sort();
        v.dedup();
        v
    };
    let nodes_cache: HashMap<(usize, i32), Vec<EffectNode>> = modes
        .iter()
        .enumerate()
        .flat_map(|(mi, mode)| {
            unique_value_percents.iter().map(move |&vp| {
                let leaf = hp_leaf(Attribute::PhysicalAttack, vp);
                let node = build_check_node(mode, leaf);
                ((mi, vp), wrap_area(node, skill_target))
            })
        })
        .collect();

    // Precompute：units_on_board 只依賴 (mode, stat_value)
    let unique_stat_values: Vec<i32> = {
        let mut v: Vec<i32> = STAT_TEST_DATA.iter().map(|&(sv, _, _)| sv).collect();
        v.sort();
        v.dedup();
        v
    };
    let position_map_template = to_position_map(unit_markers);
    let mut units_cache: HashMap<(usize, i32), HashMap<Position, CombatStats>> = HashMap::new();
    for (mi, mode) in modes.iter().enumerate() {
        for &sv in &unique_stat_values {
            let units: HashMap<Position, CombatStats> = position_map_template
                .iter()
                .map(|(pos, info)| {
                    let stat = build_stats_with_defenses(info.clone(), mode, sv);
                    (*pos, stat)
                })
                .collect();
            units_cache.insert((mi, sv), units);
        }
    }

    for &(atk, value_percent, expected_amount) in SKILL_TEST_DATA {
        let mut caster_stats = build_stats(unit_markers["C"][0].unit_info.clone());
        caster_stats.attribute.physical_attack = PhysicalAttack(atk);
        caster_stats.attribute.magical_attack = MagicalAttack(atk);

        for &(stat_value, random, expected_hit) in STAT_TEST_DATA {
            for (mode_idx, mode) in modes.iter().enumerate() {
                let nodes = &nodes_cache[&(mode_idx, value_percent)];
                let units_on_board = &units_cache[&(mode_idx, stat_value)];

                let mut rng = fixed_rng(random);
                let all_entries = run_for_targets(
                    nodes,
                    &caster_stats,
                    caster_position,
                    target_positions,
                    units_on_board,
                    board,
                    &mut rng,
                );

                assert_eq!(
                    all_entries.len(),
                    expected_count,
                    "mode={mode:?} atk={atk} percent={value_percent} 應產生 {expected_count} 個效果\nmsg: {msg}",
                );

                let expected_result = expected_check(mode, expected_hit);
                let hp_effect = ResolvedEffect::HpChange {
                    raw_amount: expected_amount,
                    final_amount: expected_amount,
                };

                for expected_occupant in &expected {
                    let found = find_entry_for(&all_entries, expected_occupant)
                        .unwrap_or_else(|| panic!(
                            "mode={mode:?} atk={atk} percent={value_percent} 應包含對 {expected_occupant:?} 的效果\nmsg: {msg}\nall_entries: {all_entries:#?}"
                        ));

                    let ctx = format!(
                        "mode={mode:?} atk={atk} percent={value_percent} stat={stat_value} roll={random}\nmsg: {msg}"
                    );

                    match expected_result {
                        ExpectedCheck::Auto => {
                            assert_eq!(found.check, CheckResult::Auto, "{ctx}");
                            assert_eq!(found.effect, hp_effect, "{ctx}");
                        }
                        ExpectedCheck::Hit => {
                            assert!(
                                matches!(found.check, CheckResult::Hit { .. }),
                                "預期命中, 實際 {:?}\n{ctx}",
                                found.check,
                            );
                            assert_eq!(found.effect, hp_effect, "{ctx}");
                        }
                        ExpectedCheck::Evade => {
                            assert_eq!(found.check, CheckResult::Evade, "{ctx}");
                            assert_eq!(found.effect, ResolvedEffect::NoEffect, "{ctx}");
                        }
                        ExpectedCheck::Affected => {
                            assert_eq!(found.check, CheckResult::Affected, "預期魔法命中\n{ctx}",);
                            assert_eq!(found.effect, hp_effect, "{ctx}");
                        }
                        ExpectedCheck::Resisted => {
                            assert_eq!(found.check, CheckResult::Resisted, "預期魔法被抗\n{ctx}",);
                            assert_eq!(found.effect, ResolvedEffect::NoEffect, "{ctx}");
                        }
                    }
                }
            }
        }
    }
}

// ============================================================================
// 實際測試
// ============================================================================

#[test]
fn test_normal_case() {
    let target_with_fixed_range =
        |filter: TargetFilter, area: Area| target_with_unit_target(filter, 1, false, area);

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
                    target_with(
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
                    target_with_fixed_range(TargetFilter::Any, Area::Single),
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
                    target_with_fixed_range(TargetFilter::AnyExceptCaster, Area::Single),
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
                    target_with_fixed_range(TargetFilter::Enemy, Area::Single),
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
                    target_with_fixed_range(TargetFilter::Ally, Area::Single),
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
                    target_with_fixed_range(TargetFilter::AllyExceptCaster, Area::Single),
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
                    target_with_fixed_range(TargetFilter::CasterOnly, Area::Single),
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
                    target_with(
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
                    target_with(
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
                    target_with(
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
                    target_with(
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
                    target_with(
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
                    target_with(
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
                    target_with(
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
                    target_with(
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
                    target_with(
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
                    target_with_fixed_range(TargetFilter::Any, Area::Diamond { radius: 1 }),
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
                    target_with_fixed_range(TargetFilter::Any, Area::Diamond { radius: 2 }),
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
                    target_with_fixed_range(
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
                    target_with_fixed_range(TargetFilter::Enemy, Area::Diamond { radius: 1 }),
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
                    target_with_fixed_range(TargetFilter::Ally, Area::Diamond { radius: 1 }),
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
                    target_with_fixed_range(
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
                    target_with_fixed_range(TargetFilter::CasterOnly, Area::Diamond { radius: 1 }),
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
                    target_with_fixed_range(TargetFilter::Any, Area::Cross { length: 1 }),
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
                    target_with_fixed_range(TargetFilter::Any, Area::Cross { length: 2 }),
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
                    target_with_fixed_range(
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
                    target_with_fixed_range(TargetFilter::Enemy, Area::Cross { length: 1 }),
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
                    target_with_fixed_range(TargetFilter::Ally, Area::Cross { length: 1 }),
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
                    target_with_fixed_range(
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
                    target_with_fixed_range(TargetFilter::CasterOnly, Area::Cross { length: 1 }),
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
                    target_with(
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
                    target_with_fixed_range(TargetFilter::Any, Area::Line { length: 2 }),
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
                    target_with_fixed_range(
                        TargetFilter::AnyExceptCaster,
                        Area::Line { length: 2 },
                    ),
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
                    target_with_fixed_range(TargetFilter::Enemy, Area::Line { length: 2 }),
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
                    target_with_fixed_range(TargetFilter::Ally, Area::Line { length: 2 }),
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
                    target_with_fixed_range(
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

    for (description, levels, target_cases) in test_data.iter() {
        for level in levels.iter() {
            let (board, _, unit_markers) =
                standard_board(level).expect(&format!("建立測試棋盤失敗：{}", description));
            let position_map = to_position_map(&unit_markers);
            let caster = CasterInfo {
                position: unit_markers["C"][0].position,
                unit_info: unit_markers["C"][0].unit_info.clone(),
            };

            for (skill_target, target_cases) in target_cases.iter() {
                for (target_markers, expected_markers) in target_cases.iter() {
                    let targets: Vec<Position> = target_markers
                        .iter()
                        .flat_map(|marker| all_positions_of(&unit_markers, marker))
                        .collect();

                    match expected_markers {
                        Some(expected_markers) => {
                            let expected: Vec<Occupant> = expected_markers
                                .iter()
                                .flat_map(|marker| all_occupants_of(&unit_markers, marker))
                                .collect();

                            let result = validate_skill_targets(
                                &caster,
                                skill_target,
                                &targets,
                                &position_map,
                                board,
                            );
                            assert!(result.is_ok(), "應成功：{}", description);
                            assert_hp(
                                caster.position,
                                &targets,
                                skill_target,
                                board,
                                &unit_markers,
                                expected,
                                format!("level={}\nskill_target={:?}", level, skill_target),
                            );
                        }
                        None => {
                            let result = validate_skill_targets(
                                &caster,
                                skill_target,
                                &targets,
                                &position_map,
                                board,
                            );
                            assert!(
                                result.is_err(),
                                "應失敗但成功了：{description}, targets={target_markers:?}, skill_target={skill_target:?}, result={result:?}"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn test_singletarget() {
    let level = r#"
        C  Ea Eb
        Pa T .
        Aa . .
        "#;
    let (board, markers, unit_markers) = standard_board(level).expect("建立測試棋盤失敗");
    let position_map = to_position_map(&unit_markers);
    let caster = CasterInfo {
        position: unit_markers["C"][0].position,
        unit_info: unit_markers["C"][0].unit_info.clone(),
    };

    for filter in TargetFilter::iter() {
        let msg = format!("filter={filter:?} 不可以瞄準空地");
        let skill_target = target_with_unit_target(filter, 1, false, Area::Single);
        let m = "T";
        let targets = vec![markers[m][0]];
        let result = validate_skill_targets(&caster, &skill_target, &targets, &position_map, board);
        assert!(result.is_err(), "{}", msg);
    }

    for filter in TargetFilter::iter() {
        let msg = format!("filter={filter:?} Ground 可以瞄準空地");
        let skill_target = target_with_ground_target(filter, 1, false, Area::Single);
        let m = "T";
        let targets = vec![markers[m][0]];
        let result = validate_skill_targets(&caster, &skill_target, &targets, &position_map, board);
        assert!(result.is_ok(), "{}", msg);
        assert_hp(
            caster.position,
            &targets,
            &skill_target,
            board,
            &unit_markers,
            vec![],
            format!("level={}\nskill_target={:?}", level, skill_target),
        );
    }
}

#[test]
fn test_multitarget() {
    let level = r#"
        C  Ea Eb
        Pa T .
        Aa . .
        "#;
    let (board, markers, unit_markers) = standard_board(level).expect("建立測試棋盤失敗");
    let position_map = to_position_map(&unit_markers);
    let caster = CasterInfo {
        position: unit_markers["C"][0].position,
        unit_info: unit_markers["C"][0].unit_info.clone(),
    };

    for allow_duplicate in [false, true] {
        for filter in TargetFilter::iter() {
            let msg = format!(
                "filter={filter:?} 不可以瞄準空地 - 重複瞄準同一個目標 {allow_duplicate:?}"
            );
            let skill_target = target_with_unit_target(filter, 2, allow_duplicate, Area::Single);
            let m = "T";
            let targets = vec![markers[m][0]];
            let result =
                validate_skill_targets(&caster, &skill_target, &targets, &position_map, board);
            assert!(result.is_err(), "{}", msg);
        }

        for filter in TargetFilter::iter() {
            let msg =
                format!("filter={filter:?} 可以瞄準空地 - 重複瞄準同一個目標 {allow_duplicate:?}");
            let skill_target = target_with_ground_target(filter, 2, allow_duplicate, Area::Single);
            let m = "T";
            let targets = vec![markers[m][0]];
            let result =
                validate_skill_targets(&caster, &skill_target, &targets, &position_map, board);
            assert!(result.is_ok(), "{}", msg);
            assert_hp(
                caster.position,
                &targets,
                &skill_target,
                board,
                &unit_markers,
                vec![],
                format!("level={}\nskill_target={:?}", level, skill_target),
            );
        }

        for filter in TargetFilter::iter() {
            let msg = format!("filter={filter:?} 重複瞄準同一個目標 {allow_duplicate:?}");
            let skill_target = target_with_unit_target(filter, 2, allow_duplicate, Area::Single);
            let m = match &filter {
                TargetFilter::Any | TargetFilter::AnyExceptCaster | TargetFilter::Enemy => "Ea",
                TargetFilter::Ally | TargetFilter::AllyExceptCaster => "Aa",
                TargetFilter::CasterOnly => "C",
            };
            let targets = vec![markers[m][0], markers[m][0]];
            let result =
                validate_skill_targets(&caster, &skill_target, &targets, &position_map, board);
            if allow_duplicate {
                assert!(result.is_ok(), "{}", msg);
                let expected = all_occupants_of(&unit_markers, m);
                let expected = vec![expected[0], expected[0]];
                assert_hp(
                    caster.position,
                    &targets,
                    &skill_target,
                    board,
                    &unit_markers,
                    expected,
                    format!("level={}\nskill_target={:?}", level, skill_target),
                );
            } else {
                assert!(result.is_err(), "{}", msg);
            }
        }
    }
}

#[test]
fn test_area_diamond_and_cross() {
    let level = r#"
        C  Ea Eb
        Pa T .
        Aa . .
        "#;
    let (board, markers, unit_markers) = standard_board(level).expect("建立測試棋盤失敗");
    let position_map = to_position_map(&unit_markers);
    let caster = CasterInfo {
        position: unit_markers["C"][0].position,
        unit_info: unit_markers["C"][0].unit_info.clone(),
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
                let skill_target = target_with_ground_target(filter, 1, false, area);
                let targets = vec![markers[m][0]];
                let result =
                    validate_skill_targets(&caster, &skill_target, &targets, &position_map, board);
                assert!(result.is_ok(), "{}", msg);
                let expected: Vec<_> = expected
                    .into_iter()
                    .map(|m| all_occupants_of(&unit_markers, m)[0])
                    .collect();
                assert_hp(
                    caster.position,
                    &targets,
                    &skill_target,
                    board,
                    &unit_markers,
                    expected,
                    format!("level={}\nskill_target={:?}", level, skill_target),
                );
            }

            // Unit target：瞄準空地必須失敗
            {
                let msg = format!("{area:?} - {filter:?} - 瞄準單位:空地必須失敗");
                let skill_target = target_with_unit_target(filter, 1, false, area);
                let targets = vec![markers["T"][0]];
                let result =
                    validate_skill_targets(&caster, &skill_target, &targets, &position_map, board);
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
                let skill_target = target_with_unit_target(filter, 1, false, area);
                let targets = vec![markers[m][0]];
                let result =
                    validate_skill_targets(&caster, &skill_target, &targets, &position_map, board);
                match expected {
                    Ok(expected_markers) => {
                        assert!(result.is_ok(), "{}", msg);
                        let expected: Vec<_> = expected_markers
                            .into_iter()
                            .map(|m| all_occupants_of(&unit_markers, m)[0])
                            .collect();
                        assert_hp(
                            caster.position,
                            &targets,
                            &skill_target,
                            board,
                            &unit_markers,
                            expected,
                            format!("level={}\nskill_target={:?}", level, skill_target),
                        );
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
fn test_area_line() {
    let level = r#"
        C  T Eb
        Pa . .
        Aa . .
        "#;
    let (board, markers, unit_markers) = standard_board(level).expect("建立測試棋盤失敗");
    let position_map = to_position_map(&unit_markers);
    let caster = CasterInfo {
        position: unit_markers["C"][0].position,
        unit_info: unit_markers["C"][0].unit_info.clone(),
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
            let skill_target =
                target_with_ground_target(filter, 1, false, Area::Line { length: 2 });
            let targets = vec![markers[m][0]];
            let result =
                validate_skill_targets(&caster, &skill_target, &targets, &position_map, board);
            assert!(result.is_ok(), "{}", msg);
            let expected: Vec<_> = expected
                .into_iter()
                .map(|m| all_occupants_of(&unit_markers, m)[0])
                .collect();
            assert_hp(
                caster.position,
                &targets,
                &skill_target,
                board,
                &unit_markers,
                expected,
                format!("level={}\nskill_target={:?}", level, skill_target),
            );
        }
    }
}

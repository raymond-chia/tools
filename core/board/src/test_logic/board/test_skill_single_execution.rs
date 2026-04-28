//! 單一技能效果執行測試（resolve_effect_tree）

use crate::domain::alias::ID;
use crate::domain::constants::PLAYER_FACTION_ID;
use crate::domain::core_types::*;
use crate::ecs_types::components::*;
use crate::ecs_types::resources::Board;
use crate::logic::skill::UnitInfo;
use crate::logic::skill::skill_execution::{
    CheckDetail, CheckResult, CheckTarget, CombatStats, EffectEntry, ObjectOnBoard, ResolvedEffect,
    resolve_effect_tree,
};
use crate::test_helpers::level_builder::LevelBuilder;
use std::collections::HashMap;

const ALLY_FACTION_ID: ID = 1;
const ENEMY_FACTION_ID: ID = 2;
const TEST_CASTER_ID: ID = 9999;
const TEST_SKILL_NAME: &str = "test_skill";

// ============================================================================
// 建構工具
// ============================================================================

/// 共享棋盤佈局：
/// ```text
/// .  A  .
/// E  C  W
/// .  T  .
/// ```
/// C=施放者(player), A=友軍(ally), E=敵軍(enemy), W=不可通過物件(牆壁), T=可通過物件(陷阱)
struct SharedBoard {
    board: Board,
    caster_pos: Position,
    ally_pos: Position,
    enemy_pos: Position,
    wall_pos: Position,
    trap_pos: Position,
    caster_occupant: Occupant,
    ally_occupant: Occupant,
    enemy_occupant: Occupant,
    units_on_board: HashMap<Position, CombatStats>,
    objects_on_board: HashMap<Position, ObjectOnBoard>,
    /// 空格位置（不含單位和物件的格子）
    empty_positions: Vec<Position>,
}

fn build_shared_board() -> SharedBoard {
    let (board, positions, unit_markers) = LevelBuilder::from_ascii(
        r#"
        .  A  .
        E  C  W
        .  T  .
        "#,
    )
    .unit("C", "caster", PLAYER_FACTION_ID)
    .unit("A", "ally", ALLY_FACTION_ID)
    .unit("E", "enemy", ENEMY_FACTION_ID)
    .object("W", "wall")
    .object("T", "trap")
    .to_unit_map()
    .expect("建構共享棋盤應成功");

    let caster_pos = unit_markers["C"][0].position;
    let ally_pos = unit_markers["A"][0].position;
    let enemy_pos = unit_markers["E"][0].position;
    let wall_pos = positions["W"][0];
    let trap_pos = positions["T"][0];

    let caster_occupant = unit_markers["C"][0].unit_info.occupant;
    let ally_occupant = unit_markers["A"][0].unit_info.occupant;
    let enemy_occupant = unit_markers["E"][0].unit_info.occupant;

    let units_on_board: HashMap<Position, CombatStats> = unit_markers
        .values()
        .flatten()
        .map(|entry| (entry.position, build_stats(entry.unit_info.clone())))
        .collect();

    let mut objects_on_board: HashMap<Position, ObjectOnBoard> = HashMap::new();
    objects_on_board.insert(
        wall_pos,
        ObjectOnBoard {
            occupant: Occupant::Object(100),
            occupies_tile: true,
        },
    );
    objects_on_board.insert(
        trap_pos,
        ObjectOnBoard {
            occupant: Occupant::Object(101),
            occupies_tile: false,
        },
    );

    let occupied: std::collections::HashSet<Position> =
        positions.values().flatten().copied().collect();
    let empty_positions: Vec<Position> = (0..board.height)
        .flat_map(|y| (0..board.width).map(move |x| Position { x, y }))
        .filter(|pos| !occupied.contains(pos))
        .collect();

    SharedBoard {
        board,
        caster_pos,
        ally_pos,
        enemy_pos,
        wall_pos,
        trap_pos,
        caster_occupant,
        ally_occupant,
        enemy_occupant,
        units_on_board,
        objects_on_board,
        empty_positions,
    }
}

fn build_stats(unit_info: UnitInfo) -> CombatStats {
    CombatStats {
        unit_info,
        attribute: AttributeBundle::default(),
    }
}

fn build_stats_with_atk(unit_info: UnitInfo, physical_attack: i32) -> CombatStats {
    let mut stats = build_stats(unit_info);
    stats.attribute.physical_attack = PhysicalAttack(physical_attack);
    stats
}

fn fixed_rng(value: i32) -> impl FnMut() -> i32 {
    move || value
}

/// 必定命中的 rng
fn always_hit_rng() -> impl FnMut() -> i32 {
    fixed_rng(100)
}

fn hp_leaf_target(source_attribute: Attribute, value_percent: i32) -> EffectNode {
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

fn hp_leaf_caster(source_attribute: Attribute, value_percent: i32) -> EffectNode {
    EffectNode::Leaf {
        who: CasterOrTarget::Caster,
        effect: Effect::HpEffect {
            scaling: Scaling {
                source: CasterOrTarget::Caster,
                source_attribute,
                value_percent,
            },
        },
    }
}

fn spawn_leaf(object_type: &str) -> EffectNode {
    EffectNode::Leaf {
        who: CasterOrTarget::Target,
        effect: Effect::SpawnObject {
            object_type: object_type.to_string(),
            duration: None,
            contact_effects: vec![],
        },
    }
}

fn apply_buff_leaf(buff: BuffType) -> EffectNode {
    EffectNode::Leaf {
        who: CasterOrTarget::Target,
        effect: Effect::ApplyBuff { buff },
    }
}

fn poison_buff() -> BuffType {
    BuffType {
        name: "poison".to_string(),
        stackable: false,
        while_active: vec![ContinuousEffect::AttributeFlat {
            attribute: Attribute::PhysicalAttack,
            value: -10,
        }],
        per_turn_effects: vec![],
        end_conditions: vec![EndCondition::Duration(3)],
    }
}

fn physical_hit_branch(
    defense_type: DefenseType,
    on_success: Vec<EffectNode>,
    on_failure: Vec<EffectNode>,
) -> EffectNode {
    EffectNode::Branch {
        condition: EffectCondition {
            defense_type,
            accuracy_source: AccuracySource::Physical,
            accuracy_bonus: 0,
            crit_bonus: 0,
        },
        on_success,
        on_failure,
    }
}

fn occupant_to_check_target(occupant: Occupant) -> CheckTarget {
    match occupant {
        Occupant::Unit(id) => CheckTarget::Unit(id),
        Occupant::Object(id) => panic!("預期 Unit occupant，實際為 Object({id})"),
    }
}

fn find_entries_for<'a>(entries: &'a [EffectEntry], occupant: &Occupant) -> Vec<&'a EffectEntry> {
    entries
        .iter()
        .filter(|e| matches!(e.target, CheckTarget::Unit(id) if Occupant::Unit(id) == *occupant))
        .collect()
}

fn find_entries_for_position<'a>(
    entries: &'a [EffectEntry],
    pos: Position,
) -> Vec<&'a EffectEntry> {
    entries
        .iter()
        .filter(|e| matches!(e.target, CheckTarget::Position(p) if p == pos))
        .collect()
}

// ============================================================================
// 案例 1：多頂層 Leaf — 友軍補血、敵軍扣血、自身補血
// ============================================================================

/// 三個頂層 Leaf：
/// - Leaf 1: filter=Ally, who=Target, HpEffect +50%（友軍補血）
/// - Leaf 2: filter=Enemy, who=Target, HpEffect -100%（敵軍扣血）
/// - Leaf 3: filter=Any, who=Caster, HpEffect +30%（自身補血）
///
/// 分別瞄準友軍和敵軍各一次
#[test]
fn test_multi_leaf_ally_and_enemy() {
    let mut sb = build_shared_board();

    let caster_atk = 1000;
    let mut units_on_board = std::mem::take(&mut sb.units_on_board);
    units_on_board.insert(
        sb.caster_pos,
        build_stats_with_atk(units_on_board[&sb.caster_pos].unit_info.clone(), caster_atk),
    );
    let caster_stats = units_on_board[&sb.caster_pos].clone();

    // 三個頂層 Leaf，各自帶 filter
    let nodes = vec![
        EffectNode::Area {
            area: Area::Single,
            filter: TargetFilter::Ally,
            nodes: vec![hp_leaf_target(Attribute::PhysicalAttack, 50)],
        },
        EffectNode::Area {
            area: Area::Single,
            filter: TargetFilter::Enemy,
            nodes: vec![hp_leaf_target(Attribute::PhysicalAttack, -100)],
        },
        hp_leaf_caster(Attribute::PhysicalAttack, 30),
    ];

    let test_data = [
        ("瞄準友軍", sb.ally_pos, sb.ally_occupant, 500),
        ("瞄準敵軍", sb.enemy_pos, sb.enemy_occupant, -1000),
    ];

    for (label, target_pos, target_occupant, expected_hp) in test_data {
        let mut rng = always_hit_rng();
        let entries = resolve_effect_tree(
            TEST_CASTER_ID,
            TEST_SKILL_NAME,
            &[],
            &nodes,
            &caster_stats,
            sb.caster_pos,
            target_pos,
            &units_on_board,
            &sb.objects_on_board,
            sb.board,
            &mut rng,
        )
        .expect("resolve_effect_tree 應成功執行");

        // 目標應收到對應效果
        let target_entries = find_entries_for(&entries, &target_occupant);
        assert_eq!(target_entries.len(), 1, "{label}: 目標應有 1 個效果條目");
        assert_eq!(
            target_entries[0],
            &EffectEntry {
                caster: TEST_CASTER_ID,
                skill_name: TEST_SKILL_NAME.to_string(),
                target: occupant_to_check_target(target_occupant),
                check: CheckResult::Auto,
                check_detail: None,
                effect: ResolvedEffect::HpChange {
                    raw_amount: expected_hp,
                    final_amount: expected_hp,
                },
            },
            "{label}: 目標應有 {expected_hp} 的血量變化"
        );

        // 自身補血：who=Caster，無論瞄準誰都生效
        let caster_entries = find_entries_for(&entries, &sb.caster_occupant);
        assert_eq!(caster_entries.len(), 1, "{label}: 施放者應有 1 個效果條目");
        assert_eq!(
            caster_entries[0],
            &EffectEntry {
                caster: TEST_CASTER_ID,
                skill_name: TEST_SKILL_NAME.to_string(),
                target: occupant_to_check_target(sb.caster_occupant),
                check: CheckResult::Auto,
                check_detail: None,
                effect: ResolvedEffect::HpChange {
                    raw_amount: 300,
                    final_amount: 300,
                }
            },
            "{label}: 施放者應補血 300"
        );
    }
}

// ============================================================================
// 案例 2：頂層 Leaf 在空地召喚牆壁
// ============================================================================

/// 頂層 Leaf SpawnObject，分別瞄準有單位的格子和空格
#[test]
fn test_spawn_object_on_empty_and_occupied() {
    let sb = build_shared_board();

    let caster_stats = sb.units_on_board[&sb.caster_pos].clone();
    let empty_pos = sb.empty_positions[0];
    let wall = "wall";
    let nodes = vec![spawn_leaf(wall)];

    let test_data = [
        ("瞄準空格", empty_pos, true),
        ("瞄準有單位的格子", sb.enemy_pos, false),
        ("瞄準牆壁格子", sb.wall_pos, false),
        ("瞄準陷阱格子", sb.trap_pos, true),
    ];

    for (label, target_pos, should_spawn) in test_data {
        let mut rng = always_hit_rng();
        let entries = resolve_effect_tree(
            TEST_CASTER_ID,
            TEST_SKILL_NAME,
            &[],
            &nodes,
            &caster_stats,
            sb.caster_pos,
            target_pos,
            &sb.units_on_board,
            &sb.objects_on_board,
            sb.board,
            &mut rng,
        )
        .expect("resolve_effect_tree 應成功執行");

        if should_spawn {
            let pos_entries = find_entries_for_position(&entries, target_pos);
            assert_eq!(pos_entries.len(), 1, "{label}: 應產生 1 個召喚條目");
            assert_eq!(
                pos_entries[0],
                &EffectEntry {
                    caster: TEST_CASTER_ID,
                    skill_name: TEST_SKILL_NAME.to_string(),
                    target: CheckTarget::Position(target_pos),
                    check: CheckResult::Auto,
                    check_detail: None,
                    effect: ResolvedEffect::SpawnObject {
                        object_type: wall.to_string(),
                    },
                },
                "{label}: 應召喚 wall"
            );
        } else {
            assert!(
                entries.is_empty(),
                "{label}: 被占據的格子不應產生召喚條目，實際: {entries:?}"
            );
        }
    }
}

// ============================================================================
// 案例 3：Area + Leaf 在空地召喚牆壁
// ============================================================================

/// Area 下的 Leaf SpawnObject
/// 共享棋盤中 C 的鄰居包含：友軍(A)、敵軍(E)、牆壁(W)、陷阱(T)、空格
/// 預期：只在空格和可通過物件格上召喚
#[test]
fn test_area_spawn_object() {
    let sb = build_shared_board();

    let caster_stats = sb.units_on_board[&sb.caster_pos].clone();

    let test_data = [
        ("Diamond radius=1", Area::Diamond { radius: 1 }, 0),
        ("Diamond radius=2", Area::Diamond { radius: 2 }, 4),
    ];

    for (label, area, expected_empty_spaces) in test_data {
        let wall = "wall";
        let nodes = vec![EffectNode::Area {
            area,
            filter: TargetFilter::Any,
            nodes: vec![spawn_leaf(wall)],
        }];

        let mut rng = always_hit_rng();
        let entries = resolve_effect_tree(
            TEST_CASTER_ID,
            TEST_SKILL_NAME,
            &[],
            &nodes,
            &caster_stats,
            sb.caster_pos,
            sb.caster_pos,
            &sb.units_on_board,
            &sb.objects_on_board,
            sb.board,
            &mut rng,
        )
        .expect("resolve_effect_tree 應成功執行");

        // 友軍格不應召喚
        let ally_entry = find_entries_for_position(&entries, sb.ally_pos);
        assert!(
            ally_entry.is_empty(),
            "{label}: 有單位的格子不應召喚，實際: {ally_entry:?}"
        );

        // 敵軍格不應召喚
        let enemy_entry = find_entries_for_position(&entries, sb.enemy_pos);
        assert!(
            enemy_entry.is_empty(),
            "{label}: 有單位的格子不應召喚，實際: {enemy_entry:?}"
        );

        // 不可通過物件格不應召喚
        let wall_entry = find_entries_for_position(&entries, sb.wall_pos);
        assert!(
            wall_entry.is_empty(),
            "{label}: 不可通過物件格不應召喚，實際: {wall_entry:?}"
        );

        // 可通過物件格應召喚
        let trap_entries = find_entries_for_position(&entries, sb.trap_pos);
        assert_eq!(
            trap_entries.len(),
            1,
            "{label}: 可通過物件格應產生 1 個召喚條目"
        );
        assert_eq!(
            trap_entries[0],
            &EffectEntry {
                caster: TEST_CASTER_ID,
                skill_name: TEST_SKILL_NAME.to_string(),
                target: CheckTarget::Position(sb.trap_pos),
                check: CheckResult::Auto,
                check_detail: None,
                effect: ResolvedEffect::SpawnObject {
                    object_type: wall.to_string(),
                },
            },
            "{label}: 可通過物件格應召喚 wall"
        );

        // 空地應召喚
        if expected_empty_spaces == 4 {
            for target_pos in &sb.empty_positions {
                let target_entries = find_entries_for_position(&entries, *target_pos);
                assert_eq!(target_entries.len(), 1, "{label}: 應產生 1 個召喚條目");
                assert_eq!(
                    target_entries[0],
                    &EffectEntry {
                        caster: TEST_CASTER_ID,
                        skill_name: TEST_SKILL_NAME.to_string(),
                        target: CheckTarget::Position(*target_pos),
                        check: CheckResult::Auto,
                        check_detail: None,
                        effect: ResolvedEffect::SpawnObject {
                            object_type: wall.to_string(),
                        },
                    },
                    "{label}: 可通過物件格應召喚 wall"
                );
            }
        }
    }
}

// ============================================================================
// 案例 4：巢狀 Branch — 命中 → 扣血 + fort 判定 → 成功上毒/失敗再扣血
// ============================================================================

/// Branch 結構：
///   Branch(Physical, AgilityAndBlock)
///     on_success:
///       Leaf(HpEffect -100%)         ← 扣血
///       Branch(Physical, Fortitude)   ← fort 判定
///         on_success:
///           Leaf(ApplyBuff poison)    ← 上毒
///         on_failure:
///           Leaf(HpEffect -50%)       ← 再扣血
///     on_failure: []                  ← NoEffect
#[test]
fn test_nested_branch_hit_then_fort() {
    let mut sb = build_shared_board();

    let caster_atk = 1000;
    let mut units_on_board = std::mem::take(&mut sb.units_on_board);
    units_on_board.insert(
        sb.caster_pos,
        build_stats_with_atk(units_on_board[&sb.caster_pos].unit_info.clone(), caster_atk),
    );
    let caster_stats = units_on_board[&sb.caster_pos].clone();

    let inner_branch = physical_hit_branch(
        DefenseType::Fortitude,
        vec![apply_buff_leaf(poison_buff())],
        vec![hp_leaf_target(Attribute::PhysicalAttack, -50)],
    );
    let outer_branch = physical_hit_branch(
        DefenseType::AgilityAndBlock,
        vec![
            hp_leaf_target(Attribute::PhysicalAttack, -100),
            inner_branch,
        ],
        vec![],
    );
    let nodes = vec![outer_branch];

    // 外層命中判定用的防禦值（agility）與內層 fort 判定用的防禦值分開設定
    #[derive(Debug, Clone, Copy)]
    enum ExpectedBranchResult {
        Evade,
        HitAndPoison,
        HitAndExtraDamage,
    }
    let enemy_agility_high = 90;
    let enemy_agility_low = 10;
    let enemy_fortitude_high = 90;
    let enemy_fortitude_low = 10;

    let test_data = [
        // (label, agility, fortitude, 預期行為)
        (
            "外層閃避 → NoEffect",
            enemy_agility_high,
            enemy_fortitude_low,
            ExpectedBranchResult::Evade,
        ),
        (
            "外層命中、內層命中 → 扣血 + 上毒",
            enemy_agility_low,
            enemy_fortitude_low,
            ExpectedBranchResult::HitAndPoison,
        ),
        (
            "外層命中、內層閃避 → 扣血 + 再扣血",
            enemy_agility_low,
            enemy_fortitude_high,
            ExpectedBranchResult::HitAndExtraDamage,
        ),
    ];

    for (label, agility, fortitude, expected) in test_data {
        let mut units_on_board = units_on_board.clone();
        let mut enemy_stats = build_stats(units_on_board[&sb.enemy_pos].unit_info.clone());
        enemy_stats.attribute.agility = Agility(agility);
        // 確保格擋生效
        enemy_stats.attribute.block = Block(1000000);
        // 減少傷害 25%
        enemy_stats.attribute.block_protection = BlockProtection(25);
        enemy_stats.attribute.fortitude = Fortitude(fortitude);
        units_on_board.insert(sb.enemy_pos, enemy_stats);

        let mut rng = fixed_rng(50);
        let entries = resolve_effect_tree(
            TEST_CASTER_ID,
            TEST_SKILL_NAME,
            &[],
            &nodes,
            &caster_stats,
            sb.caster_pos,
            sb.enemy_pos,
            &units_on_board,
            &sb.objects_on_board,
            sb.board,
            &mut rng,
        )
        .expect("resolve_effect_tree 應成功執行");

        let enemy_entries = find_entries_for(&entries, &sb.enemy_occupant);
        match expected {
            ExpectedBranchResult::Evade => {
                assert_eq!(
                    enemy_entries.len(),
                    1,
                    "{label}: 閃避應產生 1 個 NoEffect 條目"
                );
                assert_eq!(
                    enemy_entries[0],
                    &EffectEntry {
                        caster: TEST_CASTER_ID,
                        skill_name: TEST_SKILL_NAME.to_string(),
                        target: occupant_to_check_target(sb.enemy_occupant),
                        check: CheckResult::Evade,
                        check_detail: Some(CheckDetail {
                            accuracy_source: AccuracySource::Physical,
                            defense_type: DefenseType::AgilityAndBlock,
                            attacker_accuracy: 0,
                            defender_evasion: enemy_agility_high,
                            defender_block: 1000000,
                            crit_rate: 0,
                            roll: 50,
                        }),
                        effect: ResolvedEffect::NoEffect,
                    },
                    "{label}"
                );
            }
            ExpectedBranchResult::HitAndPoison => {
                assert_eq!(
                    enemy_entries.len(),
                    2,
                    "{label}: 命中+毒應產生 2 個條目，實際: {enemy_entries:#?}"
                );
                // 第一個：扣血
                assert_eq!(
                    enemy_entries[0],
                    &EffectEntry {
                        caster: TEST_CASTER_ID,
                        skill_name: TEST_SKILL_NAME.to_string(),
                        target: occupant_to_check_target(sb.enemy_occupant),
                        check: CheckResult::Block { crit: false },
                        check_detail: Some(CheckDetail {
                            accuracy_source: AccuracySource::Physical,
                            defense_type: DefenseType::AgilityAndBlock,
                            attacker_accuracy: 0,
                            defender_evasion: enemy_agility_low,
                            defender_block: 1000000,
                            crit_rate: 0,
                            roll: 50,
                        }),
                        effect: ResolvedEffect::HpChange {
                            raw_amount: -1000,
                            final_amount: -750
                        },
                    },
                    "{label}: 扣血條目應為 block"
                );
                // 第二個：上毒（內層 fort 判定成功）
                assert_eq!(
                    enemy_entries[1],
                    &EffectEntry {
                        caster: TEST_CASTER_ID,
                        skill_name: TEST_SKILL_NAME.to_string(),
                        target: occupant_to_check_target(sb.enemy_occupant),
                        check: CheckResult::Affected,
                        check_detail: Some(CheckDetail {
                            accuracy_source: AccuracySource::Physical,
                            defense_type: DefenseType::Fortitude,
                            attacker_accuracy: 0,
                            defender_evasion: enemy_fortitude_low,
                            defender_block: 0,
                            crit_rate: 0,
                            roll: 50,
                        }),
                        effect: ResolvedEffect::ApplyBuff("poison".to_string()),
                    },
                    "{label}: 上毒條目應該生效（物理 fort 判定成功）"
                );
            }
            ExpectedBranchResult::HitAndExtraDamage => {
                // 外層命中、內層閃避 → 2 個條目：扣血 + 再扣血
                assert_eq!(
                    enemy_entries.len(),
                    2,
                    "{label}: 命中+再扣血應產生 2 個條目，實際: {enemy_entries:#?}"
                );
                // 第一個：扣血
                assert_eq!(
                    enemy_entries[0],
                    &EffectEntry {
                        caster: TEST_CASTER_ID,
                        skill_name: TEST_SKILL_NAME.to_string(),
                        target: occupant_to_check_target(sb.enemy_occupant),
                        check: CheckResult::Block { crit: false },
                        check_detail: Some(CheckDetail {
                            accuracy_source: AccuracySource::Physical,
                            defense_type: DefenseType::AgilityAndBlock,
                            attacker_accuracy: 0,
                            defender_evasion: enemy_agility_low,
                            defender_block: 1000000,
                            crit_rate: 0,
                            roll: 50,
                        }),
                        effect: ResolvedEffect::HpChange {
                            raw_amount: -1000,
                            final_amount: -750
                        },
                    },
                    "{label}: 扣血條目應為 block"
                );
                // 第二個：再扣血（內層 fort 判定失敗走 on_failure）
                assert_eq!(
                    enemy_entries[1],
                    &EffectEntry {
                        caster: TEST_CASTER_ID,
                        skill_name: TEST_SKILL_NAME.to_string(),
                        target: occupant_to_check_target(sb.enemy_occupant),
                        check: CheckResult::Resisted,
                        check_detail: Some(CheckDetail {
                            accuracy_source: AccuracySource::Physical,
                            defense_type: DefenseType::Fortitude,
                            attacker_accuracy: 0,
                            defender_evasion: enemy_fortitude_high,
                            defender_block: 0,
                            crit_rate: 0,
                            roll: 50,
                        }),
                        effect: ResolvedEffect::HpChange {
                            raw_amount: -500,
                            final_amount: -500
                        },
                    },
                    "{label}: 扣血條目應為 resisted"
                );
            }
        }
    }
}

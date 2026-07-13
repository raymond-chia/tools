//! 相鄰敵人降低命中判定整合測試（透過 resolve_effect_tree 驗證命中懲罰）

use crate::domain::alias::ID;
use crate::domain::constants::{ACCURACY_PENALTY_WHEN_ENEMY_ADJACENT, PLAYER_FACTION_ID};
use crate::domain::core_types::{
    AccuracySource, CasterOrTarget, DefenseType, Effect, EffectCondition, EffectNode, Scaling,
    SkillTag,
};
use crate::ecs_types::components::*;
use crate::ecs_types::resources::Board;
use crate::logic::skill::skill_execution::{
    CheckResult, CombatStats, ObjectOnBoard, resolve_effect_tree,
};
use crate::test_helpers::level_builder::LevelBuilder;
use std::collections::HashMap;

const ALLY_FACTION_ID: ID = PLAYER_FACTION_ID;
const ENEMY_FACTION_ID: ID = 2;
const TEST_CASTER_ID: ID = 9999;
const TEST_SKILL_NAME: &str = "adjacent_penalty_test";

/// 骰值落在「無懲罰則命中、有懲罰 -20 則閃避」的區間：
/// - 無懲罰：attacker_acc=20、evasion_threshold=30，骰 35 ≥ 30 → Hit
/// - 有懲罰 -20：attacker_acc=0、evasion_threshold=50，骰 35 < 50 → Evade
const ROLL_MID: i32 = 35;
const TARGET_AGILITY: i32 = 50;
const CASTER_BASE_ACCURACY: i32 = 20;

/// 第三單位 X 是否與施放者 C 相鄰。施放者 C 與目標 T 一律不相鄰。
struct Setup {
    board: Board,
    caster_pos: Position,
    target_pos: Position,
    units_on_board: HashMap<Position, CombatStats>,
    objects_on_board: HashMap<Position, ObjectOnBoard>,
    caster_stats: CombatStats,
}

fn build_setup(adjacent: bool, third_faction_id: ID) -> Setup {
    let ascii = if adjacent {
        r#"
        . X C . T
        . . . . .
        "#
    } else {
        r#"
        X . C . T
        . . . . .
        "#
    };
    let builder = LevelBuilder::from_ascii(ascii)
        .unit("C", "caster", PLAYER_FACTION_ID)
        .unit("T", "target", ENEMY_FACTION_ID)
        .unit("X", "third", third_faction_id);
    let (board, _positions, unit_markers) =
        builder.to_unit_map().expect("建構相鄰懲罰測試棋盤應成功");

    let caster_pos = unit_markers["C"][0].position;
    let target_pos = unit_markers["T"][0].position;

    let mut units_on_board: HashMap<Position, CombatStats> = unit_markers
        .values()
        .flatten()
        .map(|entry| {
            let mut stats = CombatStats {
                unit_info: entry.unit_info.clone(),
                attribute: AttributeBundle::default(),
            };
            // 目標設定固定的閃避門檻，方便驗證命中是否被推過門檻
            if entry.unit_info.occupant == unit_markers["T"][0].unit_info.occupant {
                stats.attribute.agility = Agility(TARGET_AGILITY);
            }
            (entry.position, stats)
        })
        .collect();

    let caster_stats = {
        let stats = units_on_board
            .get_mut(&caster_pos)
            .expect("caster 應該存在於 units_on_board");
        stats.attribute.physical_accuracy = PhysicalAccuracy(CASTER_BASE_ACCURACY);
        stats.clone()
    };

    Setup {
        board,
        caster_pos,
        target_pos,
        units_on_board,
        objects_on_board: HashMap::new(),
        caster_stats,
    }
}

/// 物理命中判定 + 扣血的 Branch 節點
fn physical_attack_node() -> EffectNode {
    EffectNode::Branch {
        condition: EffectCondition {
            defense_type: DefenseType::AgilityAndBlock,
            accuracy_source: AccuracySource::Physical,
            accuracy_bonus: 0,
            crit_bonus: 0,
        },
        on_success: vec![EffectNode::Leaf {
            who: CasterOrTarget::Target,
            effect: Effect::HpEffect {
                scaling: Scaling {
                    source: CasterOrTarget::Caster,
                    source_attribute: crate::domain::core_types::Attribute::PhysicalAttack,
                    value_percent: 100,
                },
            },
        }],
        on_failure: vec![],
    }
}

fn fixed_rng(value: i32) -> impl FnMut() -> i32 {
    move || value
}

/// 表格驅動：驗證相鄰敵人命中懲罰在「相鄰有敵人 + tag」兩條件齊備時才生效。
/// 施放者與目標一律不相鄰，故目標本身不會觸發懲罰。
#[test]
fn test_adjacent_enemy_accuracy_penalty() {
    #[derive(Debug, Clone, Copy)]
    enum Expected {
        Hit,
        Evade,
    }

    let with_tag = &[SkillTag::AccuracyPenaltyWhenEnemyAdjacent][..];
    let no_tag = &[][..];

    let test_data = [
        (
            "相鄰敵人 + tag → 懲罰生效（閃避）",
            true,
            ENEMY_FACTION_ID,
            with_tag,
            Expected::Evade,
            CASTER_BASE_ACCURACY + ACCURACY_PENALTY_WHEN_ENEMY_ADJACENT,
        ),
        (
            "不相鄰敵人 + tag → 懲罰不生效（命中）",
            false,
            ENEMY_FACTION_ID,
            with_tag,
            Expected::Hit,
            CASTER_BASE_ACCURACY,
        ),
        (
            "相鄰敵人 + 無 tag → 懲罰不生效（命中）",
            true,
            ENEMY_FACTION_ID,
            no_tag,
            Expected::Hit,
            CASTER_BASE_ACCURACY,
        ),
        (
            "不相鄰敵人 + 無 tag → 懲罰不生效（命中）",
            false,
            ENEMY_FACTION_ID,
            no_tag,
            Expected::Hit,
            CASTER_BASE_ACCURACY,
        ),
        (
            "相鄰友軍 + tag → 懲罰不生效（命中）",
            true,
            ALLY_FACTION_ID,
            with_tag,
            Expected::Hit,
            CASTER_BASE_ACCURACY,
        ),
        (
            "不相鄰友軍 + tag → 懲罰不生效（命中）",
            false,
            ALLY_FACTION_ID,
            with_tag,
            Expected::Hit,
            CASTER_BASE_ACCURACY,
        ),
    ];

    for (label, adjacent, third_faction_id, tags, expected, expected_attacker_accuracy) in test_data
    {
        let setup = build_setup(adjacent, third_faction_id);

        let mut rng = fixed_rng(ROLL_MID);
        let entries = resolve_effect_tree(
            TEST_CASTER_ID,
            TEST_SKILL_NAME,
            tags,
            std::slice::from_ref(&physical_attack_node()),
            &setup.caster_stats,
            setup.caster_pos,
            setup.target_pos,
            &setup.units_on_board,
            &setup.objects_on_board,
            setup.board,
            &mut rng,
            false,
        )
        .expect("resolve_effect_tree 應成功執行");

        assert_eq!(entries.len(), 1, "{label}: 應有 1 筆條目");
        let detail = entries[0]
            .check_detail
            .as_ref()
            .expect("命中判定應有 detail");
        assert_eq!(
            detail.breakdowns.attacker_accuracy.total, expected_attacker_accuracy,
            "{label}: attacker_accuracy 不符"
        );
        match expected {
            Expected::Hit => assert!(
                matches!(entries[0].check, CheckResult::Hit { .. }),
                "{label}: 應命中，實際: {:?}",
                entries[0].check,
            ),
            Expected::Evade => assert_eq!(entries[0].check, CheckResult::Evade, "{label}: 應閃避"),
        }
    }
}

//! 夾擊判定整合測試（透過 resolve_effect_tree 驗證命中加成）

use crate::domain::alias::ID;
use crate::domain::constants::PLAYER_FACTION_ID;
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
const TEST_SKILL_NAME: &str = "flank_test";
/// 骰值落在「無夾擊加成則閃避、有加成則命中」的區間：
/// - 無加成：attacker_acc=0、evasion_threshold=50，骰 49 < 50 → Evade
/// - 有加成 +20：attacker_acc=20、evasion_threshold=30，骰 49 ≥ 30 → Hit
const ROLL_BELOW_AGILITY: i32 = 49;
const ENEMY_AGILITY: i32 = 50;
const FLANKING_BONUS: i32 = 20;

/// 夾擊棋盤佈局（含友軍 A）：caster C 與隊友 A 都跟敵軍 E 相鄰，形成夾擊
/// ```text
/// .  A  .
/// .  E  C
/// .  .  .
/// ```
/// 非夾擊佈局（無友軍）：只有 caster C 與敵軍 E 相鄰
/// ```text
/// .  .  .
/// .  E  C
/// .  .  .
/// ```
struct Setup {
    board: Board,
    caster_pos: Position,
    enemy_pos: Position,
    units_on_board: HashMap<Position, CombatStats>,
    objects_on_board: HashMap<Position, ObjectOnBoard>,
    caster_stats: CombatStats,
}

fn build_setup(include_ally: bool, flanking_bonus: i32) -> Setup {
    let ascii = if include_ally {
        r#"
        .  A  .
        .  E  C
        .  .  .
        "#
    } else {
        r#"
        .  .  .
        .  E  C
        .  .  .
        "#
    };
    let builder = LevelBuilder::from_ascii(ascii)
        .unit("C", "caster", PLAYER_FACTION_ID)
        .unit("E", "enemy", ENEMY_FACTION_ID)
        .unit("A", "ally", ALLY_FACTION_ID);
    let (board, _positions, unit_markers) = builder.to_unit_map().expect("建構夾擊測試棋盤應成功");

    let caster_pos = unit_markers["C"][0].position;
    let enemy_pos = unit_markers["E"][0].position;

    let mut units_on_board: HashMap<Position, CombatStats> = unit_markers
        .values()
        .flatten()
        .map(|entry| {
            let mut stats = CombatStats {
                unit_info: entry.unit_info.clone(),
                attribute: AttributeBundle::default(),
            };
            // 敵軍設定固定的閃避門檻，方便驗證命中是否被推過門檻
            if entry.unit_info.faction_id == ENEMY_FACTION_ID {
                stats.attribute.agility = Agility(ENEMY_AGILITY);
            }
            (entry.position, stats)
        })
        .collect();

    let caster_stats = {
        let stats = units_on_board
            .get_mut(&caster_pos)
            .expect("caster 應該存在於 units_on_board");
        stats.attribute.flanking_accuracy_bonus = FlankingAccuracyBonus(flanking_bonus);
        stats.clone()
    };

    Setup {
        board,
        caster_pos,
        enemy_pos,
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

/// 表格驅動：驗證夾擊加成在「夾擊成立 + Flankable tag」三個條件齊備時才生效
#[test]
fn test_flanking_accuracy_bonus() {
    #[derive(Debug, Clone, Copy)]
    enum Expected {
        Hit,
        Evade,
    }

    let test_data = [
        (
            "夾擊成立 + Flankable tag → 加成生效（命中）",
            true,
            &[SkillTag::Flankable][..],
            Expected::Hit,
            FLANKING_BONUS,
        ),
        (
            "夾擊成立但無 Flankable tag → 加成不生效（閃避）",
            true,
            &[][..],
            Expected::Evade,
            0,
        ),
        (
            "未夾擊（只有 caster 相鄰）+ Flankable tag → 加成不生效（閃避）",
            false,
            &[SkillTag::Flankable][..],
            Expected::Evade,
            0,
        ),
    ];

    for (label, include_ally, tags, expected, expected_attacker_accuracy) in test_data {
        let setup = build_setup(include_ally, FLANKING_BONUS);

        let mut rng = fixed_rng(ROLL_BELOW_AGILITY);
        let entries = resolve_effect_tree(
            TEST_CASTER_ID,
            TEST_SKILL_NAME,
            tags,
            std::slice::from_ref(&physical_attack_node()),
            &setup.caster_stats,
            setup.caster_pos,
            setup.enemy_pos,
            &setup.units_on_board,
            &setup.objects_on_board,
            setup.board,
            &mut rng,
        )
        .expect("resolve_effect_tree 應成功執行");

        assert_eq!(entries.len(), 1, "{label}: 應有 1 筆條目");
        let detail = entries[0]
            .check_detail
            .as_ref()
            .expect("命中判定應有 detail");
        assert_eq!(
            detail.attacker_accuracy, expected_attacker_accuracy,
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

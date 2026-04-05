//! 技能執行效果樹測試

use crate::helpers::level_builder::load_from_ascii;
use board::domain::alias::{Coord, ID};
use board::domain::constants::{PLAYER_ALLIANCE_ID, PLAYER_FACTION_ID};
use board::domain::core_types::*;
use board::ecs_types::components::*;
use board::logic::skill::UnitInfo;
use board::logic::skill_execute::{
    CheckResult, CheckTarget, CombatStats, ResolvedEffect, resolve_effect_tree,
};
use std::collections::{HashMap, HashSet};

const ALLY_FACTION: u32 = 1;
const ENEMY_FACTION: u32 = 2;
const ENEMY_ALLIANCE: u32 = 1;

fn caster_unit_info() -> UnitInfo {
    UnitInfo {
        occupant: Occupant::Unit(1),
        faction_id: PLAYER_FACTION_ID,
        alliance_id: PLAYER_ALLIANCE_ID,
    }
}

fn ally_unit_info(id: ID) -> UnitInfo {
    UnitInfo {
        occupant: Occupant::Unit(id),
        faction_id: ALLY_FACTION,
        alliance_id: PLAYER_ALLIANCE_ID,
    }
}

fn enemy_unit_info(id: ID) -> UnitInfo {
    UnitInfo {
        occupant: Occupant::Unit(id),
        faction_id: ENEMY_FACTION,
        alliance_id: ENEMY_ALLIANCE,
    }
}

fn default_stats(unit_info: UnitInfo) -> CombatStats {
    CombatStats {
        unit_info,
        attribute: AttributeBundle::default(),
        crit_rate: 0,
    }
}

fn area_node(radius: Coord, node: EffectNode) -> EffectNode {
    EffectNode::Area {
        area: Area::Diamond { radius },
        filter: TargetFilter::Enemy,
        nodes: vec![node],
    }
}

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

fn hit_branch(
    accuracy_bonus: i32,
    crit_bonus: i32,
    on_success: Vec<EffectNode>,
    on_failure: Vec<EffectNode>,
) -> EffectNode {
    EffectNode::Branch {
        who: CasterOrTarget::Target,
        condition: EffectCondition::HitCheck {
            accuracy_bonus,
            crit_bonus,
        },
        on_success,
        on_failure,
    }
}

fn dc_branch(
    dc_type: DcType,
    dc_bonus: i32,
    on_success: Vec<EffectNode>,
    on_failure: Vec<EffectNode>,
) -> EffectNode {
    EffectNode::Branch {
        who: CasterOrTarget::Target,
        condition: EffectCondition::DcCheck { dc_type, dc_bonus },
        on_success,
        on_failure,
    }
}

/// 建立固定回傳值的 rng
fn fixed_rng(values: &[i32]) -> impl FnMut() -> i32 + '_ {
    let mut index = 0;
    move || {
        let value = values[index];
        index += 1;
        value
    }
}

// ============================================================================
// 無判定效果（必定命中）
// ============================================================================

#[test]
fn test_auto_check_no_branch_single() {
    // P=施法者, E=目標
    let ascii = r#"
        . . . . .
        . . P . .
        . . E F .
        . . . . .
    "#;
    let (board, markers) = load_from_ascii(ascii).expect("載入棋盤失敗");
    let caster_pos = markers["P"][0];
    let target_pos = markers["E"][0];
    let other_pos = markers["F"][0];

    let test_data = [
        // (physical_attack, value_percent, expected_amount)
        (100, -100, -100),
        (100, -50, -50),
        (100, -120, -120),
        (200, -75, -150),
        (80, -100, -80),
        (0, -100, 0),
        (100, 0, 0),
        (50, -200, -100),
    ];

    for stat in [1, 10, 100, 1000] {
        for (physical_attack, value_percent, expected_amount) in &test_data {
            let enemy_id = 2;
            let mut caster_stats = default_stats(caster_unit_info());
            caster_stats.attribute.physical_attack = PhysicalAttack(*physical_attack);
            let mut target_stats = default_stats(enemy_unit_info(enemy_id));
            target_stats.attribute.evasion = Evasion(stat);
            target_stats.attribute.block = Block(stat);
            target_stats.attribute.block_protection = BlockProtection(stat);
            let mut other_stats = default_stats(enemy_unit_info(3));
            let nodes = vec![hp_leaf(Attribute::PhysicalAttack, *value_percent)];

            let mut units_on_board = HashMap::new();
            units_on_board.insert(caster_pos, caster_stats.clone());
            units_on_board.insert(target_pos, target_stats.clone());
            units_on_board.insert(other_pos, other_stats.clone());

            let mut rng = fixed_rng(&[]);
            let entries = resolve_effect_tree(
                &nodes,
                &caster_stats,
                caster_pos,
                target_pos,
                &units_on_board,
                board,
                &mut rng,
            );

            assert_eq!(
                entries.len(),
                1,
                "attack={physical_attack} percent={value_percent} 應產生 1 個效果\nascii: {ascii}",
            );
            let entry = &entries[0];
            assert_eq!(
                entry.target,
                CheckTarget::Unit(enemy_id),
                "效果應作用在目標\nascii: {ascii}"
            );
            assert!(
                matches!(entry.check, CheckResult::Auto),
                "無 Branch 時 check 應為 Auto\nascii: {ascii}",
            );
            match &entry.effect {
                ResolvedEffect::HpChange {
                    raw_amount,
                    final_amount,
                } => {
                    assert_eq!(
                        *raw_amount, *expected_amount,
                        "attack={physical_attack} percent={value_percent} raw_amount\nascii: {ascii}",
                    );
                    assert_eq!(
                        *final_amount, *expected_amount,
                        "attack={physical_attack} percent={value_percent} final_amount（Auto 無修正）",
                    );
                }
                other => panic!("預期 HpChange，實際為 {:?}\nascii: {ascii}", other),
            }
        }
    }
}

#[test]
fn test_auto_check_no_branch_area_diamond() {
    // P=施法者, T=目標位置（施法中心）
    // diamond radius=1：以 T 為中心，曼哈頓距離 <=1
    // E 範圍內敵人。F 範圍外敵人。A 範圍內友軍
    let asciis = [
        r#"
            T E . . . . .
            . P . . . . .
            F . . . . . .
            . . . . . . .
            . . . . . . .
            . . . . . . .
        "#,
        r#"
            T E . . . . .
            A P . . . . .
            F . . . . . .
            . . . . . . .
            . . . . . . .
            . . . . . . .
        "#,
        r#"
            T E . . . . .
            E P . . . . .
            F . . . . . .
            . . . . . . .
            . . . . . . .
            . . . . . . .
        "#,
        r#"
            . . . . . . .
            . . . . . . .
            . . . . . . .
            . . . . . . .
            . . . F . . .
            . . T E P . .
        "#,
        r#"
            . . . . . . .
            . . . . . . .
            . . . . . . .
            . . . . . . .
            . . A F . . .
            . E T A P . .
        "#,
        r#"
            . . . . . . .
            . . . . . . .
            . . . . . . .
            . . . . . . .
            . . E F . . .
            . E T E P . .
        "#,
        r#"
            . . . . . . .
            . . . P . . .
            . . E T . . .
            . . . . . . .
            . . . F . . .
            . . . . . . .
        "#,
        r#"
            . . . . . . .
            . . . P . . .
            . . A T A . .
            . . . E . . .
            . . . F . . .
            . . . . . . .
        "#,
        r#"
            . . . . . . .
            . . . P . . .
            . . E T E . .
            . . . E . . .
            . . . F . . .
            . . . . . . .
        "#,
        r#"
            . . . . . . .
            . . . P . . .
            . . . . F . .
            . . . T A . .
            . . . E . . .
            . . . . . . .
        "#,
        r#"
            . . . . . . .
            . . . P . . .
            . . . E F . .
            . . E T E . .
            . . . E . . .
            . . . . . . .
        "#,
    ];

    let test_data = [
        // (physical_attack, value_percent, expected_amount)
        (100, -100, -100),
        (200, -50, -100),
        (0, -100, 0),
        (100, 0, 0),
        (100, 100, 100),
    ];

    for ascii in asciis {
        let (board, markers) = load_from_ascii(ascii).expect("載入棋盤失敗\nascii: {ascii}");
        let caster_pos = markers["P"][0];
        let target_pos = markers["T"][0];
        let a_pos = markers.get("A").cloned().unwrap_or_default();
        let e_pos = markers.get("E").cloned().unwrap_or_default();
        let f_pos = markers.get("F").cloned().unwrap_or_default();

        for (physical_attack, value_percent, expected_amount) in &test_data {
            let mut caster_stats = default_stats(caster_unit_info());
            caster_stats.attribute.physical_attack = PhysicalAttack(*physical_attack);

            let mut units_on_board = HashMap::new();
            for (i, pos) in a_pos.iter().enumerate() {
                let stats = default_stats(ally_unit_info(10 + i as u32));
                units_on_board.insert(*pos, stats);
            }
            for (i, pos) in e_pos.iter().enumerate() {
                let mut stats = default_stats(enemy_unit_info(20 + i as u32));
                stats.attribute.evasion = Evasion(100);
                stats.attribute.block = Block(100);
                stats.attribute.block_protection = BlockProtection(100);
                units_on_board.insert(*pos, stats);
            }
            for (i, pos) in f_pos.iter().enumerate() {
                let stats = default_stats(enemy_unit_info(30 + i as u32));
                units_on_board.insert(*pos, stats);
            }

            let nodes = vec![area_node(
                1,
                hp_leaf(Attribute::PhysicalAttack, *value_percent),
            )];

            let mut rng = fixed_rng(&[]);
            let entries = resolve_effect_tree(
                &nodes,
                &caster_stats,
                caster_pos,
                target_pos,
                &units_on_board,
                board,
                &mut rng,
            );

            assert_eq!(
                entries.len(),
                e_pos.len(),
                "attack={physical_attack} percent={value_percent} 應產生 {} 個效果（{} 個目標）\nascii: {ascii}",
                e_pos.len(),
                e_pos.len(),
            );
            let affected_positions: HashSet<CheckTarget> =
                entries.iter().map(|e| e.target).collect();
            let expected_positions: HashSet<CheckTarget> = e_pos
                .iter()
                .enumerate()
                .map(|(i, _)| CheckTarget::Unit(20 + i as u32))
                .collect();
            assert_eq!(
                affected_positions, expected_positions,
                "attack={physical_attack} percent={value_percent} 應作用在所有 E 位置的敵人\nascii: {ascii}",
            );

            for entry in &entries {
                assert!(
                    matches!(entry.check, CheckResult::Auto),
                    "無 Branch 時 check 應為 Auto\nascii: {ascii}",
                );
                match &entry.effect {
                    ResolvedEffect::HpChange {
                        raw_amount,
                        final_amount,
                    } => {
                        assert_eq!(
                            *raw_amount, *expected_amount,
                            "attack={physical_attack} percent={value_percent} raw_amount\nascii: {ascii}",
                        );
                        assert_eq!(
                            *final_amount, *expected_amount,
                            "attack={physical_attack} percent={value_percent} final_amount（Auto 無修正）\nascii: {ascii}",
                        );
                    }
                    other => panic!("預期 HpChange，實際為 {:?}\nascii: {ascii}", other),
                }
            }
        }
    }
}

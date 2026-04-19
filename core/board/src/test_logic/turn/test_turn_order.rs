//! 回合順序計算與管理的測試

use crate::domain::core_types::TurnEntry;
use crate::ecs_types::components::Occupant;
use crate::logic::turn_order::{
    TurnOrderInput, calculate_turn_order, delay_unit, get_active_index, remove_unit,
};

/// 取出當前未行動單位的 Occupant，方便斷言
fn get_active_unit(entries: &[TurnEntry]) -> Option<Occupant> {
    get_active_index(entries).map(|idx| entries[idx].occupant)
}

fn input(id: u32, initiative: i32, is_player: bool) -> TurnOrderInput {
    TurnOrderInput {
        occupant: unit(id),
        initiative,
        is_player,
    }
}

/// 從 entries 取出 occupant id 序列，方便斷言
fn occupant_ids(entries: &[TurnEntry]) -> Vec<u32> {
    entries
        .iter()
        .map(|e| match e.occupant {
            Occupant::Unit(id) => id,
            Occupant::Object(id) => id,
        })
        .collect()
}

fn unit(id: u32) -> Occupant {
    Occupant::Unit(id)
}

// ========================================================================
// 欄位正確性測試
// ========================================================================

#[test]
fn test_entry_fields_are_correct() {
    let mut rng_int = || 3;
    let mut rng_float = || 0.5;

    let inputs = vec![input(1, 7, true), input(2, 10, false), input(3, 5, true)];
    let test_data = [
        TurnEntry {
            occupant: unit(2),
            initiative: 10,
            roll: 3,
            total: 13,
            tiebreaker: 100.5,
            has_acted: false,
        },
        TurnEntry {
            occupant: unit(1),
            initiative: 7,
            roll: 3,
            total: 10,
            tiebreaker: 71.5,
            has_acted: false,
        },
        TurnEntry {
            occupant: unit(3),
            initiative: 5,
            roll: 3,
            total: 8,
            tiebreaker: 51.5,
            has_acted: false,
        },
    ];
    let entries = calculate_turn_order(&inputs, &mut rng_int, &mut rng_float);

    for (idx, d) in test_data.iter().enumerate() {
        assert_eq!(entries[idx].occupant, d.occupant);
        assert_eq!(entries[idx].initiative, d.initiative);
        assert_eq!(entries[idx].roll, d.roll);
        assert_eq!(entries[idx].total, d.total);
        assert_eq!(entries[idx].tiebreaker, d.tiebreaker);
        assert_eq!(entries[idx].has_acted, d.has_acted);
    }
}

// ========================================================================
// 排序測試
// ========================================================================

#[test]
fn test_sorting_scenarios() {
    // (描述, 輸入, 骰子值, float 值, 預期順序 ID)
    let test_data = [
        (
            "總和高先行動",
            vec![input(1, 10, true), input(2, 8, true), input(3, 9, true)],
            vec![3, 4, 2],
            vec![0.5, 0.5, 0.5],
            vec![1, 2, 3],
        ),
        (
            "INI 影響順序",
            vec![input(1, 10, true), input(2, 8, true), input(3, 9, true)],
            vec![3, 3, 3],
            vec![0.5, 0.5, 0.5],
            vec![1, 3, 2],
        ),
        (
            "骰子影響順序",
            vec![input(1, 10, true), input(2, 8, true), input(3, 9, true)],
            vec![1, 5, 3],
            vec![0.5, 0.5, 0.5],
            vec![2, 3, 1],
        ),
        (
            "骰子影響順序-2",
            vec![input(1, 1, true), input(2, 1, true), input(3, 1, true)],
            vec![1, 5, 3],
            vec![0.5, 0.5, 0.5],
            vec![2, 3, 1],
        ),
        (
            "同分以原始 INI 優先",
            vec![input(1, 10, true), input(2, 8, true), input(3, 9, true)],
            vec![1, 3, 2],
            vec![0.5, 0.5, 0.5],
            vec![1, 3, 2],
        ),
        (
            "同分以玩家優先",
            vec![input(1, 5, false), input(2, 5, true), input(3, 4, false)],
            vec![3, 3, 3],
            vec![0.5, 0.5, 0.5],
            vec![2, 1, 3],
        ),
        (
            "同分看小數點亂數",
            vec![input(1, 5, false), input(2, 5, false), input(3, 5, false)],
            vec![3, 3, 3],
            vec![0.3, 0.9, 0.7],
            vec![2, 3, 1],
        ),
        (
            "同分看小數點亂數-2",
            vec![input(1, 5, true), input(2, 5, true), input(3, 5, true)],
            vec![3, 3, 3],
            vec![0.3, 0.9, 0.7],
            vec![2, 3, 1],
        ),
        (
            "完全同分",
            vec![input(1, 5, true), input(2, 5, true), input(3, 5, true)],
            vec![3, 3, 3],
            vec![0.5, 0.5, 0.5],
            vec![1, 2, 3],
        ),
    ];

    for (desc, inputs, dice_values, float_values, expected_ids) in &test_data {
        let mut dice_iter = dice_values.iter();
        let mut rng_int = || *dice_iter.next().expect("骰值不足");

        let mut float_iter = float_values.iter();
        let mut rng_float = || *float_iter.next().expect("float 值不足");

        let entries = calculate_turn_order(&inputs, &mut rng_int, &mut rng_float);

        assert_eq!(
            occupant_ids(&entries),
            *expected_ids,
            "測試 '{}' 失敗",
            desc
        );
    }
}

// ========================================================================
// 完整回合流程測試
// ========================================================================

/// 模擬一輪正常流程：依序行動直到回合結束
#[test]
fn test_normal_round_flow() {
    let inputs = vec![input(1, 10, true), input(2, 8, false), input(3, 5, true)];
    let mut rng_int = || 3;
    let mut rng_float = || 0.5;
    let mut entries = calculate_turn_order(&inputs, &mut rng_int, &mut rng_float);

    // 順序：Unit(1) total=13, Unit(2) total=11, Unit(3) total=8
    assert_eq!(occupant_ids(&entries), vec![1, 2, 3]);

    // 第一個行動者（連續呼叫應回傳相同結果）
    assert_eq!(get_active_unit(&entries), Some(unit(1)));
    assert_eq!(get_active_unit(&entries), Some(unit(1)));
    entries[0].has_acted = true;

    // 第二個行動者
    assert_eq!(get_active_unit(&entries), Some(unit(2)));
    entries[1].has_acted = true;

    // 第三個行動者
    assert_eq!(get_active_unit(&entries), Some(unit(3)));
    entries[2].has_acted = true;

    // 回合結束
    assert_eq!(get_active_unit(&entries), None);
}

/// 模擬延後行動：當前單位延後，下一個人先行動
#[test]
fn test_delay_then_continue() {
    let inputs = vec![input(1, 10, true), input(2, 8, false), input(3, 5, true)];
    let mut rng_int = || 3;
    let mut rng_float = || 0.5;
    let mut entries = calculate_turn_order(&inputs, &mut rng_int, &mut rng_float);
    let idx = 0;

    // 順序：[1, 2, 3]
    assert_eq!(get_active_unit(&entries), Some(unit(1)));
    delay_unit(&mut entries, 1).expect("delay 應該成功");
    assert_eq!(occupant_ids(&entries), vec![2, 1, 3]);

    assert_eq!(get_active_unit(&entries), Some(unit(2)));
    delay_unit(&mut entries, 2).expect("delay 應該成功");
    assert_eq!(occupant_ids(&entries), vec![1, 3, 2]);

    assert_eq!(get_active_unit(&entries), Some(unit(1)));
    entries[idx].has_acted = true;
    let idx = idx + 1;

    assert_eq!(get_active_unit(&entries), Some(unit(3)));
    delay_unit(&mut entries, 2).expect("delay 應該成功");
    assert_eq!(occupant_ids(&entries), vec![1, 2, 3]);

    assert_eq!(get_active_unit(&entries), Some(unit(2)));
    entries[idx].has_acted = true;
    let idx = idx + 1;

    assert_eq!(get_active_unit(&entries), Some(unit(3)));
    entries[idx].has_acted = true;

    assert_eq!(get_active_unit(&entries), None);
}

/// 模擬中途移除單位後繼續回合
#[test]
fn test_remove_unit_then_continue() {
    let inputs = vec![input(1, 10, true), input(2, 8, false), input(3, 5, true)];
    let mut rng_int = || 3;
    let mut rng_float = || 0.5;
    let mut entries = calculate_turn_order(&inputs, &mut rng_int, &mut rng_float);
    let idx = 0;

    assert_eq!(occupant_ids(&entries), vec![1, 2, 3]);

    assert_eq!(get_active_unit(&entries), Some(unit(1)));
    entries[idx].has_acted = true;
    let idx = idx + 1;

    let removed = remove_unit(&mut entries, Occupant::Unit(2)).expect("應移除成功");
    assert_eq!(removed.occupant, Occupant::Unit(2));
    assert_eq!(occupant_ids(&entries), vec![1, 3]);

    assert_eq!(get_active_unit(&entries), Some(unit(3)));
    entries[idx].has_acted = true;

    assert_eq!(get_active_unit(&entries), None);

    let removed = remove_unit(&mut entries, Occupant::Unit(3)).expect("應移除成功");
    assert_eq!(removed.occupant, Occupant::Unit(3));
    assert_eq!(occupant_ids(&entries), vec![1]);

    assert_eq!(get_active_unit(&entries), None);
}

// ========================================================================
// 錯誤情境
// ========================================================================

#[test]
fn test_delay_unit_invalid_targets() {
    let test_data = [
        ("同位置應失敗", 1),
        ("往前應失敗", 0),
        ("超出範圍應失敗", 3),
    ];

    for (desc, target) in &test_data {
        let inputs = vec![input(1, 10, true), input(2, 8, false), input(3, 5, true)];
        let mut rng_int = || 3;
        let mut rng_float = || 0.5;
        let mut entries = calculate_turn_order(&inputs, &mut rng_int, &mut rng_float);

        entries[0].has_acted = true;
        assert!(
            delay_unit(&mut entries, *target).is_err(),
            "測試 '{}' 應回傳錯誤",
            desc
        );
    }
}

/// 移除不存在的單位應回傳錯誤
#[test]
fn test_remove_nonexistent_unit() {
    let inputs = vec![input(1, 10, true)];
    let mut rng_int = || 3;
    let mut rng_float = || 0.5;
    let mut entries = calculate_turn_order(&inputs, &mut rng_int, &mut rng_float);

    assert!(remove_unit(&mut entries, Occupant::Unit(99)).is_err());
    assert_eq!(occupant_ids(&entries), vec![1]);

    let removed = remove_unit(&mut entries, Occupant::Unit(1)).expect("應移除成功");
    assert_eq!(removed.occupant, Occupant::Unit(1));
    assert_eq!(occupant_ids(&entries), vec![]);
}

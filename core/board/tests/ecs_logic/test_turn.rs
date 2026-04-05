//! 回合順序 ECS 操作測試

use super::constants::{UNIT_TYPE_MAGE, UNIT_TYPE_WARRIOR};
use super::setup_world_with_level;
use board::domain::constants::PLAYER_FACTION_ID;
use board::ecs_logic::movement::execute_move;
use board::ecs_logic::turn::{
    can_delay_current_unit, delay_current_unit, end_battle, end_current_turn, get_turn_order,
    remove_dead_unit, start_new_round,
};
use board::ecs_types::components::{Occupant, Position};
use board::test_helpers::level_builder::LevelBuilder;

// ============================================================================
// start_new_round 測試
// ============================================================================

/// 驗證 start_new_round 正確初始化回合順序
#[test]
fn test_start_new_round_creates_turn_order() {
    let level_toml = LevelBuilder::from_ascii(
        "
        . . . . .
        . U1 . . .
        . . U2 . .
        . . . U3 .
        . . . . .
    ",
    )
    .unit("U1", UNIT_TYPE_WARRIOR, 1)
    .unit("U2", UNIT_TYPE_WARRIOR, 2)
    .unit("U3", UNIT_TYPE_MAGE, 2)
    .to_toml()
    .expect("LevelBuilder::to_toml 應成功");
    let mut world = setup_world_with_level(&level_toml);

    let turn_order = start_new_round(&mut world).expect("start_new_round 應成功");
    assert_eq!(turn_order.round, 1, "第一輪應為 round = 1");
    assert_eq!(turn_order.entries.len(), 3, "應有 3 個單位");
    assert_eq!(turn_order.current_index, 0, "應從第一個單位開始");
    for entry in &turn_order.entries {
        assert!(!entry.has_acted, "新建立的回合中，所有單位應未行動");
    }

    let result = start_new_round(&mut world);
    assert!(
        result.is_err(),
        "TurnOrder 已存在時，第二次 start_new_round 應報錯"
    );

    end_battle(&mut world).expect("結束戰鬥應成功");
    let result = get_turn_order(&world);
    assert!(result.is_err(), "結束戰鬥後應無法取得 TurnOrder");

    let turn_order = start_new_round(&mut world).expect("開始第二場戰鬥應成功");
    assert_eq!(turn_order.round, 1, "新戰鬥應從 round = 1 開始");
    assert_eq!(turn_order.current_index, 0, "新戰鬥應從第一個單位開始");
}

// ============================================================================
// end_current_turn 測試
// ============================================================================

/// 驗證 end_current_turn 正確推進到下一個單位
#[test]
fn test_end_current_turn_advances_to_next_unit() {
    let level_toml = LevelBuilder::from_ascii(
        "
        . U1 . U2 . U3 .
        . . . . . . .
        . . . . . . .
    ",
    )
    .unit("U1", UNIT_TYPE_WARRIOR, 1)
    .unit("U2", UNIT_TYPE_WARRIOR, 1)
    .unit("U3", UNIT_TYPE_MAGE, 1)
    .to_toml()
    .expect("LevelBuilder::to_toml 應成功");
    let mut world = setup_world_with_level(&level_toml);

    start_new_round(&mut world).expect("開始回合應成功");
    for i in 0..=2 {
        let turn_order = get_turn_order(&world).expect("應取得 TurnOrder");
        assert_eq!(turn_order.entries[i].has_acted, false,);
        assert_eq!(turn_order.round, 1, "第一輪應為 round = 1");
        assert_eq!(turn_order.current_index, i, "應從索引 {} 的單位開始", i);
        end_current_turn(&mut world).expect("結束回合應成功");
        if i < 2 {
            let turn_order = get_turn_order(&world).expect("應取得 TurnOrder");
            assert_eq!(turn_order.entries[i].has_acted, true);
        } else {
            let turn_order = get_turn_order(&world).expect("應取得 TurnOrder");
            assert_eq!(turn_order.round, 2, "第二輪應為 round = 2");
            assert_eq!(turn_order.current_index, 0, "初始應從第一個單位開始");
        }
    }
}

/// 驗證只剩一個單位時，end_current_turn 正確換輪
#[test]
fn test_end_current_turn_single_unit_advances_round() {
    let level_toml = LevelBuilder::from_ascii(
        "
        . U1 .
        . . .
        . . .
    ",
    )
    .unit("U1", UNIT_TYPE_WARRIOR, 1)
    .to_toml()
    .expect("LevelBuilder::to_toml 應成功");
    let mut world = setup_world_with_level(&level_toml);

    start_new_round(&mut world).expect("開始回合應成功");
    let turn_order = get_turn_order(&world).expect("應取得 TurnOrder");
    assert_eq!(turn_order.round, 1);
    assert_eq!(turn_order.current_index, 0);
    assert_eq!(turn_order.entries[0].has_acted, false, "新輪的單位應未行動");

    end_current_turn(&mut world).expect("結束回合應成功");
    let turn_order = get_turn_order(&world).expect("應取得 TurnOrder");
    assert_eq!(turn_order.round, 2, "唯一單位行動完應換輪");
    assert_eq!(turn_order.current_index, 0);
    assert_eq!(turn_order.entries[0].has_acted, false, "新輪的單位應未行動");
}

// ============================================================================
// delay_current_unit 測試
// ============================================================================

/// 驗證 delay_current_unit 正確延後單位
#[test]
fn test_delay_current_unit_moves_unit_to_target_position() {
    let level_toml = LevelBuilder::from_ascii(
        "
        . U1 . U2 . U3 .
        . . . . . . .
        . . . . . . .
    ",
    )
    .unit("U1", UNIT_TYPE_WARRIOR, 1)
    .unit("U2", UNIT_TYPE_WARRIOR, 1)
    .unit("U3", UNIT_TYPE_MAGE, 1)
    .to_toml()
    .expect("LevelBuilder::to_toml 應成功");
    let mut world = setup_world_with_level(&level_toml);

    let turn_order = start_new_round(&mut world).expect("開始回合應成功");
    assert_eq!(turn_order.entries.len(), 3, "初始應有 3 個單位");
    let first = turn_order.entries[0].occupant;
    let second = turn_order.entries[1].occupant;
    let third = turn_order.entries[2].occupant;

    delay_current_unit(&mut world, 2).expect("延後應成功");
    let turn_order = get_turn_order(&world).expect("應取得 TurnOrder");
    let entries = turn_order.entries.clone();
    let result_occupants: Vec<Occupant> = entries.iter().map(|e| e.occupant).collect();
    assert_eq!(
        result_occupants,
        vec![second, third, first],
        "延後後順序應為第二、第三、第一個單位"
    );
    assert_eq!(turn_order.round, 1, "回合數不應改變");
    assert_eq!(
        turn_order.current_index, 0,
        "延後後 current_index 應指向第一個單位"
    );

    delay_current_unit(&mut world, 1).expect("延後應成功");
    let turn_order = get_turn_order(&world).expect("應取得 TurnOrder");
    let entries = turn_order.entries.clone();
    let result_occupants: Vec<Occupant> = entries.iter().map(|e| e.occupant).collect();
    assert_eq!(
        result_occupants,
        vec![third, second, first],
        "延後後順序應為第三、第二、第一個單位"
    );
    assert_eq!(turn_order.round, 1, "回合數不應改變");
    assert_eq!(
        turn_order.current_index, 0,
        "延後後 current_index 應指向第一個單位"
    );

    // 第一個單位結束回合，第二個單位延後
    end_current_turn(&mut world).expect("結束回合應成功");
    delay_current_unit(&mut world, 2).expect("延後應成功");
    let turn_order = get_turn_order(&world).expect("應取得 TurnOrder");
    let entries = turn_order.entries.clone();
    let result_occupants: Vec<Occupant> = entries.iter().map(|e| e.occupant).collect();
    assert_eq!(
        result_occupants,
        vec![third, first, second],
        "延後後順序應為第三、第一、第二個單位"
    );
    assert_eq!(turn_order.round, 1, "回合數不應改變");
    assert_eq!(turn_order.current_index, 1);
}

/// 驗證 delay 後 end_current_turn 正確完成輪次
#[test]
fn test_delay_then_end_turn_completes_round() {
    let level_toml = LevelBuilder::from_ascii(
        "
        . U1 . U2 .
        . . . . .
        . . . . .
    ",
    )
    .unit("U1", UNIT_TYPE_WARRIOR, 1)
    .unit("U2", UNIT_TYPE_WARRIOR, 2)
    .to_toml()
    .expect("LevelBuilder::to_toml 應成功");
    let mut world = setup_world_with_level(&level_toml);

    let turn_order = start_new_round(&mut world).expect("開始回合應成功");
    let first = turn_order.entries[0].occupant;
    let second = turn_order.entries[1].occupant;

    // 第一個單位延後到最後
    delay_current_unit(&mut world, 1).expect("延後應成功");
    let turn_order = get_turn_order(&world).expect("應取得 TurnOrder");
    let occupants: Vec<Occupant> = turn_order.entries.iter().map(|e| e.occupant).collect();
    assert_eq!(occupants, vec![second, first], "延後後順序應為第二、第一");
    assert_eq!(turn_order.round, 1, "回合數不應改變");
    assert_eq!(turn_order.current_index, 0);

    // 第二個單位行動
    end_current_turn(&mut world).expect("結束回合應成功");
    let turn_order = get_turn_order(&world).expect("應取得 TurnOrder");
    assert_eq!(turn_order.round, 1, "尚有延後的單位，不應換輪");
    assert_eq!(turn_order.current_index, 1);

    // 第一個單位（被延後的）行動，應換輪
    end_current_turn(&mut world).expect("結束回合應成功");
    let turn_order = get_turn_order(&world).expect("應取得 TurnOrder");
    assert_eq!(turn_order.round, 2, "所有單位行動完應換輪");
    assert_eq!(turn_order.current_index, 0);
}

#[test]
fn test_delay_current_unit_fails() {
    let level_toml = LevelBuilder::from_ascii(
        "
        . U1 . U2 . U3 .
        . . . . . . .
        . . . . . . .
    ",
    )
    .unit("U1", UNIT_TYPE_WARRIOR, 1)
    .unit("U2", UNIT_TYPE_WARRIOR, 1)
    .unit("U3", UNIT_TYPE_MAGE, 1)
    .to_toml()
    .expect("LevelBuilder::to_toml 應成功");
    let mut world = setup_world_with_level(&level_toml);

    start_new_round(&mut world).expect("開始回合應成功");

    // target_index = 0（等於 current_index），應失敗
    let result = delay_current_unit(&mut world, 0);
    assert!(
        result.is_err(),
        "target_index 等於 current_index 時應回傳錯誤"
    );

    // target_index 超出範圍，應失敗
    let result = delay_current_unit(&mut world, 3);
    assert!(result.is_err(), "target_index 超出範圍時應回傳錯誤");

    // 不能延後到前面
    end_current_turn(&mut world).expect("結束回合應成功");
    let result = delay_current_unit(&mut world, 0);
    assert!(result.is_err(), "target_index 在已行動單位前面時應回傳錯誤");
}

// ============================================================================
// can_delay_current_unit 測試
// ============================================================================

/// 驗證 can_delay_current_unit 根據移動狀態回傳正確結果
#[test]
fn test_can_delay_current_unit_based_on_movement() {
    let level_toml = LevelBuilder::from_ascii(
        "
        . . . . .
        . U1 . . .
        . . U2 . .
        . . . . .
        . . . . .
    ",
    )
    .unit("U1", UNIT_TYPE_WARRIOR, PLAYER_FACTION_ID)
    .unit("U2", UNIT_TYPE_WARRIOR, 2)
    .to_toml()
    .expect("LevelBuilder::to_toml 應成功");
    let mut world = setup_world_with_level(&level_toml);

    // 無 TurnOrder 時應回傳錯誤
    let result = can_delay_current_unit(&mut world);
    assert!(result.is_err(), "無 TurnOrder 時應回傳錯誤");

    // 初始化 TurnOrder
    start_new_round(&mut world).expect("start_new_round 應成功");

    // 未移動時應可延遲
    let can_delay = can_delay_current_unit(&mut world).expect("查詢應成功");
    assert!(can_delay, "未移動時應可延遲");
    delay_current_unit(&mut world, 1).expect("延遲應成功");
    delay_current_unit(&mut world, 1).expect("延遲應成功");

    // 移動後不可延遲
    let target = Position { x: 1, y: 0 };
    execute_move(&mut world, target).expect("移動應成功");
    let can_delay = can_delay_current_unit(&mut world).expect("查詢應成功");
    assert!(!can_delay, "移動後不可延遲");
    let result = delay_current_unit(&mut world, 1);
    assert!(result.is_err(), "移動後延遲應回傳錯誤");
}

// ============================================================================
// remove_dead_unit 測試
// ============================================================================

/// 驗證 remove_dead_unit 正確移除單位
#[test]
fn test_remove_dead_unit_removes_from_turn_order() {
    let level_toml = LevelBuilder::from_ascii(
        "
        . U1 . U2 . U3 .
        . . . . . . .
        . . . . . . .
    ",
    )
    .unit("U1", UNIT_TYPE_WARRIOR, 1)
    .unit("U2", UNIT_TYPE_WARRIOR, 1)
    .unit("U3", UNIT_TYPE_MAGE, 1)
    .to_toml()
    .expect("LevelBuilder::to_toml 應成功");
    let mut world = setup_world_with_level(&level_toml);

    let turn_order = start_new_round(&mut world).expect("開始回合應成功");
    assert_eq!(turn_order.entries.len(), 3, "初始應有 3 個單位");
    let first = turn_order.entries[0].occupant;
    let second = turn_order.entries[1].occupant;
    let third = turn_order.entries[2].occupant;

    // 移除該單位
    remove_dead_unit(&mut world, second).expect("移除應成功");
    let turn_order = get_turn_order(&world).expect("應取得 TurnOrder");
    let result_occupants: Vec<Occupant> = turn_order.entries.iter().map(|e| e.occupant).collect();
    assert_eq!(
        result_occupants,
        vec![first, third],
        "移除後順序應為第一、第三個單位"
    );
    assert_eq!(turn_order.current_index, 0);
    assert_eq!(turn_order.round, 1, "回合數不應改變");

    end_current_turn(&mut world).expect("結束回合應成功");
    let turn_order = get_turn_order(&world).expect("應取得 TurnOrder");
    assert_eq!(
        result_occupants,
        vec![first, third],
        "結束回合不應改變順序，仍為第一、第三個單位"
    );
    assert_eq!(turn_order.current_index, 1);
    assert_eq!(turn_order.round, 1, "回合數不應改變");

    // 移除當前單位（第三個），應自動調整 current_index
    remove_dead_unit(&mut world, third).expect("移除應成功");
    let turn_order = get_turn_order(&world).expect("應取得 TurnOrder");
    let result_occupants: Vec<Occupant> = turn_order.entries.iter().map(|e| e.occupant).collect();
    assert_eq!(result_occupants, vec![first], "移除後順序應為第一個單位");
    assert_eq!(turn_order.current_index, 0);
    assert_eq!(turn_order.round, 2, "所有單位行動完應換輪");
}

/// 驗證移除不存在的單位應報錯
#[test]
fn test_remove_dead_unit_fails_when_occupant_not_found() {
    let level_toml = LevelBuilder::from_ascii(
        "
        . U1 . U2 .
        . . . . .
        . . . . .
    ",
    )
    .unit("U1", UNIT_TYPE_WARRIOR, 1)
    .unit("U2", UNIT_TYPE_WARRIOR, 2)
    .to_toml()
    .expect("LevelBuilder::to_toml 應成功");
    let mut world = setup_world_with_level(&level_toml);

    start_new_round(&mut world).expect("開始回合應成功");

    let fake_occupant = Occupant::Unit(9999);
    let result = remove_dead_unit(&mut world, fake_occupant);
    assert!(result.is_err(), "移除不存在的單位應報錯");
}

// ============================================================================
// get_turn_order 測試
// ============================================================================

/// 驗證未初始化時返回錯誤
#[test]
fn test_get_turn_order_without_initialization_returns_error() {
    let level_toml = LevelBuilder::from_ascii(
        "
        . U1 .
        . . .
        . . .
    ",
    )
    .unit("U1", UNIT_TYPE_WARRIOR, 1)
    .to_toml()
    .expect("LevelBuilder::to_toml 應成功");
    let world = setup_world_with_level(&level_toml);

    let result = get_turn_order(&world);
    assert!(
        result.is_err(),
        "未初始化時應返回錯誤，實際結果：{:?}",
        result
    );
}

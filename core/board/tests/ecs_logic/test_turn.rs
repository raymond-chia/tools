//! 回合順序 ECS 操作測試

use super::constants::{UNIT_TYPE_MAGE, UNIT_TYPE_WARRIOR};
use super::setup_world_with_level;
use bevy_ecs::prelude::{Entity, With, World};
use board::domain::battle_log::LogEvent;
use board::domain::constants::PLAYER_FACTION_ID;
use board::domain::core_types::{BuffType, EndCondition, PendingReaction, ReactionTrigger};
use board::ecs_logic::movement::{advance_move, plan_move};
use board::ecs_logic::query::get_battle_log;
use board::ecs_logic::turn::{
    can_delay_current_unit, delay_current_unit, end_battle, end_current_turn, get_current_unit,
    get_turn_order, resolve_deaths, start_new_round,
};
use board::ecs_types::components::{
    AppliedBuff, CurrentHp, MaxReactionPoint, Occupant, Position, ReactionPoint, Unit,
};
use board::ecs_types::resources::ReactionState;
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
    plan_move(&mut world, target).expect("plan_move 應成功");
    advance_move(&mut world).expect("移動應成功");
    let can_delay = can_delay_current_unit(&mut world).expect("查詢應成功");
    assert!(!can_delay, "移動後不可延遲");
    let result = delay_current_unit(&mut world, 1);
    assert!(result.is_err(), "移動後延遲應回傳錯誤");
}

// ============================================================================
// resolve_deaths 測試
// ============================================================================

/// 將指定 occupant 的單位 HP 設為 0（模擬被打死）
fn kill_unit(world: &mut World, occupant: Occupant) {
    let entity = {
        let mut query = world.query::<(Entity, &Occupant)>();
        query
            .iter(world)
            .find(|(_, occ)| **occ == occupant)
            .map(|(entity, _)| entity)
            .expect("應找到指定單位")
    };
    world.entity_mut(entity).insert(CurrentHp(0));
}

/// 在指定 occupant 身上掛一個有限期 buff（remaining_duration == Some(duration)）。
///
/// - duration == 0：已過期，回合開始流程會清除它（用於偵測某單位是否跑過回合開始）。
/// - duration > 0：用於偵測換輪時的 buff duration tick。
fn spawn_buff_with_duration(world: &mut World, target: Occupant, duration: u32) {
    world.spawn((AppliedBuff {
        def: BuffType {
            name: "timed".to_string(),
            stackable: false,
            while_active: vec![],
            per_turn_effects: vec![],
            end_conditions: vec![EndCondition::Duration(duration)],
        },
        caster: target,
        target,
        remaining_duration: Some(duration),
        inherited_defense: None,
    },));
}

/// 查詢指定 occupant 身上的 buff 數量。
fn buff_count_for(world: &mut World, target: Occupant) -> usize {
    world
        .query::<&AppliedBuff>()
        .iter(world)
        .filter(|buff| buff.target == target)
        .count()
}

/// 讀取指定 occupant 身上唯一 buff 的剩餘回合（測試前置保證僅一個）。
fn buff_remaining_for(world: &mut World, target: Occupant) -> Option<u32> {
    world
        .query::<&AppliedBuff>()
        .iter(world)
        .find(|buff| buff.target == target)
        .expect("應找到指定 occupant 的 buff")
        .remaining_duration
}

/// 驗證 resolve_deaths 移出 HP≤0 單位並 despawn，且死當前/非當前單位都正確重設 current_index
///
/// 涵蓋（開新一輪另由 test_resolve_deaths_batch_starts_new_round_once 測試）：
/// - 無死者：早返回、不改變任何狀態
/// - 死當前單位：current_index 重設指向下一個未行動者，不誤開新一輪
/// - 死下一個（非當前）單位：current_index 與回合數不變
#[test]
fn test_resolve_deaths_removes_dead_unit_from_turn_order() {
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

    // 沒有死者時 resolve_deaths 不改變任何狀態
    resolve_deaths(&mut world).expect("無死者時應成功");
    let turn_order = get_turn_order(&world).expect("應取得 TurnOrder");
    assert_eq!(turn_order.entries.len(), 3, "無死者時不應移除任何單位");

    // 打死當前單位（第一個），current_index 應重設指向下一個未行動者（第二個）
    kill_unit(&mut world, first);
    resolve_deaths(&mut world).expect("resolve_deaths 應成功");
    let turn_order = get_turn_order(&world).expect("應取得 TurnOrder");
    let result_occupants: Vec<Occupant> = turn_order.entries.iter().map(|e| e.occupant).collect();
    assert_eq!(
        result_occupants,
        vec![second, third],
        "移除當前單位後順序應為第二、第三個單位"
    );
    assert_eq!(
        turn_order.current_index, 0,
        "current_index 應指向第二個單位"
    );
    assert_eq!(turn_order.round, 1, "尚有未行動單位，不應換輪");
    assert_eq!(
        get_current_unit(turn_order).expect("應取得當前單位"),
        second,
        "當前單位應為第二個（原當前單位已死）"
    );

    // 打死下一個單位（第三個，非當前），current_index 與回合數應不變
    kill_unit(&mut world, third);
    resolve_deaths(&mut world).expect("resolve_deaths 應成功");
    let turn_order = get_turn_order(&world).expect("應取得 TurnOrder");
    let result_occupants: Vec<Occupant> = turn_order.entries.iter().map(|e| e.occupant).collect();
    assert_eq!(result_occupants, vec![second], "移除後只剩第二個單位");
    assert_eq!(
        turn_order.current_index, 0,
        "current_index 應仍指向第二個單位"
    );
    assert_eq!(turn_order.round, 1, "回合數不應改變");
    assert_eq!(
        get_current_unit(turn_order).expect("應取得當前單位"),
        second,
        "當前單位仍為第二個"
    );

    // 死者 entity 應已 despawn
    let remaining: Vec<Occupant> = {
        let mut query = world.query::<&Occupant>();
        query.iter(&world).copied().collect()
    };
    assert!(!remaining.contains(&first), "死者 entity 應被 despawn");
    assert!(!remaining.contains(&third), "死者 entity 應被 despawn");
}

/// 驗證 AOE 同時打死多個單位時批次處理，只開一次新一輪
#[test]
fn test_resolve_deaths_batch_starts_new_round_once() {
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
    let first = turn_order.entries[0].occupant;
    let second = turn_order.entries[1].occupant;
    let third = turn_order.entries[2].occupant;

    // 當前單位（第一個）行動完畢，輪到第二個
    end_current_turn(&mut world).expect("結束回合應成功");
    let turn_order = get_turn_order(&world).expect("應取得 TurnOrder");
    assert_eq!(turn_order.current_index, 1, "應輪到第二個單位");
    assert_eq!(turn_order.round, 1, "回合數不應改變");

    // AOE 同時打死所有剩餘未行動單位（第二、第三個）
    kill_unit(&mut world, second);
    kill_unit(&mut world, third);
    resolve_deaths(&mut world).expect("resolve_deaths 應成功");

    let turn_order = get_turn_order(&world).expect("應取得 TurnOrder");
    let result_occupants: Vec<Occupant> = turn_order.entries.iter().map(|e| e.occupant).collect();
    assert_eq!(result_occupants, vec![first], "只剩第一個單位");
    assert_eq!(turn_order.current_index, 0, "移除後順序應為第一個單位");
    assert_eq!(turn_order.round, 2, "所有單位行動完應換輪");
}

/// 驗證批次死光剩餘單位而開新一輪時，存活單位的 buff 剩餘回合會遞減。
///
/// 鎖住「`resolve_deaths` 換輪須與 `end_current_turn` 換輪一樣 tick buff duration」
/// 的行為，避免 AOE 團滅換輪時存活單位 buff 不遞減。
#[test]
fn test_resolve_deaths_batch_new_round_ticks_buff_duration() {
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
    let first = turn_order.entries[0].occupant;
    let second = turn_order.entries[1].occupant;
    let third = turn_order.entries[2].occupant;

    // 給將存活的第一個單位掛一個剩餘 2 回合的 buff
    spawn_buff_with_duration(&mut world, first, 2);
    assert_eq!(
        buff_remaining_for(&mut world, first),
        Some(2),
        "前置條件：第一個單位的 buff 剩餘 2 回合"
    );

    // 第一個單位行動完畢，AOE 打死剩餘未行動單位（第二、第三個）→ 開新一輪
    end_current_turn(&mut world).expect("結束回合應成功");
    kill_unit(&mut world, second);
    kill_unit(&mut world, third);
    resolve_deaths(&mut world).expect("resolve_deaths 應成功");

    let turn_order = get_turn_order(&world).expect("應取得 TurnOrder");
    assert_eq!(turn_order.round, 2, "批次死光剩餘單位應換輪");
    assert_eq!(
        buff_remaining_for(&mut world, first),
        Some(1),
        "換輪後存活單位的 buff 剩餘回合應遞減為 1"
    );
}

/// 驗證每個死者各產生一筆死亡 log 事件（含名稱快照）
#[test]
fn test_resolve_deaths_appends_death_log() {
    let level_toml = LevelBuilder::from_ascii(
        "
        . U1 . U2 .
        . . . . .
    ",
    )
    .unit("U1", UNIT_TYPE_WARRIOR, 1)
    .unit("U2", UNIT_TYPE_WARRIOR, 1)
    .to_toml()
    .expect("LevelBuilder::to_toml 應成功");
    let mut world = setup_world_with_level(&level_toml);

    let turn_order = start_new_round(&mut world).expect("開始回合應成功");
    let first = turn_order.entries[0].occupant;

    kill_unit(&mut world, first);
    resolve_deaths(&mut world).expect("resolve_deaths 應成功");

    let log = get_battle_log(&world).expect("spawn_level 後應可取得 BattleLog");
    assert_eq!(log.len(), 1, "應產生一筆死亡 log");
    match &log[0] {
        LogEvent::Death { unit } => {
            assert_eq!(
                unit, UNIT_TYPE_WARRIOR,
                "死亡 log 應記錄死者 type name 快照"
            );
        }
        other => panic!("應為 Death 事件，實際為 {:?}", other),
    }
}

/// 驗證 resolve_deaths 清除死者身上的 buff entity，且不影響存活單位的 buff
///
/// 死者 despawn 後，掛在死者身上（target == 死者）的 AppliedBuff 是獨立 entity，
/// 不會隨死者 entity 一併移除。resolve_deaths 須主動清除這些孤兒 buff。
/// 存活單位身上的 buff（含死者施放在存活單位身上的）不應被清除。
#[test]
fn test_resolve_deaths_removes_dead_unit_buffs() {
    let level_toml = LevelBuilder::from_ascii(
        "
        . U1 . U2 .
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

    // 死者（first）身上掛一個未過期的 buff，存活者（second）也掛一個
    spawn_buff_with_duration(&mut world, first, 3);
    spawn_buff_with_duration(&mut world, second, 3);
    assert_eq!(
        buff_count_for(&mut world, first),
        1,
        "前置條件：死者身上應有 1 個 buff"
    );
    assert_eq!(
        buff_count_for(&mut world, second),
        1,
        "前置條件：存活者身上應有 1 個 buff"
    );

    kill_unit(&mut world, first);
    resolve_deaths(&mut world).expect("resolve_deaths 應成功");

    assert_eq!(
        buff_count_for(&mut world, first),
        0,
        "死者身上的 buff entity 應被清除（不留孤兒）"
    );
    assert_eq!(
        buff_count_for(&mut world, second),
        1,
        "存活單位身上的 buff 不應受影響"
    );
}

/// 驗證 resolve_deaths 從 ReactionState.pending 剔除死者
#[test]
fn test_resolve_deaths_removes_dead_from_pending() {
    let level_toml = LevelBuilder::from_ascii(
        "
        . U1 . U2 .
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

    // 模擬反應流程中 pending 同時含「即將死亡的 first」與「存活的 second」
    world.insert_resource(ReactionState {
        pending: vec![
            PendingReaction {
                reactor: first,
                trigger: second,
                trigger_event: ReactionTrigger::TakesDamage,
                available_skills: vec![],
            },
            PendingReaction {
                reactor: second,
                trigger: first,
                trigger_event: ReactionTrigger::TakesDamage,
                available_skills: vec![],
            },
        ],
        decided: vec![],
    });

    kill_unit(&mut world, first);
    resolve_deaths(&mut world).expect("resolve_deaths 應成功");

    let state = world
        .get_resource::<ReactionState>()
        .expect("ReactionState 應仍存在");
    let reactors: Vec<Occupant> = state.pending.iter().map(|p| p.reactor).collect();
    assert_eq!(reactors, vec![second], "死者應從 pending 剔除，存活者保留");
}

/// 驗證無 ReactionState 時 resolve_deaths 仍安全處理（execute_skill 後的情境）
#[test]
fn test_resolve_deaths_without_reaction_state() {
    let level_toml = LevelBuilder::from_ascii(
        "
        . U1 . U2 .
        . . . . .
    ",
    )
    .unit("U1", UNIT_TYPE_WARRIOR, 1)
    .unit("U2", UNIT_TYPE_WARRIOR, 2)
    .to_toml()
    .expect("LevelBuilder::to_toml 應成功");
    let mut world = setup_world_with_level(&level_toml);

    let turn_order = start_new_round(&mut world).expect("開始回合應成功");
    let second = turn_order.entries[1].occupant;

    assert!(
        world.get_resource::<ReactionState>().is_none(),
        "前置條件：無 ReactionState"
    );

    kill_unit(&mut world, second);
    resolve_deaths(&mut world).expect("無 ReactionState 時仍應成功");

    let turn_order = get_turn_order(&world).expect("應取得 TurnOrder");
    assert_eq!(turn_order.entries.len(), 1, "死者應被移除");
}

/// 驗證：死當前單位後，遞補為當前的下一個單位會跑回合開始流程（清掉其過期 buff）。
#[test]
fn test_resolve_deaths_runs_turn_start_for_new_current_unit() {
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
    let first = turn_order.entries[0].occupant;
    let second = turn_order.entries[1].occupant;

    // 給遞補後將成為當前單位的第二個單位掛一個過期 buff
    spawn_buff_with_duration(&mut world, second, 0);
    assert_eq!(
        buff_count_for(&mut world, second),
        1,
        "前置條件：第二個單位身上應有過期 buff"
    );

    // 打死當前單位（第一個），第二個單位遞補為當前
    kill_unit(&mut world, first);
    resolve_deaths(&mut world).expect("resolve_deaths 應成功");

    let turn_order = get_turn_order(&world).expect("應取得 TurnOrder");
    assert_eq!(
        get_current_unit(turn_order).expect("應取得當前單位"),
        second,
        "第二個單位應遞補為當前"
    );
    assert_eq!(
        buff_count_for(&mut world, second),
        0,
        "新當前單位應跑回合開始流程，過期 buff 應被清除"
    );
}

/// 驗證：死非當前單位後，當前單位不變、不重跑回合開始（其過期 buff 保留）。
#[test]
fn test_resolve_deaths_skips_turn_start_when_current_unchanged() {
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
    let first = turn_order.entries[0].occupant;
    let third = turn_order.entries[2].occupant;

    // 給當前單位（第一個）掛一個過期 buff
    spawn_buff_with_duration(&mut world, first, 0);
    assert_eq!(
        buff_count_for(&mut world, first),
        1,
        "前置條件：當前單位身上應有過期 buff"
    );

    // 打死非當前單位（第三個），當前單位仍為第一個
    kill_unit(&mut world, third);
    resolve_deaths(&mut world).expect("resolve_deaths 應成功");

    let turn_order = get_turn_order(&world).expect("應取得 TurnOrder");
    assert_eq!(
        get_current_unit(turn_order).expect("應取得當前單位"),
        first,
        "當前單位應仍為第一個"
    );
    assert_eq!(
        buff_count_for(&mut world, first),
        1,
        "當前單位未改變，不應重跑回合開始，過期 buff 應保留"
    );
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

// ============================================================================
// 回合結束恢復藉機攻擊次數測試
// ============================================================================

/// 驗證 end_current_turn 正確恢復 ReactionPoint 到 MaxReactionPoint
///
/// - U1 的 MaxReactionPoint = 2，使用一次反應後 ReactionPoint = 1
/// - 回合結束後，ReactionPoint 應恢復為 2
#[test]
fn test_end_current_turn_restores_reaction_point() {
    let level_toml = LevelBuilder::from_ascii(
        "
        . U . . .
        . . . . .
        . . . . .
    ",
    )
    .unit("U", UNIT_TYPE_WARRIOR, 1)
    .to_toml()
    .expect("LevelBuilder::to_toml 應成功");
    let mut world = setup_world_with_level(&level_toml);

    let entity: Entity = {
        let mut query = world.query_filtered::<Entity, With<Unit>>();
        query.iter(&world).next().expect("應有至少一個單位")
    };
    world
        .entity_mut(entity)
        .insert(MaxReactionPoint(2))
        .insert(ReactionPoint(1));

    let reaction_point_before = world
        .entity(entity)
        .get::<ReactionPoint>()
        .expect("應有 ReactionPoint")
        .0;
    assert_eq!(reaction_point_before, 1, "設定後應為 1");

    start_new_round(&mut world).expect("開始回合應成功");
    end_current_turn(&mut world).expect("結束回合應成功");

    let reaction_point_after = world
        .entity(entity)
        .get::<ReactionPoint>()
        .expect("應有 ReactionPoint")
        .0;
    assert_eq!(
        reaction_point_after, 2,
        "回合結束後，ReactionPoint 應恢復為 MaxReactionPoint（2）"
    );
}

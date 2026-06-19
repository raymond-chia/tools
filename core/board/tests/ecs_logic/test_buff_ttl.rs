use super::build_warrior_world;
use board::domain::core_types::{BuffType, EndCondition};
use board::ecs_logic::turn::{end_current_turn, start_new_round};
use board::ecs_types::components::{AppliedBuff, Occupant, Position};

fn make_buff(name: &str, end_conditions: Vec<EndCondition>) -> BuffType {
    BuffType {
        name: name.to_string(),
        stackable: false,
        while_active: vec![],
        per_turn_effects: vec![],
        end_conditions,
    }
}

/// 用單一 ttl 來源產生 buff，避免 EndCondition 與 remaining_duration 不一致。
/// ttl 為 0 視為無限期(None)。
fn spawn_buff_with_ttl(
    world: &mut bevy_ecs::prelude::World,
    name: &str,
    ttl: u32,
    target: Occupant,
) {
    let remaining = (ttl != 0).then_some(ttl);
    world.spawn((AppliedBuff {
        def: make_buff(name, vec![EndCondition::Duration(ttl)]),
        caster: target,
        target,
        remaining_duration: remaining,
        inherited_defense: None,
    },));
}

fn applied_buff_count(world: &mut bevy_ecs::prelude::World) -> usize {
    world.query::<&AppliedBuff>().iter(world).count()
}

fn first_buff_ttl(world: &mut bevy_ecs::prelude::World) -> Option<u32> {
    world
        .query::<&AppliedBuff>()
        .iter(world)
        .next()
        .expect("expected at least one AppliedBuff")
        .remaining_duration
}

/// 找出位於指定位置的單位。
fn occupant_at(world: &mut bevy_ecs::prelude::World, pos: Position) -> Occupant {
    let mut query = world.query::<(&Occupant, &Position)>();
    query
        .iter(world)
        .find(|(_, p)| **p == pos)
        .map(|(occ, _)| *occ)
        .expect("expected to find occupant at position")
}

#[test]
fn test_buff_ttl_decrements_only_at_round_end() {
    for &initial_ttl in &[3, 2, 0] {
        let (mut world, player_occupant, _) = build_warrior_world("P A");
        let ttl_option = (initial_ttl != 0).then_some(initial_ttl);
        let deducted_ttl_option = (initial_ttl != 0).then(|| initial_ttl - 1);

        start_new_round(&mut world).expect("start_new_round should succeed");
        spawn_buff_with_ttl(&mut world, "poison", initial_ttl, player_occupant);

        assert_eq!(
            applied_buff_count(&mut world),
            1,
            "initial buff count mismatch: {initial_ttl}",
        );
        assert_eq!(first_buff_ttl(&mut world), ttl_option);

        end_current_turn(&mut world).expect("first end_current_turn should succeed");
        assert_eq!(
            first_buff_ttl(&mut world),
            ttl_option,
            "ttl should not decrease at the end of a normal unit turn: {initial_ttl}",
        );
        assert_eq!(
            applied_buff_count(&mut world),
            1,
            "buff should still exist after a normal unit turn: {initial_ttl}",
        );

        end_current_turn(&mut world).expect("second end_current_turn should succeed");
        assert_eq!(
            first_buff_ttl(&mut world),
            deducted_ttl_option,
            "ttl should decrease only when the round rolls over: {initial_ttl}",
        );
        assert_eq!(
            applied_buff_count(&mut world),
            1,
            "buff should still exist after the round-end ttl tick: {initial_ttl}",
        );
    }
}

/// 驗證:某單位身上 ttl 為 initial_ttl 的 buff,只會在「該單位自己的回合開始」時被移除。
/// turns_until_own_turn_start 表示從 spawn 後到該單位下一次回合開始,需經過幾次 end_current_turn。
fn assert_buff_expires_at_own_turn_start(
    world: &mut bevy_ecs::prelude::World,
    target: Occupant,
    initial_ttl: u32,
) {
    spawn_buff_with_ttl(world, "bleed", initial_ttl, target);

    // ttl 在回合輪替時遞減,需經過 initial_ttl 個完整輪替才歸零,
    // 期間 buff 都不應被移除。
    for i in 0..(initial_ttl * 2) {
        assert_eq!(
            applied_buff_count(world),
            1,
            "expired buffs should not be removed yet: init ttl {initial_ttl}, round {i}",
        );
        end_current_turn(world).expect("end_current_turn should succeed");
    }
}

#[test]
fn test_expired_buff_is_removed_on_next_turn_start() {
    for initial_ttl in [1, 2, 3] {
        let (mut world, player_occupant, markers) = build_warrior_world("P A");
        start_new_round(&mut world).expect("start_new_round should succeed");

        // P 是當前回合單位:跑完迴圈後,下一個 turn start 就是 P 自己,buff 立即被移除。
        assert_buff_expires_at_own_turn_start(&mut world, player_occupant, initial_ttl);
        assert_eq!(
            applied_buff_count(&mut world),
            0,
            "expired buffs should be removed when the unit's own turn starts: {initial_ttl}",
        );

        // A 不是當前回合單位:過期後還要再等一次 end_current_turn 才輪到 A 的回合開始。
        let ally_occupant = occupant_at(&mut world, markers["A"][0]);
        assert_buff_expires_at_own_turn_start(&mut world, ally_occupant, initial_ttl);
        assert_eq!(
            applied_buff_count(&mut world),
            1,
            "expired buffs should not be removed before its own turn start: {initial_ttl}",
        );
        end_current_turn(&mut world).expect("end_current_turn should succeed");
        assert_eq!(
            applied_buff_count(&mut world),
            0,
            "expired buffs should be removed when the unit's own turn starts: {initial_ttl}",
        );
    }
}

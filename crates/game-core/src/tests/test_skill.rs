use super::support::{
    ACTOR_ID, BLOCKER_ID, TARGET_ID, game_with_skill_range_and_effect, push_collision_game,
};
use crate::*;

// 驗證推擊撞上另一單位時停止移動，並以實例 ID 記錄受碰撞傷害的單位。
#[test]
fn push_collision_damages_both_units() {
    let mut game = push_collision_game();
    game.start().expect("測試戰鬥應可開始");
    let skill = game
        .world
        .resource::<Skills>()
        .definitions
        .get("push")
        .expect("測試推擊技能應存在")
        .clone();
    game.use_skill_at_cell(ACTOR_ID, GridPos { x: 2, y: 1 }, skill)
        .expect("推擊應成功結算");

    let target = game.entity(TARGET_ID).expect("測試目標應存在");
    let blocker = game.entity(BLOCKER_ID).expect("測試碰撞單位應存在");
    let log = game.world.resource::<Log>();
    let (damage, collision_damage, collision_units) = match log.0.last() {
        Some(CombatLogEvent::Skill {
            damage,
            collision_damage,
            collision_units,
            push_blocked: true,
            pushed: false,
            ..
        }) => (*damage, *collision_damage, collision_units),
        _ => panic!("推擊應記錄單位碰撞且不移動"),
    };

    assert_eq!(collision_damage, gameplay_config::COLLISION_DAMAGE);
    assert_eq!(game.world.get::<Pos>(target).expect("目標應有位置").0.x, 2);
    assert_eq!(
        game.world.get::<Hp>(target).expect("目標應有 HP").current,
        100 - damage - collision_damage
    );
    assert_eq!(
        game.world
            .get::<Hp>(blocker)
            .expect("碰撞單位應有 HP")
            .current,
        100 - collision_damage
    );
    assert_eq!(collision_units.len(), 1);
    assert_eq!(collision_units[0].unit, BLOCKER_ID);
    assert_eq!(collision_units[0].unit_type, "test_unit");
    assert_eq!(collision_units[0].remaining_hp, 100 - collision_damage);
}

// 驗證最小射程排除過近格子，施放失敗時會回傳固定 ID 與詳細訊息。
#[test]
fn skill_min_range_limits_preview_and_action() {
    let mut game = game_with_skill_range_and_effect(
        2,
        2,
        SkillEffect::Push {
            attack_bonus: 100,
            power_bonus: 0,
        },
    );
    let actor = game.entity(ACTOR_ID).expect("測試攻擊者應存在");
    let range = crate::skill::skill_ranges(&game.world, actor);
    assert!(!range[0].cells.contains(&GridPos { x: 2, y: 1 }));
    assert!(range[0].cells.contains(&GridPos { x: 3, y: 1 }));

    game.start().expect("測試戰鬥應可開始");
    let skill = game.world.resource::<Skills>().definitions["push"].clone();
    let error = game
        .use_skill_at_cell(ACTOR_ID, GridPos { x: 2, y: 1 }, skill)
        .expect_err("過近的目標應被拒絕");
    assert_eq!(error.id(), "target_too_close");
    assert_eq!(error.message(), "目標距離太近");
}

// 驗證零射程治療可對自己施放，完整治療預覽與實際治療量符合力量加技能加值。
#[test]
fn zero_range_heal_targets_self() {
    let mut game = game_with_skill_range_and_effect(0, 0, SkillEffect::Heal { power_bonus: 4 });
    let actor = game.entity(ACTOR_ID).expect("測試攻擊者應存在");
    let range = crate::skill::skill_ranges(&game.world, actor);
    assert_eq!(range[0].cells, vec![GridPos { x: 1, y: 1 }]);

    game.start().expect("測試戰鬥應可開始");
    game.world
        .get_mut::<Hp>(actor)
        .expect("施放者應有生命值")
        .current = 90;
    let preview = game
        .preview_skill(ACTOR_ID, GridPos { x: 1, y: 1 }, "push")
        .expect("零距離治療應可預覽");
    match preview {
        SkillPreview::Healing(HealingPreview {
            target: _,
            target_type: _,
            target_hp: _,
            target_max_hp: _,
            target_mana: _,
            healing,
            remaining_hp: _,
            missing_hp: _,
            health_segments: _,
        }) => assert_eq!(healing, 5),
        _ => panic!("治療技能應產生治療預覽"),
    }
    let skill = game.world.resource::<Skills>().definitions["push"].clone();
    game.use_skill_at_cell(ACTOR_ID, GridPos { x: 1, y: 1 }, skill)
        .expect("零距離治療應可對自己施放");
    assert_eq!(
        game.world
            .get::<Hp>(actor)
            .expect("施放者應有生命值")
            .current,
        95
    );
}

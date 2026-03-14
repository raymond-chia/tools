// //! 技能系統 ECS 操作測試

// use super::constants::{
//     SKILL_FIREBALL, SKILL_HEAL, SKILL_MELEE, SKILL_MULTI_STRIKE, SKILL_PLAGUE_BURST,
//     UNIT_TYPE_MAGE, UNIT_TYPE_WARRIOR,
// };
// use super::setup_world_with_level;
// use crate::helpers::level_builder::LevelBuilder;
// use board::domain::constants::PLAYER_FACTION_ID;
// use board::ecs_logic::skill::{get_active_skills_with_status, validate_and_resolve_targets};
// use board::ecs_logic::turn::start_new_round;
// use board::ecs_types::components::{CurrentMp, Occupant, Position};

// const ENEMY_FACTION_ID: u32 = 2;

// // ============================================================================
// // get_active_skills_with_status 測試
// // ============================================================================

// #[test]
// fn test_get_active_skills_with_status() {
//     // melee-attack: mp_change=0, fireball: mp_change=-5, heal: mp_change=-3
//     let test_data = [
//         // (description, current_mp, melee_available, heal_available, fireball_available)
//         ("MP 充足，全部可用", 10, true, true, true),
//         ("fireball 恰好足夠", 5, true, true, true),
//         ("fireball 不夠、heal 夠", 4, true, true, false),
//         ("heal 恰好足夠", 3, true, true, false),
//         ("只有無消耗的可用", 2, true, false, false),
//         ("MP 為 0", 0, true, false, false),
//     ];

//     for (description, current_mp, melee_expected, heal_expected, fireball_expected) in &test_data {
//         let level_toml = LevelBuilder::from_ascii(
//             "
//             . . .
//             . U1 .
//             . . .
//         ",
//         )
//         .unit("U1", UNIT_TYPE_MAGE, PLAYER_FACTION_ID)
//         .to_toml()
//         .expect("LevelBuilder::to_toml 應成功");
//         let mut world = setup_world_with_level(&level_toml);

//         // TODO: 透過 World 查詢 occupant 對應的 Entity，設定 CurrentMp(*current_mp)

//         start_new_round(&mut world).expect("start_new_round 應成功");

//         let occupant = Occupant::Unit(1);
//         let skills = get_active_skills_with_status(&mut world, occupant)
//             .expect("get_active_skills_with_status 應成功");

//         // 被動技能不應出現
//         assert!(
//             skills.iter().all(|s| s.name != "mage-passive"),
//             "{description}: 被動技能不應出現"
//         );

//         // 驗證各技能的可用狀態
//         let melee = skills
//             .iter()
//             .find(|s| s.name == SKILL_MELEE)
//             .expect(&format!("{description}: 應找到 {SKILL_MELEE}"));
//         assert_eq!(
//             melee.available, *melee_expected,
//             "{description}: {SKILL_MELEE}"
//         );

//         let heal = skills
//             .iter()
//             .find(|s| s.name == SKILL_HEAL)
//             .expect(&format!("{description}: 應找到 {SKILL_HEAL}"));
//         assert_eq!(
//             heal.available, *heal_expected,
//             "{description}: {SKILL_HEAL}"
//         );

//         let fireball = skills
//             .iter()
//             .find(|s| s.name == SKILL_FIREBALL)
//             .expect(&format!("{description}: 應找到 {SKILL_FIREBALL}"));
//         assert_eq!(
//             fireball.available, *fireball_expected,
//             "{description}: {SKILL_FIREBALL}"
//         );
//     }
// }

//! 關卡結局判定整合測試（透過 world 建關卡驗證結局）

use super::constants::{OBJECTS_TOML, SKILLS_TOML, UNIT_TYPE_WARRIOR, UNITS_TOML};
use bevy_ecs::prelude::{Entity, World};
use board::domain::constants::PLAYER_FACTION_ID;
use board::domain::core_types::{EndLevelCondition, LevelOutcome};
use board::ecs_logic::level_outcome::resolve_level_outcome;
use board::ecs_logic::loader::parse_and_insert_game_data;
use board::ecs_logic::spawner::spawn_level;
use board::ecs_logic::turn::{resolve_deaths, start_new_round};
use board::ecs_types::components::{CurrentHp, Position};
use board::test_helpers::level_builder::{LevelBuilder, load_from_ascii};
use std::collections::HashMap;

const ALLY_FACTION_ID: u32 = 1;
const ENEMY_FACTION_ID: u32 = 2;

const VICTORY_KEY: &str = "victory_reason";
const DEFEAT_KEY: &str = "defeat_reason";

const PLAYER_MARKER: &str = "P";
const ALLY_MARKER: &str = "A";
const ENEMY_MARKER: &str = "E";

/// 棋盤佈局：P(player) A(ally) E(enemy) 各一
const LEVEL_ASCII: &str = "
    P . E
    . A .
";

/// 依 marker 反查該單位在棋盤上的位置（測試佈局中每個 marker 僅一格）
fn marker_position(markers: &HashMap<String, Vec<Position>>, marker: &str) -> Position {
    markers
        .get(marker)
        .and_then(|positions| positions.first())
        .copied()
        .unwrap_or_else(|| panic!("ASCII 佈局中應存在 marker {marker}"))
}

fn kill_unit_at(world: &mut World, pos: Position) {
    let entity: Entity = {
        let mut query = world.query::<(Entity, &Position)>();
        query
            .iter(world)
            .find(|(_, p)| **p == pos)
            .map(|(entity, _)| entity)
            .expect("應找到指定位置的單位")
    };
    world.entity_mut(entity).insert(CurrentHp(0));
}

/// 回傳建立好的 world 以及 marker → position 對應，供測試以 marker 定位單位
fn build_world(
    victory_conditions: Vec<(String, Vec<EndLevelCondition>)>,
    defeat_conditions: Vec<(String, Vec<EndLevelCondition>)>,
) -> (World, HashMap<String, Vec<Position>>) {
    let (_, markers) = load_from_ascii(LEVEL_ASCII).expect("load_from_ascii 應成功");

    let level_toml = LevelBuilder::from_ascii(LEVEL_ASCII)
        .unit(PLAYER_MARKER, UNIT_TYPE_WARRIOR, PLAYER_FACTION_ID)
        .unit(ALLY_MARKER, UNIT_TYPE_WARRIOR, ALLY_FACTION_ID)
        .unit(ENEMY_MARKER, UNIT_TYPE_WARRIOR, ENEMY_FACTION_ID)
        .victory_conditions(victory_conditions)
        .defeat_conditions(defeat_conditions)
        .to_toml()
        .expect("LevelBuilder::to_toml 應成功");

    let mut world = World::new();
    parse_and_insert_game_data(&mut world, UNITS_TOML, SKILLS_TOML, OBJECTS_TOML)
        .expect("parse_and_insert_game_data 應成功");
    spawn_level(&mut world, &level_toml, "test-level").expect("spawn_level 應成功");
    start_new_round(&mut world).expect("start_new_round 應成功");
    (world, markers)
}

#[test]
fn test_evaluate_level_undetermined_when_no_condition_met() {
    let (mut world, _markers) = build_world(
        vec![(
            VICTORY_KEY.to_string(),
            vec![EndLevelCondition::EliminateFaction(ENEMY_FACTION_ID)],
        )],
        vec![(
            DEFEAT_KEY.to_string(),
            vec![EndLevelCondition::EliminateFaction(PLAYER_FACTION_ID)],
        )],
    );

    let outcome = resolve_level_outcome(&mut world).expect("resolve_level_outcome 應成功");
    assert_eq!(
        outcome,
        LevelOutcome::Undetermined,
        "初始（spawn 後尚未死人）應為 Undetermined"
    );
}

#[test]
fn test_evaluate_level_victory_single_condition() {
    let (mut world, markers) = build_world(
        vec![(
            VICTORY_KEY.to_string(),
            vec![EndLevelCondition::EliminateFaction(ENEMY_FACTION_ID)],
        )],
        vec![(
            DEFEAT_KEY.to_string(),
            vec![EndLevelCondition::EliminateFaction(PLAYER_FACTION_ID)],
        )],
    );

    kill_unit_at(&mut world, marker_position(&markers, ENEMY_MARKER));
    resolve_deaths(&mut world).expect("resolve_deaths 應成功");
    let outcome = resolve_level_outcome(&mut world).expect("resolve_level_outcome 應成功");

    assert_eq!(
        outcome,
        LevelOutcome::Victory(VICTORY_KEY.to_string()),
        "消滅敵方 faction 後應觸發勝利分支"
    );
}

#[test]
fn test_evaluate_level_branch_and_requires_all_conditions() {
    // 分支內 AND：需同時消滅 ally 與 enemy 才觸發
    let (mut world, markers) = build_world(
        vec![(
            VICTORY_KEY.to_string(),
            vec![
                EndLevelCondition::EliminateFaction(ALLY_FACTION_ID),
                EndLevelCondition::EliminateFaction(ENEMY_FACTION_ID),
            ],
        )],
        vec![(
            DEFEAT_KEY.to_string(),
            vec![EndLevelCondition::EliminateFaction(PLAYER_FACTION_ID)],
        )],
    );

    // 只消滅 enemy，ally 仍存活 → 分支不成立
    kill_unit_at(&mut world, marker_position(&markers, ENEMY_MARKER));
    resolve_deaths(&mut world).expect("resolve_deaths 應成功");
    let outcome = resolve_level_outcome(&mut world).expect("resolve_level_outcome 應成功");
    assert_eq!(
        outcome,
        LevelOutcome::Undetermined,
        "只滿足分支內一個條件，AND 應不成立"
    );

    // 再消滅 ally → 分支內全部條件成立
    kill_unit_at(&mut world, marker_position(&markers, ALLY_MARKER));
    resolve_deaths(&mut world).expect("resolve_deaths 應成功");
    let outcome = resolve_level_outcome(&mut world).expect("resolve_level_outcome 應成功");
    assert_eq!(
        outcome,
        LevelOutcome::Victory(VICTORY_KEY.to_string()),
        "分支內兩個條件皆成立時，AND 應成立觸發此分支"
    );
}

#[test]
fn test_evaluate_level_branch_or_takes_first_triggered() {
    // 分支間 OR：兩個分支各自成立，取定義順序第一個
    let first_key = "first_branch";
    let second_key = "second_branch";
    let (mut world, markers) = build_world(
        vec![
            (
                first_key.to_string(),
                vec![EndLevelCondition::EliminateFaction(ENEMY_FACTION_ID)],
            ),
            (
                second_key.to_string(),
                vec![EndLevelCondition::EliminateFaction(ALLY_FACTION_ID)],
            ),
        ],
        vec![(
            DEFEAT_KEY.to_string(),
            vec![EndLevelCondition::EliminateFaction(PLAYER_FACTION_ID)],
        )],
    );

    // 同時消滅 ally 與 enemy，兩分支同時成立
    kill_unit_at(&mut world, marker_position(&markers, ENEMY_MARKER));
    kill_unit_at(&mut world, marker_position(&markers, ALLY_MARKER));
    resolve_deaths(&mut world).expect("resolve_deaths 應成功");
    let outcome = resolve_level_outcome(&mut world).expect("resolve_level_outcome 應成功");

    assert_eq!(
        outcome,
        LevelOutcome::Victory(first_key.to_string()),
        "多分支同時成立時應取定義順序第一個"
    );
}

#[test]
fn test_evaluate_level_defeat_takes_priority_over_victory() {
    // victory 與 defeat 同時成立時，defeat 優先
    let (mut world, markers) = build_world(
        vec![(
            VICTORY_KEY.to_string(),
            vec![EndLevelCondition::EliminateFaction(ENEMY_FACTION_ID)],
        )],
        vec![(
            DEFEAT_KEY.to_string(),
            vec![EndLevelCondition::EliminateFaction(ALLY_FACTION_ID)],
        )],
    );

    kill_unit_at(&mut world, marker_position(&markers, ENEMY_MARKER));
    kill_unit_at(&mut world, marker_position(&markers, ALLY_MARKER));
    resolve_deaths(&mut world).expect("resolve_deaths 應成功");
    let outcome = resolve_level_outcome(&mut world).expect("resolve_level_outcome 應成功");

    assert_eq!(
        outcome,
        LevelOutcome::Defeat(DEFEAT_KEY.to_string()),
        "victory 與 defeat 同時成立時，defeat 應優先"
    );
}

#[test]
fn test_resolve_level_outcome_without_spawn_returns_error() {
    let mut world = World::new();
    let result = resolve_level_outcome(&mut world);
    assert!(result.is_err(), "未先 spawn_level 時應回傳錯誤");
}

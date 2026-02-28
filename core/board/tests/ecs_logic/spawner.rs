use super::super::test_helpers::level_builder::LevelBuilder;
use super::constants::{OBJECTS_TOML, SKILLS_TOML, UNIT_TYPE_WARRIOR, UNITS_TOML};
use bevy_ecs::prelude::{Without, World};
use board::ecs_logic::loader::parse_and_insert_game_data;
use board::ecs_logic::spawner::spawn_level;
use board::ecs_types::components::{
    BlocksSight, BlocksSound, CurrentHp, MaxHp, Object, Occupant, OccupantTypeName, Position, Unit,
};
use board::ecs_types::resources::Board;
use board::error::{DataError, ErrorKind, LoadError};

// ============================================================================
// 測試：主要
// ============================================================================

#[test]
fn test_spawn_level_without_object() {
    let level_toml = LevelBuilder::from_ascii(
        "
        D . . . .
        . . . . .
        . . . . .
        . . . . .
        . . . . W
    ",
    )
    .unit("W", UNIT_TYPE_WARRIOR, 1)
    .deploy("D")
    .to_toml()
    .expect("LevelBuilder::to_toml 應成功");

    let mut world = World::new();
    parse_and_insert_game_data(&mut world, UNITS_TOML, SKILLS_TOML, OBJECTS_TOML)
        .expect("parse_and_insert_game_data 應成功");

    spawn_level(&mut world, &level_toml, "test-level").expect("spawn_level 應成功");

    let board = world.get_resource::<Board>().expect("應有 Board resource");
    assert_eq!(board.width, 5, "Board 寬度應為 5");
    assert_eq!(board.height, 5, "Board 高度應為 5");

    let unit_count = world.query::<&Unit>().iter(&world).count();
    assert_eq!(unit_count, 1, "應 spawn 恰好一個 Unit entity");

    // warrior-passive 技能給予 Hp=100，所以 MaxHp 和 CurrentHp 應為 100
    let (_, occupant, type_name, max_hp, current_hp, position) = world
        .query::<(
            &Unit,
            &Occupant,
            &OccupantTypeName,
            &MaxHp,
            &CurrentHp,
            &Position,
        )>()
        .iter(&world)
        .next()
        .expect("應有帶完整 Component 的 Unit entity");
    assert!(
        matches!(occupant, Occupant::Unit(_)),
        "Unit entity 的 Occupant 應為 Occupant::Unit"
    );
    assert_eq!(type_name.0, UNIT_TYPE_WARRIOR, "Unit name 應為 warrior");
    assert_eq!(
        max_hp.0, 100,
        "warrior MaxHp 應為 100（來自 warrior-passive 技能）"
    );
    assert_eq!(
        current_hp.0, 100,
        "warrior CurrentHp 應等於 MaxHp（初始狀態）"
    );
    assert_eq!(position.x, 4, "warrior 位置 x 應為 4");
    assert_eq!(position.y, 4, "warrior 位置 y 應為 4");
}

#[test]
fn test_spawn_level_with_object() {
    let level_toml = LevelBuilder::from_ascii(
        "
        D . . . .
        . W . . .
        . . w . .
        . . . p .
        . . . . .
    ",
    )
    .unit("W", UNIT_TYPE_WARRIOR, 0)
    .object("w", "wall")
    .object("p", "pit")
    .deploy("D")
    .to_toml()
    .expect("LevelBuilder::to_toml 應成功");

    let mut world = World::new();
    parse_and_insert_game_data(&mut world, UNITS_TOML, SKILLS_TOML, OBJECTS_TOML)
        .expect("parse_and_insert_game_data 應成功");

    spawn_level(&mut world, &level_toml, "test-level").expect("spawn_level 應成功");

    let count = world
        .query::<(&Object, &Occupant, &BlocksSight, &BlocksSound)>()
        .iter(&world)
        .count();
    assert_eq!(count, 1, "應 spawn 恰好一個 Object entity");

    let (_, occupant, _, _) = world
        .query::<(&Object, &Occupant, &BlocksSight, &BlocksSound)>()
        .iter(&world)
        .next()
        .expect("應有恰好一個帶完整 Component 的 Object entity");
    assert!(
        matches!(occupant, Occupant::Object(_)),
        "Object entity 的 Occupant 應為 Occupant::Object"
    );

    let count = world.query::<(&Object, &Occupant)>().iter(&world).count();
    assert_eq!(count, 2, "應 spawn 恰好兩個 Object entity");

    let count = world
        .query_filtered::<(&Object, &Occupant), Without<BlocksSight>>()
        .iter(&world)
        .count();
    assert_eq!(count, 1, "應 spawn 恰好一個 Object entity");
}

#[test]
fn test_spawn_level_occupant_ids_are_unique() {
    let level_toml = LevelBuilder::from_ascii(
        "
        D . . . .
        . W . . .
        . . w . .
        . . . p .
        . . . . .
    ",
    )
    .unit("W", UNIT_TYPE_WARRIOR, 0)
    .object("w", "wall")
    .object("p", "pit")
    .deploy("D")
    .to_toml()
    .expect("LevelBuilder::to_toml 應成功");

    let mut world = World::new();
    parse_and_insert_game_data(&mut world, UNITS_TOML, SKILLS_TOML, OBJECTS_TOML)
        .expect("parse_and_insert_game_data 應成功");

    spawn_level(&mut world, &level_toml, "test-level").expect("spawn_level 應成功");

    let ids: Vec<u32> = world
        .query::<&Occupant>()
        .iter(&world)
        .map(|occ| match occ {
            Occupant::Unit(id) => *id,
            Occupant::Object(id) => *id,
        })
        .collect();

    let unique_count = ids.iter().collect::<std::collections::HashSet<_>>().len();
    assert_eq!(ids.len(), unique_count, "所有 Occupant ID 應唯一，無碰撞");
}

// ============================================================================
// 測試：錯誤情境
// ============================================================================

#[test]
fn test_parse_and_insert_game_data_invalid_toml() {
    let mut world = World::new();

    let result =
        parse_and_insert_game_data(&mut world, "not valid toml ][", SKILLS_TOML, OBJECTS_TOML);
    assert!(result.is_err(), "無效 TOML 應回傳錯誤");

    let error = result.expect_err("應有錯誤");
    assert!(
        matches!(
            error.kind(),
            ErrorKind::Load(LoadError::DeserializeError { .. })
        ),
        "應為 LoadError，實際為 {:?}",
        error.kind()
    );
}

#[test]
fn test_spawn_level_without_game_data_returns_error() {
    let level_toml = LevelBuilder::from_ascii(
        "
        D . . . .
        . . . . .
        . . . . .
        . . . . .
        . . . . W
    ",
    )
    .unit("W", UNIT_TYPE_WARRIOR, 1)
    .deploy("D")
    .to_toml()
    .expect("LevelBuilder::to_toml 應成功");

    let mut world = World::new();
    // 故意不呼叫 parse_and_insert_game_data

    let result = spawn_level(&mut world, &level_toml, "test-level");
    assert!(result.is_err(), "未先載入 GameData 時應回傳錯誤");

    let error = result.expect_err("應有錯誤");
    assert!(
        matches!(error.kind(), ErrorKind::Data(DataError::GameDataNotFound)),
        "應為 DataError，實際為 {:?}",
        error.kind()
    );
}

#[test]
fn test_spawn_level_with_unknown_unit_type_returns_error() {
    let level_toml = LevelBuilder::from_ascii(
        "
        D . . . .
        . . . . .
        . . . . .
        . . . . .
        . . . . X
    ",
    )
    .unit("X", "nonexistent-unit-type", 0)
    .deploy("D")
    .to_toml()
    .expect("LevelBuilder::to_toml 應成功");

    let mut world = World::new();
    parse_and_insert_game_data(&mut world, UNITS_TOML, SKILLS_TOML, OBJECTS_TOML)
        .expect("parse_and_insert_game_data 應成功");

    let result = spawn_level(&mut world, &level_toml, "test-level");
    assert!(result.is_err(), "引用不存在的 unit_type_name 應回傳錯誤");

    let error = result.expect_err("應有錯誤");
    assert!(
        matches!(
            error.kind(),
            ErrorKind::Data(DataError::UnitTypeNotFound { .. })
        ),
        "應為 DataError，實際為 {:?}",
        error.kind()
    );
    assert!(
        error.to_string().contains("nonexistent-unit-type"),
        "錯誤訊息應包含不存在的類型名，實際為：{}",
        error
    );
}

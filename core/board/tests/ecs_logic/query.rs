use super::super::test_helpers::level_builder::LevelBuilder;
use super::constants::{OBJECT_TYPE_PIT, OBJECT_TYPE_WALL, UNIT_TYPE_MAGE, UNIT_TYPE_WARRIOR};
use super::setup_world_with_level;
use board::domain::alias::ID;
use board::domain::constants::PLAYER_FACTION_ID;
use board::ecs_logic::deployment::deploy_unit;
use board::ecs_logic::query::{get_all_objects, get_all_units};
use board::ecs_types::components::{Occupant, Position, UnitBundle};
use std::collections::HashMap;

#[test]
fn test_get_all_units_returns_correct_data() {
    let level_toml = LevelBuilder::from_ascii(
        "
                D D . . .
                . . . . .
                . . . . .
                . . . . .
                . . . . M
            ",
    )
    .deploy("D")
    .unit("M", UNIT_TYPE_MAGE, 1)
    .to_toml()
    .expect("LevelBuilder::to_toml 應成功");

    let enemy_pos = Position { x: 4, y: 4 };
    let pos1 = Position { x: 0, y: 0 };
    let pos2 = Position { x: 1, y: 0 };

    let mut world = setup_world_with_level(&level_toml);

    let assert_unit = |units: &HashMap<Position, UnitBundle>,
                       pos: Position,
                       typ: &str,
                       faction: ID,
                       max_hp: i32| {
        let info = units.get(&pos).expect("應有該位置的單位");
        assert!(
            matches!(info.occupant, Occupant::Unit(_)),
            "occupant 應為 Unit"
        );
        assert_eq!(info.occupant_type_name.0, typ, "類型名稱應為 {typ}");
        assert_eq!(info.faction.0, faction, "陣營應為 {faction}");
        assert_eq!(info.attributes.max_hp.0, max_hp, "max_hp 應為 {max_hp}");
    };

    // 部署前只有敵方單位 M
    let units = get_all_units(&mut world).expect("get_all_units 應成功");
    assert_eq!(units.len(), 1, "應只有 1 個單位");
    assert_unit(&units, enemy_pos, UNIT_TYPE_MAGE, 1, 80);

    // 部署一個單位
    deploy_unit(&mut world, &UNIT_TYPE_WARRIOR.to_string(), pos1).expect("deploy_unit 應成功");
    let units = get_all_units(&mut world).expect("get_all_units 應成功");
    assert_eq!(units.len(), 2, "應有 2 個單位（1 玩家 + 1 敵方）");
    for (pos, typ, faction, max_hp) in [
        (enemy_pos, UNIT_TYPE_MAGE, 1, 80),
        (pos1, UNIT_TYPE_WARRIOR, PLAYER_FACTION_ID, 100),
    ] {
        assert_unit(&units, pos, typ, faction, max_hp);
    }

    // 部署兩個單位
    deploy_unit(&mut world, &UNIT_TYPE_MAGE.to_string(), pos2).expect("deploy_unit 應成功");
    let units = get_all_units(&mut world).expect("get_all_units 應成功");
    assert_eq!(units.len(), 3, "應有 3 個單位（2 玩家 + 1 敵方）");
    for (pos, typ, faction, max_hp) in [
        (enemy_pos, UNIT_TYPE_MAGE, 1, 80),
        (pos1, UNIT_TYPE_WARRIOR, PLAYER_FACTION_ID, 100),
        (pos2, UNIT_TYPE_MAGE, PLAYER_FACTION_ID, 80),
    ] {
        assert_unit(&units, pos, typ, faction, max_hp);
    }
}

#[test]
fn test_get_all_units_empty_board() {
    let level_toml = LevelBuilder::from_ascii(
        "
        . . . . .
        . . . . .
        . . . . .
        . . . . .
        . . . . .
    ",
    )
    .to_toml()
    .expect("LevelBuilder::to_toml 應成功");

    let mut world = setup_world_with_level(&level_toml);
    let units = get_all_units(&mut world).expect("get_all_units 應成功");
    assert!(units.is_empty(), "空棋盤應無單位");
}

#[test]
fn test_get_all_objects_returns_correct_data() {
    let test_data = [
        (
            "單一 wall",
            "
                w . .
                . . .
                . . .
            ",
            vec![("w", OBJECT_TYPE_WALL)],
            vec![(
                Position { x: 0, y: 0 },
                OBJECT_TYPE_WALL,
                10000,
                0,
                true,
                true,
            )],
        ),
        (
            "單一 pit",
            "
                . . .
                . p .
                . . .
            ",
            vec![("p", OBJECT_TYPE_PIT)],
            vec![(
                Position { x: 1, y: 1 },
                OBJECT_TYPE_PIT,
                0,
                -10000,
                false,
                false,
            )],
        ),
        (
            "混合多種物件",
            "
                w . .
                . . .
                . . p
                . w .
            ",
            vec![("w", OBJECT_TYPE_WALL), ("p", OBJECT_TYPE_PIT)],
            vec![
                (
                    Position { x: 0, y: 0 },
                    OBJECT_TYPE_WALL,
                    10000,
                    0,
                    true,
                    true,
                ),
                (
                    Position { x: 1, y: 3 },
                    OBJECT_TYPE_WALL,
                    10000,
                    0,
                    true,
                    true,
                ),
                (
                    Position { x: 2, y: 2 },
                    OBJECT_TYPE_PIT,
                    0,
                    -10000,
                    false,
                    false,
                ),
            ],
        ),
        (
            "同類物件排成一列",
            "
                w w w
                . . .
                . . .
            ",
            vec![("w", OBJECT_TYPE_WALL)],
            vec![
                (
                    Position { x: 0, y: 0 },
                    OBJECT_TYPE_WALL,
                    10000,
                    0,
                    true,
                    true,
                ),
                (
                    Position { x: 1, y: 0 },
                    OBJECT_TYPE_WALL,
                    10000,
                    0,
                    true,
                    true,
                ),
                (
                    Position { x: 2, y: 0 },
                    OBJECT_TYPE_WALL,
                    10000,
                    0,
                    true,
                    true,
                ),
            ],
        ),
    ];

    for (label, ascii, object_defs, expected) in test_data {
        let mut builder = LevelBuilder::from_ascii(ascii);
        for (marker, typ) in &object_defs {
            builder = builder.object(marker, typ);
        }
        let level_toml = builder.to_toml().expect("LevelBuilder::to_toml 應成功");

        let mut world = setup_world_with_level(&level_toml);
        let objects = get_all_objects(&mut world).expect("get_all_objects 應成功");

        assert_eq!(objects.len(), expected.len(), "[{label}] 物件數量不符");
        for (pos, typ, movement_cost, hp_modify, blocks_sight, blocks_sound) in &expected {
            let result = objects
                .get(pos)
                .expect(&format!("應有位置 ({}, {}) 的物件", pos.x, pos.y));
            assert!(
                matches!(result.bundle.occupant, Occupant::Object(_)),
                "occupant 應為 Object"
            );
            assert_eq!(
                result.bundle.occupant_type_name.0,
                typ.to_string(),
                "類型名稱應為 {typ}"
            );
            assert_eq!(
                result.bundle.terrain_movement_cost.0, *movement_cost,
                "movement_cost 應為 {movement_cost}"
            );
            assert_eq!(
                result.bundle.hp_modify.0, *hp_modify,
                "hp_modify 應為 {hp_modify}"
            );
            assert_eq!(
                result.blocks_sight, *blocks_sight,
                "[{label}] blocks_sight 應為 {blocks_sight}"
            );
            assert_eq!(
                result.blocks_sound, *blocks_sound,
                "[{label}] blocks_sound 應為 {blocks_sound}"
            );
        }
    }
}

#[test]
fn test_get_all_objects_empty_board() {
    let level_toml = LevelBuilder::from_ascii(
        "
        . . . . .
        . . . . .
        . . . . .
        . . . . .
        . . . . .
    ",
    )
    .to_toml()
    .expect("LevelBuilder::to_toml 應成功");

    let mut world = setup_world_with_level(&level_toml);
    let objects = get_all_objects(&mut world).expect("get_all_objects 應成功");
    assert!(objects.is_empty(), "空棋盤應無物件");
}

use super::constants::{
    EQUIPMENT_GREAT_SWORD, EQUIPMENT_IRON_SWORD, EQUIPMENT_STEEL_SWORD, EQUIPMENT_WOODEN_SHIELD,
    EQUIPMENTS_TOML, OBJECTS_TOML, OFF_HAND_UNITS_TOML, SKILL_IRON_SLASH, SKILL_RUBY_BURST,
    SKILLS_TOML, UNIT_TYPE_DUAL_WIELDER, UNIT_TYPE_KNIGHT, UNIT_TYPE_SWORD_USER, UNIT_TYPE_WARRIOR,
    UNITS_TOML,
};
use bevy_ecs::prelude::{Entity, World};
use board::domain::core_types::{EquipmentType, OffHandPermission};
use board::ecs_logic::loader::{GameDataToml, parse_and_insert_game_data};
use board::ecs_logic::spawner::spawn_level;
use board::ecs_types::components::{
    EquippedItems, MaxHp, Occupant, PhysicalAttack, Position, Skills,
};
use board::ecs_types::resources::GameData;
use board::test_helpers::level_builder::LevelBuilder;

fn setup_world(level_toml: &str) -> World {
    let mut world = World::new();
    parse_and_insert_game_data(
        &mut world,
        GameDataToml {
            units: UNITS_TOML,
            skills: SKILLS_TOML,
            equipments: EQUIPMENTS_TOML,
            objects: OBJECTS_TOML,
        },
    )
    .expect("裝備資料應成功載入");
    spawn_level(&mut world, level_toml, "equipment-test").expect("關卡應成功生成");
    world
}

fn occupant_at(world: &mut World, position: Position) -> Occupant {
    world
        .query::<(&Occupant, &Position)>()
        .iter(world)
        .find(|(_, actual)| **actual == position)
        .map(|(occupant, _)| *occupant)
        .expect("指定位置應有單位")
}

fn entity_of(world: &mut World, occupant: Occupant) -> Entity {
    world
        .query::<(Entity, &Occupant)>()
        .iter(world)
        .find(|(_, actual)| **actual == occupant)
        .map(|(entity, _)| entity)
        .expect("指定 Occupant 應對應至 Entity")
}

fn parse_off_hand_game_data() -> World {
    let mut world = World::new();
    parse_and_insert_game_data(
        &mut world,
        GameDataToml {
            units: OFF_HAND_UNITS_TOML,
            skills: SKILLS_TOML,
            equipments: EQUIPMENTS_TOML,
            objects: OBJECTS_TOML,
        },
    )
    .expect("副手裝備資料應能載入");
    world
}

// ==================================================
// 載入裝備資料
// ==================================================

#[test]
fn test_loader_preserves_equipment_types_and_off_hand_permissions() {
    let world = parse_off_hand_game_data();
    let game_data = world
        .get_resource::<GameData>()
        .expect("應建立 GameData resource");

    let test_data = [
        (
            UNIT_TYPE_SWORD_USER,
            OffHandPermission::None,
            Some(EQUIPMENT_IRON_SWORD),
            None,
            EquipmentType::Weapon,
        ),
        (
            "great-sword-user",
            OffHandPermission::None,
            Some(EQUIPMENT_GREAT_SWORD),
            None,
            EquipmentType::TwoHandedWeapon,
        ),
        (
            UNIT_TYPE_DUAL_WIELDER,
            OffHandPermission::Weapon,
            Some(EQUIPMENT_IRON_SWORD),
            Some(EQUIPMENT_STEEL_SWORD),
            EquipmentType::Weapon,
        ),
        (
            UNIT_TYPE_KNIGHT,
            OffHandPermission::Shield,
            Some(EQUIPMENT_IRON_SWORD),
            Some(EQUIPMENT_WOODEN_SHIELD),
            EquipmentType::Weapon,
        ),
    ];

    for (
        unit_name,
        expected_permission,
        expected_main_hand,
        expected_off_hand,
        expected_main_hand_type,
    ) in test_data
    {
        let unit = game_data
            .unit_type_map
            .get(unit_name)
            .expect("單位類型應存在");

        assert_eq!(unit.off_hand_permission, expected_permission);
        assert_eq!(unit.equipment.main_hand.as_deref(), expected_main_hand);
        assert_eq!(unit.equipment.off_hand.as_deref(), expected_off_hand);
        assert_eq!(
            game_data
                .equipment_type_map
                .get(expected_main_hand.expect("測試單位應有主手裝備"))
                .expect("主手裝備類型應存在")
                .typ,
            expected_main_hand_type,
        );
    }

    assert_eq!(
        game_data
            .equipment_type_map
            .get(EQUIPMENT_WOODEN_SHIELD)
            .expect("盾牌裝備類型應存在")
            .typ,
        EquipmentType::Shield,
    );
}

#[test]
fn test_loader_rejects_off_hand_equipment_disallowed_by_unit_permission() {
    let invalid_units_tomls = [
        r#"
[[units]]
name = "none-off-hand-user"
skills = []
off_hand_permission = "None"

[units.equipment]
main_hand = "iron-sword"
off_hand = "steel-sword"
"#,
        r#"
[[units]]
name = "none-off-hand-user"
skills = []
off_hand_permission = "None"

[units.equipment]
main_hand = "iron-sword"
off_hand = "wooden-shield"
"#,
        r#"
[[units]]
name = "weapon-off-hand-user"
skills = []
off_hand_permission = "Weapon"

[units.equipment]
main_hand = "iron-sword"
off_hand = "wooden-shield"
"#,
        r#"
[[units]]
name = "shield-off-hand-user"
skills = []
off_hand_permission = "Shield"

[units.equipment]
main_hand = "iron-sword"
off_hand = "steel-sword"
"#,
        r#"
[[units]]
name = "two-handed-with-off-hand-user"
skills = []
off_hand_permission = "Shield"

[units.equipment]
main_hand = "great-sword"
off_hand = "wooden-shield"
"#,
    ];

    for units in invalid_units_tomls {
        let mut world = World::new();
        let result = parse_and_insert_game_data(
            &mut world,
            GameDataToml {
                units,
                skills: SKILLS_TOML,
                equipments: EQUIPMENTS_TOML,
                objects: OBJECTS_TOML,
            },
        );

        assert!(result.is_err(), "副手裝備權限不符時應拒絕載入");
    }
}

// ==================================================
// 生成單位
// ==================================================

#[test]
fn test_spawn_level_copies_main_and_off_hand_equipment_to_unit() {
    let level_toml = LevelBuilder::from_ascii("K")
        .unit("K", UNIT_TYPE_KNIGHT, 0)
        .to_toml()
        .expect("關卡 TOML 應能序列化");
    let mut world = parse_off_hand_game_data();
    spawn_level(&mut world, &level_toml, "off-hand-test").expect("關卡應能生成");

    let equipment = world
        .query::<&EquippedItems>()
        .iter(&world)
        .next()
        .expect("已生成單位應有裝備元件");

    assert_eq!(equipment.main_hand.as_deref(), Some(EQUIPMENT_IRON_SWORD));
    assert_eq!(equipment.off_hand.as_deref(), Some(EQUIPMENT_WOODEN_SHIELD));
}

#[test]
fn test_spawn_level_applies_default_equipment_effects_and_skills() {
    let level_toml = LevelBuilder::from_ascii("W")
        .unit("W", UNIT_TYPE_WARRIOR, 0)
        .to_toml()
        .expect("關卡 TOML 應成功序列化");
    let mut world = setup_world(&level_toml);
    let occupant = occupant_at(&mut world, Position { x: 0, y: 0 });
    let entity = entity_of(&mut world, occupant);
    let entity_ref = world.entity(entity);

    assert_eq!(
        entity_ref.get::<MaxHp>().expect("單位應有 MaxHp").0,
        120,
        "預設防具效果應套用"
    );
    assert_eq!(
        entity_ref
            .get::<PhysicalAttack>()
            .expect("單位應有 PhysicalAttack")
            .0,
        15,
        "預設武器的物理攻擊效果應在生成時套用"
    );
    let skills = &entity_ref.get::<Skills>().expect("單位應有 Skills").0;
    assert!(
        skills.contains(&SKILL_IRON_SLASH.to_string()),
        "武器應授予技能"
    );
    assert!(
        skills.contains(&SKILL_RUBY_BURST.to_string()),
        "飾品應授予技能"
    );
}

/*
// ==================================================
// 換裝成功
// ==================================================

#[test]
fn test_equip_unit_replaces_weapon() {
    let level_toml = LevelBuilder::from_ascii("W")
        .unit("W", UNIT_TYPE_WARRIOR, 0)
        .to_toml()
        .expect("關卡 TOML 應成功序列化");
    let mut world = setup_world(&level_toml);
    let occupant = occupant_at(&mut world, Position { x: 0, y: 0 });

    equip_unit(
        &mut world,
        occupant,
        &EQUIPMENT_STEEL_SWORD.to_string(),
        EquipmentPosition::Weapon,
    )
    .expect("武器應成功替換");

    let entity = entity_of(&mut world, occupant);
    let entity_ref = world.entity(entity);
    assert_eq!(
        entity_ref
            .get::<PhysicalAttack>()
            .expect("PhysicalAttack")
            .0,
        20,
        "新武器應取代舊武器的能力效果"
    );
    let skills = &entity_ref.get::<Skills>().expect("單位應有 Skills").0;
    assert!(
        !skills.contains(&"iron-slash".to_string()),
        "舊武器技能應移除"
    );
    assert!(
        skills.contains(&"steel-slash".to_string()),
        "新武器技能應加入"
    );
}

#[test]
fn test_equip_unit_replaces_specified_accessory_position() {
    let level_toml = LevelBuilder::from_ascii("M")
        .unit("M", UNIT_TYPE_MAGE, 0)
        .to_toml()
        .expect("關卡 TOML 應成功序列化");
    let mut world = setup_world(&level_toml);
    let occupant = occupant_at(&mut world, Position { x: 0, y: 0 });

    equip_unit(
        &mut world,
        occupant,
        &EQUIPMENT_ECHO_CHARM.to_string(),
        EquipmentPosition::FirstAccessory,
    )
    .expect("飾品應成功替換指定格位");

    let entity = entity_of(&mut world, occupant);
    let skills = &world
        .entity(entity)
        .get::<Skills>()
        .expect("單位應有 Skills")
        .0;
    assert!(
        !skills.contains(&SKILL_RUBY_BURST.to_string()),
        "舊飾品技能應移除"
    );
    assert!(
        skills.contains(&SKILL_WARRIOR.to_string()),
        "新飾品技能應加入"
    );
}

#[test]
fn test_unequip_unit_clamps_current_hp_to_new_max_hp() {
    let level_toml = LevelBuilder::from_ascii("M")
        .unit("M", UNIT_TYPE_MAGE, 0)
        .to_toml()
        .expect("關卡 TOML 應成功序列化");
    let mut world = setup_world(&level_toml);
    let occupant = occupant_at(&mut world, Position { x: 0, y: 0 });
    equip_unit(
        &mut world,
        occupant,
        &EQUIPMENT_GIANT_ARMOR.to_string(),
        EquipmentPosition::Armor,
    )
    .expect("防具應成功裝備");

    let entity = entity_of(&mut world, occupant);
    world.entity_mut(entity).insert(CurrentHp(130));
    equip_unit(
        &mut world,
        occupant,
        &EQUIPMENT_LEATHER_ARMOR.to_string(),
        EquipmentPosition::Armor,
    )
    .expect("防具應成功替換");

    let entity_ref = world.entity(entity);
    assert_eq!(entity_ref.get::<MaxHp>().expect("單位應有 MaxHp").0, 100);
    assert_eq!(
        entity_ref.get::<CurrentHp>().expect("單位應有 CurrentHp").0,
        100,
        "目前 HP 應壓至新上限"
    );

    unequip_unit(&mut world, occupant, EquipmentPosition::Armor).expect("防具應成功卸下");
    let entity_ref = world.entity(entity);
    assert_eq!(entity_ref.get::<MaxHp>().expect("單位應有 MaxHp").0, 80);
    assert_eq!(
        entity_ref.get::<CurrentHp>().expect("單位應有 CurrentHp").0,
        80,
        "卸下後目前 HP 應壓至新上限"
    );
}

// ==================================================
// 被動效果去重
// ==================================================

/// warrior 基礎物理攻擊：warrior-passive 10 + iron-sword 的 iron-slash 5。
const WARRIOR_BASE_PHYSICAL_ATTACK: i32 = 15;

#[test]
fn test_same_equipment_name_in_both_accessory_positions_grants_skill_once() {
    let level_toml = LevelBuilder::from_ascii("W")
        .unit("W", UNIT_TYPE_WARRIOR, 0)
        .to_toml()
        .expect("關卡 TOML 應成功序列化");
    let mut world = setup_world(&level_toml);
    let occupant = occupant_at(&mut world, Position { x: 0, y: 0 });

    equip_unit(
        &mut world,
        occupant,
        &EQUIPMENT_RUBY_RING.to_string(),
        EquipmentPosition::FirstAccessory,
    )
    .expect("第一飾品欄應成功裝備");
    equip_unit(
        &mut world,
        occupant,
        &EQUIPMENT_RUBY_RING.to_string(),
        EquipmentPosition::SecondAccessory,
    )
    .expect("同名飾品應可裝在第二飾品欄");

    let entity = entity_of(&mut world, occupant);
    let skills = &world
        .entity(entity)
        .get::<Skills>()
        .expect("單位應有 Skills")
        .0;
    assert_eq!(
        skills
            .iter()
            .filter(|name| name.as_str() == SKILL_RUBY_BURST)
            .count(),
        1,
        "兩個同名飾品只應授予一次技能"
    );
}

#[test]
fn test_equipment_granting_existing_unit_skill_counts_effect_once() {
    let level_toml = LevelBuilder::from_ascii("W")
        .unit("W", UNIT_TYPE_WARRIOR, 0)
        .to_toml()
        .expect("關卡 TOML 應成功序列化");
    let mut world = setup_world(&level_toml);
    let occupant = occupant_at(&mut world, Position { x: 0, y: 0 });

    equip_unit(
        &mut world,
        occupant,
        &EQUIPMENT_ECHO_CHARM.to_string(),
        EquipmentPosition::FirstAccessory,
    )
    .expect("飾品應成功裝備");

    let entity = entity_of(&mut world, occupant);
    let entity_ref = world.entity(entity);
    assert_eq!(
        entity_ref
            .get::<PhysicalAttack>()
            .expect("單位應有 PhysicalAttack")
            .0,
        WARRIOR_BASE_PHYSICAL_ATTACK,
        "裝備授予單位已擁有的技能時，被動效果只應計算一次"
    );
    let skills = &entity_ref.get::<Skills>().expect("單位應有 Skills").0;
    assert_eq!(
        skills.iter().filter(|name| *name == SKILL_WARRIOR).count(),
        1,
        "重複的技能名稱不應在 Skills 中出現兩次"
    );
}

// ==================================================
// 換裝失敗
// ==================================================

#[test]
fn test_equip_unit_returns_error_for_unknown_equipment() {
    let level_toml = LevelBuilder::from_ascii("M")
        .unit("M", UNIT_TYPE_MAGE, 0)
        .to_toml()
        .expect("level TOML should serialize");
    let mut world = setup_world(&level_toml);
    let occupant = occupant_at(&mut world, Position { x: 0, y: 0 });

    let error = equip_unit(
        &mut world,
        occupant,
        &"missing-equipment".to_string(),
        EquipmentPosition::Armor,
    )
    .expect_err("不存在的裝備應失敗");
    assert!(
        matches!(
            error.kind(),
            ErrorKind::Data(DataError::EquipmentTypeNotFound { .. })
        ),
        "應回傳 EquipmentTypeNotFound，實際為 {:?}",
        error.kind()
    );
}

#[test]
fn test_equip_unit_during_battle_returns_error_without_changing_equipment() {
    let level_toml = LevelBuilder::from_ascii("W")
        .unit("W", UNIT_TYPE_WARRIOR, 0)
        .to_toml()
        .expect("Level TOML 應成功序列化");
    let mut world = setup_world(&level_toml);
    let occupant = occupant_at(&mut world, Position { x: 0, y: 0 });
    let entity = entity_of(&mut world, occupant);
    let attack_before = world
        .entity(entity)
        .get::<PhysicalAttack>()
        .expect("單位應有 PhysicalAttack")
        .0;

    start_new_round(&mut world).expect("應開始戰鬥");
    let result = equip_unit(
        &mut world,
        occupant,
        &EQUIPMENT_STEEL_SWORD.to_string(),
        EquipmentPosition::Weapon,
    );

    assert!(result.is_err(), "戰鬥中換裝應回傳錯誤");
    assert_eq!(
        world
            .entity(entity)
            .get::<PhysicalAttack>()
            .expect("單位應有 PhysicalAttack")
            .0,
        attack_before,
        "戰鬥中換裝失敗不得改變單位屬性"
    );
}

#[test]
fn test_unequip_unit_during_battle_returns_error_without_changing_equipment() {
    let level_toml = LevelBuilder::from_ascii("W")
        .unit("W", UNIT_TYPE_WARRIOR, 0)
        .to_toml()
        .expect("Level TOML 應成功序列化");
    let mut world = setup_world(&level_toml);
    let occupant = occupant_at(&mut world, Position { x: 0, y: 0 });
    let entity = entity_of(&mut world, occupant);
    let hp_before = world
        .entity(entity)
        .get::<MaxHp>()
        .expect("單位應有 MaxHp")
        .0;

    start_new_round(&mut world).expect("應開始戰鬥");
    let result = unequip_unit(&mut world, occupant, EquipmentPosition::Armor);

    assert!(result.is_err(), "戰鬥中卸裝應回傳錯誤");
    assert_eq!(
        world
            .entity(entity)
            .get::<MaxHp>()
            .expect("單位應有 MaxHp")
            .0,
        hp_before,
        "戰鬥中卸裝失敗不得改變單位屬性"
    );
}
*/

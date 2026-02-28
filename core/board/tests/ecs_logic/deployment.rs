use super::super::test_helpers::level_builder::LevelBuilder;
use super::constants::{UNIT_TYPE_MAGE, UNIT_TYPE_WARRIOR};
use super::setup_world_with_level;
use board::domain::constants::PLAYER_FACTION_ID;
use board::ecs_logic::deployment::{deploy_unit, undeploy_unit};
use board::ecs_types::components::{Faction, Occupant, OccupantTypeName, Position, Unit};
use board::error::{DeploymentError, ErrorKind};

// ============================================================================
// deploy_unit 測試
// ============================================================================

#[test]
fn test_deploy_unit_success() {
    let level_toml = LevelBuilder::from_ascii(
        "
        D . . . .
        . . . . .
        . . . . .
        . . . . .
        . . . . .
    ",
    )
    .deploy("D")
    .to_toml()
    .expect("LevelBuilder::to_toml 應成功");
    let mut world = setup_world_with_level(&level_toml);

    let result = deploy_unit(
        &mut world,
        &UNIT_TYPE_WARRIOR.to_string(),
        Position { x: 0, y: 0 },
    );
    assert!(result.is_ok(), "deploy_unit 應成功：{:?}", result);

    let (_, type_name, faction, position) = world
        .query::<(&Unit, &OccupantTypeName, &Faction, &Position)>()
        .iter(&world)
        .next()
        .expect("應找到玩家部署的單位");
    assert_eq!(type_name.0, UNIT_TYPE_WARRIOR, "部署的單位類型應為 warrior");
    assert_eq!(faction.0, PLAYER_FACTION_ID, "部署的單位應屬於玩家陣營");
    assert_eq!(
        position,
        &Position { x: 0, y: 0 },
        "部署的單位位置應為 (0, 0)"
    );
}

#[test]
fn test_deploy_unit_invalid_position_returns_error() {
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
    let mut world = setup_world_with_level(&level_toml);

    for pos in [Position { x: 1, y: 0 }, Position { x: 9, y: 9 }] {
        let result = deploy_unit(&mut world, &UNIT_TYPE_WARRIOR.to_string(), pos);
        assert!(result.is_err(), "非法位置應回傳錯誤");

        let error = result.expect_err("應有錯誤");
        assert!(
            matches!(
                error.kind(),
                ErrorKind::Deployment(DeploymentError::PositionNotDeployable { .. })
            ),
            "應為 PositionNotDeployable，實際為 {:?}",
            error.kind()
        );
    }
}

#[test]
fn test_deploy_unit_exceeds_max_returns_error() {
    for faction in [PLAYER_FACTION_ID, 1] {
        let level_toml = LevelBuilder::from_ascii(
            "
        D D . . .
        . . . . .
        . . . . .
        . . . . .
        . . . . W
    ",
        )
        .unit("W", UNIT_TYPE_WARRIOR, faction)
        .deploy("D")
        .max_player_units(1)
        .to_toml()
        .expect("LevelBuilder::to_toml 應成功");

        for (pos1, pos2) in [
            (Position { x: 0, y: 0 }, Position { x: 1, y: 0 }),
            (Position { x: 1, y: 0 }, Position { x: 0, y: 0 }),
        ] {
            let mut world = setup_world_with_level(&level_toml);
            deploy_unit(&mut world, &UNIT_TYPE_WARRIOR.to_string(), pos1)
                .expect("第一次部署應成功");

            let result = deploy_unit(&mut world, &UNIT_TYPE_WARRIOR.to_string(), pos2);
            assert!(result.is_err(), "超過上限應回傳錯誤");

            let error = result.expect_err("應有錯誤");
            assert!(
                matches!(
                    error.kind(),
                    ErrorKind::Deployment(DeploymentError::MaxPlayerUnitsReached { .. })
                ),
                "應為 MaxPlayerUnitsReached，實際為 {:?}",
                error.kind()
            );
        }
    }
}

#[test]
fn test_deploy_unit_replaces_existing_unit_at_same_position() {
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
    let mut world = setup_world_with_level(&level_toml);

    deploy_unit(
        &mut world,
        &UNIT_TYPE_WARRIOR.to_string(),
        Position { x: 0, y: 0 },
    )
    .expect("第一次部署應成功");
    let count = world.query::<&Unit>().iter(&world).count();
    assert_eq!(count, 2, "部署後應有 2 個單位（含敵軍）");
    let (_, _, typ) = world
        .query::<(&Unit, &Faction, &OccupantTypeName)>()
        .iter(&world)
        .filter(|(_, faction, _)| faction.0 == 0)
        .next()
        .expect("應找到玩家部署的單位");
    assert_eq!(typ.0, UNIT_TYPE_WARRIOR, "部署的單位類型應為 warrior");

    // 再次部署到同格（替換）
    deploy_unit(
        &mut world,
        &UNIT_TYPE_MAGE.to_string(),
        Position { x: 0, y: 0 },
    )
    .expect("替換部署應成功");
    let count = world.query::<&Unit>().iter(&world).count();
    assert_eq!(count, 2, "替換後單位總數不變（含敵軍）");
    let (_, _, typ) = world
        .query::<(&Unit, &Faction, &OccupantTypeName)>()
        .iter(&world)
        .filter(|(_, faction, _)| faction.0 == 0)
        .next()
        .expect("應找到玩家部署的單位");
    assert_eq!(typ.0, UNIT_TYPE_MAGE, "部署的單位類型應為 mage");
}

#[test]
fn test_deploy_unit_occupant_id_is_unique() {
    let level_toml = LevelBuilder::from_ascii(
        "
        D D . . .
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
    let mut world = setup_world_with_level(&level_toml);

    deploy_unit(
        &mut world,
        &UNIT_TYPE_WARRIOR.to_string(),
        Position { x: 0, y: 0 },
    )
    .expect("部署應成功");
    deploy_unit(
        &mut world,
        &UNIT_TYPE_WARRIOR.to_string(),
        Position { x: 1, y: 0 },
    )
    .expect("部署應成功");

    let ids: Vec<u32> = world
        .query::<&Occupant>()
        .iter(&world)
        .map(|occ| match occ {
            Occupant::Unit(id) => *id,
            Occupant::Object(id) => *id,
        })
        .collect();

    let unique_count = ids.iter().collect::<std::collections::HashSet<_>>().len();
    assert_eq!(ids.len(), unique_count, "所有 Occupant ID 應唯一");
}

// ============================================================================
// undeploy_unit 測試
// ============================================================================

#[test]
fn test_undeploy_unit_success() {
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
    let mut world = setup_world_with_level(&level_toml);

    deploy_unit(
        &mut world,
        &UNIT_TYPE_MAGE.to_string(),
        Position { x: 0, y: 0 },
    )
    .expect("部署應成功");

    let count_before = world.query::<&Unit>().iter(&world).count();
    assert_eq!(count_before, 2, "取消前應有 2 個單位");

    undeploy_unit(&mut world, Position { x: 0, y: 0 }).expect("undeploy_unit 應成功");

    let remaining: Vec<_> = world
        .query::<(&Faction, &OccupantTypeName)>()
        .iter(&world)
        .collect();
    assert_eq!(remaining.len(), 1, "取消後應只剩 1 個單位（敵軍）");
    assert_eq!(
        remaining[0].0.0, 1,
        "剩餘單位應為敵軍陣營（faction id = 1）"
    );
}

#[test]
fn test_undeploy_unit_invalid_position_returns_error() {
    let level_toml = LevelBuilder::from_ascii(
        "
        D . . . .
        . . . . .
        . . . . .
        . . . . .
        . . . . .
    ",
    )
    .deploy("D")
    .to_toml()
    .expect("LevelBuilder::to_toml 應成功");
    let mut world = setup_world_with_level(&level_toml);

    for pos in [Position { x: 1, y: 0 }, Position { x: 9, y: 9 }] {
        let result = undeploy_unit(&mut world, pos);
        assert!(result.is_err(), "非法位置應回傳錯誤");
        let err = result.expect_err("應有錯誤");
        assert!(
            matches!(
                err.kind(),
                ErrorKind::Deployment(DeploymentError::PositionNotDeployable { .. })
            ),
            "應為 PositionNotDeployable, 實際為 {:?}",
            err.kind()
        );
    }
}

#[test]
fn test_undeploy_unit_nothing_to_undeploy_returns_error() {
    let level_toml = LevelBuilder::from_ascii(
        "
        D . . . .
        . . . . .
        . . . . .
        . . . . .
        . . . . .
    ",
    )
    .deploy("D")
    .to_toml()
    .expect("LevelBuilder::to_toml 應成功");
    let mut world = setup_world_with_level(&level_toml);

    let result = undeploy_unit(&mut world, Position { x: 0, y: 0 });
    assert!(result.is_err(), "空部署點應回傳錯誤");
    assert!(
        matches!(
            result.expect_err("應有錯誤").kind(),
            ErrorKind::Deployment(DeploymentError::NothingToUndeploy { .. })
        ),
        "應為 NothingToUndeploy"
    );
}

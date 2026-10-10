use super::support::{TARGET_ID, tank_ai_profile};
use crate::*;

// 驗證載入時拒絕沒有技能的敵方單位，空技能清單不會自動取得所有技能。
#[test]
fn enemy_without_skill_rejected_on_load() {
    let definitions = authoring::Definitions {
        equipment: Vec::new(),
        ai_profiles: vec![tank_ai_profile()],
        terrain_types: vec![TerrainTypeDef {
            blocks_sight: false,
            id: "plain".into(),
            layer: TerrainLayer::Ground,
            entry_rule: TerrainEntryRule::Walkable,
            damage: 0,
            extra_movement_cost: 0,
            dodge_penalty: 0,
            block_penalty: 0,
        }],
        skills: Vec::new(),
        unit_types: vec![authoring::UnitType {
            ai_profile: "tank".into(),
            id: "test_unit".into(),
            visual: "test_unit".into(),
            width: 1,
            height: 1,
            hp: 10,
            movement: 1,
            initiative: 0,
            dodge: 0,
            attack: 0,
            physical_power: 1,
            magical_power: 1,
            main_hand: String::new(),
            off_hand: String::new(),
            armor: String::new(),
            accessory: String::new(),
            skills: Vec::new(),
        }],
    };
    let map = authoring::Map {
        name: "test_map".into(),
        width: 2,
        height: 1,
        terrains: Vec::new(),
        units: vec![authoring::UnitPlacement {
            id: TARGET_ID,
            unit_type: "test_unit".into(),
            team: Team::Enemy("test_enemy".into()),
            x: 0,
            y: 0,
        }],
    };
    let error = match Game::from_authoring(definitions, map) {
        Ok(_) => panic!("沒有技能的敵方應在載入時被拒絕"),
        Err(error) => error,
    };
    assert_eq!(error.id(), "missing_ai_skill");
}

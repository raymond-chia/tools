use super::support::{TARGET_ID, tank_ai_profile};
use game_core::*;

const AI_DEFINITIONS: &str = include_str!("../data/utility_ai_definitions.toml");

// 驗證載入時拒絕沒有技能的敵方單位，空技能清單不會自動取得所有技能。
#[test]
fn enemy_without_skill_rejected_on_load() {
    let definitions = authoring::Definitions {
        equipment: Vec::new(),
        default_passive_skills: Vec::new(),
        ai_profiles: vec![tank_ai_profile()],
        terrain_types: vec![
            serde_json::from_value::<TerrainTypeDef>(serde_json::json!({
                "blocks_sight": false,
                "id": "plain",
                "layer": TerrainLayer::Ground,
                "entry_rule": TerrainEntryRule::Walkable,
                "damage": 0,
                "extra_movement_cost": 0,
                "dodge_penalty": 0,
                "block_penalty": 0
            }))
            .expect("測試地形資料應可載入"),
        ],
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
            hit: 0,
            physical_power: 1,
            magical_power: 1,
            main_hand: String::new(),
            off_hand: String::new(),
            armor: String::new(),
            accessory: String::new(),
            skills: Vec::new(),
            passive_skills: Vec::new(),
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

// 驗證 TOML 轉 JSON 保留完整共用傾向與各單位種類的引用，避免設定被靜默忽略。
#[test]
fn shared_profiles_survive_authoring_conversion() {
    let source: toml::Value =
        toml::from_str(AI_DEFINITIONS).expect("Utility AI 專用測試 TOML 應有效");
    let json = authoring::definitions_to_json(AI_DEFINITIONS).expect("測試定義應可轉為 JSON");
    let actual: serde_json::Value = serde_json::from_str(&json).expect("輸出應為有效 JSON");
    let expected = serde_json::to_value(&source).expect("測試 TOML 應可轉為 JSON");
    assert_eq!(actual["ai_profiles"], expected["ai_profiles"]);
    for kind in ["tank", "other_tank", "archer", "healer"] {
        let unit = actual["unit_types"]
            .as_array()
            .expect("輸出應包含單位種類")
            .iter()
            .find(|unit| unit["id"] == kind)
            .expect("輸出應保留測試單位種類");
        let expected_profile = if kind == "other_tank" { "tank" } else { kind };
        assert_eq!(unit["ai_profile"], expected_profile, "{kind}");
    }
}

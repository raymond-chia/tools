use crate::support::{TARGET_ID, tank_ai_profile};
use game_core::*;

const AI_DEFINITIONS: &str = include_str!("../data/utility_ai_definitions.toml");

// 驗證明確設定勝敗的地圖仍拒絕無技能敵方，空技能清單不會自動取得所有技能。
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
        active_skills: Vec::new(),
        passive_skills: Vec::new(),
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
            active_skills: Vec::new(),
            passive_skills: Vec::new(),
        }],
    };
    let map = authoring::Map {
        alliances: Vec::new(),
        victory: authoring::OutcomeGroup {
            mode: authoring::ConditionMode::All,
            conditions: vec![authoring::OutcomeCondition::AllEnemiesDefeated],
        },
        defeat: authoring::OutcomeGroup {
            mode: authoring::ConditionMode::Any,
            conditions: vec![authoring::OutcomeCondition::TeamDefeated { team: Team::Player }],
        },
        cargo: None,
        name: "test_map".into(),
        width: 2,
        height: 1,
        terrains: Vec::new(),
        units: vec![authoring::UnitPlacement {
            facing: authoring::Facing::Right,
            id: TARGET_ID,
            min_difficulty: Some(authoring::Difficulty::Easy),
            unit_type: "test_unit".into(),
            team: Team::Enemy("test_enemy".into()),
            x: 0,
            y: 0,
        }],
    };
    let error = match Game::from_authoring(definitions, map, game_core::authoring::Difficulty::Easy)
    {
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

// 驗證明確設定勝敗的地圖要求怪物最低出現難度，玩家免填，未知難度拒絕。
#[test]
fn enemy_difficulty_is_required_at_authoring_boundary() {
    for (team, setting, accepted) in [
        ("[units.team]\nenemy = \"test_enemy\"", "", false),
        (
            "[units.team]\nenemy = \"test_enemy\"",
            "min_difficulty = \"easy\"",
            true,
        ),
        (
            "[units.team]\nenemy = \"test_enemy\"",
            "min_difficulty = \"invalid\"",
            false,
        ),
        ("team = \"player\"", "", true),
    ] {
        let source = format!(
            "name = \"difficulty_test\"\nwidth = 1\nheight = 1\n[[units]]\nfacing = \"right\"\nid = 1\nunit_type = \"tank\"\nx = 0\ny = 0\n{setting}\n{team}"
        );
        let source = format!("{source}{OUTCOME_SETTINGS}");
        assert_eq!(
            authoring::map_to_json(&source).is_ok(),
            accepted,
            "{source}"
        );
        assert_eq!(
            Game::from_documents(AI_DEFINITIONS, &source, authoring::Difficulty::Easy).is_ok(),
            accepted,
            "正式載入：{source}"
        );
    }
}

// 驗證含明確勝敗設定的 TOML 轉換保留怪物最低出現難度。
#[test]
fn enemy_difficulty_survives_map_conversion() {
    for difficulty in ["easy", "normal", "hard"] {
        let source = format!(
            "name = \"difficulty_test\"\n[[units]]\nfacing = \"right\"\nid = 2\nunit_type = \"tank\"\nmin_difficulty = \"{difficulty}\"\n[units.team]\nenemy = \"test_enemy\""
        );
        let source = format!("{source}{OUTCOME_SETTINGS}");
        let json = authoring::map_to_json(&source).expect("明確設定難度的地圖應可轉換");
        let actual: serde_json::Value = serde_json::from_str(&json).expect("轉換結果應為 JSON");
        assert_eq!(actual["units"][0]["min_difficulty"], difficulty);
    }
}

// 驗證編輯器儲存與重載保留各怪物的出現難度，缺少設定時禁止儲存。
#[cfg(feature = "editor")]
#[test]
fn editor_saving_preserves_required_enemy_difficulty() {
    let definitions = authoring::definitions_to_json(AI_DEFINITIONS).expect("測試定義應可轉換");
    let mut map = difficulty_map();
    let (_, saved) =
        editor::documents_from_json(&definitions, &map.to_string()).expect("有效地圖應可儲存");
    let actual: serde_json::Value =
        serde_json::from_str(&authoring::map_to_json(&saved).expect("儲存地圖應可重載"))
            .expect("重載應為 JSON");
    for index in 1..=3 {
        assert_eq!(
            actual["units"][index]["min_difficulty"],
            map["units"][index]["min_difficulty"]
        );
    }
    map["units"][2]
        .as_object_mut()
        .expect("單位應為物件")
        .remove("min_difficulty");
    assert_eq!(
        editor::documents_from_json(&definitions, &map.to_string()).is_err(),
        true
    );
}

// 驗證作者地圖轉為 TOML 後，開局依最低出現難度篩選怪物，玩家在所有難度保留。
#[test]
fn selected_difficulty_filters_enemy_placements() {
    let map = difficulty_map();
    let map: authoring::Map = serde_json::from_value(map).expect("測試地圖應符合作者格式");
    let map = toml::to_string(&map).expect("測試地圖應可序列化");
    for (difficulty, expected) in [
        ("easy", vec![1, 2]),
        ("normal", vec![1, 2, 3]),
        ("hard", vec![1, 2, 3, 4]),
    ] {
        let selected = serde_json::from_value(serde_json::json!(difficulty)).expect("難度應可解析");
        let game = Game::from_documents(AI_DEFINITIONS, &map, selected).expect("難度戰鬥應可載入");
        let mut actual: Vec<_> = game
            .snapshot(None)
            .units
            .iter()
            .map(|unit| unit.id)
            .collect();
        actual.sort();
        assert_eq!(actual, expected, "{difficulty}");
    }
}

// 驗證作者輸入拒絕不會生效或彼此衝突的設定，避免設定被靜默忽略。
#[test]
fn ineffective_or_conflicting_settings_are_rejected() {
    let cases: [(&str, fn(&mut serde_json::Value), &str); 9] = [
        // 不可通行地形不能設定傷害、移動消耗與防禦懲罰。
        (
            "牆壁傷害",
            |d| entry(d, "terrain_types", "wall")["damage"] = 1.into(),
            "terrain_values_require_walkable",
        ),
        (
            "牆壁移動消耗",
            |d| entry(d, "terrain_types", "wall")["extra_movement_cost"] = 1.into(),
            "terrain_values_require_walkable",
        ),
        (
            "牆壁閃避懲罰",
            |d| entry(d, "terrain_types", "wall")["dodge_penalty"] = 1.into(),
            "terrain_values_require_walkable",
        ),
        (
            "牆壁格擋懲罰",
            |d| entry(d, "terrain_types", "wall")["block_penalty"] = 1.into(),
            "terrain_values_require_walkable",
        ),
        // 推入陣亡地形同樣不可通行。
        (
            "推入陣亡地形閃避懲罰",
            |d| {
                let terrain = entry(d, "terrain_types", "wall");
                terrain["entry_rule"] = "instant_down_when_pushed".into();
                terrain["dodge_penalty"] = 1.into();
            },
            "terrain_values_require_walkable",
        ),
        // 單位閃避不可為負。
        (
            "負閃避",
            |d| entry(d, "unit_types", "tank")["dodge"] = (-1).into(),
            "invalid_unit_type",
        ),
        // 主動與被動技能共用名稱翻譯，ID 不可重複。
        (
            "被動與主動同 ID",
            |d| add_flanking(d, "melee"),
            "duplicate_skill_id",
        ),
        // 主動技能清單不可引用被動技能。
        (
            "主動清單引用被動",
            |d| {
                add_flanking(d, "test_flanking");
                entry(d, "unit_types", "tank")["active_skills"] =
                    serde_json::json!(["melee", "test_flanking"]);
            },
            "unknown_unit_skill",
        ),
        // 被動技能清單不可引用主動技能。
        (
            "被動清單引用主動",
            |d| entry(d, "unit_types", "tank")["passive_skills"] = serde_json::json!(["melee"]),
            "invalid_passive_skills",
        ),
    ];
    let source: toml::Value =
        toml::from_str(AI_DEFINITIONS).expect("Utility AI 專用測試 TOML 應有效");
    let map: authoring::Map =
        serde_json::from_value(difficulty_map()).expect("測試地圖應符合作者格式");
    for (name, change, expected) in cases {
        let mut definitions = serde_json::to_value(&source).expect("測試 TOML 應可轉為 JSON");
        change(&mut definitions);
        let definitions: authoring::Definitions =
            serde_json::from_value(definitions).expect("修改後的測試定義應符合作者格式");
        let error =
            match Game::from_authoring(definitions, map.clone(), authoring::Difficulty::Easy) {
                Ok(_) => panic!("{name} 應拒絕載入"),
                Err(error) => error,
            };
        assert_eq!(error.id(), expected, "{name}");
    }
}

// 依 ID 取得作者定義中的單筆資料。
fn entry<'a>(
    definitions: &'a mut serde_json::Value,
    category: &str,
    id: &str,
) -> &'a mut serde_json::Value {
    definitions[category]
        .as_array_mut()
        .expect("測試定義應包含該類別")
        .iter_mut()
        .find(|entry| entry["id"] == id)
        .expect("測試定義應包含該 ID")
}

fn add_flanking(definitions: &mut serde_json::Value, id: &str) {
    definitions["passive_skills"] =
        serde_json::json!([{"id": id, "effect": "flanking", "hit_bonus": 2}]);
}

// 簡單、普通、困難各一隻怪物，所有難度共用同一份作者配置。
fn difficulty_map() -> serde_json::Value {
    use crate::support::ascii_map;
    let map = ascii_map(
        "difficulty_test",
        "
        ATUV
        ....
    ",
        &[
            ('A', 1, "archer", Team::Player),
            ('T', 2, "target", Team::Enemy("test_enemy".into())),
            ('U', 3, "target", Team::Enemy("test_enemy".into())),
            ('V', 4, "target", Team::Enemy("test_enemy".into())),
        ],
    );
    let mut map = serde_json::to_value(map).expect("測試地圖應可轉換");
    for (index, difficulty) in ["easy", "normal", "hard"].into_iter().enumerate() {
        map["units"][index + 1]["min_difficulty"] = difficulty.into();
    }
    map
}

// 作者格式案例明確提供殲滅戰勝敗設定。
const OUTCOME_SETTINGS: &str = r#"
[victory]
mode = "all"
conditions = [{ type = "all_enemies_defeated" }]
[defeat]
mode = "any"
conditions = [{ type = "team_defeated", team = "player" }]
"#;

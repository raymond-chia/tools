//! 先定義共用傾向設定與可見行為；不實作測試專用評分器或 AI 替身。
use super::support::{ACTOR_ID, TARGET_ID};
use crate::*;

const DEFINITIONS: &str = include_str!("data/utility_ai_definitions.toml");
const ALLY_ID: i64 = 3;
const SECOND_ALLY_ID: i64 = 4;

fn definitions() -> toml::Value {
    toml::from_str(DEFINITIONS).expect("Utility AI 專用測試 TOML 應有效")
}

fn unit_type_mut<'a>(definitions: &'a mut toml::Value, id: &str) -> &'a mut toml::Value {
    definitions["unit_types"]
        .as_array_mut()
        .expect("測試定義應包含單位種類陣列")
        .iter_mut()
        .find(|unit| unit["id"].as_str() == Some(id))
        .expect("測試定義應包含指定單位種類")
}

fn profile_mut<'a>(definitions: &'a mut toml::Value, id: &str) -> &'a mut toml::Value {
    definitions["ai_profiles"]
        .as_array_mut()
        .expect("測試定義應包含傾向陣列")
        .iter_mut()
        .find(|profile| profile["id"].as_str() == Some(id))
        .expect("測試定義應包含指定傾向")
}

fn placement(id: i64, kind: &str, team: Team, x: i32) -> authoring::UnitPlacement {
    authoring::UnitPlacement {
        id,
        unit_type: kind.into(),
        team,
        x,
        y: 0,
    }
}

fn enemy() -> Team {
    Team::Enemy("test_enemy".into())
}

fn game(definitions: &toml::Value, placements: Vec<authoring::UnitPlacement>) -> Game {
    let map = authoring::Map {
        name: "utility_ai_test".into(),
        width: 12,
        height: 1,
        terrains: Vec::new(),
        units: placements,
    };
    let source = toml::to_string(definitions).expect("測試定義應可序列化");
    let map = toml::to_string(&map).expect("測試地圖應可序列化");
    let mut game = Game::from_documents(&source, &map).expect("測試戰鬥應可載入");
    game.set_random_seed(1);
    game
}

fn duel(definitions: &toml::Value, kind: &str, actor_x: i32, target_x: i32) -> Game {
    game(
        definitions,
        vec![
            placement(ACTOR_ID, kind, enemy(), actor_x),
            placement(TARGET_ID, "target", Team::Player, target_x),
        ],
    )
}

fn wound(game: &mut Game, id: i64, remaining_hp: i32) {
    let entity = game.entity(id).expect("受傷單位應存在於測試戰鬥");
    game.world
        .get_mut::<Hp>(entity)
        .expect("測試單位應具有 Hp 元件")
        .current = remaining_hp;
}

fn act(game: &mut Game) -> serde_json::Value {
    game.command(Command::Start).expect("測試戰鬥應可開始");
    assert_eq!(game.world.resource::<Turn>().actor, Some(ACTOR_ID));
    let snapshot = game
        .command(Command::Continue)
        .expect("敵方應可完成一次 AI 行動");
    serde_json::to_value(snapshot).expect("回合快照應可序列化")
}

fn action(snapshot: &serde_json::Value) -> Option<(&str, i64)> {
    snapshot["log"].as_array()?.iter().find_map(|event| {
        if event["actor"].as_i64() == Some(ACTOR_ID) {
            Some((event["skill"].as_str()?, event["target"].as_i64()?))
        } else {
            None
        }
    })
}

fn actor_x(snapshot: &serde_json::Value) -> i64 {
    snapshot["units"]
        .as_array()
        .expect("快照應包含單位陣列")
        .iter()
        .find(|unit| unit["id"].as_i64() == Some(ACTOR_ID))
        .expect("AI 行動後施放者應仍存在")["x"]
        .as_i64()
        .expect("單位快照應包含 x 座標")
}

// 驗證 TOML 轉 JSON 保留完整共用傾向與各單位種類的引用，避免設定被靜默忽略。
#[test]
fn shared_profiles_survive_authoring_conversion() {
    let source = definitions();
    let json = authoring::definitions_to_json(DEFINITIONS).expect("測試定義應可轉為 JSON");
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

// 驗證共用肉盾傾向會在第一段移動後近戰，無法攻擊時則利用兩段額度接敵。
#[test]
fn tanks_share_approach_and_melee_behavior() {
    for kind in ["tank", "other_tank"] {
        for (target_x, expected_x, expected_action) in
            [(5, 4, Some(("melee", TARGET_ID))), (8, 6, None)]
        {
            let mut game = duel(&definitions(), kind, 2, target_x);
            let snapshot = act(&mut game);
            assert_eq!(actor_x(&snapshot), expected_x, "{kind}, 目標 {target_x}");
            assert_eq!(
                action(&snapshot),
                expected_action,
                "{kind}, 目標 {target_x}"
            );
        }
    }
}

// 驗證遠距離射手依技能最大射程站位，太近時後退、太遠時靠近。
#[test]
fn archer_positions_before_shooting() {
    for (name, target_x, expected_distance) in [
        ("拉至最大射程", 5, 4),
        ("近身時後退", 3, 3),
        ("遠處時接近", 7, 4),
    ] {
        let mut game = duel(&definitions(), "archer", 2, target_x);
        let snapshot = act(&mut game);
        assert_eq!(
            (actor_x(&snapshot) - i64::from(target_x)).abs(),
            expected_distance,
            "{name}"
        );
        assert_eq!(action(&snapshot), Some(("shot", TARGET_ID)), "{name}");
    }
}

// 驗證治癒者比較所有技能並優先治療重傷隊友，不受技能清單順序影響。
#[test]
fn healer_prioritizes_healing_independent_of_skill_order() {
    for skills in [["heal", "shot"], ["shot", "heal"]] {
        let mut definitions = definitions();
        unit_type_mut(&mut definitions, "healer")["skills"] =
            toml::Value::Array(skills.map(|skill| toml::Value::String(skill.into())).into());
        let mut game = game(
            &definitions,
            vec![
                placement(ACTOR_ID, "healer", enemy(), 2),
                placement(TARGET_ID, "target", Team::Player, 5),
                placement(ALLY_ID, "target", enemy(), 1),
            ],
        );
        wound(&mut game, ALLY_ID, 20);
        let snapshot = act(&mut game);
        assert_eq!(action(&snapshot), Some(("heal", ALLY_ID)), "{skills:?}");
        assert_eq!(
            game.world
                .get::<Hp>(game.entity(ALLY_ID).expect("隊友應仍存在"))
                .expect("隊友應具有 Hp")
                .current,
            50
        );
    }
}

// 驗證治癒者優先治療較遠但傷勢嚴重的隊友，而非固定選擇最近的受傷隊友。
#[test]
fn healer_prefers_severe_wound_over_nearest_ally() {
    let mut game = game(
        &definitions(),
        vec![
            placement(ACTOR_ID, "healer", enemy(), 2),
            placement(TARGET_ID, "target", Team::Player, 6),
            placement(ALLY_ID, "target", enemy(), 1),
            placement(SECOND_ALLY_ID, "target", enemy(), 4),
        ],
    );
    wound(&mut game, ALLY_ID, 95);
    wound(&mut game, SECOND_ALLY_ID, 20);
    assert_eq!(action(&act(&mut game)), Some(("heal", SECOND_ALLY_ID)));
}

// 驗證治癒者會移動至重傷隊友的治療射程內，且不超過移動額度。
#[test]
fn healer_moves_into_healing_range() {
    let mut game = game(
        &definitions(),
        vec![
            placement(ACTOR_ID, "healer", enemy(), 2),
            placement(TARGET_ID, "target", Team::Player, 0),
            placement(ALLY_ID, "target", enemy(), 7),
        ],
    );
    wound(&mut game, ALLY_ID, 20);
    let snapshot = act(&mut game);
    assert_eq!(actor_x(&snapshot), 4);
    assert_eq!(action(&snapshot), Some(("heal", ALLY_ID)));
}

// 驗證朝不可及重傷隊友移動時，走滿或縮短第一段都必須保留治療輕傷隊友的能力。
#[test]
fn healer_approaches_unreachable_severe_wound_and_heals_light_wound() {
    let cases = [
        // 走滿第一段兩格後，輕傷隊友仍在治療射程內。
        ("可走滿第一段", 5, 4, vec![2, 3, 4]),
        // 走滿兩格會使輕傷隊友超出射程，因此只走一格後治療。
        ("須縮短第一段", 0, 3, vec![2, 3]),
    ];
    let mut failures = Vec::new();
    for (name, light_x, expected_x, expected_path) in cases {
        let mut game = game(
            &definitions(),
            vec![
                placement(ACTOR_ID, "healer", enemy(), 2),
                placement(TARGET_ID, "target", Team::Player, 7),
                placement(ALLY_ID, "target", enemy(), light_x),
                placement(SECOND_ALLY_ID, "target", enemy(), 10),
            ],
        );
        wound(&mut game, ALLY_ID, 95);
        wound(&mut game, SECOND_ALLY_ID, 20);

        let snapshot = act(&mut game);
        // 先彙整決策差異，讓尚未實作 AI 時也能一次看到兩個案例的失敗。
        let actual_x = actor_x(&snapshot);
        let actual_action = action(&snapshot);
        if actual_x != expected_x || actual_action != Some(("heal", ALLY_ID)) {
            failures.push(format!(
                "{name}：預期移至 {expected_x} 並治療輕傷隊友，實際位置 {actual_x}、行動 {actual_action:?}"
            ));
            continue;
        }
        assert!(
            (actual_x - i64::from(light_x)).abs() <= 3,
            "{name}：輕傷隊友應在治療射程內"
        );
        assert!(
            (10 - actual_x).abs() > 3,
            "{name}：重傷隊友應仍在治療射程外"
        );
        let movements = snapshot["movements"]
            .as_array()
            .expect("快照應包含移動紀錄");
        assert_eq!(movements.len(), 1, "{name}：應先移動一次再施放治療");
        assert_eq!(movements[0]["unit_id"], ACTOR_ID, "{name}");
        let expected_path: Vec<_> = expected_path
            .into_iter()
            .map(|x| serde_json::json!({ "x": x, "y": 0 }))
            .collect();
        assert_eq!(
            movements[0]["path"],
            serde_json::json!(expected_path),
            "{name}"
        );
        // 移動索引對應完整戰鬥日誌；command 快照只包含本次新增的日誌。
        let full_log =
            serde_json::to_value(&game.world.resource::<Log>().0).expect("完整戰鬥日誌應可序列化");
        let healing_index = full_log
            .as_array()
            .expect("完整日誌應為陣列")
            .iter()
            .position(|event| event["type"] == "healing" && event["actor"] == ACTOR_ID)
            .expect("應產生施放者的治療紀錄");
        assert!(
            movements[0]["before_log_index"]
                .as_u64()
                .expect("移動紀錄應包含對應日誌索引")
                <= healing_index as u64,
            "{name}：移動應發生在治療前"
        );
        for (id, expected_hp) in [(ALLY_ID, 100), (SECOND_ALLY_ID, 20)] {
            let entity = game.entity(id).expect("隊友應仍存在");
            let Hp {
                current,
                maximum: _,
            } = game.world.get::<Hp>(entity).expect("隊友應具有 Hp");
            assert_eq!(*current, expected_hp, "{name}，隊友 {id}");
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// 驗證不同生命上限下優先治療低血量隊友，但其會溢補時改治療能完整接受治療的高血量隊友。
#[test]
fn healer_compares_current_health_and_overhealing() {
    let cases = [
        // 兩者都能接受完整 30 HP 治療時，優先治療總血量與目前血量較低者。
        ("低血量且不會溢補", 60, ALLY_ID, 50, 160),
        // 低血量者只缺 20 HP 時，優先治療總血量與目前血量較高、可完整接受 30 HP 者。
        ("避免低血量目標溢補", 40, SECOND_ALLY_ID, 20, 190),
    ];
    for (name, low_max_hp, expected_target, low_expected_hp, high_expected_hp) in cases {
        let mut definitions = definitions();
        let mut low = unit_type_mut(&mut definitions, "target").clone();
        low["id"] = "low_hp_ally".into();
        low["hp"] = low_max_hp.into();
        let mut high = unit_type_mut(&mut definitions, "target").clone();
        high["id"] = "high_hp_ally".into();
        high["hp"] = 200.into();
        definitions["unit_types"]
            .as_array_mut()
            .expect("測試定義應包含單位種類陣列")
            .extend([low, high]);
        let mut game = game(
            &definitions,
            vec![
                placement(ACTOR_ID, "healer", enemy(), 5),
                placement(TARGET_ID, "target", Team::Player, 8),
                placement(ALLY_ID, "low_hp_ally", enemy(), 4),
                placement(SECOND_ALLY_ID, "high_hp_ally", enemy(), 6),
            ],
        );
        wound(&mut game, ALLY_ID, 20);
        wound(&mut game, SECOND_ALLY_ID, 160);

        let snapshot = act(&mut game);
        assert_eq!(action(&snapshot), Some(("heal", expected_target)), "{name}");
        for (id, expected_hp, expected_max_hp) in [
            (ALLY_ID, low_expected_hp, low_max_hp),
            (SECOND_ALLY_ID, high_expected_hp, 200),
        ] {
            let entity = game.entity(id).expect("隊友應仍存在");
            let Hp { current, maximum } = game.world.get::<Hp>(entity).expect("隊友應具有 Hp");
            assert_eq!(
                (*current, *maximum),
                (expected_hp, expected_max_hp),
                "{name}，隊友 {id}"
            );
        }
    }
}

// 驗證沒有受傷隊友時治癒者改用攻擊，不對滿血單位施放治療或直接空過回合。
#[test]
fn healer_attacks_when_healing_is_unnecessary() {
    let mut game = duel(&definitions(), "healer", 2, 5);
    assert_eq!(action(&act(&mut game)), Some(("shot", TARGET_ID)));
}

// 驗證調整同一傾向的傷害與治療權重即可改變選擇，不必修改單位種類或技能。
#[test]
fn profile_weights_change_heal_or_attack_choice() {
    for (damage, healing, expected) in [(1, 100, ("heal", ALLY_ID)), (100, 0, ("shot", TARGET_ID))]
    {
        let mut definitions = definitions();
        profile_mut(&mut definitions, "healer")["damage_weight"] = damage.into();
        profile_mut(&mut definitions, "healer")["healing_weight"] = healing.into();
        let mut game = game(
            &definitions,
            vec![
                placement(ACTOR_ID, "healer", enemy(), 2),
                placement(TARGET_ID, "target", Team::Player, 5),
                placement(ALLY_ID, "target", enemy(), 1),
            ],
        );
        wound(&mut game, ALLY_ID, 20);
        assert_eq!(
            action(&act(&mut game)),
            Some(expected),
            "傷害 {damage}、治療 {healing}"
        );
    }
}

// 驗證近遠傾向跨技能比較射程：近距離選近戰一格，遠距離選射擊四格。
#[test]
fn unit_type_uses_referenced_profile() {
    for (profile, expected_distance) in [("archer", 4), ("tank", 1)] {
        let mut definitions = definitions();
        let archer = unit_type_mut(&mut definitions, "archer");
        archer["ai_profile"] = profile.into();
        archer["skills"] = toml::Value::Array(vec!["shot".into(), "melee".into()]);
        let mut game = duel(&definitions, "archer", 2, 5);
        let snapshot = act(&mut game);
        assert_eq!(
            (actor_x(&snapshot) - 5).abs(),
            expected_distance,
            "{profile}"
        );
        assert_eq!(
            action(&snapshot),
            Some((if profile == "tank" { "melee" } else { "shot" }, TARGET_ID))
        );
    }
}

// 驗證相同射程與傷害條件下優先攻擊能擊倒的敵人，而非依單位建立順序選擇。
#[test]
fn archer_prioritizes_a_defeatable_target() {
    for reverse in [false, true] {
        let mut placements = vec![
            placement(ACTOR_ID, "archer", enemy(), 5),
            placement(TARGET_ID, "target", Team::Player, 2),
            placement(ALLY_ID, "target", Team::Player, 8),
        ];
        if reverse {
            placements.reverse();
        }
        let mut game = game(&definitions(), placements);
        wound(&mut game, ALLY_ID, 1);
        let snapshot = act(&mut game);
        assert_eq!(
            action(&snapshot),
            Some(("shot", ALLY_ID)),
            "反向載入 {reverse}"
        );
        assert!(game.entity(ALLY_ID).is_none(), "可擊倒目標應已離場");
    }
}

// 驗證同分候選在相同亂數種子下穩定選擇，不受單位配置載入順序影響。
#[test]
fn equal_utility_has_a_stable_target_choice() {
    let mut choices = Vec::new();
    for reverse in [false, true, false] {
        let mut placements = vec![
            placement(ACTOR_ID, "archer", enemy(), 5),
            placement(TARGET_ID, "target", Team::Player, 2),
            placement(ALLY_ID, "target", Team::Player, 8),
        ];
        if reverse {
            placements.reverse();
        }
        let mut game = game(&definitions(), placements);
        let snapshot = act(&mut game);
        let (skill, target) = action(&snapshot).expect("同分候選中應選出一次攻擊");
        assert_eq!(skill, "shot");
        choices.push(target);
    }
    assert!(
        choices.iter().all(|target| *target == choices[0]),
        "同分選擇應穩定：{choices:?}"
    );
}

// 驗證只切換近遠偏好或技能射程便會改變攻擊站位，無須手動同步理想格數。
#[test]
fn distance_preference_tracks_skill_range() {
    for (preference, min_range, max_range, expected_distance) in [
        ("near", 2, 4, 2),
        ("far", 2, 4, 4),
        ("near", 3, 5, 3),
        ("far", 3, 5, 5),
    ] {
        let mut definitions = definitions();
        profile_mut(&mut definitions, "archer")["distance_preference"] = preference.into();
        let shot = definitions["skills"]
            .as_array_mut()
            .expect("測試定義應包含技能陣列")
            .iter_mut()
            .find(|skill| skill["id"].as_str() == Some("shot"))
            .expect("測試定義應包含射擊技能");
        shot["min_range"] = min_range.into();
        shot["max_range"] = max_range.into();
        let mut game = duel(&definitions, "archer", 2, 5);
        let snapshot = act(&mut game);
        assert_eq!(
            (actor_x(&snapshot) - 5).abs(),
            expected_distance,
            "{preference}, {min_range}..{max_range}"
        );
        assert_eq!(action(&snapshot), Some(("shot", TARGET_ID)));
    }
}

// 驗證第一段內可攻擊就施放，否則利用兩段接近射程但不施放，且沒有攻擊技能不追敵。
#[test]
fn fallback_uses_available_skill_ranges() {
    for (skills, target_x, expected_x) in [
        (vec!["shot"], 7, 3),
        (vec!["shot"], 9, 5),
        (vec!["shot"], 10, 6),
        (vec!["shot"], 11, 6),
        (vec!["heal"], 10, 2),
    ] {
        let mut definitions = definitions();
        unit_type_mut(&mut definitions, "archer")["skills"] = toml::Value::Array(
            skills
                .iter()
                .map(|skill| toml::Value::String((*skill).into()))
                .collect(),
        );
        let mut game = duel(&definitions, "archer", 2, target_x);
        let snapshot = act(&mut game);
        assert_eq!(
            actor_x(&snapshot),
            expected_x,
            "{skills:?}, 目標 {target_x}"
        );
        let expected_action = if target_x == 7 {
            Some(("shot", TARGET_ID))
        } else {
            None
        };
        assert_eq!(
            action(&snapshot),
            expected_action,
            "{skills:?}, 目標 {target_x}"
        );
    }
}

// 驗證治療技能也依近遠偏好選擇合法站位，而非套用攻擊射程。
#[test]
fn healer_distance_uses_healing_range() {
    for (preference, expected_x) in [("near", 3), ("far", 1)] {
        let mut definitions = definitions();
        profile_mut(&mut definitions, "healer")["distance_preference"] = preference.into();
        let mut game = game(
            &definitions,
            vec![
                placement(ACTOR_ID, "healer", enemy(), 2),
                placement(TARGET_ID, "target", Team::Player, 10),
                placement(ALLY_ID, "target", enemy(), 4),
            ],
        );
        wound(&mut game, ALLY_ID, 20);
        let snapshot = act(&mut game);
        assert_eq!(actor_x(&snapshot), expected_x, "{preference}");
        assert_eq!(action(&snapshot), Some(("heal", ALLY_ID)));
    }
}

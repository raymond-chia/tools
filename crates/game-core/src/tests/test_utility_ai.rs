//! 先定義共用傾向設定與可見行為；不實作測試專用評分器或 AI 替身。
use super::support::{ACTOR_ID, AsciiBoard, TARGET_ID, ascii_board, ascii_map};
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

fn game(definitions: &toml::Value, diagram: &str, units: &[(char, i64, &str, Team)]) -> Game {
    game_on_map(definitions, ascii_map("utility_ai_test", diagram, units))
}

fn game_on_map(definitions: &toml::Value, map: authoring::Map) -> Game {
    let source = toml::to_string(definitions).expect("測試定義應可序列化");
    let map = toml::to_string(&map).expect("測試地圖應可序列化");
    let mut game = Game::from_documents(&source, &map).expect("測試戰鬥應可載入");
    game.set_random_seed(1);
    game
}

fn duel(definitions: &toml::Value, kind: &str, diagram: &str) -> Game {
    game(
        definitions,
        diagram,
        &[
            ('A', ACTOR_ID, kind, enemy()),
            ('T', TARGET_ID, "target", Team::Player),
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
        for (diagram, expected_x, expected_action) in [
            ("..A..T......", 4, Some(("melee", TARGET_ID))),
            ("..A.....T...", 6, None),
        ] {
            let mut game = duel(&definitions(), kind, diagram);
            let snapshot = act(&mut game);
            assert_eq!(actor_x(&snapshot), expected_x, "{kind}, 地圖 {diagram}");
            assert_eq!(action(&snapshot), expected_action, "{kind}, 地圖 {diagram}");
        }
    }
}

// 驗證射手依 ASCII 目標位置及技能最大射程站位，太近時後退、太遠時靠近。
#[test]
fn archer_positions_before_shooting() {
    for (name, diagram, expected_distance) in [
        ("拉至最大射程", "..A..T......", 4),
        ("近身時後退", "..AT........", 3),
        ("遠處時接近", "..A....T....", 4),
    ] {
        let mut game = duel(&definitions(), "archer", diagram);
        let target = game.entity(TARGET_ID).expect("ASCII 的 T 應建立目標單位");
        let target_x = game.world.get::<Pos>(target).expect("目標應有位置").0.x;
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
            ".LA..T......",
            &[
                ('A', ACTOR_ID, "healer", enemy()),
                ('T', TARGET_ID, "target", Team::Player),
                ('L', ALLY_ID, "target", enemy()),
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
        ".LA.S.T.....",
        &[
            ('A', ACTOR_ID, "healer", enemy()),
            ('T', TARGET_ID, "target", Team::Player),
            ('L', ALLY_ID, "target", enemy()),
            ('S', SECOND_ALLY_ID, "target", enemy()),
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
        "T.A....L....",
        &[
            ('A', ACTOR_ID, "healer", enemy()),
            ('T', TARGET_ID, "target", Team::Player),
            ('L', ALLY_ID, "target", enemy()),
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
        ("可走滿第一段", "..A..L.T..S.", 4, vec![2, 3, 4]),
        // 走滿兩格會使輕傷隊友超出射程，因此只走一格後治療。
        ("須縮短第一段", "L.A....T..S.", 3, vec![2, 3]),
    ];
    let mut failures = Vec::new();
    for (name, diagram, expected_x, expected_path) in cases {
        let mut game = game(
            &definitions(),
            diagram,
            &[
                ('A', ACTOR_ID, "healer", enemy()),
                ('T', TARGET_ID, "target", Team::Player),
                ('L', ALLY_ID, "target", enemy()),
                ('S', SECOND_ALLY_ID, "target", enemy()),
            ],
        );
        wound(&mut game, ALLY_ID, 95);
        wound(&mut game, SECOND_ALLY_ID, 20);

        let light = game.entity(ALLY_ID).expect("ASCII 的 L 應建立輕傷隊友");
        let light_x = game.world.get::<Pos>(light).expect("輕傷隊友應有位置").0.x;
        let severe = game
            .entity(SECOND_ALLY_ID)
            .expect("ASCII 的 S 應建立重傷隊友");
        let severe_x = game.world.get::<Pos>(severe).expect("重傷隊友應有位置").0.x;
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
            (i64::from(severe_x) - actual_x).abs() > 3,
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
            "....LAS.T...",
            &[
                ('A', ACTOR_ID, "healer", enemy()),
                ('T', TARGET_ID, "target", Team::Player),
                ('L', ALLY_ID, "low_hp_ally", enemy()),
                ('S', SECOND_ALLY_ID, "high_hp_ally", enemy()),
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
    let mut game = duel(&definitions(), "healer", "..A..T......");
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
            ".LA..T......",
            &[
                ('A', ACTOR_ID, "healer", enemy()),
                ('T', TARGET_ID, "target", Team::Player),
                ('L', ALLY_ID, "target", enemy()),
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
        let mut game = duel(&definitions, "archer", "..A..T......");
        let target = game.entity(TARGET_ID).expect("ASCII 的 T 應建立目標單位");
        let target_x = game.world.get::<Pos>(target).expect("目標應有位置").0.x;
        let snapshot = act(&mut game);
        assert_eq!(
            (actor_x(&snapshot) - i64::from(target_x)).abs(),
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
        let mut map = ascii_map(
            "utility_ai_test",
            "..T..A..L...",
            &[
                ('A', ACTOR_ID, "archer", enemy()),
                ('T', TARGET_ID, "target", Team::Player),
                ('L', ALLY_ID, "target", Team::Player),
            ],
        );
        if reverse {
            map.units.reverse();
        }
        let mut game = game_on_map(&definitions(), map);
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

// 驗證尖刺後重新規劃仍追擊較遠的上回合目標；未設定追擊時改攻擊較近的敵人。
#[test]
fn pursuit_survives_replanning_after_spikes() {
    const PURSUED_TARGET_ID: i64 = 2;
    const ALTERNATIVE_TARGET_ID: i64 = 3;
    for (pursuit, expected_target) in [(0, ALTERNATIVE_TARGET_ID), (100, PURSUED_TARGET_ID)] {
        let mut definitions = definitions();
        profile_mut(&mut definitions, "archer")["pursuit_weight"] = pursuit.into();
        profile_mut(&mut definitions, "archer")["distance_preference"] = "near".into();
        unit_type_mut(&mut definitions, "archer")["movement"] = 3.into();
        let shot = definitions["skills"]
            .as_array_mut()
            .expect("測試定義應包含技能")
            .iter_mut()
            .find(|skill| skill["id"].as_str() == Some("shot"))
            .expect("測試定義應包含射擊技能");
        shot["max_range"] = 3.into();
        let mut game = game(
            &definitions,
            "
            A^T....
            ####L##
            ",
            &[
                ('A', ACTOR_ID, "archer", enemy()),
                ('T', PURSUED_TARGET_ID, "target", Team::Player),
                ('L', ALTERNATIVE_TARGET_ID, "target", Team::Player),
            ],
        );
        // 第一回合 T 距離兩格，已在偏好的最近射程；L 距離五格，移動會被尖刺截斷，無法攻擊。
        let first_snapshot = act(&mut game);
        assert_eq!(action(&first_snapshot), Some(("shot", PURSUED_TARGET_ID)));
        assert_eq!(actor_x(&first_snapshot), 0, "第一回合應原地攻擊");
        assert_eq!(first_snapshot["movements"], serde_json::json!([]));
        let actor = game.entity(ACTOR_ID).expect("AI 應仍存在");
        let pursued = game.entity(PURSUED_TARGET_ID).expect("追擊目標應仍存在");
        // 第二回合前 T 後退四格：T 距離六格、L 距離五格，踩尖刺停在 x=1 時仍都無法攻擊。
        game.world
            .get_mut::<Pos>(pursued)
            .expect("追擊目標應有位置")
            .0 = GridPos { x: 6, y: 0 };
        wound(&mut game, PURSUED_TARGET_ID, 100);
        *game.world.resource_mut::<Turn>() = Turn {
            actor: Some(ACTOR_ID),
            phase: Phase::Ready,
            movement_remaining: 3,
            movement_segments_used: 0,
        };
        for target_position in [GridPos { x: 6, y: 0 }, GridPos { x: 4, y: 1 }] {
            let error = game
                .preview_skill(ACTOR_ID, target_position, "shot")
                .err()
                .expect("第二回合移動前，兩個目標都應在射程外");
            assert_eq!(error.id(), "target_too_far");
        }
        let snapshot = game
            .command(Command::Continue)
            .expect("AI 應可在尖刺中斷後完成行動");
        let snapshot = serde_json::to_value(snapshot).expect("快照應可序列化");
        assert_eq!(
            actor_x(&snapshot),
            3,
            "追擊權重 {pursuit}：應在尖刺後繼續移動"
        );
        assert_eq!(game.world.get::<Hp>(actor).expect("AI 應有 HP").current, 97);
        let paths: Vec<Vec<GridPos>> = snapshot["movements"]
            .as_array()
            .expect("快照應包含移動紀錄")
            .iter()
            .map(|movement| {
                serde_json::from_value(movement["path"].clone()).expect("移動路徑應有效")
            })
            .collect();
        assert_eq!(
            paths,
            vec![
                vec![GridPos { x: 0, y: 0 }, GridPos { x: 1, y: 0 }],
                vec![
                    GridPos { x: 1, y: 0 },
                    GridPos { x: 2, y: 0 },
                    GridPos { x: 3, y: 0 }
                ],
            ],
            "應先在尖刺停下，再重新規劃並繼續移動"
        );
        assert_eq!(
            action(&snapshot),
            Some(("shot", expected_target)),
            "追擊權重 {pursuit}：無追擊時選兩格外的 L，有追擊時仍選三格外的 T"
        );
    }
}

// 驗證同分候選在相同亂數種子下穩定選擇，不受單位配置載入順序影響。
#[test]
fn equal_utility_has_a_stable_target_choice() {
    let mut choices = Vec::new();
    for reverse in [false, true, false] {
        let mut map = ascii_map(
            "utility_ai_test",
            "..T..A..L...",
            &[
                ('A', ACTOR_ID, "archer", enemy()),
                ('T', TARGET_ID, "target", Team::Player),
                ('L', ALLY_ID, "target", Team::Player),
            ],
        );
        if reverse {
            map.units.reverse();
        }
        let mut game = game_on_map(&definitions(), map);
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
        let mut game = duel(&definitions, "archer", "..A..T......");
        let target = game.entity(TARGET_ID).expect("ASCII 的 T 應建立目標單位");
        let target_x = game.world.get::<Pos>(target).expect("目標應有位置").0.x;
        let snapshot = act(&mut game);
        assert_eq!(
            (actor_x(&snapshot) - i64::from(target_x)).abs(),
            expected_distance,
            "{preference}, {min_range}..{max_range}"
        );
        assert_eq!(action(&snapshot), Some(("shot", TARGET_ID)));
    }
}

// 驗證第一段內可攻擊就施放，否則利用兩段接近射程但不施放，且沒有攻擊技能不追敵。
#[test]
fn fallback_uses_available_skill_ranges() {
    for (skills, diagram, expected_x, expected_action) in [
        (vec!["shot"], "..A....T....", 3, Some(("shot", TARGET_ID))),
        (vec!["shot"], "..A......T..", 5, None),
        (vec!["shot"], "..A.......T.", 6, None),
        (vec!["shot"], "..A........T", 6, None),
        (vec!["heal"], "..A.......T.", 2, None),
    ] {
        let mut definitions = definitions();
        unit_type_mut(&mut definitions, "archer")["skills"] = toml::Value::Array(
            skills
                .iter()
                .map(|skill| toml::Value::String((*skill).into()))
                .collect(),
        );
        let mut game = duel(&definitions, "archer", diagram);
        let snapshot = act(&mut game);
        assert_eq!(actor_x(&snapshot), expected_x, "{skills:?}, 地圖 {diagram}");
        assert_eq!(
            action(&snapshot),
            expected_action,
            "{skills:?}, 地圖 {diagram}"
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
            "..A.L.....T.",
            &[
                ('A', ACTOR_ID, "healer", enemy()),
                ('T', TARGET_ID, "target", Team::Player),
                ('L', ALLY_ID, "target", enemy()),
            ],
        );
        wound(&mut game, ALLY_ID, 20);
        let snapshot = act(&mut game);
        assert_eq!(actor_x(&snapshot), expected_x, "{preference}");
        assert_eq!(action(&snapshot), Some(("heal", ALLY_ID)));
    }
}

struct RouteCase {
    unit_type: &'static str,
    name: &'static str,
    diagram: &'static str,
    movement: u32,
    end: (i32, i32),
    segments_used: u8,
    remaining: u32,
    hp: i32,
    attacks: bool,
    avoids_terrain: bool,
}

// 驗證完整佔用範圍與地形成本影響路徑及兩段預算，經過尖刺後同回合繼續移動。
#[test]
fn ai_routes_respect_footprints_hazards_and_movement_costs() {
    let cases = [
        // 驗證大型單位從牆下方繞路，目標下側被牆封住，只能在左側接敵。
        RouteCase {
            unit_type: "tank",
            name: "大型單位繞過阻擋兩排的牆",
            diagram: "
                AA.#..T.
                AA.#..~.
                ........
                ........
            ",
            movement: 8,
            end: (4, 0),
            segments_used: 1,
            remaining: 0,
            hp: 100,
            attacks: true,
            avoids_terrain: true,
        },
        RouteCase {
            unit_type: "tank",
            name: "大型單位繞過阻擋兩排的牆",
            diagram: "
                ........
                ........
                AA.#..~.
                AA.#..T.
            ",
            movement: 8,
            end: (4, 2),
            segments_used: 1,
            remaining: 0,
            hp: 100,
            attacks: true,
            avoids_terrain: true,
        },
        RouteCase {
            unit_type: "tank",
            name: "大型單位繞過阻擋兩排的牆",
            diagram: "
                ........
                ........
                AA.#~...
                AA.#..T.
            ",
            movement: 8,
            end: (5, 1),
            segments_used: 1,
            remaining: 0,
            hp: 100,
            attacks: true,
            avoids_terrain: true,
        },
        // 驗證只有近戰技能的大型單位無法穿過單格通道時，不向無法接敵的牆邊移動。
        RouteCase {
            unit_type: "tank",
            name: "大型單位不能穿過一格高的通道",
            diagram: "
                AA.#..T.
                AA......
                ...#....
                ...#....
            ",
            movement: 8,
            end: (0, 0),
            segments_used: 0,
            remaining: 8,
            hp: 100,
            attacks: false,
            avoids_terrain: true,
        },
        RouteCase {
            unit_type: "tank",
            name: "大型單位可以貼近牆壁攻擊",
            diagram: "
                AA.#....
                AA.T....
                ...#....
                ...#....
            ",
            movement: 8,
            end: (1, 0),
            segments_used: 0,
            remaining: 7,
            hp: 100,
            attacks: true,
            avoids_terrain: true,
        },
        // 驗證近戰無法接敵但遠距射程足夠時，本回合先用兩段移動靠近，即使尚不能攻擊。
        RouteCase {
            unit_type: "ranged_tank",
            name: "遠距射程足夠時先靠近",
            diagram: "
                AA....#.T..
                AA.........
            ",
            movement: 1,
            end: (2, 0),
            segments_used: 2,
            remaining: 0,
            hp: 100,
            attacks: false,
            avoids_terrain: true,
        },
        // 驗證近戰無法接敵且遠距射程不足時，本回合不向牆邊移動。
        RouteCase {
            unit_type: "ranged_tank",
            name: "有遠距技能但射程不足時不動",
            diagram: "
                AA....#..T.
                AA.........
            ",
            movement: 1,
            end: (0, 0),
            segments_used: 0,
            remaining: 1,
            hp: 100,
            attacks: false,
            avoids_terrain: true,
        },
        // 驗證五步額度恰好抵達大型目標左側，依佔用邊緣進行近戰。
        RouteCase {
            unit_type: "tank",
            name: "依大型目標的近側邊緣進入近戰射程",
            diagram: "
                ........
                A.....TT
                ......TT
                ........
            ",
            movement: 5,
            end: (5, 1),
            segments_used: 1,
            remaining: 0,
            hp: 100,
            attacks: true,
            avoids_terrain: true,
        },
        // 驗證左側與下側被牆封住時，第一段內安全繞過尖刺至目標上方並攻擊。
        RouteCase {
            unit_type: "tank",
            name: "第一段內安全繞過尖刺後攻擊",
            diagram: "
                ...~.
                A^..T
                ...~.
            ",
            movement: 5,
            end: (3, 1),
            segments_used: 1,
            remaining: 0,
            hp: 100,
            attacks: true,
            avoids_terrain: true,
        },
        // 驗證目標左側被牆封住時，用第二段安全繞到下側，且本回合不能攻擊。
        RouteCase {
            unit_type: "tank",
            name: "第一段不足時用第二段安全繞到近戰射程",
            diagram: "
                A^..T
                ....#
            ",
            movement: 3,
            end: (3, 0),
            segments_used: 1,
            remaining: 1,
            hp: 100,
            attacks: false,
            avoids_terrain: true,
        },
        // 驗證單列棋盤無法繞路，經過尖刺受傷後，同回合繼續接敵。
        RouteCase {
            unit_type: "tank",
            name: "無法繞路時經過尖刺後同回合繼續接敵",
            diagram: "
                A^..T
            ",
            movement: 2,
            end: (3, 0),
            segments_used: 1,
            remaining: 1,
            hp: 97,
            attacks: false,
            avoids_terrain: false,
        },
        // 驗證大型單位連續兩步覆蓋尖刺時各扣血，同回合用完兩段額度繼續走。
        RouteCase {
            unit_type: "tank",
            name: "大型單位側邊經過尖刺後同回合繼續走",
            diagram: "
                AA....T.
                AA^.....
            ",
            movement: 2,
            end: (4, 0),
            segments_used: 2,
            remaining: 0,
            hp: 94,
            attacks: false,
            avoids_terrain: false,
        },
        // 驗證下側封牆、左側為粗糙地面時，上側繞路成本 5，小於左側接敵成本 7。
        RouteCase {
            unit_type: "tank",
            name: "繞路比兩格粗糙地面便宜且保留攻擊",
            diagram: "
                .....
                A.~~T
                ....#
            ",
            movement: 5,
            end: (4, 0),
            segments_used: 1,
            remaining: 0,
            hp: 100,
            attacks: true,
            avoids_terrain: true,
        },
        RouteCase {
            unit_type: "tank",
            name: "繞路比兩格粗糙地面便宜且保留攻擊",
            diagram: "
                ....#
                A.~~T
                .....
            ",
            movement: 5,
            end: (4, 2),
            segments_used: 1,
            remaining: 0,
            hp: 100,
            attacks: true,
            avoids_terrain: true,
        },
        // 驗證單列棋盤中粗糙地面成本為三，兩段四點額度恰好走到第二格。
        RouteCase {
            unit_type: "tank",
            name: "粗糙地面超過第一段但可用第二段進入",
            diagram: "
                A~....T
            ",
            movement: 2,
            end: (2, 0),
            segments_used: 2,
            remaining: 0,
            hp: 100,
            attacks: false,
            avoids_terrain: false,
        },
        // 驗證兩格高的單位側邊踩粗糙地面，四點額度只足以向右移動一格。
        RouteCase {
            unit_type: "tank",
            name: "大型單位側邊的粗糙地面也扣除移動成本",
            diagram: "
                AA....T.
                AA~.....
            ",
            movement: 2,
            end: (1, 0),
            segments_used: 1,
            remaining: 1,
            hp: 100,
            attacks: false,
            avoids_terrain: false,
        },
    ];
    for RouteCase {
        unit_type,
        name,
        diagram,
        movement,
        end,
        segments_used,
        remaining,
        hp,
        attacks,
        avoids_terrain,
    } in cases
    {
        let AsciiBoard {
            board,
            markers,
            terrains,
        } = ascii_board(diagram);
        let (start, actor_size) = *markers.get(&'A').expect("棋盤應包含 AI");
        let (target, target_size) = *markers.get(&'T').expect("棋盤應包含目標");
        let mut definitions = definitions();
        for (kind, size) in [(unit_type, actor_size), ("target", target_size)] {
            let unit = unit_type_mut(&mut definitions, kind);
            let table = unit.as_table_mut().expect("測試單位種類應為 table");
            table.insert("width".into(), size.0.into());
            table.insert("height".into(), size.1.into());
        }
        unit_type_mut(&mut definitions, unit_type)["movement"] = i64::from(movement).into();
        let mut actor = placement(ACTOR_ID, unit_type, enemy(), start.0);
        actor.y = start.1;
        let mut target_unit = placement(TARGET_ID, "target", Team::Player, target.0);
        target_unit.y = target.1;
        let mut game = game_on_map(
            &definitions,
            authoring::Map {
                name: name.into(),
                width: board.0,
                height: board.1,
                terrains: terrains.clone(),
                units: vec![actor, target_unit],
            },
        );
        let snapshot = act(&mut game);
        let entity = game.entity(ACTOR_ID).expect("路徑測試 AI 應仍存活");
        assert_eq!(
            game.world.get::<Pos>(entity).expect("AI 應有位置").0,
            GridPos { x: end.0, y: end.1 },
            "{name}：實際停點"
        );
        assert_eq!(
            action(&snapshot),
            attacks.then_some(("melee", TARGET_ID)),
            "{name}：後續攻擊"
        );
        assert_eq!(
            game.world.get::<Hp>(entity).expect("AI 應有 HP").current,
            hp,
            "{name}：地形傷害"
        );
        let Turn {
            actor: _,
            phase,
            movement_remaining,
            movement_segments_used,
        } = game.world.resource::<Turn>();
        assert_eq!(
            (*movement_segments_used, *movement_remaining),
            (segments_used, remaining),
            "{name}：實際移動必須依地形成本扣除兩段預算"
        );
        assert_eq!(*phase, Phase::Ended, "{name}：AI 完成行動後結束回合");
        let movements = snapshot["movements"]
            .as_array()
            .expect("快照應包含移動紀錄");
        let actual_path: Vec<GridPos> = if end == start {
            assert!(movements.is_empty(), "{name}：不可產生移動紀錄");
            Vec::new()
        } else {
            let mut path = Vec::new();
            for movement in movements {
                assert_eq!(movement["unit_id"], ACTOR_ID, "{name}：移動者應為 AI");
                let segment: Vec<GridPos> = serde_json::from_value(movement["path"].clone())
                    .expect("AI 移動紀錄應包含有效路徑");
                if path.is_empty() {
                    path.extend(segment);
                } else {
                    assert_eq!(segment.first(), path.last(), "{name}：移動路徑應連續");
                    path.extend(segment.into_iter().skip(1));
                }
            }
            assert_eq!(
                path.first(),
                Some(&GridPos {
                    x: start.0,
                    y: start.1
                }),
                "{name}：實際路徑應從起點開始"
            );
            assert_eq!(
                path.last(),
                Some(&GridPos { x: end.0, y: end.1 }),
                "{name}：實際路徑應抵達停點"
            );
            path
        };
        for step in actual_path.windows(2) {
            assert_eq!(
                (step[1].x - step[0].x).abs() + (step[1].y - step[0].y).abs(),
                1,
                "{name}：每步只能移動到相鄰格"
            );
        }
        for position in &actual_path {
            assert!(
                position.x >= 0
                    && position.y >= 0
                    && position.x + actor_size.0 <= board.0
                    && position.y + actor_size.1 <= board.1,
                "{name}：完整佔用範圍不可超出棋盤"
            );
            if avoids_terrain {
                assert!(
                    !terrains.iter().any(|TerrainPlacement { x, y, kind: _ }| {
                        let (x, y) = (*x, *y);
                        x >= position.x
                            && x < position.x + actor_size.0
                            && y >= position.y
                            && y < position.y + actor_size.1
                    }),
                    "{name}：完整佔用範圍不可進入需避開的地形"
                );
            }
        }
    }
}

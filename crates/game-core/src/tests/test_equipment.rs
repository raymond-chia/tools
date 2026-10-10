use super::support::{ACTOR_ID, BLOCKER_ID, TARGET_ID};
use crate::*;
use authoring::{Definitions, Map, UnitType};
use serde_json::{Value, json};

const DEFINITIONS: &str = include_str!("data/equipment_definitions.toml");
const MAP: &str = include_str!("data/equipment_map.toml");

fn definitions() -> Definitions {
    toml::from_str(DEFINITIONS).expect("裝備專用測試定義應為有效 TOML")
}

fn map() -> Map {
    toml::from_str(MAP).expect("裝備專用測試地圖應為有效 TOML")
}

fn unit_type_mut<'a>(definitions: &'a mut Definitions, id: &str) -> &'a mut UnitType {
    definitions
        .unit_types
        .iter_mut()
        .find(|unit| unit.id == id)
        .expect("測試單位種類應存在")
}

fn game(definitions: Definitions) -> Game {
    Game::from_authoring(definitions, map()).expect("裝備專用測試戰鬥應可載入")
}

fn snapshot(game: &Game) -> Value {
    serde_json::to_value(game.snapshot(None)).expect("裝備快照應可序列化")
}

fn unit(snapshot: &Value, id: i64) -> &Value {
    snapshot["units"]
        .as_array()
        .expect("快照應有單位清單")
        .iter()
        .find(|unit| unit["id"] == id)
        .expect("測試單位應存在於快照")
}

fn preview(game: &Game, skill: &str, cell: GridPos) -> Value {
    serde_json::to_value(
        game.preview_skill(ACTOR_ID, cell, skill)
            .expect("測試技能應可預覽"),
    )
    .expect("技能預覽應可序列化")
}

// 驗證四個裝備欄位能力加總、雙手武器配裝與無裝備能力，輕重甲均不增加閃避。
#[test]
fn equipment_stats_are_resolved_in_the_snapshot() {
    let cases = [
        // 劍、盾、重甲與飾品各自提供能力，合併後顯示完整配裝。
        (
            "劍盾重甲飾品",
            ["sword", "shield", "heavy_armor", "power_ring"],
            [16, 5, 3, 5, 3],
        ),
        // 輕甲僅加血量，雙手弓占用兩隻手而副手保持空白。
        ("弓與輕甲", ["bow", "", "light_armor", ""], [13, 7, 1, 0, 0]),
        // 單獨重甲也能提供格擋及減傷。
        ("只有重甲", ["", "", "heavy_armor", ""], [15, 3, 1, 2, 1]),
        // 沒有裝備時保留單位能力，格擋與減傷歸零。
        ("無裝備", ["", "", "", ""], [10, 3, 1, 0, 0]),
    ];
    for (name, loadout, [hp, physical, magical, block, reduction]) in cases {
        let mut definitions = definitions();
        let actor = unit_type_mut(&mut definitions, "fighter");
        actor.main_hand = loadout[0].into();
        actor.off_hand = loadout[1].into();
        actor.armor = loadout[2].into();
        actor.accessory = loadout[3].into();
        let snapshot = snapshot(&game(definitions));
        let actor = unit(&snapshot, ACTOR_ID);
        assert_eq!(actor["hp"], hp, "{name}");
        assert_eq!(actor["max_hp"], hp, "{name}");
        assert_eq!(actor["physical_power"], physical, "{name}");
        assert_eq!(actor["magical_power"], magical, "{name}");
        assert_eq!(actor["block"], block, "{name}");
        assert_eq!(actor["block_reduction"], reduction, "{name}");
        assert_eq!(actor["dodge"], 2, "{name}：裝備不改變閃避");
        assert_eq!(
            actor["equipment"],
            json!({"main_hand": loadout[0], "off_hand": loadout[1], "armor": loadout[2], "accessory": loadout[3]}),
            "{name}"
        );
    }
}

// 驗證未知引用、錯誤欄位、超過兩手、負值與能力溢位都在作者輸入邊界拒絕。
#[test]
fn invalid_equipment_is_rejected_on_load() {
    let cases: &[(&str, fn(&mut Definitions), &str)] = &[
        // 配裝不可引用不存在的裝備。
        (
            "未知裝備",
            |d| unit_type_mut(d, "fighter").main_hand = "missing".into(),
            "invalid_equipment",
        ),
        // 護具不可放進手持欄位。
        (
            "錯誤欄位",
            |d| unit_type_mut(d, "fighter").main_hand = "light_armor".into(),
            "invalid_equipment",
        ),
        // 弓與盾占用三手，不能同時裝備。
        (
            "弓盾衝突",
            |d| unit_type_mut(d, "fighter").main_hand = "bow".into(),
            "invalid_equipment",
        ),
        // 兩件雙手武器不能同時裝備。
        (
            "兩把弓",
            |d| {
                let unit = unit_type_mut(d, "fighter");
                unit.main_hand = "bow".into();
                unit.off_hand = "bow".into();
            },
            "invalid_equipment",
        ),
        // 裝備 ID 必須非空白且不重複。
        (
            "空白 ID",
            |d| d.equipment[0].id = " ".into(),
            "invalid_equipment",
        ),
        (
            "重複 ID",
            |d| d.equipment.push(d.equipment[0].clone()),
            "invalid_equipment",
        ),
        // 各項能力加值都不可為負。
        ("負血量", |d| d.equipment[0].hp = -1, "invalid_equipment"),
        (
            "負物理",
            |d| d.equipment[0].physical_power = -1,
            "invalid_equipment",
        ),
        (
            "負魔法",
            |d| d.equipment[0].magical_power = -1,
            "invalid_equipment",
        ),
        ("負格擋", |d| d.equipment[0].block = -1, "invalid_equipment"),
        (
            "負減傷",
            |d| d.equipment[0].block_reduction = -1,
            "invalid_equipment",
        ),
        // 裝備加總後也必須符合核心數值範圍。
        (
            "血量溢位",
            |d| d.equipment[0].hp = i32::MAX,
            "numeric_range",
        ),
        (
            "物理溢位",
            |d| d.equipment[0].physical_power = i32::MAX,
            "numeric_range",
        ),
        (
            "魔法溢位",
            |d| d.equipment[0].magical_power = i32::MAX,
            "numeric_range",
        ),
        (
            "格擋溢位",
            |d| d.equipment[0].block = i32::MAX,
            "numeric_range",
        ),
        (
            "減傷溢位",
            |d| d.equipment[0].block_reduction = i32::MAX,
            "numeric_range",
        ),
    ];
    for (name, change, expected) in cases {
        let mut definitions = definitions();
        change(&mut definitions);
        let error = match Game::from_authoring(definitions, map()) {
            Ok(_) => panic!("{name} 應拒絕載入"),
            Err(error) => error,
        };
        assert_eq!(error.id(), *expected, "{name}");
    }
}

// 驗證相同近距離下戰士與法師依技能指定的物理／魔法威力計算，預覽與實際傷害或治療一致。
#[test]
fn skill_power_sources_apply_to_preview_and_resolution() {
    let cases = [
        // 戰士近戰使用物理能力與劍的加值。
        (
            "戰士近戰",
            "fighter",
            "melee",
            TARGET_ID,
            GridPos { x: 2, y: 1 },
            "hit_damage",
            "damage",
            5,
        ),
        // 法師使用相同近戰技能時不借用較高魔法能力。
        (
            "法師近戰",
            "mage",
            "melee",
            TARGET_ID,
            GridPos { x: 2, y: 1 },
            "hit_damage",
            "damage",
            1,
        ),
        // 貼身魔法攻擊仍使用魔法能力與魔法裝備加值。
        (
            "戰士魔法",
            "fighter",
            "spell",
            TARGET_ID,
            GridPos { x: 2, y: 1 },
            "hit_damage",
            "damage",
            3,
        ),
        (
            "法師魔法",
            "mage",
            "spell",
            TARGET_ID,
            GridPos { x: 2, y: 1 },
            "hit_damage",
            "damage",
            11,
        ),
        // 推擊的技能加值使用物理能力。
        (
            "戰士推擊",
            "fighter",
            "push",
            TARGET_ID,
            GridPos { x: 2, y: 1 },
            "hit_damage",
            "damage",
            6,
        ),
        // 治療使用魔法能力，並加上治療技能的加值。
        (
            "戰士治療",
            "fighter",
            "heal",
            BLOCKER_ID,
            GridPos { x: 1, y: 2 },
            "healing",
            "healing",
            5,
        ),
        (
            "法師治療",
            "mage",
            "heal",
            BLOCKER_ID,
            GridPos { x: 1, y: 2 },
            "healing",
            "healing",
            13,
        ),
    ];
    for (name, kind, skill, target, cell, preview_field, log_field, expected) in cases {
        let mut map = map();
        map.units[0].unit_type = kind.into();
        let mut game = Game::from_authoring(definitions(), map).expect("威力測試應可載入");
        game.start().expect("威力測試應可開始");
        game.set_random_seed(1);
        let entity = game.entity(target).expect("測試目標應存在");
        game.world
            .get_mut::<Hp>(entity)
            .expect("目標應有生命值")
            .current = 20;
        assert_eq!(
            preview(&game, skill, cell)[preview_field],
            expected,
            "{name}"
        );
        let snapshot = serde_json::to_value(
            game.command(Command::Skill {
                actor: ACTOR_ID,
                x: cell.x,
                y: cell.y,
                skill: skill.into(),
            })
            .expect("技能應成功結算"),
        )
        .expect("技能快照應可序列化");
        let event = snapshot["log"]
            .as_array()
            .expect("快照應有日誌")
            .iter()
            .find(|event| event["skill"] == skill)
            .expect("技能結算應記錄日誌");
        assert_eq!(event[log_field], expected, "{name}");
        assert_eq!(
            unit(&snapshot, target)["hp"],
            if log_field == "healing" {
                20 + expected
            } else {
                20 - expected
            },
            "{name}"
        );
    }
}

// 驗證裝備減傷與格擋機率、暴擊和完全減傷均符合預覽，無裝備時實際結果不會格擋。
#[test]
fn equipment_block_reduction_matches_preview_and_resolution() {
    let cases = [
        // 無裝備沒有格擋，原本會格擋的骰值變為命中。
        ("無裝備", "", "", 5, "hit", false, 0, 5, 10, 5, 45),
        // 盾牌提供 2 點格擋減傷。
        ("盾牌", "shield", "", 6, "block", false, 15, 3, 6, 3, 47),
        // 盾牌與重甲的減傷加總為 3。
        (
            "盾牌重甲",
            "shield",
            "heavy_armor",
            5,
            "block",
            false,
            25,
            2,
            4,
            2,
            53,
        ),
        // 強格擋仍可擋住自然 20，先扣 4 點再加倍。
        (
            "暴擊格擋",
            "tower_shield",
            "",
            15,
            "block",
            true,
            70,
            1,
            2,
            2,
            48,
        ),
        // 減傷完全吸收傷害時，暴擊也不會產生傷害。
        (
            "完全格擋",
            "tower_shield",
            "heavy_armor",
            15,
            "block",
            true,
            70,
            0,
            0,
            0,
            55,
        ),
    ];
    for (
        name,
        shield,
        armor,
        seed,
        result,
        critical,
        block_chance,
        block_damage,
        critical_block_damage,
        damage,
        remaining_hp,
    ) in cases
    {
        let mut definitions = definitions();
        let target = unit_type_mut(&mut definitions, "target");
        target.off_hand = shield.into();
        target.armor = armor.into();
        let mut game = game(definitions);
        game.start().expect("格擋測試應可開始");
        game.set_random_seed(seed);
        let preview = preview(&game, "melee", GridPos { x: 2, y: 1 });
        assert_eq!(preview["block_chance"], block_chance, "{name}");
        assert_eq!(preview["block_damage"], block_damage, "{name}");
        assert_eq!(
            preview["critical_block_damage"], critical_block_damage,
            "{name}"
        );
        let snapshot = serde_json::to_value(
            game.command(Command::Skill {
                actor: ACTOR_ID,
                x: 2,
                y: 1,
                skill: "melee".into(),
            })
            .expect("格擋測試應結算成功"),
        )
        .expect("快照應可序列化");
        let event = snapshot["log"]
            .as_array()
            .expect("快照應有日誌")
            .iter()
            .find(|event| event["type"] == "skill")
            .expect("技能應產生日誌");
        assert_eq!(event["result"], result, "{name}");
        assert_eq!(event["critical"], critical, "{name}");
        assert_eq!(event["damage"], damage, "{name}");
        assert_eq!(unit(&snapshot, TARGET_ID)["hp"], remaining_hp, "{name}");
    }
}

// 驗證作者文件往返保留裝備種類、配裝與技能威力來源，避免編輯器儲存時遺失規格。
#[test]
fn equipment_authoring_round_trip_preserves_definitions() {
    let json = authoring::definitions_to_json(DEFINITIONS).expect("裝備測試定義應可轉為 JSON");
    let definitions: Definitions = serde_json::from_str(&json).expect("轉換後應符合作者格式");
    let toml = toml::to_string(&definitions).expect("裝備作者資料應可存成 TOML");
    let reloaded = authoring::definitions_to_json(&toml).expect("儲存後裝備資料應可重新載入");
    let before: Value = serde_json::from_str(&json).expect("原始 JSON 應有效");
    let after: Value = serde_json::from_str(&reloaded).expect("重新載入的 JSON 應有效");
    assert_eq!(before, after);
    assert_eq!(before["skills"][1]["power_source"], "magical");
    assert_eq!(before["equipment"][2]["slot"], "two_hand");
}

#[cfg(feature = "editor")]
fn editor_command(
    definitions: &Definitions,
    command: Value,
) -> Result<String, editor::EditorError> {
    let source = serde_json::to_string(definitions).expect("編輯器測試定義應可轉為 JSON");
    editor::edit_definition_from_json(&source, "{}", &command.to_string())
}

#[cfg(feature = "editor")]
fn edited_definitions(result: &str) -> Definitions {
    let result: Value = serde_json::from_str(result).expect("編輯器應回傳有效 JSON");
    serde_json::from_value(result["definitions"].clone()).expect("編輯結果應符合作者格式")
}

// 驗證編輯器配裝選項由核心決定，劍盾時排除弓，裝上弓後另一隻手僅允許空欄位。
#[cfg(feature = "editor")]
#[test]
fn editor_equipment_choices_enforce_slots_and_two_hands() {
    let mut definitions = definitions();
    let result = editor_command(&definitions, json!({"action": "skill_effect_options"}))
        .expect("應可查詢配裝選項");
    let result: Value = serde_json::from_str(&result).expect("配裝選項應為有效 JSON");
    let choices = &result["equipment_choices"]["fighter"];
    assert_eq!(
        choices["main_hand"]
            .as_array()
            .expect("主手選項應為陣列")
            .contains(&json!("bow")),
        false
    );
    assert_eq!(choices["armor"], json!(["", "heavy_armor", "light_armor"]));
    assert_eq!(choices["accessory"], json!(["", "power_ring"]));
    for (key, value) in [("off_hand", ""), ("main_hand", "bow")] {
        let result = editor_command(&definitions, json!({"action": "update_field", "category": "unit_types", "id": "fighter", "key": key, "value": value})).expect("依序卸盾、裝弓應合法");
        definitions = edited_definitions(&result);
    }
    let result = editor_command(&definitions, json!({"action": "skill_effect_options"}))
        .expect("裝弓後應可查詢選項");
    let result: Value = serde_json::from_str(&result).expect("配裝選項應為有效 JSON");
    assert_eq!(
        result["equipment_choices"]["fighter"]["off_hand"],
        json!([""])
    );
    let error = editor_command(&definitions, json!({"action": "update_field", "category": "unit_types", "id": "fighter", "key": "off_hand", "value": "shield"})).expect_err("直接提交弓盾衝突也應拒絕");
    let error: Value = serde_json::from_str(&error.response_json()).expect("錯誤應可序列化");
    assert_eq!(error["error_id"], "invalid_equipment");
}

// 驗證裝備頁可新增、複製、修改、排序與刪除，且被配裝引用的裝備禁止刪除。
#[cfg(feature = "editor")]
#[test]
fn editor_equipment_operations_preserve_references() {
    let mut definitions = definitions();
    let commands = [
        // 建立空白單手裝備，沿用作者資料型別。
        json!({"action": "add", "category": "equipment", "id": "spare"}),
        // 複製裝備保留能力，只變更 ID。
        json!({"action": "duplicate", "category": "equipment", "id": "heavy_copy", "source_id": "heavy_armor"}),
        // 修改未配裝裝備的能力。
        json!({"action": "update_field", "category": "equipment", "id": "spare", "key": "physical_power", "value": 7}),
        // 裝備清單依作者操作調整順序。
        json!({"action": "move", "category": "equipment", "id": "heavy_copy", "offset": -1}),
    ];
    for command in commands {
        let result = editor_command(&definitions, command).expect("合法裝備操作應成功");
        definitions = edited_definitions(&result);
    }
    let entries = serde_json::to_value(&definitions.equipment).expect("裝備應可序列化");
    assert_eq!(
        entries[entries.as_array().expect("裝備應為清單").len() - 2]["id"],
        "heavy_copy"
    );
    let spare = entries
        .as_array()
        .expect("裝備應為清單")
        .iter()
        .find(|entry| entry["id"] == "spare")
        .expect("新增的裝備應存在");
    assert_eq!(spare["slot"], "one_hand");
    assert_eq!(spare["physical_power"], 7);
    let copied = entries
        .as_array()
        .expect("裝備應為清單")
        .iter()
        .find(|entry| entry["id"] == "heavy_copy")
        .expect("複製裝備應存在");
    assert_eq!(copied["hp"], 5);
    assert_eq!(copied["block"], 2);
    assert_eq!(copied["block_reduction"], 1);
    for action in ["check_remove", "remove"] {
        let error = editor_command(
            &definitions,
            json!({"action": action, "category": "equipment", "id": "shield"}),
        )
        .expect_err("被引用裝備不可刪除");
        let error: Value =
            serde_json::from_str(&error.response_json()).expect("引用錯誤應為有效 JSON");
        assert_eq!(error["error_details"]["kind"], "referenced");
        assert_eq!(
            error["error_details"]["references"],
            json!([{"category": "unit_types", "id": "fighter"}])
        );
    }
    let checked = editor_command(
        &definitions,
        json!({"action": "check_remove", "category": "equipment", "id": "heavy_copy"}),
    )
    .expect("未引用裝備應允許刪除");
    let checked: Value = serde_json::from_str(&checked).expect("刪除預檢應為有效 JSON");
    assert_eq!(checked["can_remove"], true);
    let result = editor_command(
        &definitions,
        json!({"action": "remove", "category": "equipment", "id": "heavy_copy"}),
    )
    .expect("未引用裝備應可刪除");
    definitions = edited_definitions(&result);
    assert_eq!(
        definitions
            .equipment
            .iter()
            .any(|entry| entry.id == "heavy_copy"),
        false
    );
    assert_eq!(
        unit(&snapshot(&game(definitions)), ACTOR_ID)["block_reduction"],
        3
    );
}

// 驗證編輯器切換治療／攻擊時設定正確威力來源，儲存後仍保留裝備與可執行資料。
#[cfg(feature = "editor")]
#[test]
fn editor_skill_power_sources_survive_document_saving() {
    for (effect, source) in [("heal", "magical"), ("attack", "physical")] {
        let result = editor_command(
            &definitions(),
            json!({"action": "change_skill_effect", "id": "melee", "effect": effect}),
        )
        .expect("切換技能效果應成功");
        let definitions = edited_definitions(&result);
        let result: Value = serde_json::from_str(&result).expect("編輯結果應為有效 JSON");
        assert_eq!(result["definitions"]["skills"][0]["power_source"], source);
        let definitions_json = serde_json::to_string(&definitions).expect("定義應可序列化");
        let map_json = serde_json::to_string(&map()).expect("地圖應可序列化");
        let (definitions_toml, map_toml) =
            editor::documents_from_json(&definitions_json, &map_json).expect("編輯器文件應可儲存");
        let reloaded =
            Game::from_documents(&definitions_toml, &map_toml).expect("儲存文件應可執行");
        assert_eq!(
            unit(&snapshot(&reloaded), ACTOR_ID)["equipment"],
            json!({"main_hand": "sword", "off_hand": "shield", "armor": "heavy_armor", "accessory": "power_ring"})
        );
        assert_eq!(unit(&snapshot(&reloaded), ACTOR_ID)["magical_power"], 3);
    }
}

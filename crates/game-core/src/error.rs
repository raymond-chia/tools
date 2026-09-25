//! 核心錯誤的固定 ID 與詳細訊息。
//! 調整錯誤 ID 時，須同步檢查 Godot 翻譯與戰鬥預覽的忽略清單。

#[derive(Debug)]
pub struct GameError {
    id: &'static str,
    message: String,
}

impl GameError {
    fn new(id: &'static str, message: String) -> Self {
        Self { id, message }
    }

    pub fn id(&self) -> &'static str {
        self.id
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

pub(crate) fn empty_map_name() -> GameError {
    GameError::new("empty_map_name", "地圖名稱不可為空".to_owned())
}

pub(crate) fn empty_enemy_faction() -> GameError {
    GameError::new("empty_enemy_faction", "敵方派系名稱不可為空".to_owned())
}

pub(crate) fn duplicate_unit_type_id() -> GameError {
    GameError::new("duplicate_unit_type_id", "單位定義 id 重複".to_owned())
}

pub(crate) fn empty_unit_placement_id() -> GameError {
    GameError::new("empty_unit_placement_id", "單位配置 id 不可為空".to_owned())
}

pub(crate) fn invalid_map_dimensions() -> GameError {
    GameError::new("invalid_map_dimensions", "地圖尺寸必須為正數".to_owned())
}

pub(crate) fn terrain_out_of_bounds() -> GameError {
    GameError::new("terrain_out_of_bounds", "terrain 超出地圖範圍".to_owned())
}

pub(crate) fn duplicate_ai_default() -> GameError {
    GameError::new(
        "duplicate_ai_default",
        "只能有一個 ai_default 技能".to_owned(),
    )
}

pub(crate) fn duplicate_skill_id() -> GameError {
    GameError::new("duplicate_skill_id", "duplicate skill id".to_owned())
}

pub(crate) fn missing_ai_default() -> GameError {
    GameError::new("missing_ai_default", "缺少 ai_default 技能".to_owned())
}

pub(crate) fn no_auto_step() -> GameError {
    GameError::new("no_auto_step", "目前沒有可推進的自動回合".to_owned())
}

pub(crate) fn delay_after_action() -> GameError {
    GameError::new("delay_after_action", "開始行動後不能延後".to_owned())
}

pub(crate) fn missing_delay_target() -> GameError {
    GameError::new("missing_delay_target", "找不到延後目標".to_owned())
}

pub(crate) fn invalid_delay_target() -> GameError {
    GameError::new(
        "invalid_delay_target",
        "只能延後到尚未行動的單位之後".to_owned(),
    )
}

pub(crate) fn wrong_turn() -> GameError {
    GameError::new("wrong_turn", "不是該單位的回合".to_owned())
}

pub(crate) fn missing_initiative_unit() -> GameError {
    GameError::new("missing_initiative_unit", "先攻單位不存在".to_owned())
}

pub(crate) fn missing_move_unit() -> GameError {
    GameError::new("missing_move_unit", "找不到移動單位".to_owned())
}

pub(crate) fn cannot_move() -> GameError {
    GameError::new("cannot_move", "目前不能移動".to_owned())
}

pub(crate) fn unreachable_destination() -> GameError {
    GameError::new("unreachable_destination", "目的地不可達".to_owned())
}

pub(crate) fn cannot_use_skill() -> GameError {
    GameError::new("cannot_use_skill", "目前不能使用技能".to_owned())
}

pub(crate) fn missing_attacker() -> GameError {
    GameError::new("missing_attacker", "找不到攻擊者".to_owned())
}

pub(crate) fn missing_target() -> GameError {
    GameError::new("missing_target", "找不到目標".to_owned())
}

pub(crate) fn missing_actor() -> GameError {
    GameError::new("missing_actor", "找不到行動角色".to_owned())
}

pub(crate) fn unit_cannot_use_skill() -> GameError {
    GameError::new("unit_cannot_use_skill", "此角色不能使用這個技能".to_owned())
}

pub(crate) fn target_cell_out_of_bounds() -> GameError {
    GameError::new("target_cell_out_of_bounds", "目標格超出地圖".to_owned())
}

pub(crate) fn target_too_close() -> GameError {
    GameError::new("target_too_close", "目標距離太近".to_owned())
}

pub(crate) fn target_too_far() -> GameError {
    GameError::new("target_too_far", "目標距離太遠".to_owned())
}

pub(crate) fn target_downed() -> GameError {
    GameError::new("target_downed", "目標已倒下".to_owned())
}

pub(crate) fn cell_not_on_target() -> GameError {
    GameError::new("cell_not_on_target", "所選格不屬於目標".to_owned())
}

pub(crate) fn heal_allies_only() -> GameError {
    GameError::new("heal_allies_only", "只能治療自己或友軍".to_owned())
}

pub(crate) fn cannot_attack_ally() -> GameError {
    GameError::new("cannot_attack_ally", "不能攻擊友軍".to_owned())
}

pub(crate) fn definitions_toml_parse(message: String) -> GameError {
    GameError::new("definitions_toml_parse", message)
}

pub(crate) fn map_toml_parse(message: String) -> GameError {
    GameError::new("map_toml_parse", message)
}

pub(crate) fn definitions_json_parse(message: String) -> GameError {
    GameError::new("definitions_json_parse", message)
}

pub(crate) fn map_json_parse(message: String) -> GameError {
    GameError::new("map_json_parse", message)
}

pub(crate) fn json_serialize(message: String) -> GameError {
    GameError::new("json_serialize", message)
}

pub(crate) fn toml_serialize(message: String) -> GameError {
    GameError::new("toml_serialize", message)
}

pub(crate) fn invalid_unit_type(id: &str) -> GameError {
    GameError::new(
        "invalid_unit_type",
        format!("{id} {}", "的 id、HP 與佔用尺寸無效"),
    )
}

pub(crate) fn unknown_unit_skill(id: &str, skill: &str) -> GameError {
    GameError::new(
        "unknown_unit_skill",
        format!("{id} {} {skill}", "使用未知技能"),
    )
}

pub(crate) fn duplicate_unit_placement_id(id: &str) -> GameError {
    GameError::new(
        "duplicate_unit_placement_id",
        format!("{}{id}", "單位配置 id 重複："),
    )
}

pub(crate) fn unknown_unit_type(id: &str) -> GameError {
    GameError::new("unknown_unit_type", format!("{} {id}", "找不到單位定義"))
}

pub(crate) fn unknown_terrain_type(kind: &str) -> GameError {
    GameError::new(
        "unknown_terrain_type",
        format!("{} {kind}", "找不到地形種類"),
    )
}

pub(crate) fn duplicate_terrain(x: i32, y: i32) -> GameError {
    GameError::new(
        "duplicate_terrain",
        format!("{} ({x}, {y})", "同一格的 terrain 重複"),
    )
}

pub(crate) fn invalid_skill_range(id: &str) -> GameError {
    GameError::new(
        "invalid_skill_range",
        format!("{id} {}", "的距離必須符合 0 ≤ min_range ≤ max_range"),
    )
}

pub(crate) fn invalid_heal_amount(id: &str) -> GameError {
    GameError::new(
        "invalid_heal_amount",
        format!("{id} {}", "的 heal_amount 必須大於 0"),
    )
}

pub(crate) fn invalid_duration(id: &str) -> GameError {
    GameError::new(
        "invalid_duration",
        format!("{id} {}", "的 duration 必須大於 0"),
    )
}

pub(crate) fn invalid_skill_terrain(id: &str) -> GameError {
    GameError::new(
        "invalid_skill_terrain",
        format!("{id} {}", "必須指定已定義的 terrain"),
    )
}

pub(crate) fn duplicate_unit_id(id: &str) -> GameError {
    GameError::new("duplicate_unit_id", format!("{} {id}", "重複 id"))
}

pub(crate) fn invalid_unit_size_or_hp(id: &str) -> GameError {
    GameError::new(
        "invalid_unit_size_or_hp",
        format!("{id} {}", "的佔用尺寸與 HP 必須大於 0"),
    )
}

pub(crate) fn unit_out_of_bounds(id: &str) -> GameError {
    GameError::new("unit_out_of_bounds", format!("{id} {}", "超出地圖"))
}

pub(crate) fn unit_on_impassable(id: &str) -> GameError {
    GameError::new(
        "unit_on_impassable",
        format!("{id} {}", "不可放置在峭壁或懸崖"),
    )
}

pub(crate) fn overlapping_unit(id: &str) -> GameError {
    GameError::new("overlapping_unit", format!("{id} {}", "與其他單位重疊"))
}

pub(crate) fn unknown_skill(id: &str) -> GameError {
    GameError::new("unknown_skill", format!("{} {id}", "unknown skill:"))
}

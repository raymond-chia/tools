//! 以 `editor` feature 選用的編輯器專用操作；正式遊戲載入不依賴本模組。
//! ID 建立後不可更動，因此不需要更新既有引用；刪除前仍須檢查引用。
mod definitions;
mod error;
mod json;
mod map;

use crate::Game;
use crate::authoring::{Definitions, Map};
pub use definitions::edit_definition_from_json;
pub use error::EditorError;
pub use map::paint_terrain_from_json;

/// 依地圖配置順序列出既有敵方派系，供編輯器選用。
pub fn enemy_factions_from_json(map: &str) -> Result<String, EditorError> {
    let map: Map = json::from_str(map)
        .map_err(|e| EditorError::conversion("map_json_parse", e.to_string()))?;
    let mut factions = Vec::new();
    for unit in map.units {
        if let crate::Team::Enemy(faction) = unit.team {
            if !factions.contains(&faction) {
                factions.push(faction);
            }
        }
    }
    serde_json::to_string(&serde_json::json!({ "factions": factions }))
        .map_err(|e| EditorError::conversion("json_serialize", e.to_string()))
}

/// 將 Godot 編輯器傳來的 JSON 定義與地圖資料驗證後，轉成儲存或試玩用的 TOML 文件。
/// JSON 僅用於編輯器與 Rust 之間傳遞資料；實際保存的檔案仍是 TOML。
pub fn documents_from_json(definitions: &str, map: &str) -> Result<(String, String), EditorError> {
    let mut definitions: Definitions = json::from_str(definitions)
        .map_err(|e| EditorError::conversion("definitions_json_parse", e.to_string()))?;
    let map: Map = json::from_str(map)
        .map_err(|e| EditorError::conversion("map_json_parse", e.to_string()))?;
    Game::from_authoring(
        definitions.clone(),
        map.clone(),
        crate::authoring::Difficulty::Hard,
    )?;
    // 單位的技能依技能頁籤排列存檔，不沿用勾選的先後順序。
    for unit in &mut definitions.unit_types {
        unit.skills.sort_by_key(|id| {
            definitions
                .skills
                .iter()
                .position(|skill| skill.id == *id)
                .expect("遊戲資料驗證已確認單位引用的技能存在")
        });
    }
    Ok((
        toml::to_string_pretty(&definitions)
            .map_err(|e| EditorError::conversion("toml_serialize", e.to_string()))?,
        toml::to_string_pretty(&map)
            .map_err(|e| EditorError::conversion("toml_serialize", e.to_string()))?,
    ))
}

/// 編輯器預覽使用真實遊戲核心的 presentation snapshot，不另行推導規則。
pub fn preview_from_json(definitions: &str, map: &str) -> Result<String, EditorError> {
    inspected_preview_from_json(definitions, map, None)
}

/// 依編輯器選取的單位產生範圍；移動規則沿用遊戲 snapshot。
pub fn inspected_preview_from_json(
    definitions: &str,
    map: &str,
    inspected_actor: Option<i64>,
) -> Result<String, EditorError> {
    let definitions: Definitions = json::from_str(definitions)
        .map_err(|e| EditorError::conversion("definitions_json_parse", e.to_string()))?;
    let map: Map = json::from_str(map)
        .map_err(|e| EditorError::conversion("map_json_parse", e.to_string()))?;
    let game = Game::from_authoring(definitions, map, crate::authoring::Difficulty::Hard)?;
    serde_json::to_string(&game.snapshot(inspected_actor))
        .map_err(|e| EditorError::conversion("json_serialize", e.to_string()))
}

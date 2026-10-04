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

/// 將 Godot 編輯器傳來的 JSON 定義與地圖資料驗證後，轉成儲存或試玩用的 TOML 文件。
/// JSON 僅用於編輯器與 Rust 之間傳遞資料；實際保存的檔案仍是 TOML。
pub fn documents_from_json(definitions: &str, map: &str) -> Result<(String, String), EditorError> {
    let definitions: Definitions = json::from_str(definitions)
        .map_err(|e| EditorError::input("definitions_json_parse", e.to_string()))?;
    let map: Map =
        json::from_str(map).map_err(|e| EditorError::input("map_json_parse", e.to_string()))?;
    Game::from_authoring(definitions.clone(), map.clone())?;
    Ok((
        toml::to_string_pretty(&definitions)
            .map_err(|e| EditorError::input("toml_serialize", e.to_string()))?,
        toml::to_string_pretty(&map)
            .map_err(|e| EditorError::input("toml_serialize", e.to_string()))?,
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
        .map_err(|e| EditorError::input("definitions_json_parse", e.to_string()))?;
    let map: Map =
        json::from_str(map).map_err(|e| EditorError::input("map_json_parse", e.to_string()))?;
    let game = Game::from_authoring(definitions, map)?;
    serde_json::to_string(&game.snapshot(inspected_actor))
        .map_err(|e| EditorError::input("json_serialize", e.to_string()))
}

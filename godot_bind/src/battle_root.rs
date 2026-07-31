use crate::board_error::BoardError;
use bevy_ecs::world::World;
use board::ecs_logic::loader::parse_and_insert_game_data;
use board::error::{Error, LoadError};
use godot::classes::{Control, FileAccess};
use godot::prelude::*;

// 遊戲資料 TOML 位於 res://data/，與 editor 共用同一份。
const UNITS_TOML_PATH: &str = "res://data/units.toml";
const SKILLS_TOML_PATH: &str = "res://data/skills.toml";
const OBJECTS_TOML_PATH: &str = "res://data/objects.toml";

/// 戰鬥 UI 的根節點，持有整場戰鬥的 ECS World。
#[derive(GodotClass)]
#[class(init, base = Control)]
pub struct BattleRoot {
    base: Base<Control>,
    // 空 World 是合法狀態（尚未 spawn 任何東西）；load_level 灌入資料後才有內容。
    #[init(val = World::new())]
    world: World,
}

#[godot_api]
impl BattleRoot {
    /// 讀取 res://data/ 的遊戲資料 TOML 並存入 World。開場呼叫一次即可。
    #[func]
    fn parse_and_insert_game_data(&mut self) -> Result<(), BoardError> {
        let units_toml = read_data_file(UNITS_TOML_PATH)?;
        let skills_toml = read_data_file(SKILLS_TOML_PATH)?;
        let objects_toml = read_data_file(OBJECTS_TOML_PATH)?;

        parse_and_insert_game_data(&mut self.world, &units_toml, &skills_toml, &objects_toml)?;
        Ok(())
    }
}

/// 用 Godot FileAccess 讀 res:// 檔案內容（匯出打包後仍可從 .pck 讀取）。
/// 讀不到檔或檔案為空時 FileAccess 回空字串，一律視為載入失敗。
fn read_data_file(path: &str) -> Result<String, Error> {
    let content = FileAccess::get_file_as_string(path).to_string();
    if content.is_empty() {
        return Err(LoadError::ParseError(format!("檔案不存在或為空: {path}")).into());
    }
    Ok(content)
}

use game_core::{Command, Game};
use godot::prelude::*;
use std::sync::Mutex;
#[derive(GodotClass)]
#[class(base=RefCounted)]
struct TacticalGame {
    game: Mutex<Option<Game>>,
    base: Base<RefCounted>,
}
#[godot_api]
impl IRefCounted for TacticalGame {
    fn init(base: Base<RefCounted>) -> Self {
        Self {
            game: Mutex::new(None),
            base,
        }
    }
}
#[godot_api]
impl TacticalGame {
    #[func]
    fn load_definition(&self, text: GString) -> GString {
        match Game::from_toml(&text.to_string()) {
            Ok(mut game) => {
                let out = serde_json::to_string(&game.snapshot()).unwrap();
                *self.game.lock().unwrap() = Some(game);
                GString::from(&out)
            }
            Err(e) => error(e),
        }
    }
    #[func]
    fn dispatch(&self, json: GString) -> GString {
        let command: Command = match serde_json::from_str(&json.to_string()) {
            Ok(v) => v,
            Err(e) => return error(e.to_string()),
        };
        let mut lock = self.game.lock().unwrap();
        match lock.as_mut() {
            Some(game) => match game.command(command) {
                Ok(s) => GString::from(&serde_json::to_string(&s).unwrap()),
                Err(e) => error(e),
            },
            None => error("尚未載入定義".into()),
        }
    }
}
fn error(message: String) -> GString {
    GString::from(&serde_json::json!({"error":message}).to_string())
}
struct Extension;
#[gdextension]
unsafe impl ExtensionLibrary for Extension {}

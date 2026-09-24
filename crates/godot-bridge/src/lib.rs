use game_core::{Command, Game, GridPos, authoring};
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
    fn load_documents(&self, definitions: GString, map: GString) -> GString {
        match Game::from_documents(&definitions.to_string(), &map.to_string()) {
            Ok(mut game) => {
                let out = json_response(serde_json::to_string(&game.snapshot()));
                *self.game.lock().expect("核心鎖不應中毒") = Some(game);
                out
            }
            Err(e) => error(e),
        }
    }
    #[func]
    fn definitions_to_json(&self, text: GString) -> GString {
        match authoring::definitions_to_json(&text.to_string()) {
            Ok(json) => GString::from(&json),
            Err(e) => error(e),
        }
    }
    #[func]
    fn map_to_json(&self, text: GString) -> GString {
        match authoring::map_to_json(&text.to_string()) {
            Ok(json) => GString::from(&json),
            Err(e) => error(e),
        }
    }
    #[func]
    fn documents_from_json(&self, definitions: GString, map: GString) -> GString {
        match authoring::documents_from_json(&definitions.to_string(), &map.to_string()) {
            Ok((definitions, map)) => {
                GString::from(&serde_json::json!({"definitions":definitions,"map":map}).to_string())
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
        let mut lock = self.game.lock().expect("核心鎖不應因先前的 panic 而中毒");
        match lock.as_mut() {
            Some(game) => match game.command(command) {
                Ok(s) => json_response(serde_json::to_string(&s)),
                Err(e) => error(e),
            },
            None => error("尚未載入定義".into()),
        }
    }
    #[func]
    fn set_random_seed(&self, seed: i64) {
        if let Some(game) = self
            .game
            .lock()
            .expect("核心鎖不應因先前的 panic 而中毒")
            .as_mut()
        {
            game.set_random_seed(seed as u64);
        }
    }
    #[func]
    fn preview_move(&self, actor: GString, x: i32, y: i32) -> GString {
        let lock = self.game.lock().expect("核心鎖不應因先前的 panic 而中毒");
        match lock.as_ref() {
            Some(game) => match game.preview_move(&actor.to_string(), GridPos { x, y }) {
                Ok(preview) => json_response(serde_json::to_string(&preview)),
                Err(e) => error(e),
            },
            None => error("尚未載入定義".into()),
        }
    }
    #[func]
    fn preview_skill(
        &self,
        actor: GString,
        target: GString,
        x: i32,
        y: i32,
        skill: GString,
    ) -> GString {
        let lock = self.game.lock().expect("核心鎖不應因先前的 panic 而中毒");
        match lock.as_ref() {
            Some(game) => match game.preview_skill(
                &actor.to_string(),
                &target.to_string(),
                GridPos { x, y },
                &skill.to_string(),
            ) {
                Ok(preview) => json_response(serde_json::to_string(&preview)),
                Err(e) => error(e),
            },
            None => error("尚未載入定義".into()),
        }
    }
}
fn json_response(result: Result<String, serde_json::Error>) -> GString {
    match result {
        Ok(json) => GString::from(&json),
        Err(e) => error(e.to_string()),
    }
}
fn error(message: String) -> GString {
    GString::from(&serde_json::json!({"error":message}).to_string())
}
struct Extension;
#[gdextension]
unsafe impl ExtensionLibrary for Extension {}

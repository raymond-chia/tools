//! 編輯器錯誤只回傳語意分類與引用 ID；文字由 Godot 介面處理。
use crate::GameError;

#[derive(Debug)]
pub enum EditorError {
    Game(GameError),
    Map {
        path: String,
        error: GameError,
    },
    Input {
        id: &'static str,
        message: String,
    },
    Operation {
        kind: &'static str,
        id: String,
        references: Vec<(&'static str, String)>,
    },
}

impl From<GameError> for EditorError {
    fn from(error: GameError) -> Self {
        Self::Game(error)
    }
}

impl EditorError {
    pub(crate) fn input(id: &'static str, message: String) -> Self {
        Self::Input { id, message }
    }

    pub(crate) fn operation(
        kind: &'static str,
        id: &str,
        references: Vec<(&'static str, String)>,
    ) -> Self {
        Self::Operation {
            kind,
            id: id.to_owned(),
            references,
        }
    }

    pub fn response_json(&self) -> String {
        match self {
            Self::Game(error) => {
                serde_json::json!({"error_id": error.id(), "error": error.message()})
            }
            Self::Map { path, error } => serde_json::json!({
                "error_id": error.id(), "error": error.message(), "map_path": path,
            }),
            Self::Input { id, message } => serde_json::json!({"error_id": id, "error": message}),
            Self::Operation {
                kind,
                id,
                references,
            } => serde_json::json!({
                "error_id": "authoring_edit", "error": kind,
                "error_details": {
                    "kind": kind, "id": id,
                    "references": references.iter().map(|(category, id)| {
                        serde_json::json!({"category": category, "id": id})
                    }).collect::<Vec<_>>(),
                },
            }),
        }
        .to_string()
    }
}

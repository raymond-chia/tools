use crate::editor_item::EditorItem;
use crate::generic_editor::{GenericEditorState, MessageState};
use crate::generic_io::{load_file, save_file};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
struct TestItem {
    name: String,
    value: i32,
}

impl EditorItem for TestItem {
    type UIState = ();

    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn type_name() -> &'static str {
        "test item"
    }
}

fn temp_file_path(name: &str) -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("editor_generic_io_{name}_{unique}.toml"))
}

struct TempFileGuard {
    path: PathBuf,
}

impl TempFileGuard {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

// Implement Drop so the temp file is cleaned up automatically when the guard
// goes out of scope, including when the test exits early due to panic.
impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[test]
fn save_file_rewrites_toml_without_legacy_fields() {
    let path = TempFileGuard::new(temp_file_path("legacy_cleanup"));

    let legacy_content = r#"
[[skills]]
name = "Slash"
value = 7
removed_field = "legacy"
"#;
    fs::write(path.path(), legacy_content).expect("should write legacy toml");

    let mut state = GenericEditorState::<TestItem> {
        message_state: MessageState::default(),
        ..Default::default()
    };
    load_file(&mut state, path.path(), "skills");
    assert_eq!(
        state.items,
        vec![TestItem {
            name: "Slash".to_string(),
            value: 7,
        }]
    );

    save_file(&mut state, path.path(), "skills");

    let saved = fs::read_to_string(path.path()).expect("should read rewritten toml");
    assert!(saved.contains("name = \"Slash\""));
    assert!(saved.contains("value = 7"));
    assert!(!saved.contains("removed_field"));
}

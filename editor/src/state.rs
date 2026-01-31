#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    #[default]
    BattlefieldEditor,
    ObjectEditor,
}

#[derive(Debug, Default)]
pub struct EditorApp {
    pub state: EditorState,
}

#[derive(Debug, Default)]
pub struct EditorState {
    pub current_page: Page,
}

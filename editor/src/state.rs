#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    #[default]
    BattlefieldEditor,
    ObjectEditor,
}

#[derive(Debug, Default)]
pub struct Message {
    pub text: String,
    pub visible: bool,
}

#[derive(Debug, Default)]
pub struct EditorApp {
    pub current_page: Page,
    pub message: Message,
}

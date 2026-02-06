use crate::constants::DATA_DIRECTORY_PATH;
use crate::generic_editor::GenericEditorState;
use crate::generic_io::load_file;
use crate::tabs;
use board::loader_schema::{LevelType, ObjectType, SkillType, UnitType};
use std::path::PathBuf;
use strum_macros::{Display, EnumIter};

/// 編輯器標籤頁
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, EnumIter, Display)]
pub enum EditorTab {
    #[default]
    #[strum(to_string = "物件")]
    Object,
    #[strum(to_string = "技能")]
    Skill,
    #[strum(to_string = "單位")]
    Unit,
    #[strum(to_string = "關卡")]
    Level,
}

/// 編輯器應用程式狀態
#[derive(Debug, Default)]
pub struct EditorApp {
    pub current_tab: EditorTab,
    pub object_editor: GenericEditorState<ObjectType>,
    pub skill_editor: GenericEditorState<SkillType>,
    pub unit_editor: GenericEditorState<UnitType>,
    pub level_editor: GenericEditorState<LevelType>,
}

impl EditorApp {
    /// 建立編輯器並載入所有資料檔案
    pub fn new() -> Self {
        let mut app = Self::default();

        // 啟動時自動載入所有資料
        let data_dir = PathBuf::from(DATA_DIRECTORY_PATH);
        load_file(
            &mut app.object_editor,
            &data_dir.join(format!("{}.toml", tabs::object_tab::file_name())),
            tabs::object_tab::file_name(),
        );
        load_file(
            &mut app.skill_editor,
            &data_dir.join(format!("{}.toml", tabs::skill_tab::file_name())),
            tabs::skill_tab::file_name(),
        );
        load_file(
            &mut app.unit_editor,
            &data_dir.join(format!("{}.toml", tabs::unit_tab::file_name())),
            tabs::unit_tab::file_name(),
        );
        load_file(
            &mut app.level_editor,
            &data_dir.join(format!("{}.toml", tabs::level_tab::file_name())),
            tabs::level_tab::file_name(),
        );

        app
    }
}

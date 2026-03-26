//! 技能編輯器 tab

use crate::constants::*;
use crate::editor_item::{EditorItem, validate_name};
use crate::generic_editor::MessageState;
use board::domain::alias::Coord;
use board::domain::core_types::SkillType;
use std::fmt::Debug;
use strum::IntoEnumIterator;

// ==================== EditorItem 實作 ====================

impl EditorItem for SkillType {
    type UIState = ();

    fn name(&self) -> &str {
        &self.name()
    }

    fn set_name(&mut self, name: String) {
        self.set_name(name);
    }

    fn type_name() -> &'static str {
        "技能"
    }

    fn validate(&self, all_items: &[Self], editing_index: Option<usize>) -> Result<(), String> {
        validate_name(self, all_items, editing_index)?;
        Ok(())
    }
}

/// 取得技能的檔案名稱
pub fn file_name() -> &'static str {
    "skills"
}

// ==================== 表單渲染 ====================

/// 渲染技能編輯表單
pub fn render_form(
    ui: &mut egui::Ui,
    skill: &mut SkillType,
    _ui_state: &mut (),
    _message_state: &mut MessageState,
) {
}

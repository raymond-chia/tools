//! 單位編輯器 tab

use crate::constants::{SPACING_MEDIUM, SPACING_SMALL};
use crate::editor_item::EditorItem;
use crate::generic_editor::MessageState;
use crate::utils::search::{filter_by_search, render_search_input};
use board::domain::alias::SkillName;
use board::loader_schema::UnitType;

/// 單位編輯器的 UI 狀態
#[derive(Debug, Default)]
pub struct UnitTabUIState {
    pub available_skills: Vec<SkillName>,

    pub skill_search_query: SkillName,
}

// ==================== EditorItem 實作 ====================

impl EditorItem for UnitType {
    type UIState = UnitTabUIState;

    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn type_name() -> &'static str {
        "單位"
    }

    fn after_confirm(&mut self, ui_state: &Self::UIState) {
        // 依技能列表的順序重建已選技能，使儲存順序與列表一致
        self.skills = ui_state
            .available_skills
            .iter()
            .filter(|name| self.skills.contains(name))
            .cloned()
            .collect();
    }
}

/// 取得單位的檔案名稱
pub fn file_name() -> &'static str {
    "units"
}

// ==================== 表單渲染 ====================

/// 渲染單位編輯表單
pub fn render_form(
    ui: &mut egui::Ui,
    unit: &mut UnitType,
    ui_state: &mut UnitTabUIState,
    _message_state: &mut MessageState,
) {
    ui.horizontal(|ui| {
        ui.label("名稱：");
        ui.text_edit_singleline(&mut unit.name);
    });

    ui.add_space(SPACING_SMALL);
    ui.separator();
    ui.heading("技能選擇");

    if ui_state.available_skills.is_empty() {
        ui.label("（尚未定義任何技能，請先到「技能」tab 創建技能）");
    } else {
        // 搜尋框
        render_search_input(ui, &mut ui_state.skill_search_query);
        ui.add_space(SPACING_SMALL);

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let visible_skills =
                    filter_by_search(&ui_state.available_skills, &ui_state.skill_search_query);

                if visible_skills.is_empty() && !ui_state.skill_search_query.is_empty() {
                    ui.label("找不到符合的技能");
                } else {
                    for skill_name in visible_skills {
                        let mut selected = unit.skills.contains(skill_name);
                        if ui.checkbox(&mut selected, skill_name).changed() {
                            if selected {
                                unit.skills.push(skill_name.clone());
                            } else {
                                unit.skills.retain(|s| s != skill_name);
                            }
                        }
                    }
                }
            });
    }

    ui.separator();

    // 依儲存順序列出技能總數與已選技能名稱，方便快速檢視
    ui.label(format!("已選擇：{} 個技能", unit.skills.len()));
    ui.horizontal_wrapped(|ui| {
        for skill_name in &unit.skills {
            ui.label(skill_name);
            ui.add_space(SPACING_MEDIUM);
        }
    });
}

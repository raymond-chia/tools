//! 單位編輯器 tab

use crate::constants::SPACING_SMALL;
use crate::editor_item::EditorItem;
use board::alias::SkillName;
use board::loader_schema::UnitType;

/// 單位編輯器的 UI 狀態
#[derive(Debug, Clone, Default)]
pub struct UnitTabUIState {
    pub available_skills: Vec<SkillName>,
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

    fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("名稱不能為空".to_string());
        }
        Ok(())
    }
}

/// 取得單位的檔案名稱
pub fn file_name() -> &'static str {
    "units"
}

// ==================== 表單渲染 ====================

/// 渲染單位編輯表單
pub fn render_form(ui: &mut egui::Ui, unit: &mut UnitType, ui_state: &mut UnitTabUIState) {
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
        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                for skill_name in &ui_state.available_skills {
                    let mut selected = unit.skills.contains(skill_name);
                    if ui.checkbox(&mut selected, skill_name).changed() {
                        if selected {
                            unit.skills.push(skill_name.clone());
                        } else {
                            unit.skills.retain(|s| s != skill_name);
                        }
                    }
                }
            });
    }

    ui.separator();
    ui.label(format!("已選擇：{} 個技能", unit.skills.len()));
}

//! 裝備與單位共用的技能選擇元件。

use crate::constants::{SPACING_MEDIUM, SPACING_SMALL};
use crate::utils::search::{filter_by_search, render_search_input};
use board::domain::alias::SkillName;

/// 小視窗中仍保留可操作的技能清單高度。
const SKILL_LIST_MIN_HEIGHT: f32 = 160.0;

/// 渲染已選技能的數量與名稱摘要。
pub(crate) fn render_selected_skills_summary(ui: &mut egui::Ui, selected_skills: &[SkillName]) {
    ui.label(format!("已選擇：{} 個技能", selected_skills.len()));
    ui.horizontal_wrapped(|ui| {
        for skill_name in selected_skills {
            ui.label(skill_name);
            ui.add_space(SPACING_MEDIUM);
        }
    });
}

/// 渲染可搜尋、可勾選且自動調整高度的技能清單。
pub(crate) fn render_skill_selector(
    ui: &mut egui::Ui,
    available_skills: &[SkillName],
    search_query: &mut SkillName,
    selected_skills: &mut Vec<SkillName>,
) {
    if available_skills.is_empty() {
        ui.label("（尚未定義任何技能，請先到「技能」tab 創建技能）");
        return;
    }

    render_search_input(ui, search_query);
    ui.add_space(SPACING_SMALL);

    let skill_list_max_height = skill_list_max_height(ui);
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .max_height(skill_list_max_height)
        .show(ui, |ui| {
            let visible_skills = filter_by_search(available_skills, search_query);

            if visible_skills.is_empty() && !search_query.is_empty() {
                ui.label("找不到符合的技能");
            } else {
                for skill_name in visible_skills {
                    let mut selected = selected_skills.contains(skill_name);
                    if ui.checkbox(&mut selected, skill_name).changed() {
                        if selected {
                            selected_skills.push(skill_name.clone());
                        } else {
                            selected_skills.retain(|name| name != skill_name);
                        }
                    }
                }
            }
        });
}

/// 依技能清單到目前視窗底部的剩餘距離決定高度。
fn skill_list_max_height(ui: &egui::Ui) -> f32 {
    (ui.ctx().viewport_rect().bottom() - ui.cursor().top() - SPACING_SMALL)
        .max(SKILL_LIST_MIN_HEIGHT)
}

//! 關卡編輯器 tab

mod battle;
mod battlefield;
mod deployment;
mod edit;

use crate::editor_item::{EditorItem, validate_name};
use crate::generic_editor::MessageState;
use bevy_ecs::world::World;
use board::domain::alias::TypeName;
use board::ecs_types::components::Position;
use board::ecs_types::resources::Board;
use board::loader_schema::{LevelType, ObjectType, SkillType, UnitType};
use std::collections::HashSet;

/// 拖曳物體的類型和索引
#[derive(Clone, Copy, Debug)]
pub enum DraggedObject {
    Deployment(usize),
    Unit(usize),
    Object(usize),
}

/// 拖曳狀態
#[derive(Clone, Copy, Debug)]
pub struct DragState {
    pub object: DraggedObject,
}

/// 關卡編輯器的模式
#[derive(Debug, Default)]
pub enum LevelTabMode {
    #[default]
    Edit,
    Deploy,
    Battle,
}

/// 關卡編輯器的 UI 狀態
#[derive(Debug, Default)]
pub struct LevelTabUIState {
    /// 可選的單位類型（完整資料，供部署時序列化用）
    pub available_units: Vec<UnitType>,
    /// 可選的技能類型（完整資料，供部署時序列化用）
    pub available_skills: Vec<SkillType>,
    /// 可選的物件類型（完整資料，供部署時序列化用）
    pub available_objects: Vec<ObjectType>,

    pub unit_search_query: TypeName,
    pub object_search_query: TypeName,

    pub drag_state: Option<DragState>,
    pub scroll_offset: egui::Vec2,

    /// ECS World，模擬模式時存放所有 entity
    pub world: World,
    /// 左鍵選中
    pub selected_left_pos: Option<Position>,
    /// 右鍵選中
    pub selected_right_pos: Option<Position>,

    /// 當前標籤頁的模式
    pub mode: LevelTabMode,
}

// ==================== EditorItem 實作 ====================

impl EditorItem for LevelType {
    type UIState = LevelTabUIState;

    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn type_name() -> &'static str {
        "關卡"
    }

    fn validate(&self, all_items: &[Self], editing_index: Option<usize>) -> Result<(), String> {
        validate_name(self, all_items, editing_index)?;

        if self.board_width == 0 || self.board_height == 0 {
            return Err("棋盤尺寸必須大於 0".to_string());
        }
        if self.max_player_units == 0 {
            return Err("人數上限必須大於 0".to_string());
        }
        if self.deployment_positions.len() < self.max_player_units {
            return Err(format!(
                "部署點數量 ({}) 少於上限 ({})",
                self.deployment_positions.len(),
                self.max_player_units
            ));
        }
        if self.factions.is_empty() {
            return Err("至少需要一個陣營".to_string());
        }

        let board = Board {
            width: self.board_width,
            height: self.board_height,
        };
        // 檢查部署點超出棋盤範圍
        for (idx, pos) in self.deployment_positions.iter().enumerate() {
            check_position_in_bounds(board, *pos, idx + 1, "部署點")?;
        }
        // 檢查單位位置超出棋盤範圍
        for (idx, unit) in self.unit_placements.iter().enumerate() {
            check_position_in_bounds(board, unit.position, idx + 1, "單位")?;
        }
        // 檢查物件位置超出棋盤範圍
        for (idx, obj) in self.object_placements.iter().enumerate() {
            check_position_in_bounds(board, obj.position, idx + 1, "物件")?;
        }

        // 檢查部署點互相重複
        let deployment_positions_set: HashSet<Position> =
            self.deployment_positions.iter().cloned().collect();
        if deployment_positions_set.len() != self.deployment_positions.len() {
            return Err("部署點存在重複位置".to_string());
        }

        // 檢查單位位置互相重複
        let unit_positions_set: HashSet<Position> =
            self.unit_placements.iter().map(|u| u.position).collect();
        if unit_positions_set.len() != self.unit_placements.len() {
            return Err("單位位置存在重複".to_string());
        }

        // 檢查部署點與單位位置不重複
        if !deployment_positions_set.is_disjoint(&unit_positions_set) {
            return Err("部署點和單位位置存在重複".to_string());
        }

        Ok(())
    }

    fn after_confirm(&mut self) {
        // 按位置排序（X 座標優先，再按 Y 座標）
        self.deployment_positions.sort_by_key(|pos| (pos.x, pos.y));
        self.unit_placements
            .sort_by_key(|unit| (unit.position.x, unit.position.y));
        self.object_placements
            .sort_by_key(|obj| (obj.position.x, obj.position.y));
    }
}

/// 取得關卡的檔案名稱
pub fn file_name() -> &'static str {
    "levels"
}

// ==================== 本地輔助函數 ====================

fn check_position_in_bounds(
    board: Board,
    pos: Position,
    index: usize,
    label: &str,
) -> Result<(), String> {
    if !board::logic::board::is_valid_position(board, pos) {
        return Err(format!(
            "{} #{} ({}, {}) 超出棋盤範圍 (寬: {}, 高: {})",
            label, index, pos.x, pos.y, board.width, board.height
        ));
    }
    Ok(())
}

// ==================== 表單渲染 ====================

/// 渲染關卡編輯表單
pub fn render_form(
    ui: &mut egui::Ui,
    level: &mut LevelType,
    ui_state: &mut LevelTabUIState,
    message_state: &mut MessageState,
) {
    type RenderFn = fn(&mut egui::Ui, &mut LevelTabUIState, &mut MessageState);
    let (window_name, render_fn): (&str, RenderFn) = match &ui_state.mode {
        LevelTabMode::Edit => return edit::render_form(ui, level, ui_state, message_state),
        // 根據模式決定窗口標題和渲染函數
        LevelTabMode::Deploy => ("單位部署", deployment::render_form),
        LevelTabMode::Battle => ("模擬戰鬥", battle::render_form),
    };

    // 繪製半透明遮罩，完全遮蔽背景
    let viewport = ui.ctx().viewport_rect();
    ui.painter()
        .rect_filled(viewport, 0.0, egui::Color32::from_black_alpha(200));

    egui::Window::new(window_name)
        .fixed_pos(viewport.min)
        .fixed_size(viewport.size())
        .resizable(false)
        .collapsible(false)
        .show(ui.ctx(), |ui| {
            render_fn(ui, ui_state, message_state);
        });
}

//! 關卡編輯器 tab

mod battle;
mod battlefield;
mod deployment;
mod edit;

use crate::editor_item::{EditorItem, validate_name};
use crate::generic_editor::{GenericEditorState, MessageState};
use crate::generic_io::save_file;
use crate::tabs::reference;
use bevy_ecs::world::World;
use board::domain::alias::{SkillName, TypeName};
use board::domain::constants::PLAYER_FACTION_ID;
use board::domain::core_types::{LevelOutcome, OutcomeBranches, SkillType};
use board::ecs_types::components::{Occupant, Position};
use board::ecs_types::resources::Board;
use board::loader_schema::{EquipmentType, LevelType, ObjectType, UnitType};
use std::collections::HashSet;
use std::path::Path;

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

/// 反應決策草稿：玩家安排的執行順序 + 每人選的技能（None = 跳過）
#[derive(Debug, Default)]
pub struct ReactionDecisionState {
    pub decisions: Vec<(Occupant, Option<SkillName>)>,
}

/// 戰鬥模式底部面板的動作狀態
#[derive(Debug, Default, PartialEq)]
pub enum BattleAction {
    #[default]
    Normal,
    Delaying,
    /// 技能模式：彈窗一直開著、戰場可互動預覽 targetable/AOE/picked
    /// 實際選中的技能與 picked 由 core 的 SkillTargeting resource 持有（未選技能時 resource 不存在）
    SkillMode,
}

/// 右側面板顯示模式
#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub enum RightPanelView {
    #[default]
    Details,
    Log,
}

/// 關卡編輯器的模式
#[derive(Debug, Default)]
pub enum LevelTabMode {
    #[default]
    Edit,
    Deploy,
    Battle,
}

// ==================== 重要 ====================
/// 禁止存放 UI 與 world 以外的資料，確保邏輯都在 board crate 中實現
// ==================== 重要 ====================
#[derive(Debug, Default)]
pub struct LevelTabUIState {
    /// 可選的單位類型（完整資料，供部署時序列化用）
    pub available_units: Vec<UnitType>,
    /// 可選的技能類型（完整資料，供部署時序列化用）
    pub available_skills: Vec<SkillType>,
    /// 可選的裝備類型（完整資料，供部署時序列化用）
    pub available_equipments: Vec<EquipmentType>,
    /// 可選的物件類型（完整資料，供部署時序列化用）
    pub available_objects: Vec<ObjectType>,

    pub unit_search_query: TypeName,
    pub object_search_query: TypeName,

    pub drag_state: Option<DragState>,
    pub scroll_offset: egui::Vec2,

    /// 模擬戰鬥專用：統一在 tabs\level_tab\edit.rs 初始化
    /// ECS World，模擬模式時存放所有 entity
    pub world: World,
    /// 左鍵選中
    pub selected_left_pos: Option<Position>,
    /// 右鍵選中
    pub selected_right_pos: Option<Position>,
    /// 底部操作面板的當前動作狀態
    pub battle_action: BattleAction,
    /// 延遲置中：下一幀 render_battlefield 時消費
    pub pending_center_pos: Option<Position>,

    /// 右側面板顯示模式（單位詳情 / 戰鬥 log）
    pub right_panel_view: RightPanelView,

    /// 反應決策草稿（pending 為空時 decisions 也為空）
    pub reaction_decision: ReactionDecisionState,

    /// 關卡結局字幕：切換模式時清為 Undetermined，非 Undetermined 時在戰場上方顯示
    pub level_outcome: LevelOutcome,

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

        let has_player_unit = self
            .unit_placements
            .iter()
            .any(|u| u.faction_id == PLAYER_FACTION_ID);

        // 玩家必須有兵源：人數上限為 0 時，靠預放的玩家單位墊底
        if self.max_player_units == 0 && !has_player_unit {
            return Err("人數上限為 0 時，必須至少放置一個玩家單位".to_string());
        }

        // 有設人數上限時，部署點數量必須湊滿上限
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

        validate_outcome_conditions("勝利", &self.victory_conditions)?;
        validate_outcome_conditions("失敗", &self.defeat_conditions)?;

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

        // 檢查單位未設定類型
        for (idx, unit) in self.unit_placements.iter().enumerate() {
            if unit.unit_type_name.is_empty() {
                return Err(format!("第 {} 個單位未設定類型", idx + 1));
            }
        }

        // 檢查物件未設定類型
        for (idx, obj) in self.object_placements.iter().enumerate() {
            if obj.object_type_name.is_empty() {
                return Err(format!("第 {} 個物件未設定類型", idx + 1));
            }
        }

        Ok(())
    }

    fn after_confirm(&mut self, _ui_state: &Self::UIState) {
        // 按位置排序（X 座標優先，再按 Y 座標）
        self.deployment_positions.sort_by_key(|pos| (pos.x, pos.y));
        self.unit_placements
            .sort_by_key(|unit| (unit.position.x, unit.position.y));
        self.object_placements
            .sort_by_key(|obj| (obj.position.x, obj.position.y));
    }

    fn save(state: &mut GenericEditorState<Self>, path: &Path, data_key: &str) {
        // 先寫入大檔；失敗時 save_file 已設好錯誤訊息，不再拆解
        save_file(state, path, data_key);
        if state.message_state.is_error {
            return;
        }

        // 大檔成功後，額外把每一關拆成獨立小檔
        match edit::dump_levels_split(&state.items, path) {
            Ok(()) => {
                state.message_state.set_success(format!(
                    "成功儲存並拆解 {} 個關卡到子資料夾",
                    state.items.len()
                ));
            }
            Err(msg) => {
                state
                    .message_state
                    .set_error(format!("大檔已存，但拆解關卡失敗：{}", msg));
            }
        }
    }
}

/// 取得關卡的檔案名稱
pub fn file_name() -> &'static str {
    "levels"
}

/// 是否存在已被刪除的單位或物件配置。
pub fn has_invalid_references(state: &GenericEditorState<LevelType>) -> bool {
    state
        .items
        .iter()
        .any(|level| has_invalid_reference(level, &state.ui_state))
}

/// 是否存在已被刪除的單位或物件配置。
pub fn has_invalid_reference(level: &LevelType, ui_state: &LevelTabUIState) -> bool {
    let valid_references = ValidLevelReferences::from_ui_state(ui_state);

    reference::has_invalid(
        level
            .unit_placements
            .iter()
            .map(|placement| &placement.unit_type_name),
        &valid_references.units,
    ) || reference::has_invalid(
        level
            .object_placements
            .iter()
            .map(|placement| &placement.object_type_name),
        &valid_references.objects,
    )
}

/// 清除所有關卡中已失效的單位與物件配置。
pub fn clear_invalid_references(state: &mut GenericEditorState<LevelType>) {
    let valid_references = ValidLevelReferences::from_ui_state(&state.ui_state);

    for level in &mut state.items {
        level
            .unit_placements
            .retain(|placement| valid_references.units.contains(&placement.unit_type_name));
        level.object_placements.retain(|placement| {
            valid_references
                .objects
                .contains(&placement.object_type_name)
        });
    }
}

// ==================== 本地輔助函數 ====================

struct ValidLevelReferences {
    units: HashSet<TypeName>,
    objects: HashSet<TypeName>,
}

impl ValidLevelReferences {
    fn from_ui_state(ui_state: &LevelTabUIState) -> Self {
        Self {
            units: ui_state
                .available_units
                .iter()
                .map(|unit| unit.name.clone())
                .collect(),
            objects: ui_state
                .available_objects
                .iter()
                .map(|object| object.name.clone())
                .collect(),
        }
    }
}

fn validate_outcome_conditions(label: &str, branches: &OutcomeBranches) -> Result<(), String> {
    if branches.is_empty() {
        return Err(format!("至少需要一個{}條件分支", label));
    }

    for (branch_index, (reason_key, conditions)) in branches.iter().enumerate() {
        if reason_key.trim().is_empty() {
            return Err(format!(
                "{}條件分支 #{} 的結果 key 不可空白",
                label,
                branch_index + 1
            ));
        }
        if !edit::is_outcome_key_localized(reason_key) {
            return Err(format!(
                "{}條件分支 #{} 的結果 key 不存在於英語與繁中多語系資料",
                label,
                branch_index + 1
            ));
        }
        if conditions.is_empty() {
            return Err(format!(
                "{}條件分支 #{} 至少需要一項條件",
                label,
                branch_index + 1
            ));
        }
    }
    Ok(())
}

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

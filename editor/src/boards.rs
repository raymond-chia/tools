use crate::common::*;
use crate::player_progressions::{PlayerProgressionData, Unit as ProgressionUnit};
use chess_lib::*;
use egui::*;
use indexmap::IndexMap;
use rand::Rng;
use skills_lib::*;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::io;
use strum::IntoEnumIterator;

const TILE_SIZE: f32 = 56.0;
const TILE_UNIT_SIZE: f32 = 10.0;
const TILE_OBJECT_SIZE: f32 = 100.0;
const TILE_SHRINK_SIZE: f32 = TILE_SIZE / 40.0;
const TILE_ACTION_SHRINK_SIZE: f32 = TILE_SIZE / 5.0;
const TILE_DEPLOYABLE_SIZE: f32 = 28.0;
const TILE_ACTIVE_UNIT_MARKER_SIZE: f32 = 10.0;
const TILE_ACTIVE_UNIT_MARKER_STROKE_WIDTH: f32 = 2.0;

#[derive(Debug, Default)]
pub struct BoardsEditor {
    boards: BTreeMap<BoardID, BoardConfig>,
    selected_board: Option<BoardID>,
    // 修改地圖
    brush: BrushMode,
    selected_terrain: Terrain,
    selected_object: Option<Object>,
    selected_object_duration: u32,
    selected_orientation: Orientation,
    selected_unit: Option<UnitTemplateType>,
    selected_team: TeamID,
    // 範圍選取狀態
    selecting: bool,
    selection_start: Option<Pos>,
    selection_end: Option<Pos>,
    // drunkards walk 設定
    drunkards_steps: usize,
    drunkards_start_y: Option<usize>,
    drunkards_weights: [usize; 4],
    // 模擬
    sim_board: Board,
    sim_battle: Battle,
    skill_selection: SkillSelection,
    // AI 評分結果
    ai_score_result: Option<String>,
    // 其他
    camera: Camera2D,
    unit_templates: IndexMap<UnitTemplateType, UnitTemplate>,
    skills: BTreeMap<SkillID, Skill>,
    skill_groups: SkillByTags,
    player_progression: PlayerProgressionData,
    ai_config: AIConfig,
    // status
    has_unsaved_changes: bool,
    status_message: Option<(String, bool)>,
}

#[derive(Debug, Default, PartialEq, Clone, Copy)]
enum BrushMode {
    #[default]
    None,
    Terrain,
    Object,
    Unit,
    Team,
    Deploy,
    Sim,
}

impl BoardsEditor {
    pub fn new() -> Self {
        let mut editor = Self::default();
        editor.drunkards_steps = 50;
        editor.drunkards_weights = [20, 20, 10, 50];
        editor.reload();
        editor
    }

    fn reload(&mut self) {
        // 先載入玩家進度資料
        self.player_progression = match from_file::<_, PlayerProgressionData>(progressions_file()) {
            Ok(data) => data,
            Err(err) => {
                self.set_status(format!("載入 player_progression 失敗: {}", err), true);
                return;
            }
        };

        match load_boards(boards_file()) {
            Ok(boards) => {
                self.boards = boards;
                if let Some(id) = &self.selected_board {
                    if !self.boards.contains_key(id) {
                        self.selected_board = None;
                    }
                }
            }
            Err(err) => {
                self.set_status(format!("載入戰場失敗: {}", err), true);
                return;
            }
        }
        // 載入 unit_templates
        match crate::units::load_unit_templates(unit_templates_file()) {
            Ok(unit_templates) => {
                self.unit_templates = unit_templates
                    .into_iter()
                    .map(|u| (u.name.clone(), u))
                    .collect();
                let is_selected_exist = self
                    .selected_unit
                    .as_ref()
                    .map_or(false, |selected| self.unit_templates.contains_key(selected));
                if !is_selected_exist {
                    // 如果選中的單位不存在，則清除選中狀態
                    self.selected_unit = None;
                }
            }
            Err(err) => {
                self.unit_templates = IndexMap::new();
                self.set_status(format!("載入單位類型失敗: {}", err), true);
                return;
            }
        }
        match from_file::<_, BTreeMap<SkillID, Skill>>(skills_file()) {
            Err(err) => {
                self.skills = BTreeMap::new();
                self.skill_groups = BTreeMap::new();
                self.set_status(format!("載入技能失敗: {}", err), true);
                return;
            }
            Ok(skills) => {
                let grouped = match must_group_skills_by_tags(&skills) {
                    Err(msg) => {
                        self.set_status(format!("解析技能標籤失敗: {}", msg), true);
                        return;
                    }
                    Ok(res) => res,
                };

                self.skills = skills;
                self.skill_groups = grouped;
            }
        }
        // 載入 AI 設定
        match from_file::<_, AIConfig>(ai_file()) {
            Ok(ai) => {
                self.ai_config = ai;
            }
            Err(err) => {
                self.ai_config = AIConfig::default();
                self.set_status(format!("載入 AI 設定失敗: {}", err), true);
                return;
            }
        }

        self.set_status("已重新載入戰場".to_string(), false);
    }

    pub fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        SidePanel::left("board_list_panel")
            .default_width(180.0)
            .show(ctx, |ui| {
                self.show_board_list(ui);
            });
        SidePanel::right("right_panel")
            .default_width(320.0)
            .show(ctx, |ui| {
                self.show_right_panel(ui);
            });
        // 最後產生 central panel, 以免偵測滑鼠的時候偵測到 right panel
        CentralPanel::default().show(ctx, |ui| {
            if self.brush != BrushMode::Sim {
                self.show_board_editor(ui);
            } else {
                self.show_sim(ui);
            }
        });
        self.show_status_message(ctx);
    }

    fn show_board_list(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            if ui.button("重新載入戰場").clicked() {
                self.reload();
            }
            if ui.button("儲存戰場").clicked() {
                // 檢查所有單位是否有效
                if let Some(invalid_units) = self.validate_units() {
                    self.set_status(format!("儲存失敗：發現無效單位：{}", invalid_units), true);
                    return;
                }
                if let Err(e) = save_boards(boards_file(), &self.boards) {
                    self.set_status(format!("儲存失敗: {}", e), true);
                } else {
                    self.set_status("儲存成功".to_string(), false);
                    self.has_unsaved_changes = false;
                }
            }
            if ui.button("匯出所有戰場為小檔案").clicked() {
                match export_boards_to_files(&self.boards) {
                    Ok(count) => {
                        self.set_status(format!("已匯出 {count} 個戰場檔案到 boards/ 目錄"), false)
                    }
                    Err(e) => self.set_status(format!("匯出失敗: {}", e), true),
                }
            }
            ui.heading("戰場列表");
            if ui.button("新增戰場").clicked() {
                let new_board_id = format!("新戰場{}", self.boards.len() + 1);
                let new_board = BoardConfig::default();
                self.selected_team = new_board.teams.keys().next().cloned().unwrap_or_default();
                self.selected_board = Some(new_board_id.clone());
                self.boards.insert(new_board_id, new_board);
                self.has_unsaved_changes = true;
            }

            ui.separator();

            let mut to_delete = None;
            let mut edited_id: Option<(String, String)> = None;
            for (board_id, _) in &self.boards {
                let selected = self.selected_board.as_ref() == Some(board_id);
                if selected {
                    let mut id_buf = board_id.clone();
                    let resp = ui.text_edit_singleline(&mut id_buf);
                    if resp.changed() && !id_buf.is_empty() && !self.boards.contains_key(&id_buf) {
                        edited_id = Some((board_id.clone(), id_buf.clone()));
                    }
                } else {
                    let button = Button::new(board_id).fill(Color32::TRANSPARENT);
                    if ui.add(button).clicked() {
                        self.selected_team = self
                            .boards
                            .get(board_id)
                            .and_then(|b| b.teams.keys().next().cloned())
                            .unwrap_or_default();
                        self.selected_board = Some(board_id.clone());
                    }
                }
                ui.horizontal(|ui| {
                    if ui.button("刪除").clicked() {
                        to_delete = Some(board_id.clone());
                    }
                });
            }
            if let Some((old_id, new_id)) = edited_id {
                if let Some(board) = self.boards.remove(&old_id) {
                    self.selected_team = board.teams.keys().next().cloned().unwrap_or_default();
                    self.selected_board = Some(new_id.clone());
                    self.boards.insert(new_id, board);
                    self.has_unsaved_changes = true;
                }
            }
            if let Some(board_id) = to_delete {
                self.boards.remove(&board_id);
                self.selected_board = None;
                self.has_unsaved_changes = true;
            }
        });
    }

    fn show_board_editor(&mut self, ui: &mut Ui) {
        // 棋盤視覺化編輯區
        let Some(board_id) = &self.selected_board else {
            ui.label("請先選擇戰場");
            return;
        };
        let board = self.boards.get(board_id).expect("選擇的戰場應該存在");
        show_tiles(
            ui,
            &mut self.camera,
            &board.tiles,
            show_static_others(board),
        );

        // 繪製選取範圍外框
        if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
            draw_selection_outline(ui.painter(), &self.camera, start, end);
        }

        if let (Some(start), Some(end), Some(start_y)) = (
            self.selection_start,
            self.selection_end,
            self.drunkards_start_y,
        ) {
            draw_drunkard_start_indicator(ui.painter(), &self.camera, board, start, end, start_y);
        }

        // 處理滑鼠選取（僅在 Object 筆刷且滑鼠在面板內啟用）
        let pointer_state = ui.ctx().input(|i| i.pointer.clone());
        let is_panel_hover = ui.rect_contains_pointer(ui.max_rect());
        if self.brush == BrushMode::Object && is_panel_hover {
            // 如果已啟用「選取模式」（self.selecting 作為模式開關），使用滑鼠更新 selection，但不要改變 self.selecting
            if self.selecting {
                if pointer_state.primary_pressed() {
                    if let Ok(pos) = cursor_to_pos(&self.camera, ui) {
                        // 開始新一次選取
                        self.selection_start = Some(pos);
                        self.selection_end = Some(pos);
                    }
                } else if pointer_state.primary_down() {
                    // 按住拖曳中，持續更新終點（如果已開始選取）
                    if let Some(_) = self.selection_start {
                        if let Ok(pos) = cursor_to_pos(&self.camera, ui) {
                            self.selection_end = Some(pos);
                        }
                    }
                } else if pointer_state.primary_released() {
                    // 放開時，若有 hover 則更新終點（保留 selection），但不關閉選取模式
                    if let Some(_) = self.selection_start {
                        if let Ok(pos) = cursor_to_pos(&self.camera, ui) {
                            self.selection_end = Some(pos);
                        }
                    }
                }
            } else {
                // 非選取模式：不在此處處理（正常點擊行為在下方）
            }
        }

        // 僅處理點擊（其餘編輯互動）
        if !pointer_state.primary_down() {
            return;
        }
        let Ok(painted) = cursor_to_pos(&self.camera, ui) else {
            return;
        };
        // 僅處理座標在棋盤內
        if board.get_tile(painted).is_none() {
            return;
        }

        // 修改格子
        let board = self.boards.get_mut(board_id).expect("選擇的戰場應該存在");
        let mut err_msg = String::new();
        match self.brush {
            BrushMode::Terrain => {
                paint_terrain(board, painted, self.selected_terrain);
            }
            BrushMode::Object => {
                // 若目前為「選取模式」（self.selecting == true），拖曳/點擊不會立即在地圖寫入物件（只記錄 selection）
                if self.selecting {
                    // 不在此處執行 paint；選取邏輯已在上方處理
                } else {
                    if let Err(e) = paint_object(
                        board,
                        painted,
                        self.selected_object.as_ref(),
                        self.selected_orientation,
                        self.selected_object_duration,
                    ) {
                        err_msg = format!("Error painting object: {}", e);
                    }
                }
            }
            BrushMode::Unit => {
                let marker = if let Some(template_type) = &self.selected_unit {
                    let mut rng = rand::rng();
                    // 數字太大無法存入 toml
                    // 使用 u32 max 當作 ID 上限
                    let id = rng.random_range(0..u32::MAX);
                    Some(UnitMarker {
                        id,
                        unit_template_type: template_type.clone(),
                        team: self.selected_team.clone(),
                        pos: painted,
                    })
                } else {
                    None
                };
                if let Err(e) = paint_unit(board, painted, marker) {
                    err_msg = format!("Error painting unit: {}", e);
                }
            }
            BrushMode::Deploy => {
                // 前面已經限制只有 down 可以過來
                if !pointer_state.primary_pressed() {
                    return;
                }
                if board.get_tile(painted).is_some() {
                    // 只允許在有效格子操作
                    if board.deployable.contains(&painted) {
                        board.deployable.remove(&painted);
                    } else {
                        board.deployable.insert(painted);
                    }
                    self.has_unsaved_changes = true;
                }
            }
            BrushMode::Team | BrushMode::Sim | BrushMode::None => {}
        }
        self.set_status(err_msg, true);
    }

    fn show_sim(&mut self, ui: &mut Ui) {
        // 取得當前回合角色
        let active_unit_id = match self.sim_battle.get_current_unit_id() {
            Some(id) => *id,
            None => {
                self.set_status(
                    format!(
                        "無當前回合角色，回合順序：{:?}",
                        &self.sim_battle.turn_order,
                    ),
                    true,
                );
                return;
            }
        };
        if self.sim_board.units.get(&active_unit_id).is_none() {
            self.set_status("當前回合角色不存在".to_string(), true);
            return;
        };
        let active_unit_pos = match self.sim_board.unit_to_pos(active_unit_id) {
            Some(pos) => pos,
            None => {
                self.set_status("當前單位位置不存在於棋盤上".to_string(), true);
                return;
            }
        };

        // 取得滑鼠目標座標
        let target = if let Ok(pos) = cursor_to_pos(&self.camera, ui) {
            if self.sim_board.get_tile(pos).is_some() {
                Some(pos)
            } else {
                None
            }
        } else {
            None
        };

        // 判斷是否有選擇技能
        if let Some(skill_id) = &self.skill_selection.selected_skill {
            // -------- 有選擇技能時：只顯示技能範圍 --------
            // 以繁體中文註解：只顯示技能影響範圍（座標列表），不顯示移動範圍與路徑

            // 只允許施放主動技能
            if !self
                .skills
                .get(skill_id)
                .map(|s| s.tags.contains(&Tag::Active))
                .unwrap_or(false)
            {
                self.set_status("被動技能無法施放".to_string(), true);
                return;
            }
            // -------- 有選擇主動技能時：只顯示技能範圍 --------
            let casting_area = if let Some(skill) = self.skills.get(skill_id) {
                skill_casting_area(&self.sim_board, active_unit_pos, skill.range)
            } else {
                vec![]
            };

            let affect_area = if let Some(to) = target {
                self.skill_selection.skill_affect_area(
                    &self.sim_board,
                    &self.skills,
                    active_unit_pos,
                    to,
                )
            } else {
                vec![]
            };
            show_tiles(
                ui,
                &mut self.camera,
                &self.sim_board.tiles,
                show_skill_area_others(&self.sim_board, &casting_area, &affect_area),
            );

            // 技能施放主流程
            let Some(to) = target else {
                return;
            };
            if !ui.ctx().input(|i| i.pointer.primary_clicked()) {
                return;
            }
            let unit = self
                .sim_board
                .units
                .get(&active_unit_id)
                .expect("active unit not exist");
            if let Err(e) = is_able_to_cast(unit) {
                self.set_status(e.to_string(), true);
                return;
            }
            if !casting_area.contains(&to) {
                self.set_status("技能範圍外無法施放".to_string(), true);
                return;
            }
            match self.skill_selection.cast_skill(
                &mut self.sim_board,
                &self.skills,
                active_unit_id,
                to,
            ) {
                Ok(msgs) => {
                    let msg = msgs.join("\n");
                    self.set_status(msg, false);
                }
                Err(err) => {
                    self.set_status(err.to_string(), true);
                }
            }
        } else {
            // -------- 未選擇技能時：顯示移動範圍與路徑 --------
            // 以繁體中文註解：維持原本顯示移動範圍與路徑
            let mut movable = movable_area(&self.sim_board, active_unit_pos, &self.skills);
            movable.remove(&active_unit_pos); // 移除原位置，避免自己移動到自己身上
            let movable = movable;

            // 嘗試取得滑鼠目標與路徑
            let path = if let Some(target) = target {
                reconstruct_path(&movable, active_unit_pos, target).unwrap_or_default()
            } else {
                vec![]
            };

            show_tiles(
                ui,
                &mut self.camera,
                &self.sim_board.tiles,
                show_sim_others(&self.sim_board, &movable, &active_unit_id, &path),
            );

            // 滑鼠點擊時才移動（此處不需處理技能執行）
            if let Some(target) = target {
                if !ui.ctx().input(|i| i.pointer.primary_clicked()) {
                    return;
                }
                if self.sim_board.pos_to_unit(target).is_some() {
                    self.set_status("目標位置已有單位，無法移動".to_string(), true);
                    return;
                }
                if path.is_empty() {
                    self.set_status("無法到達目標位置".to_string(), true);
                    return;
                }
                if let Err(e) = move_unit_along_path(&mut self.sim_board, path, &self.skills) {
                    self.set_status(format!("Error moving unit: {e:?}"), true);
                }
            }
        }
    }

    fn show_right_panel(&mut self, ui: &mut Ui) {
        ui.heading("編輯工具與資訊");
        ScrollArea::vertical().show(ui, |ui| {
            let mut changed = false;
            ui.horizontal_wrapped(|ui| {
                for (mode, label) in [
                    (BrushMode::None, "戰場設定"),
                    (BrushMode::Terrain, "地形筆刷"),
                    (BrushMode::Object, "物件筆刷"),
                    (BrushMode::Unit, "單位筆刷"),
                    (BrushMode::Team, "隊伍編輯"),
                    (BrushMode::Deploy, "部署格子"),
                    (BrushMode::Sim, "模擬"),
                ] {
                    if ui.selectable_label(self.brush == mode, label).clicked() {
                        if self.brush != mode {
                            changed = true;
                            // 切換 brush 時清空 selection
                            self.selecting = false;
                            self.selection_start = None;
                            self.selection_end = None;
                        }
                        self.brush = mode;
                    }
                }
            });

            match self.brush {
                BrushMode::None => {
                    self.show_board_settings(ui);
                }
                BrushMode::Terrain => {
                    self.show_terrain_brush(ui);
                }
                BrushMode::Object => {
                    self.show_object_brush(ui);
                }
                BrushMode::Unit => {
                    self.show_unit_brush(ui);
                }
                BrushMode::Team => {
                    self.show_team_settings(ui);
                }
                BrushMode::Deploy => {
                    ui.label("部署格子筆刷：點擊格子以切換是否可部署");
                }
                BrushMode::Sim => {
                    if changed {
                        self.init_sim(ui);
                    }
                    self.show_sim_status(ui);
                }
            }
        });
    }

    fn show_board_settings(&mut self, ui: &mut Ui) {
        // 棋盤格子數調整
        let Some(board_id) = &self.selected_board else {
            ui.label("請先選擇戰場");
            return;
        };
        let board = self.boards.get_mut(board_id).expect("選擇的戰場應該存在");
        let mut rows = board.tiles.len();
        let mut cols = board.tiles.get(0).map(|row| row.len()).unwrap_or(0);
        let mut changed = false;
        ui.label("棋盤格子數");
        if ui.add(DragValue::new(&mut rows).prefix("行: ")).changed() {
            changed = true;
        }
        if ui.add(DragValue::new(&mut cols).prefix("列: ")).changed() {
            changed = true;
        }

        if changed {
            // 取得預設 Tile
            let default_tile = Tile::default();
            // 調整行數
            if rows > board.tiles.len() {
                for _ in board.tiles.len()..rows {
                    board.tiles.push(vec![default_tile.clone(); cols.max(1)]);
                }
            } else if rows < board.tiles.len() {
                board.tiles.truncate(rows);
            }
            // 調整每一行的列數
            for row in &mut board.tiles {
                if cols > row.len() {
                    row.extend(std::iter::repeat(default_tile.clone()).take(cols - row.len()));
                } else if cols < row.len() {
                    row.truncate(cols);
                }
            }
            self.has_unsaved_changes = true;
        }
    }

    fn show_terrain_brush(&mut self, ui: &mut Ui) {
        for terrain in Terrain::iter() {
            if ui
                .selectable_label(self.selected_terrain == terrain, terrain.to_string())
                .clicked()
            {
                self.selected_terrain = terrain;
            }
        }
    }

    fn show_object_brush(&mut self, ui: &mut Ui) {
        ui.separator();
        if ui.button("清除所有物件").clicked() {
            self.clear_all_objects(ui);
        }

        ui.separator();

        let can_fill = self.selection_start.is_some() && self.selection_end.is_some();
        if let (Some(board_id), Some(start), Some(end)) = (
            &self.selected_board,
            self.selection_start,
            self.selection_end,
        ) {
            let board = self.boards.get_mut(board_id).expect("選擇的戰場應該存在");
            let mut status_msg = None;
            if ui
                .add_enabled(can_fill, Button::new("填滿選取範圍（僅支援單格物件）"))
                .clicked()
            {
                match fill_selected_area(
                    board,
                    start,
                    end,
                    self.selected_object.as_ref(),
                    self.selected_orientation,
                    self.selected_object_duration,
                ) {
                    Ok((success, skipped)) => {
                        self.has_unsaved_changes = true;
                        status_msg = Some((
                            format!("填滿完成：成功 {} 格，跳過 {} 格", success, skipped),
                            skipped > 0,
                        ));
                    }
                    Err(err) => {
                        status_msg = Some((format!("填滿失敗：{}", err), true));
                    }
                }
            }
            ui.horizontal(|ui| {
                ui.label("Drunkard's Walk 步數:");
                ui.add(DragValue::new(&mut self.drunkards_steps).speed(1));
            });
            ui.horizontal(|ui| {
                ui.label("權重 上:");
                ui.add(
                    DragValue::new(&mut self.drunkards_weights[0])
                        .speed(1)
                        .range(0..=100),
                );
                ui.label("下:");
                ui.add(
                    DragValue::new(&mut self.drunkards_weights[1])
                        .speed(1)
                        .range(0..=100),
                );
                ui.label("左:");
                ui.add(
                    DragValue::new(&mut self.drunkards_weights[2])
                        .speed(1)
                        .range(0..=100),
                );
                ui.label("右:");
                ui.add(
                    DragValue::new(&mut self.drunkards_weights[3])
                        .speed(1)
                        .range(0..=100),
                );
            });
            ui.horizontal(|ui| {
                ui.label("起始 Y:");
                let mut y = self.drunkards_start_y.unwrap_or(0);
                // 計算範圍限制
                let (min_y, max_y) =
                    if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
                        let min_y = start.y.min(end.y);
                        let max_y = start.y.max(end.y);
                        (min_y, max_y)
                    } else {
                        (0, usize::MAX)
                    };
                let mut changed = false;
                if ui.button("-").clicked() {
                    y = y.saturating_sub(1);
                    changed = true;
                }
                changed |= ui.add(DragValue::new(&mut y).speed(1)).changed();
                if ui.button("+").clicked() {
                    y += 1;
                    changed = true;
                }
                // 限制 y 的值在範圍內
                y = y.clamp(min_y, max_y);
                if changed {
                    self.drunkards_start_y = Some(y);
                }
                if ui.button("隨機").clicked() {
                    self.drunkards_start_y = None;
                }
            });
            if ui
                .add_enabled(can_fill, Button::new("Drunkard's Walk 填滿選取範圍"))
                .clicked()
            {
                match drunkards_walk_deploy(
                    board,
                    start,
                    end,
                    self.selected_object.as_ref(),
                    self.selected_orientation,
                    self.selected_object_duration,
                    self.drunkards_steps,
                    self.drunkards_start_y,
                    self.drunkards_weights,
                ) {
                    Ok(placed) => {
                        self.has_unsaved_changes = true;
                        status_msg = Some((
                            format!("Drunkard's Walk 部署完成：放置 {} 個物件", placed),
                            false,
                        ));
                    }
                    Err(err) => {
                        status_msg = Some((format!("Drunkard's Walk 部署失敗：{}", err), true));
                    }
                }
            }
            if let Some((msg, is_error)) = status_msg {
                self.set_status(msg, is_error);
            }
        }
        if !can_fill {
            ui.label(RichText::new("請拖曳選取範圍後再填滿").color(Color32::GRAY));
        }

        ui.separator();

        thread_local! {
            static SHIFT_DX: RefCell<i32> = RefCell::new(0);
            static SHIFT_DY: RefCell<i32> = RefCell::new(0);
        }
        let mut dx = SHIFT_DX.with(|d| *d.borrow());
        let mut dy = SHIFT_DY.with(|d| *d.borrow());
        ui.horizontal(|ui| {
            ui.label("平移 dx:");
            if ui.add(DragValue::new(&mut dx).speed(1)).changed() {
                SHIFT_DX.with(|d| *d.borrow_mut() = dx);
            }
            ui.label("dy:");
            if ui.add(DragValue::new(&mut dy).speed(1)).changed() {
                SHIFT_DY.with(|d| *d.borrow_mut() = dy);
            }
            if ui.button("平移物件").clicked() {
                self.shift_objects(ui, dx, dy);
            }
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("放置方向:");
            for orientation in Orientation::iter() {
                if ui
                    .selectable_label(
                        self.selected_orientation == orientation,
                        orientation.to_string(),
                    )
                    .clicked()
                {
                    self.selected_orientation = orientation;
                }
            }
        });
        if ui.selectable_label(self.selecting, "選取").clicked() {
            self.selecting = !self.selecting;
            self.selection_start = None;
            self.selection_end = None;
        }
        if ui
            .selectable_label(self.selected_object.is_none(), "清除")
            .clicked()
        {
            self.selected_object = None;
        }
        for object in Object::iter() {
            if ui
                .selectable_label(
                    self.selected_object.as_ref() == Some(&object),
                    object.to_string(),
                )
                .clicked()
            {
                self.selected_object = Some(object);
            }
        }
    }

    fn show_unit_brush(&mut self, ui: &mut Ui) {
        // 先顯示 team 選擇 UI
        let Some(board_id) = &self.selected_board else {
            return;
        };
        let board = self.boards.get(board_id).expect("選擇的戰場應該存在");
        let team_ids = board.teams.keys().cloned().collect::<Vec<_>>();

        ui.vertical(|ui| {
            ui.label("選擇隊伍 TeamID:");
            // 下拉選單
            let mut selected_idx = team_ids
                .iter()
                .position(|id| id == &self.selected_team)
                .unwrap_or(0);
            egui::ComboBox::from_id_salt("team_select_combo")
                .selected_text(
                    team_ids
                        .get(selected_idx)
                        .map(|s| s.as_str())
                        .unwrap_or("請選擇隊伍"),
                )
                .show_ui(ui, |ui| {
                    for (i, id) in team_ids.iter().enumerate() {
                        let response = ui.selectable_value(&mut selected_idx, i, id);
                        if !response.changed() {
                            continue;
                        }
                        let Some(new_id) = team_ids.get(selected_idx) else {
                            continue;
                        };
                        self.selected_team = new_id.clone();
                    }
                });
        });

        ui.separator();

        if ui
            .selectable_label(self.selected_unit.is_none(), "清除")
            .clicked()
        {
            self.selected_unit = None;
            return;
        }
        for (template, _) in &self.unit_templates {
            if ui
                .selectable_label(
                    self.selected_unit.as_ref().map_or(false, |t| t == template),
                    template,
                )
                .clicked()
            {
                self.selected_unit = Some(template.clone());
            }
        }
    }

    fn show_team_settings(&mut self, ui: &mut Ui) {
        let Some(board_id) = &self.selected_board else {
            ui.label("請先選擇戰場");
            return;
        };
        let board = self.boards.get_mut(board_id).expect("選擇的戰場應該存在");

        ui.heading("隊伍設定");

        // 列出所有現有 teams
        let existing_team_ids: HashSet<TeamID> = board.teams.keys().cloned().collect();
        let mut to_delete: Option<TeamID> = None;
        let mut edited_team: Option<(TeamID, TeamID)> = None;
        for (team_id, team_cfg) in board.teams.iter_mut() {
            ui.horizontal(|ui| {
                // 可編輯的 TeamID
                let mut id_buf = team_id.clone();
                let resp = ui.text_edit_singleline(&mut id_buf);
                if resp.changed()
                    && !id_buf.is_empty()
                    && id_buf != *team_id
                    && !existing_team_ids.contains(&id_buf)
                {
                    edited_team = Some((team_id.clone(), id_buf.clone()));
                }
                // 顏色編輯器
                let mut color32 = to_egui_color(team_cfg.color);
                if ui.color_edit_button_srgba(&mut color32).changed() {
                    team_cfg.color = to_team_color(color32);
                    self.has_unsaved_changes = true;
                }
                if ui.button("刪除").clicked() {
                    to_delete = Some(team_id.clone());
                }
            });
        }
        // 處理重新命名
        if let Some((old_id, new_id)) = edited_team {
            if let Some(team) = board.teams.remove(&old_id) {
                board.teams.insert(new_id.clone(), team);
                // 更新 selected_team
                if self.selected_team == old_id {
                    self.selected_team = new_id.clone();
                }
                // 更新 units 中的 team
                for unit in board.units.values_mut() {
                    if unit.team == old_id {
                        unit.team = new_id.clone();
                    }
                }
                self.has_unsaved_changes = true;
            }
        }
        // 處理刪除
        if let Some(team_id) = to_delete {
            // 檢查是否有單位屬於該隊伍
            board.units.retain(|_, u| u.team != team_id);
            board.teams.remove(&team_id);
            // 更新 selected_team 如果刪除的是當前選擇的
            if self.selected_team == team_id {
                self.selected_team = board.teams.keys().next().cloned().unwrap_or_default();
            }
            self.has_unsaved_changes = true;
        }

        ui.separator();
        ui.label("新增隊伍");

        // 新增 team 的輸入欄
        thread_local! {
            static NEW_TEAM_ID: RefCell<String> = RefCell::new(String::new());
            static NEW_TEAM_COLOR: RefCell<Color32> = RefCell::new(Color32::LIGHT_GRAY);
        }

        let mut new_team_id = NEW_TEAM_ID.with(|id| id.borrow().clone());
        let mut new_team_color = NEW_TEAM_COLOR.with(|c| *c.borrow());

        let id_changed = ui.text_edit_singleline(&mut new_team_id).changed();
        let color_changed = ui.color_edit_button_srgba(&mut new_team_color).changed();

        if id_changed {
            NEW_TEAM_ID.with(|id| *id.borrow_mut() = new_team_id.clone());
        }
        if color_changed {
            NEW_TEAM_COLOR.with(|c| *c.borrow_mut() = new_team_color);
        }

        if ui.button("新增").clicked() {
            if !new_team_id.is_empty() && !board.teams.contains_key(&new_team_id) {
                let rgb = (new_team_color.r(), new_team_color.g(), new_team_color.b());
                board.teams.insert(
                    new_team_id.clone(),
                    Team {
                        id: new_team_id.clone(),
                        color: (rgb.0, rgb.1, rgb.2),
                    },
                );
                self.has_unsaved_changes = true;
                NEW_TEAM_ID.with(|id| *id.borrow_mut() = String::new());
                NEW_TEAM_COLOR.with(|c| *c.borrow_mut() = Color32::LIGHT_GRAY);
            }
        }
    }

    fn init_sim(&mut self, ui: &mut Ui) {
        self.sim_board = Board::default();
        self.sim_battle = Battle::default();
        // 1. 取得目前選擇的 BoardConfig
        let board_id = match &self.selected_board {
            None => {
                ui.label("請先選擇戰場");
                return;
            }
            Some(board_id) => board_id,
        };
        let config = self.boards.get(board_id).expect("選擇的戰場應該存在");

        // 2. 呼叫 Board::from_config
        let m = UnitTemplateMap(&self.unit_templates);
        match Board::from_config(config.clone(), &m, &self.skills) {
            Ok(mut board) => {
                let progression = match self.player_progression.boards.get(board_id) {
                    Some(p) => p,
                    None => {
                        self.set_status(
                            format!("玩家進度資料中沒有此戰場的資料: {}", board_id),
                            true,
                        );
                        return;
                    }
                };
                if let Err(msg) =
                    override_player_unit(&mut board, &progression.roster, &self.skills)
                {
                    eprintln!("Error overriding player units: {}", msg);
                    self.set_status(msg, true);
                    return;
                }
                // 依照 calc_initiative 隨機排序單位
                let mut rng = rand::rng();
                let turn_order = board
                    .units
                    .iter()
                    .map(|(&id, unit)| {
                        let skill_refs = unit
                            .skills
                            .iter()
                            .filter_map(|sid| self.skills.get(sid).map(|s| (sid, s)));
                        let ini = calc_initiative(&mut rng, skill_refs);
                        (id, ini)
                    })
                    .collect::<Vec<_>>();
                let mut turn_order = turn_order;
                turn_order.sort_by(|a, b| b.1.cmp(&a.1)); // 由大到小排序
                let turn_order = turn_order.into_iter().map(|(id, _)| id).collect();

                self.sim_board = board;
                self.sim_battle = Battle::new(turn_order);
                self.set_status(
                    format!("轉換成功: BoardConfig 已成功轉換為 Board，並覆蓋玩家單位內容。"),
                    false,
                );
            }
            Err(e) => {
                self.set_status(format!("轉換失敗：{}", e), true);
            }
        }
    }

    fn show_sim_status(&mut self, ui: &mut Ui) {
        let unit_id = match self.sim_battle.get_current_unit_id() {
            Some(&id) => id,
            None => {
                self.set_status(
                    format!(
                        "無當前回合角色，回合順序：{:?}",
                        &self.sim_battle.turn_order,
                    ),
                    true,
                );
                return;
            }
        };
        let unit = match self.sim_board.units.get(&unit_id) {
            Some(unit) => unit,
            None => {
                self.set_status("當前回合角色不存在".to_string(), true);
                return;
            }
        };
        ui.label(format!("當前行動單位種類: {}", unit.unit_template_type));

        // 以繁體中文註解：只顯示單位擁有的技能列表
        // 顯示單位擁有的技能列表
        ui.label("單位擁有的技能：");
        // 技能選擇下拉選單（無「未選擇技能」選項），selected_idx = -1 表示未選擇技能
        // 依主/被動分類技能顯示
        let active_skill_ids: Vec<&SkillID> = unit
            .skills
            .iter()
            .filter(|id| {
                self.skills
                    .get(*id)
                    .map(|s| s.tags.contains(&Tag::Active))
                    .unwrap_or(false)
            })
            .collect();
        // 滑鼠右鍵點擊時取消技能選取
        if ui.ctx().input(|i| i.pointer.secondary_clicked()) {
            self.skill_selection.select_skill(None);
        }
        let mut selected_idx = self
            .skill_selection
            .selected_skill
            .as_ref()
            .and_then(|id| active_skill_ids.iter().position(|x| x == &id))
            .map(|idx| idx as i32)
            .unwrap_or(-1);
        egui::ComboBox::from_id_salt("unit_skill_select_combo")
            .selected_text(if selected_idx >= 0 {
                active_skill_ids
                    .get(selected_idx as usize)
                    .map(|s| s.as_str())
                    .unwrap_or("")
            } else {
                ""
            })
            .show_ui(ui, |ui| {
                ui.label("─── 主動技能 ───");
                for (i, name) in active_skill_ids.iter().enumerate() {
                    let response = ui.selectable_value(&mut selected_idx, i as i32, *name);
                    if response.changed() {
                        if selected_idx >= 0 {
                            if let Some(skill_id) = active_skill_ids.get(selected_idx as usize) {
                                self.skill_selection
                                    .select_skill(Some(skill_id.to_string()));
                            }
                        }
                    }
                }
            });

        // 顯示結束回合按鈕
        ui.separator();
        self.end_turn_button(ui);

        ui.separator();

        // AI 評分按鈕
        if ui.button("AI 評分 (score_actions)").clicked() {
            let result = score_actions(&self.sim_board, &self.skills, &self.ai_config, unit_id);
            match result {
                Ok(actions) => {
                    let mut s = String::new();
                    for (i, a) in actions.iter().enumerate() {
                        s.push_str(&format!(
                            "[{}] {:?}\n分數: {:.2}\n原因: {}\n\n",
                            i + 1,
                            a.action,
                            a.score,
                            a.reason
                        ));
                    }
                    self.ai_score_result = Some(s);
                }
                Err(e) => {
                    self.ai_score_result = Some(format!("AI 評分失敗: {:?}", e));
                }
            }
        }

        if let Some(result) = &self.ai_score_result {
            ui.separator();
            ui.label("AI 評分結果：");
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    ui.label(result);
                });
        }
    }

    /// 在模擬模式下顯示「結束回合」按鈕，並切換到下一角色
    fn end_turn_button(&mut self, ui: &mut Ui) {
        if ui.button("結束回合").clicked() {
            self.sim_battle
                .next_turn(&mut self.sim_board, &mut self.skill_selection);
            self.set_status("已結束回合，切換到下一角色".to_string(), false);
        }
    }

    fn clear_all_objects(&mut self, ui: &mut Ui) {
        let board_id = match &self.selected_board {
            None => {
                ui.label("請先選擇戰場");
                return;
            }
            Some(board_id) => board_id,
        };
        let board = self.boards.get_mut(board_id).expect("選擇的戰場應該存在");
        clear_all_objects(board);
        self.has_unsaved_changes = true;
        self.set_status("已清除所有物件".to_string(), false);
    }

    fn shift_objects(&mut self, ui: &mut Ui, dx: i32, dy: i32) {
        let board_id = match &self.selected_board {
            None => {
                ui.label("請先選擇戰場");
                return;
            }
            Some(board_id) => board_id,
        };
        let board = self.boards.get_mut(board_id).expect("選擇的戰場應該存在");

        // 收集所有物件位置
        let mut objects = vec![];
        for (y, row) in board.tiles.iter().enumerate() {
            for (x, tile) in row.iter().enumerate() {
                if tile.object.is_some() {
                    objects.push((Pos { x, y }, tile.object.clone()));
                }
            }
        }

        // 清除所有物件
        clear_all_objects(board);

        // 重新放置到新位置
        for (pos, obj) in objects {
            let new_x = pos.x as i32 + dx;
            let new_y = pos.y as i32 + dy;
            if new_x >= 0
                && new_y >= 0
                && (new_x as usize) < board.width()
                && (new_y as usize) < board.height()
            {
                let new_pos = Pos {
                    x: new_x as usize,
                    y: new_y as usize,
                };
                if let Some(tile) = board.get_tile_mut(new_pos) {
                    tile.object = obj;
                }
            }
            // 如果超出邊界，忽略
        }

        self.has_unsaved_changes = true;
        self.set_status(format!("已平移物件 dx={}, dy={}", dx, dy), false);
    }

    /// 驗證所有戰場中的單位是否都對應到有效的單位模板
    /// 如果有無效單位，返回錯誤訊息，否則返回 None
    fn validate_units(&self) -> Option<String> {
        let mut invalid_units = Vec::new();
        for (board_id, board) in &self.boards {
            for (unit_id, unit_marker) in &board.units {
                if !self
                    .unit_templates
                    .contains_key(&unit_marker.unit_template_type)
                {
                    invalid_units.push(format!(
                        "戰場 '{}' 中的單位 ID {} (類型: {})",
                        board_id, unit_id, unit_marker.unit_template_type
                    ));
                }
            }
        }
        if invalid_units.is_empty() {
            None
        } else {
            Some(invalid_units.join(", "))
        }
    }

    fn show_status_message(&mut self, ctx: &Context) {
        if let Some((message, is_error)) = &self.status_message {
            show_status_message(ctx, message, *is_error);
        }
    }

    fn set_status(&mut self, msg: String, is_error: bool) {
        self.status_message = Some((msg, is_error));
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.has_unsaved_changes
    }
}

fn load_boards<P: AsRef<std::path::Path>>(path: P) -> io::Result<BTreeMap<BoardID, BoardConfig>> {
    from_file(path)
}

fn save_boards<P: AsRef<std::path::Path>>(
    path: P,
    boards: &BTreeMap<BoardID, BoardConfig>,
) -> io::Result<()> {
    to_file(path, boards)
}

/// 匯出所有 board 為 boards/{id}.toml 檔案，不更動原本設定檔
fn export_boards_to_files(boards: &BTreeMap<BoardID, BoardConfig>) -> Result<usize, String> {
    if let Err(e) = std::fs::create_dir_all(boards_separate_dir()) {
        return Err(format!("建立目錄失敗: {}", e));
    }
    let mut count = 0;
    for (id, board) in boards {
        let file_path = boards_separate_dir().join(format!("{}.toml", id));
        match to_file(&file_path, board) {
            Ok(_) => count += 1,
            Err(e) => return Err(format!("寫入 {} 失敗: {}", file_path.display(), e)),
        }
    }
    Ok(count)
}

// - draw_tile_border_and_background: 負責邊框與地形底色
fn draw_tile_border_and_background(painter: &Painter, rect: Rect, tile: &Tile, zoom: f32) {
    // 畫 tile 邊框與地形背景
    painter.rect_filled(rect, 2.0, Color32::BLACK);
    painter.rect_filled(
        rect.shrink(TILE_SHRINK_SIZE * zoom),
        2.0,
        terrain_color(tile),
    );
}

// - draw_tile_object: 負責在格子上繪製物件符號（若有）
fn draw_tile_object(painter: &Painter, rect: Rect, tile: &Tile, zoom: f32) {
    // 畫 tile object（若有）
    let symbol = object_symbol(tile);
    if symbol.is_empty() {
        return;
    }
    let o_len = symbol.len() as f32;
    painter.text(
        rect.center(),
        Align2::CENTER_TOP,
        symbol,
        FontId::proportional(TILE_OBJECT_SIZE / o_len * zoom),
        Color32::WHITE,
    );
}

fn show_tiles(
    ui: &mut Ui,
    camera: &mut Camera2D,
    tiles: &[Vec<Tile>],
    show_others: impl Fn(&Painter, &Camera2D, Pos, Rect),
) {
    // 處理攝影機平移與縮放
    camera.handle_pan_zoom(ui);
    camera.handle_keyboard_zoom(ui);

    // 先繪製格子內容
    let painter = ui.painter();
    let zoom = camera.zoom;
    let tile_size = vec2(TILE_SIZE, TILE_SIZE) * zoom;
    for (row_idx, row) in tiles.iter().enumerate() {
        for (col_idx, tile) in row.iter().enumerate() {
            // 計算世界座標與螢幕矩形
            let world_pos = Pos2::new(col_idx as f32, row_idx as f32) * TILE_SIZE;
            let screen_pos = camera.world_to_screen(world_pos);
            let rect = Rect::from_min_size(screen_pos, tile_size);

            // 辦理格子底色與邊框
            draw_tile_border_and_background(painter, rect, tile, zoom);

            // 顯示單位或其他 overlay（由呼叫端提供）
            let pos = Pos {
                x: col_idx,
                y: row_idx,
            };
            show_others(painter, camera, pos, rect);

            // 繪製格子上的物件（若有）
            draw_tile_object(painter, rect, tile, zoom);
        }
    }
}

fn show_static_others(board: &BoardConfig) -> impl Fn(&Painter, &Camera2D, Pos, Rect) {
    // 回傳一個 closure 供 show_tiles 呼叫，負責在 BoardConfig 上繪製非地形的 overlay（例如部署格與單位）
    |painter, camera, pos, rect| {
        // 顯示部署格子（黃色圓圈）
        if board.deployable.contains(&pos) {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "★", // 部署格子符號
                FontId::proportional(TILE_DEPLOYABLE_SIZE * camera.zoom),
                Color32::LIGHT_BLUE,
            );
        }
        // 顯示單位（BoardConfig 沒有 pos_to_unit，因此以線性搜尋）
        draw_unit_on_config(board, painter, camera, pos, rect);
    }
}

/// 針對 Board 的單位繪製共用實作
fn draw_unit_on_board(board: &Board, painter: &Painter, camera: &Camera2D, pos: Pos, rect: Rect) {
    let Some(unit_id) = board.pos_to_unit(pos) else {
        // 該位置沒有單位
        return;
    };
    let (unit_template, team) = board
        .units
        .get(&unit_id)
        .map_or(("", ""), |u| (&u.unit_template_type, &u.team));
    let team_color = board
        .teams
        .get(team)
        .map_or(Color32::WHITE, |team| to_egui_color(team.color));
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        unit_symbol(&unit_template).as_ref(),
        FontId::proportional(TILE_UNIT_SIZE * camera.zoom),
        team_color,
    );
}

/// 針對 BoardConfig 的單位繪製（BoardConfig 沒有 pos_to_unit helper，因此以 values() 找到對應 pos）
fn draw_unit_on_config(
    board: &BoardConfig,
    painter: &Painter,
    camera: &Camera2D,
    pos: Pos,
    rect: Rect,
) {
    let (unit_template, team) = match board
        .units
        .values()
        .find(|u| u.pos == pos)
        .map(|u| (&u.unit_template_type, &u.team))
    {
        None => return, // 該位置沒有單位
        Some(v) => v,
    };
    let team_color = board
        .teams
        .get(team)
        .map_or(Color32::WHITE, |team| to_egui_color(team.color));
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        unit_symbol(&unit_template).as_ref(),
        FontId::proportional(TILE_UNIT_SIZE * camera.zoom),
        team_color,
    );
}

/// 顯示模擬模式下的 overlay：包含可移動範圍高亮與單位繪製
fn show_sim_others(
    board: &Board,
    movable: &HashMap<Pos, (MovementCost, Pos)>,
    active_unit_id: &UnitID,
    path: &[Pos],
) -> impl Fn(&Painter, &Camera2D, Pos, Rect) {
    // 回傳 closure 給 show_tiles 使用
    |painter, camera, pos, rect| {
        // 可移動範圍（若該 pos 有 movement info，則畫半透明顏色）
        if let Ok(color_tuple) = movement_tile_color(board, movable, active_unit_id, path, pos) {
            let color = Color32::from_rgba_premultiplied(
                color_tuple.0,
                color_tuple.1,
                color_tuple.2,
                color_tuple.3,
            );
            painter.rect_filled(
                rect.shrink(TILE_ACTION_SHRINK_SIZE * camera.zoom),
                2.0,
                color,
            );
        }

        // 若此格為目前行動單位的位置，使用中空描邊圓（隊伍顏色）
        if let Some(unit_id) = board.pos_to_unit(pos) {
            if unit_id == *active_unit_id {
                // 取得該單位的隊伍顏色，若找不到則回退為紅色
                let team_color = board
                    .units
                    .get(&unit_id)
                    .and_then(|u| board.teams.get(&u.team))
                    .map(|team| to_egui_color(team.color))
                    .unwrap_or(Color32::RED);
                painter.circle_stroke(
                    rect.center(),
                    TILE_ACTIVE_UNIT_MARKER_SIZE * camera.zoom,
                    Stroke::new(
                        TILE_ACTIVE_UNIT_MARKER_STROKE_WIDTH * camera.zoom,
                        team_color,
                    ),
                );
            }
        }

        // 顯示單位（使用共用函式）
        draw_unit_on_board(board, painter, camera, pos, rect);
    }
}

/// 顯示技能範圍的 overlay：高亮施放/預覽區域，並繪製單位
fn show_skill_area_others(
    board: &Board,
    casting_area: &[Pos],
    skill_area: &[Pos],
) -> impl Fn(&Painter, &Camera2D, Pos, Rect) {
    |painter, camera, pos, rect| {
        // 技能預覽區域高亮（紅色半透明）
        let color = if skill_area.contains(&pos) {
            Some(Color32::from_rgba_premultiplied(255, 0, 0, 80))
        }
        // 技能可施放範圍高亮（藍色半透明）
        else if casting_area.contains(&pos) {
            Some(Color32::from_rgba_premultiplied(0, 128, 255, 80))
        } else {
            None
        };
        if let Some(color) = color {
            painter.rect_filled(
                rect.shrink(TILE_ACTION_SHRINK_SIZE * camera.zoom),
                2.0,
                color,
            );
        }
        // 顯示單位（使用共用函式）
        draw_unit_on_board(board, painter, camera, pos, rect);
    }
}

/// 繪製選取範圍的外框線
fn draw_selection_outline(
    painter: &Painter,
    camera: &Camera2D,
    selection_start: Pos,
    selection_end: Pos,
) {
    // 計算選取範圍的邊界
    let min_x = selection_start.x.min(selection_end.x);
    let max_x = selection_start.x.max(selection_end.x);
    let min_y = selection_start.y.min(selection_end.y);
    let max_y = selection_start.y.max(selection_end.y);

    // 計算矩形在螢幕上的位置
    let top_left_world = Pos2::new(min_x as f32, min_y as f32) * TILE_SIZE;
    let bottom_right_world = Pos2::new((max_x + 1) as f32, (max_y + 1) as f32) * TILE_SIZE;

    let top_left_screen = camera.world_to_screen(top_left_world);
    let bottom_right_screen = camera.world_to_screen(bottom_right_world);

    let selection_rect = Rect::from_min_max(top_left_screen, bottom_right_screen);

    // 繪製細框線（白色，2px 寬度）
    painter.rect_stroke(
        selection_rect,
        0.0, // 圓角
        Stroke::new(2.0, Color32::WHITE),
        egui::StrokeKind::Middle,
    );
}

fn draw_drunkard_start_indicator(
    painter: &Painter,
    camera: &Camera2D,
    board: &BoardConfig,
    start: Pos,
    end: Pos,
    start_y: usize,
) {
    let min_x = start.x.min(end.x);
    let min_y = start.y.min(end.y);
    let max_y = start.y.max(end.y);
    let clamped_y = start_y.clamp(min_y, max_y);
    let start_pos = Pos {
        x: min_x,
        y: clamped_y,
    };
    if board.get_tile(start_pos).is_some() {
        let world_pos = Pos2::new(start_pos.x as f32, start_pos.y as f32) * TILE_SIZE;
        let screen_pos = camera.world_to_screen(world_pos);
        let rect = Rect::from_min_size(screen_pos, vec2(TILE_SIZE, TILE_SIZE) * camera.zoom);
        painter.circle_filled(
            rect.center(),
            TILE_ACTIVE_UNIT_MARKER_SIZE * camera.zoom,
            Color32::RED,
        );
    }
}

fn cursor_to_pos(camera: &Camera2D, ui: &mut Ui) -> Result<Pos, String> {
    // 僅當滑鼠在面板內才偵測 hover
    if !ui.rect_contains_pointer(ui.max_rect()) {
        return Err("滑鼠不在面板內".into());
    }
    let Some(pointer_pos) = ui.ctx().pointer_hover_pos() else {
        return Err("無法獲取滑鼠位置".into());
    };
    // 反推回世界座標
    let world_pointer = camera.screen_to_world(pointer_pos);
    let tile_x = (world_pointer.x / TILE_SIZE).floor() as usize;
    let tile_y = (world_pointer.y / TILE_SIZE).floor() as usize;
    let painted = Pos {
        x: tile_x,
        y: tile_y,
    };
    Ok(painted)
}

fn paint_terrain(board: &mut BoardConfig, pos: Pos, terrain: Terrain) -> bool {
    board
        .get_tile_mut(pos)
        .unwrap_or_else(|| panic!("painting in race condition. in {pos:?}"))
        .terrain = terrain;
    return true;
}

fn paint_object(
    board: &mut BoardConfig,
    pos: Pos,
    object: Option<&Object>,
    orientation: Orientation,
    duration: u32,
) -> Result<(), String> {
    // helper: 放置單格物件
    let mut apply_single = |obj: Option<Object>| {
        board
            .get_tile_mut(pos)
            .unwrap_or_else(|| panic!("painting in race condition. {:?} in {:?}", obj, pos))
            .object = obj.clone();
        Ok(())
    };

    match object {
        None => apply_single(None),
        Some(obj) => match obj {
            Object::Wall => apply_single(Some(Object::Wall)),
            Object::Tree => apply_single(Some(Object::Tree)),
            Object::Cliff { .. } => apply_single(Some(Object::Cliff { orientation })),
            Object::Tent2 { .. } => {
                let (w, h) = match orientation {
                    Orientation::Horizontal => (2, 1),
                    Orientation::Vertical => (1, 2),
                };
                paint_multiple_object(board, pos, (w, h), |rel| Object::Tent2 {
                    orientation,
                    rel,
                    duration,
                })
            }
            Object::Tent15 { .. } => {
                let (w, h) = match orientation {
                    Orientation::Horizontal => (5, 3),
                    Orientation::Vertical => (3, 5),
                };
                paint_multiple_object(board, pos, (w, h), |rel| Object::Tent15 {
                    orientation,
                    rel,
                    duration,
                })
            }
        },
    }
}

fn paint_multiple_object<F>(
    board: &mut BoardConfig,
    main_pos: Pos,
    size: (usize, usize),
    make_object: F,
) -> Result<(), String>
where
    F: Fn(Pos) -> Object,
{
    // 整理要放到哪些格子
    let (w, h) = size;
    let mut positions = Vec::new();
    for dx in 0..w {
        for dy in 0..h {
            positions.push(Pos {
                x: main_pos.x + dx,
                y: main_pos.y + dy,
            });
        }
    }

    // 檢查
    for pos in &positions {
        let Some(tile) = board.get_tile(*pos) else {
            return Err("some tiles are out of bounds".to_string());
        };
        if tile.object.is_some() {
            return Err("some tiles already have objects".to_string());
        }
    }

    // 放置物件
    for pos in &positions {
        let tile = board.get_tile_mut(*pos).expect("just checked");
        let rel = Pos {
            x: pos.x - main_pos.x,
            y: pos.y - main_pos.y,
        };
        tile.object = Some(make_object(rel));
    }
    Ok(())
}

fn paint_unit(board: &mut BoardConfig, pos: Pos, unit: Option<UnitMarker>) -> Result<(), String> {
    match unit {
        Some(unit) => {
            if board.units.contains_key(&unit.id) {
                return Err(format!("單位 ID 已存在: {:?}", unit.id));
            }
            if board.units.values().any(|marker| marker.pos == pos) {
                return Err(format!("該位置已經有單位: {:?}", pos));
            }
            board.units.insert(unit.id, unit);
        }
        None => {
            // 只移除第一個在該 pos 的單位
            if let Some(id) =
                board.units.iter().find_map(
                    |(id, marker)| {
                        if marker.pos == pos { Some(*id) } else { None }
                    },
                )
            {
                board.units.remove(&id);
            }
        }
    }
    Ok(())
}

fn clear_all_objects(board: &mut BoardConfig) {
    for row in &mut board.tiles {
        for tile in row {
            tile.object = None;
        }
    }
}

/// 填滿選取範圍（僅支援單格物件 Wall/Tree）
fn fill_selected_area(
    board: &mut BoardConfig,
    rect_start: Pos,
    rect_end: Pos,
    object: Option<&Object>,
    orientation: Orientation,
    _duration: u32,
) -> Result<(usize, usize), String> {
    let (min_x, max_x) = if rect_start.x <= rect_end.x {
        (rect_start.x, rect_end.x)
    } else {
        (rect_end.x, rect_start.x)
    };
    let (min_y, max_y) = if rect_start.y <= rect_end.y {
        (rect_start.y, rect_end.y)
    } else {
        (rect_end.y, rect_start.y)
    };

    let mut success = 0;
    let mut skipped = 0;
    let mut apply_single = |pos, object| {
        if let Some(tile) = board.get_tile_mut(pos) {
            tile.object = object;
            success += 1;
        } else {
            skipped += 1;
        }
    };

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let pos = Pos { x, y };
            // 根據 object 類型分別處理
            match object {
                None => apply_single(pos, None),
                Some(obj) => match obj {
                    Object::Wall | Object::Tree => apply_single(pos, object.cloned()),
                    Object::Cliff { .. } => apply_single(pos, Some(Object::Cliff { orientation })),
                    Object::Tent2 { .. } | Object::Tent15 { .. } => {
                        // 其他物件類型暫不支援
                        return Err(format!("不支援多格物件類型：{:?}", obj));
                    }
                },
            }
        }
    }
    Ok((success, skipped))
}

/// 使用 drunkard's walk 演算法在選取範圍內部署物件
/// 從最左側指定或隨機起點開始，偏向右走，放置單格物件
fn drunkards_walk_deploy(
    board: &mut BoardConfig,
    rect_start: Pos,
    rect_end: Pos,
    object: Option<&Object>,
    orientation: Orientation,
    duration: u32,
    steps: usize,
    start_y: Option<usize>,
    weights: [usize; 4],
) -> Result<usize, String> {
    // 僅支援單格物件或清除
    match object {
        Some(obj) => match obj {
            Object::Wall | Object::Tree => {}
            _ => return Err("Drunkard's walk 僅支援單格物件或清除".to_string()),
        },
        None => {} // 允許清除
    }

    let (min_x, max_x) = if rect_start.x <= rect_end.x {
        (rect_start.x, rect_end.x)
    } else {
        (rect_end.x, rect_start.x)
    };
    let (min_y, max_y) = if rect_start.y <= rect_end.y {
        (rect_start.y, rect_end.y)
    } else {
        (rect_end.y, rect_start.y)
    };

    let mut rng = rand::rng();
    // 從最左側開始，指定或隨機選擇起點 y，在選取範圍內
    let start_x = min_x;
    let start_y = start_y.map_or_else(
        || rng.random_range(min_y..=max_y),
        |y| y.clamp(min_y, max_y),
    );
    let mut current_pos = Pos {
        x: start_x,
        y: start_y,
    };

    let mut placed = 0;

    for _ in 0..steps {
        // 如果位置在範圍內，放置或清除物件
        if current_pos.x >= min_x
            && current_pos.x <= max_x
            && current_pos.y >= min_y
            && current_pos.y <= max_y
        {
            if let Some(tile) = board.get_tile(current_pos) {
                let should_act = match object {
                    Some(_) => tile.object.is_none(),
                    None => tile.object.is_some(),
                };
                if should_act {
                    // 放置或清除物件
                    paint_object(board, current_pos, object, orientation, duration)?;
                    placed += 1;
                }
            }
        }

        // 動態調整方向權重，避免超出選取範圍
        let mut weights = weights; // 上, 下, 左, 右
        if current_pos.y == min_y {
            weights[0] = 0; // 上
        }
        if current_pos.y == max_y {
            weights[1] = 0; // 下
        }
        if current_pos.x == min_x {
            weights[2] = 0; // 左
        }
        if current_pos.x == max_x {
            // 如果已到最右邊，停止
            break;
        }
        let total_weight: usize = weights.iter().sum();
        if total_weight == 0 {
            // 所有方向都被阻塞，停止
            break;
        }
        let rand_val = rng.random_range(0..total_weight);
        let mut cumulative = 0;
        let dir = weights
            .iter()
            .enumerate()
            .find_map(|(i, &w)| {
                cumulative += w;
                if rand_val < cumulative { Some(i) } else { None }
            })
            .expect("drunkard should find a direction");

        match dir {
            0 => {
                // 上
                current_pos.y -= 1;
            }
            1 => {
                // 下
                current_pos.y += 1;
            }
            2 => {
                // 左
                current_pos.x -= 1;
            }
            _ => {
                // 右
                current_pos.x += 1;
            }
        }
    }

    Ok(placed)
}

// 將玩家進度的 roster 覆蓋到 board 上 team "player" 的單位
fn override_player_unit(
    board: &mut Board,
    roster_map: &BTreeMap<UnitTemplateType, ProgressionUnit>,
    skills: &BTreeMap<SkillID, Skill>,
) -> Result<(), String> {
    // 1. 取得 board 上所有 team "player" 單位（map: unit_template_type → &mut Unit），若有重複則 set_status 跳錯並 return。
    let mut board_map: HashMap<UnitTemplateType, &mut Unit> = HashMap::new();
    for unit in board.units.values_mut() {
        if unit.team != PLAYER_TEAM {
            continue;
        }
        let unit_type = unit.unit_template_type.clone();
        if board_map.contains_key(&unit_type) {
            return Err(format!(
                "Board 上 team 'player' 單位種類重複: {}",
                unit_type
            ));
        }
        board_map.insert(unit_type, unit);
    }

    // 2. 檢查兩邊 key 是否完全一致
    let board_keys: BTreeSet<_> = board_map.keys().collect();
    let roster_keys: BTreeSet<_> = roster_map.keys().collect();
    if board_keys != roster_keys {
        return Err(format!(
            "玩家單位種類不一致\nBoard: {:?}\nRoster: {:?}",
            board_keys, roster_keys
        ));
    }

    // 3. 覆蓋資料
    for (key, board_unit) in board_map.iter_mut() {
        let roster_unit = roster_map.get(key).expect("roster race condition");
        board_unit.skills = roster_unit
            .skills
            .iter()
            .flat_map(|(_, skill_set)| skill_set.clone())
            .collect();
        // 重算衍生屬性
        board_unit.recalc_from_skills(skills);
    }
    Ok(())
}

fn terrain_color(tile: &Tile) -> Color32 {
    match tile.terrain {
        Terrain::Plain => Color32::DARK_GREEN,
        Terrain::Hill => Color32::from_rgb(90, 60, 30),
        Terrain::Mountain => Color32::from_rgb(60, 30, 0),
        Terrain::Forest => Color32::from_rgb(0, 60, 0),
        Terrain::ShallowWater => Color32::from_rgb(60, 60, 199),
        Terrain::DeepWater => Color32::DARK_BLUE,
    }
}

fn object_symbol(tile: &Tile) -> &'static str {
    match &tile.object {
        Some(Object::Wall) => "█",
        Some(Object::Tree) => "🌳",
        Some(Object::Cliff { orientation }) => match orientation {
            Orientation::Horizontal => "\\→",
            Orientation::Vertical => "\\↓",
        },
        Some(Object::Tent2 { orientation, .. }) => match orientation {
            Orientation::Horizontal => "⛺→2",
            Orientation::Vertical => "⛺↓2",
        },
        Some(Object::Tent15 { orientation, .. }) => match orientation {
            Orientation::Horizontal => "⛺→15",
            Orientation::Vertical => "⛺↓15",
        },
        None => "",
    }
}

fn unit_symbol<'a>(unit: &'a str) -> std::borrow::Cow<'a, str> {
    // 若名稱包含 '_'，我們需要換行顯示，必須分配新 String；
    // 若不包含 '_'，直接借用原字串以避免多餘分配。
    if unit.contains('_') {
        std::borrow::Cow::Owned(unit.replace('_', "\n"))
    } else {
        std::borrow::Cow::Borrowed(unit)
    }
}

fn to_team_color(color: Color32) -> RGB {
    (color.r(), color.g(), color.b())
}

fn to_egui_color(rgb: RGB) -> Color32 {
    Color32::from_rgb(rgb.0, rgb.1, rgb.2)
}

struct UnitTemplateMap<'a>(&'a IndexMap<UnitTemplateType, UnitTemplate>);

impl<'a> UnitTemplateGetter for UnitTemplateMap<'a> {
    fn get(&self, typ: &UnitTemplateType) -> Option<&UnitTemplate> {
        self.0.get(typ)
    }
}

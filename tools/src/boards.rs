use crate::common::*;
use chess_lib::*;
use egui::*;
use indexmap::IndexMap;
use rand::Rng;
use skills_lib::*;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::io;
use strum::IntoEnumIterator;

const TILE_SIZE: f32 = 56.0;
const TILE_OBJECT_SIZE: f32 = 10.0;
const TILE_SHRINK_SIZE: f32 = TILE_SIZE / 40.0;
const TILE_ACTION_SHRINK_SIZE: f32 = TILE_SIZE / 5.0;

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
    // 模擬
    sim_board: Board,
    sim_battle: Battle,
    skill_selection: SkillSelection,
    // 其他
    camera: Camera2D,
    unit_templates: IndexMap<UnitTemplateType, UnitTemplate>,
    skills: BTreeMap<SkillID, Skill>,
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
    Sim,
}

impl BoardsEditor {
    pub fn new() -> Self {
        let mut editor = Self::default();
        editor.reload();
        editor
    }

    fn reload(&mut self) {
        match load_boards(BOARDS_FILE) {
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
        match crate::units::load_unit_templates(UNIT_TEMPLATES_FILE) {
            Ok(unit_templates) => {
                self.unit_templates = unit_templates
                    .into_iter()
                    .map(|u| (u.name.clone(), u))
                    .collect();
                if let Some(selected) = &self.selected_unit {
                    if !self.unit_templates.contains_key(selected) {
                        self.selected_unit = None;
                    }
                }
            }
            Err(err) => {
                self.unit_templates = IndexMap::new();
                self.set_status(format!("載入單位類型失敗: {}", err), true);
                return;
            }
        }
        match from_file::<_, BTreeMap<SkillID, Skill>>(SKILLS_FILE) {
            Ok(skills) => {
                self.skills = skills;
            }
            Err(err) => {
                self.skills = BTreeMap::new();
                self.set_status(format!("載入技能失敗: {}", err), true);
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
                self.sim(ui);
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
                if let Err(e) = save_boards(BOARDS_FILE, &self.boards) {
                    self.set_status(format!("儲存失敗: {}", e), true);
                } else {
                    self.set_status("儲存成功".to_string(), false);
                    self.has_unsaved_changes = false;
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

        // 僅處理點擊
        if !ui.ctx().input(|i| i.pointer.primary_down()) {
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
                if let Err(e) = paint_object(
                    board,
                    painted,
                    self.selected_object.clone(),
                    self.selected_orientation,
                    self.selected_object_duration,
                ) {
                    err_msg = format!("Error painting object: {}", e);
                }
            }
            BrushMode::Unit => {
                let marker = if let Some(template_type) = &self.selected_unit {
                    let mut rng = rand::thread_rng();
                    // 數字太大無法存入 toml
                    // 使用 i64 max 當作 ID 上限
                    let id = rng.gen_range(0..u64::MAX / 2 - 1);
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
            BrushMode::Team | BrushMode::Sim | BrushMode::None => {}
        }
        self.set_status(err_msg, true);
    }

    fn sim(&mut self, ui: &mut Ui) {
        // 取得當前回合角色
        let Some(active_unit_id) = self.sim_battle.get_current_unit_id().cloned() else {
            return;
        };
        let Some(_) = self.sim_board.units.get(&active_unit_id) else {
            return;
        };
        let active_unit_pos = self.sim_board.unit_pos(&active_unit_id);
        let Some(active_unit_pos) = active_unit_pos else {
            return;
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
        if self.skill_selection.selected_skill.is_some() {
            // -------- 有選擇技能時：只顯示技能範圍 --------
            // 以繁體中文註解：只顯示技能影響範圍（座標列表），不顯示移動範圍與路徑

            // 取得技能物件與 range，並呼叫 skill_casting_area_around
            let casting_area = if let Some(skill_id) = &self.skill_selection.selected_skill {
                if let Some(skill) = self.skills.get(skill_id) {
                    skill_casting_area(&self.sim_board, active_unit_pos, skill.range)
                } else {
                    vec![]
                }
            } else {
                vec![]
            };

            let affect_area = if let Some(to) = target {
                self.skill_selection.skill_affect_area(
                    &self.sim_board,
                    &self.skills,
                    active_unit_id,
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
            let (Some(skill_id), Some(to)) = (&self.skill_selection.selected_skill, target) else {
                return;
            };
            if !ui.ctx().input(|i| i.pointer.primary_clicked()) {
                return;
            }
            if !casting_area.contains(&to) {
                self.set_status("技能範圍外無法施放".to_string(), true);
                return;
            }
            let msg = format!("{} 在 ({}, {}) 施放", skill_id, to.x, to.y);
            self.set_status(msg, false);
        } else {
            // -------- 未選擇技能時：顯示移動範圍與路徑 --------
            // 以繁體中文註解：維持原本顯示移動範圍與路徑
            let mut movable = movable_area(&self.sim_board, active_unit_pos);
            movable.remove(&active_unit_pos); // 移除原位置，避免自己移動到自己身上
            let movable = movable;

            // 嘗試取得滑鼠目標與路徑
            let path = if let Some(target) = target {
                if self.sim_board.get_tile(target).is_some() {
                    let path =
                        reconstruct_path(&movable, active_unit_pos, target).unwrap_or_default();
                    path
                } else {
                    vec![]
                }
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
                if self.sim_board.pos_to_unit.get(&target).is_some() {
                    self.set_status("目標位置已有單位，無法移動".to_string(), true);
                    return;
                }
                if path.is_empty() {
                    self.set_status("無法到達目標位置".to_string(), true);
                    return;
                }
                if let Err(e) = move_unit_along_path(&mut self.sim_board, path) {
                    self.set_status(format!("Error moving unit: {e:?}"), true);
                }
            }
        }
    }

    fn show_right_panel(&mut self, ui: &mut Ui) {
        ui.heading("編輯工具與資訊");
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            for (mode, label) in [
                (BrushMode::None, "戰場設定"),
                (BrushMode::Terrain, "地形筆刷"),
                (BrushMode::Object, "物件筆刷"),
                (BrushMode::Unit, "單位筆刷"),
                (BrushMode::Team, "隊伍編輯"),
                (BrushMode::Sim, "模擬"),
            ] {
                if ui.selectable_label(self.brush == mode, label).clicked() {
                    if self.brush != mode {
                        changed = true;
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
            BrushMode::Sim => {
                if changed {
                    self.init_sim(ui);
                }
                self.show_sim_status(ui);
            }
        }
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
        let mut to_delete: Option<String> = None;
        for (team_id, team_cfg) in board.teams.iter_mut() {
            ui.horizontal(|ui| {
                ui.label(format!("TeamID: {}", team_id));
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
        if let Some(team_id) = to_delete {
            board.teams.remove(&team_id);
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
        // 1. 取得目前選擇的 BoardConfig
        let Some(board_id) = &self.selected_board else {
            ui.label("請先選擇戰場");
            return;
        };
        let config = self.boards.get(board_id).expect("選擇的戰場應該存在");

        // 2. 呼叫 Board::from_config
        let m = UnitTemplateMap(&self.unit_templates);
        match Board::from_config(config.clone(), &m, &self.skills) {
            Ok(board) => {
                let turn_order = board.units.keys().cloned().collect();
                self.sim_board = board;
                self.sim_battle = Battle::new(turn_order);
                self.set_status(format!("轉換成功: BoardConfig 已成功轉換為 Board。"), false);
            }
            Err(e) => {
                self.set_status(format!("轉換失敗：{}", e), true);
            }
        }
    }

    fn show_sim_status(&mut self, ui: &mut Ui) {
        let Some(unit_id) = self.sim_battle.get_current_unit_id().cloned() else {
            return;
        };
        let Some(unit) = self.sim_board.units.get(&unit_id) else {
            return;
        };
        ui.label(format!("當前行動單位種類: {}", unit.unit_template_type));

        // 以繁體中文註解：只顯示單位擁有的技能列表
        // 顯示單位擁有的技能列表
        ui.label("單位擁有的技能：");
        // 技能選擇下拉選單（無「未選擇技能」選項），selected_idx = -1 表示未選擇技能
        let skill_ids: Vec<&SkillID> = unit
            .skills
            .iter()
            .filter(|id| self.skills.contains_key(*id))
            .collect();
        // 滑鼠右鍵點擊時取消技能選取
        if ui.ctx().input(|i| i.pointer.secondary_clicked()) {
            self.skill_selection.select_skill(None);
        }
        let mut selected_idx = self
            .skill_selection
            .selected_skill
            .as_ref()
            .and_then(|id| skill_ids.iter().position(|x| x == &id))
            .map(|idx| idx as i32)
            .unwrap_or(-1);
        egui::ComboBox::from_id_salt("unit_skill_select_combo")
            .selected_text(if selected_idx >= 0 {
                skill_ids
                    .get(selected_idx as usize)
                    .map(|s| s.as_str())
                    .unwrap_or("")
            } else {
                ""
            })
            .show_ui(ui, |ui| {
                for (i, name) in skill_ids.iter().enumerate() {
                    let response = ui.selectable_value(&mut selected_idx, i as i32, *name);
                    if response.changed() {
                        if selected_idx >= 0 {
                            if let Some(skill_id) = skill_ids.get(selected_idx as usize) {
                                self.skill_selection
                                    .select_skill(Some(skill_id.to_string()));
                            }
                        }
                    }
                }
            });
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

fn load_boards(path: &str) -> io::Result<BTreeMap<BoardID, BoardConfig>> {
    from_file(path)
}

fn save_boards(path: &str, boards: &BTreeMap<BoardID, BoardConfig>) -> io::Result<()> {
    to_file(path, boards)
}

// 避免 mut editor 出現太多次，只使用 editor 的 member
fn show_tiles(
    ui: &mut Ui,
    camera: &mut Camera2D,
    tiles: &Vec<Vec<Tile>>,
    show_others: impl Fn(&Painter, &Camera2D, Pos, Rect),
) {
    // 處理攝影機平移與縮放
    camera.handle_pan_zoom(ui);

    // 先繪製格子內容
    let painter = ui.painter();
    for (row_idx, row) in tiles.iter().enumerate() {
        for (col_idx, tile) in row.iter().enumerate() {
            // 計算世界座標
            let world_pos = Pos2::new(col_idx as f32, row_idx as f32) * TILE_SIZE;
            let screen_pos = camera.world_to_screen(world_pos);
            let rect = Rect::from_min_size(screen_pos, vec2(TILE_SIZE, TILE_SIZE) * camera.zoom);

            // 畫 tile 邊框
            painter.rect_filled(rect, 2.0, Color32::BLACK);
            // 畫 tile 地形
            painter.rect_filled(
                rect.shrink(TILE_SHRINK_SIZE * camera.zoom),
                2.0,
                terrain_color(tile),
            );
            // 畫 unit
            let pos = Pos {
                x: col_idx,
                y: row_idx,
            };
            show_others(painter, camera, pos, rect);
            // 畫 tile object
            painter.text(
                rect.center(),
                // 會在格子下半顯示
                Align2::CENTER_TOP,
                object_symbol(tile),
                FontId::proportional(TILE_OBJECT_SIZE * camera.zoom),
                Color32::WHITE,
            );
        }
    }
}

fn show_static_others(board: &BoardConfig) -> impl Fn(&Painter, &Camera2D, Pos, Rect) {
    |painter, camera, pos, rect| {
        // 顯示單位
        let Some((unit_template, team)) = board
            .units
            .values()
            .find(|u| u.pos == pos)
            .map(|u| (&u.unit_template_type, &u.team))
        else {
            // 該位置沒有單位
            return;
        };
        let team_color = board
            .teams
            .get(team)
            .map_or(Color32::WHITE, |team| to_egui_color(team.color));
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            unit_symbol(&unit_template),
            FontId::proportional(TILE_OBJECT_SIZE * camera.zoom),
            team_color,
        );
    }
}

fn show_sim_others(
    board: &Board,
    movable: &HashMap<Pos, (MovementCost, Pos)>,
    active_unit_id: &UnitID,
    path: &[Pos],
) -> impl Fn(&Painter, &Camera2D, Pos, Rect) {
    |painter, camera, pos, rect| {
        // 可移動範圍
        let show_movement = || {
            let color = movement_tile_color(board, movable, active_unit_id, path, pos);
            let Ok(color) = color else {
                // 無法取得顏色，可能是因為不在可移動範圍
                return;
            };
            let color = Color32::from_rgba_premultiplied(color.0, color.1, color.2, color.3);

            painter.rect_filled(
                rect.shrink(TILE_ACTION_SHRINK_SIZE * camera.zoom),
                2.0,
                color,
            );
        };
        show_movement();

        // 顯示單位
        show_unit(board, painter, camera, pos, rect);
    }
}

/// 顯示技能範圍（座標高亮），同時高亮可施放區域與技能預覽區域
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
        // 顯示單位
        show_unit(board, painter, camera, pos, rect);
    }
}

fn show_unit(board: &Board, painter: &Painter, camera: &Camera2D, pos: Pos, rect: Rect) {
    let Some(unit_id) = board.pos_to_unit.get(&pos) else {
        // 該位置沒有單位
        return;
    };
    let (unit_template, team) = board
        .units
        .get(unit_id)
        .map_or(("", ""), |u| (&u.unit_template_type, &u.team));
    let team_color = board
        .teams
        .get(team)
        .map_or(Color32::WHITE, |team| to_egui_color(team.color));
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        unit_symbol(&unit_template),
        FontId::proportional(TILE_OBJECT_SIZE * camera.zoom),
        team_color,
    );
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
    object: Option<Object>,
    orientation: Orientation,
    duration: u32,
) -> Result<(), String> {
    match object {
        Some(Object::Tent2 { .. }) => {
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
        Some(Object::Tent15 { .. }) => {
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
        None | Some(Object::Wall) => {
            // 牆壁或無物件，直接設定
            board
                .get_tile_mut(pos)
                .unwrap_or_else(|| panic!("painting in race condition. {object:?} in {pos:?}"))
                .object = object.clone();
            Ok(())
        }
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
    for &pos in &positions {
        let Some(tile) = board.get_tile(pos) else {
            return Err("some tiles are out of bounds".to_string());
        };
        if tile.object.is_some() {
            return Err("some tiles already have objects".to_string());
        }
    }

    // 放置物件
    for &pos in &positions {
        let tile = board.get_tile_mut(pos).expect("just checked");
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

fn unit_symbol(unit: &str) -> String {
    unit.replace("_", "\n")
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

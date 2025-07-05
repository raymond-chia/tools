use crate::common::*;
use chess_lib::*;
use egui::*;
use rand::Rng;
use std::collections::{BTreeMap, HashMap};
use std::io;
use strum::IntoEnumIterator;

const TILE_SIZE: f32 = 56.0;
const TILE_OBJECT_SIZE: f32 = 10.0;

#[derive(Debug, Default)]
pub struct BoardsEditor {
    boards: BTreeMap<BoardID, BoardConfig>,
    selected_board: Option<BoardID>,
    brush: BrushMode,
    selected_terrain: Terrain,
    selected_object: Option<Object>,
    selected_object_duration: u32,
    selected_orientation: Orientation,
    selected_unit: Option<UnitTemplateType>,
    selected_team: TeamID,
    // 其他
    camera: Camera2D,
    unit_templates: HashMap<UnitTemplateType, UnitTemplate>,
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
                self.unit_templates = HashMap::new();
                self.set_status(format!("載入單位類型失敗: {}", err), true);
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
            self.show_board_editor(ui);
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
                self.boards.insert(new_board_id.clone(), new_board);
                self.selected_board = Some(new_board_id);
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
                    self.boards.insert(new_id.clone(), board);
                    self.selected_board = Some(new_id);
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
        // 處理攝影機平移與縮放
        self.camera.handle_pan_zoom(ui);

        // 棋盤視覺化編輯區
        let Some(board_id) = &self.selected_board else {
            ui.label("請先選擇戰場");
            return;
        };
        let board = self.boards.get_mut(board_id).expect("選擇的戰場應該存在");

        // 先繪製格子內容
        let painter = ui.painter();
        for (row_idx, row) in board.tiles.iter_mut().enumerate() {
            for (col_idx, tile) in row.iter_mut().enumerate() {
                // 計算世界座標
                let world_pos = Pos2::new(col_idx as f32, row_idx as f32) * TILE_SIZE;
                let screen_pos = self.camera.world_to_screen(world_pos);
                let rect =
                    Rect::from_min_size(screen_pos, vec2(TILE_SIZE, TILE_SIZE) * self.camera.zoom);

                // 畫 tile 邊框
                painter.rect_filled(rect, 2.0, Color32::BLACK);
                // 畫 tile 地形
                painter.rect_filled(
                    rect.shrink(3.0 * self.camera.zoom),
                    2.0,
                    terrain_color(tile),
                );
                // 畫 tile object
                painter.text(
                    rect.center(),
                    // 會在格子下半顯示
                    Align2::CENTER_TOP,
                    object_symbol(tile),
                    FontId::proportional(TILE_OBJECT_SIZE * self.camera.zoom),
                    Color32::WHITE,
                );
                // 畫 unit
                let pos = Pos {
                    x: col_idx,
                    y: row_idx,
                };
                let unit = board
                    .units
                    .values()
                    .find(|u| u.pos == pos)
                    .map_or("", |u| &u.unit_template_type);
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    unit_symbol(unit),
                    FontId::proportional(TILE_OBJECT_SIZE * self.camera.zoom),
                    Color32::WHITE,
                );
            }
        }

        let mut painted = None;
        // 僅當滑鼠在 central panel 內才偵測 hover
        if !ui.rect_contains_pointer(ui.max_rect()) {
            return;
        }
        if let Some(pointer_pos) = ui.ctx().pointer_hover_pos() {
            // 反推回世界座標
            let world_pointer = self.camera.screen_to_world(pointer_pos);
            let tile_x = (world_pointer.x / TILE_SIZE).floor() as usize;
            let tile_y = (world_pointer.y / TILE_SIZE).floor() as usize;
            if !ui.ctx().input(|i| i.pointer.primary_down()) {
                return;
            }
            if tile_x >= board.width() || tile_y >= board.height() {
                return;
            }
            painted = Some(Pos {
                x: tile_x,
                y: tile_y,
            });
        }

        // 修改格子
        let mut err_msg = String::new();
        if let Some(pos) = painted {
            match self.brush {
                BrushMode::Terrain => {
                    paint_terrain(board, pos, self.selected_terrain);
                }
                BrushMode::Object => {
                    if let Err(e) = paint_object(
                        board,
                        pos,
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
                        Some(UnitMarker {
                            id: rng.gen_range(0..u64::MAX),
                            unit_template_type: template_type.clone(),
                            team: self.selected_team.clone(),
                            pos,
                        })
                    } else {
                        None
                    };
                    if let Err(e) = paint_unit(board, pos, marker) {
                        err_msg = format!("Error painting unit: {}", e);
                    }
                }
                BrushMode::Team => {
                    // 隊伍編輯未實作
                }
                BrushMode::None => {}
            }
        }
        self.set_status(err_msg, true);
    }

    fn show_right_panel(&mut self, ui: &mut Ui) {
        ui.heading("編輯工具與資訊");
        ui.horizontal_wrapped(|ui| {
            for (mode, label) in [
                (BrushMode::None, "戰場設定"),
                (BrushMode::Terrain, "地形筆刷"),
                (BrushMode::Object, "物件筆刷"),
                (BrushMode::Unit, "單位筆刷"),
                (BrushMode::Team, "隊伍編輯"),
            ] {
                if ui.selectable_label(self.brush == mode, label).clicked() {
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
            BrushMode::Team => {}
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
        let Some(board) = self.boards.get(board_id) else {
            return;
        };
        let team_ids = board.teams.keys().cloned().collect::<Vec<_>>();

        ui.vertical(|ui| {
            ui.label("選擇隊伍 TeamID:");
            // 下拉選單
            let mut selected_idx = team_ids
                .iter()
                .position(|id| id == &self.selected_team)
                .unwrap_or(0);
            let combo = egui::ComboBox::from_id_salt("team_select_combo")
                .selected_text(
                    team_ids
                        .get(selected_idx)
                        .map(|s| s.as_str())
                        .unwrap_or("請選擇隊伍"),
                )
                .show_ui(ui, |ui| {
                    for (i, id) in team_ids.iter().enumerate() {
                        ui.selectable_value(&mut selected_idx, i, id);
                    }
                });
            if combo.response.changed() {
                if let Some(new_id) = team_ids.get(selected_idx) {
                    self.selected_team = new_id.clone();
                }
            }
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
            // 檢查該 pos 是否已有單位
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

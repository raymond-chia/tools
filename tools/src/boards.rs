use crate::common::*;
use chess_lib::*;
use egui::*;
use std::collections::BTreeMap;
use std::io;
use strum::IntoEnumIterator;

const BOARDS_FILE: &str = "../shared-lib/test-data/ignore-boards.toml";

const TILE_SIZE: f32 = 40.0;
const TILE_OBJECT_SIZE: f32 = 16.0;

#[derive(Debug, Default)]
pub struct BoardsEditor {
    boards: BTreeMap<BoardID, BoardConfig>,
    selected_board: Option<BoardID>,
    brush: BrushMode,
    selected_terrain: Terrain,
    selected_object: Option<Object>,
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
        self.set_status("已重新載入戰場".to_string(), false);
    }

    pub fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        SidePanel::left("board_list_panel")
            .default_width(180.0)
            .show(ctx, |ui| {
                self.show_board_list(ui);
            });
        CentralPanel::default().show(ctx, |ui| {
            self.show_board_editor(ui);
        });
        SidePanel::right("right_panel")
            .default_width(320.0)
            .show(ctx, |ui| {
                self.show_right_panel(ui);
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
        // 棋盤視覺化編輯區
        let Some(board_id) = &self.selected_board else {
            ui.label("請先選擇戰場");
            return;
        };
        let board = self.boards.get_mut(board_id).expect("選擇的戰場應該存在");

        // 先繪製格子內容
        let mut painted = Vec::new();
        for (row_idx, row) in board.tiles.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                for (col_idx, tile) in row.iter_mut().enumerate() {
                    let (_, rect) = ui.allocate_space(vec2(TILE_SIZE, TILE_SIZE));
                    let painter = ui.painter();

                    // 畫 tile 邊框
                    painter.rect_filled(rect.shrink(3.0), 2.0, Color32::BLACK);
                    // 畫 tile 地形
                    painter.rect_filled(rect, 2.0, terrain_color(tile));
                    // 畫 tile object
                    painter.text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        object_symbol(tile),
                        FontId::proportional(TILE_OBJECT_SIZE),
                        Color32::BLACK,
                    );

                    let Some(pointer_pos) = ui.ctx().pointer_hover_pos() else {
                        continue;
                    };
                    if !rect.contains(pointer_pos) || !ui.ctx().input(|i| i.pointer.primary_down())
                    {
                        continue;
                    }
                    painted.push(Pos {
                        x: col_idx,
                        y: row_idx,
                    });
                }
            });
        }

        // 修改格子
        let mut err_msg = String::new();
        for pos in painted {
            match self.brush {
                BrushMode::Terrain => {
                    paint_terrain(board, pos, self.selected_terrain);
                }
                BrushMode::Object => {
                    if let Err(e) = paint_object(board, pos, self.selected_object.clone()) {
                        err_msg = format!("Error painting object: {}", e);
                    }
                }
                BrushMode::Unit => {
                    // 物件筆刷未實作
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
            BrushMode::Unit => {}
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
        .expect("painting in race condition")
        .terrain = terrain;
    return true;
}

fn paint_object(board: &mut BoardConfig, pos: Pos, object: Option<Object>) -> Result<(), String> {
    match object {
        Some(Object::Tent2 { rel: _, duration }) => {
            let positions = vec![
                pos,
                Pos {
                    x: pos.x + 1,
                    y: pos.y,
                },
            ];
            let main_pos = pos;
            paint_multiple_object(board, positions.clone(), |tile, p| {
                let rel = Pos {
                    x: p.x - main_pos.x,
                    y: p.y - main_pos.y,
                };
                tile.object = Some(Object::Tent2 { rel, duration });
            })
        }
        Some(Object::Tent15 { rel: _, duration }) => {
            let mut positions = Vec::new();
            for x in 0..5 {
                for y in 0..3 {
                    let x = pos.x + x;
                    let y = pos.y + y;
                    positions.push(Pos { x, y });
                }
            }
            let main_pos = pos;
            paint_multiple_object(board, positions.clone(), |tile, p| {
                let rel = Pos {
                    x: p.x - main_pos.x,
                    y: p.y - main_pos.y,
                };
                tile.object = Some(Object::Tent15 { rel, duration });
            })
        }
        None | Some(Object::Wall) => {
            // 牆壁或無物件，直接設定
            board
                .get_tile_mut(pos)
                .expect("painting in race condition")
                .object = object;
            Ok(())
        }
    }
}

fn paint_multiple_object(
    board: &mut BoardConfig,
    positions: Vec<Pos>,
    set_object: impl Fn(&mut Tile, Pos),
) -> Result<(), String> {
    for &pos in &positions {
        let Some(tile) = board.get_tile(pos) else {
            return Err("some tiles are out of bounds".to_string());
        };
        if tile.object.is_some() {
            return Err("some tiles already have objects".to_string());
        }
    }
    for &pos in &positions {
        let tile = board.get_tile_mut(pos).expect("just checked");
        set_object(tile, pos);
    }
    Ok(())
}

fn terrain_color(tile: &Tile) -> Color32 {
    match tile.terrain {
        Terrain::Plain => Color32::from_rgb(200, 200, 170),
        Terrain::Hill => Color32::from_rgb(180, 170, 120),
        Terrain::Mountain => Color32::from_rgb(120, 120, 120),
        Terrain::Forest => Color32::from_rgb(60, 120, 60),
        Terrain::ShallowWater => Color32::from_rgb(100, 180, 220),
        Terrain::DeepWater => Color32::from_rgb(30, 60, 120),
    }
}

fn object_symbol(tile: &Tile) -> &'static str {
    match &tile.object {
        Some(Object::Wall) => "▯",
        Some(Object::Tent2 { .. }) => "⛺ 2",
        Some(Object::Tent15 { .. }) => "⛺15",
        None => "",
    }
}

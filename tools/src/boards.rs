use crate::common::*;
use chess_lib::*;
use egui::{Button, Ui};
use std::collections::BTreeMap;
use std::io;

const BOARDS_FILE: &str = "../shared-lib/test-data/ignore-boards.toml";

#[derive(Debug, Default)]
pub struct BoardsEditor {
    boards: BTreeMap<BoardID, BoardConfig>,
    selected_board: Option<BoardID>,
    brush: BrushMode,
    selected_team: Option<TeamID>,
    selected_tile: Option<Pos>,
    has_unsaved_changes: bool,
    status_message: Option<(String, bool)>,
}

#[derive(Debug, PartialEq)]
pub enum BrushMode {
    None,
    Terrain,
    Object,
    Unit,
    Team,
}

impl Default for BrushMode {
    fn default() -> Self {
        BrushMode::None
    }
}

impl BoardsEditor {
    pub fn new() -> Self {
        let mut editor = Self::default();
        editor.reload();
        editor
    }

    pub fn reload(&mut self) {
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

    pub fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("board_list_panel")
            .default_width(180.0)
            .show(ctx, |ui| {
                self.show_board_list(ui);
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_board_editor(ui);
        });
        egui::SidePanel::right("right_panel")
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
            let mut to_delete = None;
            for (board_id, _) in &self.boards {
                let selected = self.selected_board.as_ref() == Some(board_id);
                let button = Button::new(board_id).fill(if selected {
                    egui::Color32::DARK_GRAY
                } else {
                    egui::Color32::TRANSPARENT
                });
                if ui.add(button).clicked() {
                    self.selected_board = Some(board_id.clone());
                }
                ui.horizontal(|ui| {
                    if ui.button("刪除").clicked() {
                        to_delete = Some(board_id.clone());
                    }
                });
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
        if let Some(board_id) = &self.selected_board {
            if let Some(board) = self.boards.get_mut(board_id) {
                let tile_size = 32.0;
                for (y, row) in board.tiles.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        for (x, tile) in row.iter_mut().enumerate() {
                            let pos = Pos { x, y };
                            let mut btn = Button::new(format!("{}", tile_symbol(tile)))
                                .min_size(egui::Vec2::splat(tile_size));
                            if Some(pos) == self.selected_tile {
                                btn = btn.fill(egui::Color32::LIGHT_BLUE);
                            }
                            if ui.add(btn).clicked() {
                                self.selected_tile = Some(pos);
                            }
                        }
                    });
                }
            }
        } else {
            ui.label("請先選擇戰場");
        }
    }

    fn show_right_panel(&mut self, ui: &mut Ui) {
        ui.heading("編輯工具與資訊");
        ui.horizontal_wrapped(|ui| {
            for (mode, label) in [
                (BrushMode::None, "無筆刷"),
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
    }

    fn show_status_message(&mut self, ctx: &egui::Context) {
        if let Some((message, is_error)) = &self.status_message {
            show_status_message(ctx, message, *is_error);
        }
    }

    pub fn set_status(&mut self, msg: String, is_error: bool) {
        self.status_message = Some((msg, is_error));
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.has_unsaved_changes
    }
}

pub fn load_boards(path: &str) -> io::Result<BTreeMap<BoardID, BoardConfig>> {
    from_file(path)
}

pub fn save_boards(path: &str, boards: &BTreeMap<BoardID, BoardConfig>) -> io::Result<()> {
    to_file(path, boards)
}

// 顯示地形符號（可依需求調整）
fn tile_symbol(tile: &Tile) -> &'static str {
    match tile.terrain {
        Terrain::Plain => "．",
        Terrain::Hill => "△",
        Terrain::Mountain => "▲",
        Terrain::Forest => "♣",
        Terrain::ShallowWater => "≈",
        Terrain::DeepWater => "≋",
    }
}

use eframe::egui;
use egui::*;
use rfd::FileDialog;
use serde::{Serialize, de::DeserializeOwned};
use skills_lib::*;
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Error, ErrorKind};
use std::path::{Path, PathBuf};

/// 取得跨平台對話資料路徑
pub fn dialogs_file() -> PathBuf {
    PathBuf::from_iter(["test-data", "ignore-dialogs.toml"])
}
/// 取得跨平台技能資料路徑
pub fn skills_file() -> PathBuf {
    PathBuf::from_iter(["test-data", "ignore-skills.toml"])
}
/// 取得跨平台單位模板資料路徑
pub fn unit_templates_file() -> PathBuf {
    PathBuf::from_iter(["test-data", "ignore-unit-templates.toml"])
}
/// 取得跨平台玩家進度資料路徑
pub fn progression_file() -> PathBuf {
    PathBuf::from_iter(["test-data", "ignore-player-progressions.toml"])
}
/// 取得跨平台棋盤資料路徑
pub fn boards_file() -> PathBuf {
    PathBuf::from_iter(["test-data", "ignore-boards.toml"])
}
/// 取得跨平台棋盤分開存放目錄
pub fn boards_separate_dir() -> PathBuf {
    PathBuf::from_iter(["test-data", "ignore-boards"])
}
/// 取得跨平台 AI 設定檔路徑
pub fn ai_file() -> PathBuf {
    PathBuf::from_iter(["test-data", "ignore-ai.toml"])
}

#[derive(Debug, Clone)]
pub struct Camera2D {
    pub offset: Vec2,
    pub zoom: f32,
}

impl Default for Camera2D {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

impl Camera2D {
    pub fn world_to_screen(&self, world_pos: Pos2) -> Pos2 {
        (world_pos - self.offset) * self.zoom
    }
    pub fn screen_to_world(&self, screen_pos: Pos2) -> Pos2 {
        screen_pos / self.zoom + self.offset
    }
    /// 處理滑鼠拖曳與滾輪縮放
    pub fn handle_pan_zoom(&mut self, ui: &Ui) {
        // 拖曳
        if ui.input(|i| i.pointer.secondary_down()) {
            self.offset -= ui.input(|i| i.pointer.delta()) / self.zoom;
        }
        // 縮放
        if ui.input(|i| i.raw_scroll_delta.y) != 0.0 {
            // 只有在滑鼠在中央面板時才處理縮放
            if let Some(mouse_pos) = ui.input(|i| i.pointer.latest_pos()) {
                // 確認滑鼠位置在中央面板內
                if ui.rect_contains_pointer(ui.max_rect()) {
                    self.zoom *= 1.0 + ui.input(|i| i.raw_scroll_delta.y) * 0.001;
                    self.zoom = self.zoom.clamp(0.1, 2.0); // 限制縮放範圍

                    // 調整 offset 以保持縮放中心
                    let world_mouse = self.screen_to_world(mouse_pos);
                    self.offset = world_mouse - (mouse_pos / self.zoom);
                }
            }
        }
    }
}

pub fn from_toml<T>(content: &str) -> io::Result<T>
where
    T: DeserializeOwned,
{
    return toml::de::from_str::<T>(content)
        .map_err(|err| Error::new(ErrorKind::InvalidData, format!("解析 TOML 失敗: {}", err)));
}

pub fn from_file<P: AsRef<Path>, T>(path: P) -> io::Result<T>
where
    T: DeserializeOwned,
{
    let content = fs::read_to_string(path)?;
    return from_toml(&content);
}

pub fn to_toml<T>(value: &T) -> io::Result<String>
where
    T: Serialize,
{
    return toml::ser::to_string_pretty(value)
        .map_err(|err| Error::new(ErrorKind::InvalidData, format!("序列化 TOML 失敗: {}", err)));
}

pub fn to_file<P: AsRef<Path>, T>(path: P, value: &T) -> io::Result<()>
where
    T: Serialize,
{
    let content = to_toml(value)?;
    return fs::write(path, content)
        .map_err(|err| Error::new(ErrorKind::InvalidData, format!("寫入 TOML 失敗: {}", err)));
}

pub trait FileOperator<P: AsRef<Path>> {
    fn load_file(&mut self, path: P);
    fn save_file(&mut self, path: P);
    fn current_file_path(&self) -> Option<P>;
    fn set_status(&mut self, status: String, is_error: bool);
}

pub trait New {
    fn new() -> Self;
}

pub fn show_file_menu<T: FileOperator<PathBuf> + New>(ui: &mut Ui, t: &mut T) {
    egui::menu::menu_button(ui, "檔案", |ui| {
        if ui.button("新增").clicked() {
            *t = T::new();
            t.set_status("已建立新檔案".to_string(), false);
            ui.close_menu();
        }

        if ui.button("開啟...").clicked() {
            if let Some(path) = FileDialog::new()
                .add_filter("TOML", &["toml"])
                .set_directory(".")
                .pick_file()
            {
                *t = T::new();
                t.load_file(path);
            }
            ui.close_menu();
        }

        if ui.button("儲存").clicked() {
            if let Some(path) = t.current_file_path() {
                t.save_file(path);
            } else {
                if let Some(path) = FileDialog::new()
                    .add_filter("TOML", &["toml"])
                    .set_directory(".")
                    .save_file()
                {
                    t.save_file(path);
                }
            }
            ui.close_menu();
        }

        if ui.button("另存為...").clicked() {
            if let Some(path) = FileDialog::new()
                .add_filter("TOML", &["toml"])
                .set_directory(".")
                .save_file()
            {
                t.save_file(path);
            }
            ui.close_menu();
        }
    });
}

pub fn show_status_message(ctx: &egui::Context, message: &str, is_error: bool) {
    let color = if is_error {
        egui::Color32::RED
    } else {
        egui::Color32::GREEN
    };

    egui::TopBottomPanel::bottom("status_panel").show(ctx, |ui| {
        ui.label(RichText::new(message).color(color));
    });
}

pub fn load_skills<P: AsRef<Path>>(
    path: P,
) -> io::Result<(BTreeMap<SkillID, Skill>, Vec<SkillID>, Vec<SkillID>)> {
    match from_file::<_, BTreeMap<SkillID, Skill>>(path) {
        Ok(skills) => {
            // 分類主動/被動技能
            let mut active_skill_ids = Vec::new();
            let mut passive_skill_ids = Vec::new();
            for (id, skill) in &skills {
                if skill.tags.contains(&skills_lib::Tag::Active) {
                    active_skill_ids.push(id.clone());
                } else if skill.tags.contains(&skills_lib::Tag::Passive) {
                    passive_skill_ids.push(id.clone());
                }
            }
            active_skill_ids.sort();
            passive_skill_ids.sort();
            return Ok((skills, active_skill_ids, passive_skill_ids));
        }
        Err(err) => {
            return Err(err);
        }
    }
}

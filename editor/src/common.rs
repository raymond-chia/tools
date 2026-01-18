use chess_lib::*;
use eframe::egui;
use egui::*;
use rfd::FileDialog;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::DeserializeOwned};
use skills_lib::*;
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::str::FromStr;

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
pub fn progressions_file() -> PathBuf {
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

    /// 處理鍵盤縮放（Ctrl + + / Ctrl + -）
    pub fn handle_keyboard_zoom(&mut self, ctx: &Ui) {
        ctx.input(|i| {
            // 支援 Ctrl + + / Ctrl + - / Ctrl + =（部分鍵盤 + 需用 =）
            if i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals) {
                self.zoom += 0.1;
                self.zoom = self.zoom.clamp(0.1, 2.0);
            }
            if i.key_pressed(Key::Minus) {
                self.zoom -= 0.1;
                self.zoom = self.zoom.clamp(0.1, 2.0);
            }
        });
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

    egui::TopBottomPanel::bottom("status_panel")
        .min_height(150.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                // 以免只有佔據一半的寬度
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.label(RichText::new(message).color(color));
                });
        });
}

pub type SkillByTags = BTreeMap<Vec<Tag>, Vec<SkillID>>;

/// 互斥 tag 分組（每組只能選一個）
/// 用於技能分類和 UI 編輯器
/// 注意：每組的第一個 tag 是 basic passive 技能的預設值
pub const EXCLUSIVE_TAG_GROUPS: [&[Tag]; 4] = [
    &[Tag::Passive, Tag::Active],
    &[Tag::Character, Tag::Equipment],
    &[Tag::Caster, Tag::Melee, Tag::Ranged],
    &[Tag::Single, Tag::Area],
];

/// 可選多選 tag 分組（可同時選多個）
pub const OPTIONAL_TAG_GROUPS: [&[Tag]; 2] = [
    &[Tag::Physical, Tag::Magical],
    &[Tag::Attack, Tag::Heal, Tag::Buff, Tag::Debuff],
];

pub fn grouped_unit_skills(skill_group: &SkillByTags, unit: &UnitTemplate) -> SkillByTags {
    let mut result = BTreeMap::new();
    for (tags, skill_ids) in skill_group {
        let filtered: Vec<SkillID> = unit
            .skills
            .iter()
            .filter(|id| skill_ids.contains(id))
            .cloned()
            .collect();
        if !filtered.is_empty() {
            result.insert(tags.clone(), filtered);
        }
    }
    result
}

pub fn must_group_skills_by_tags(skills: &BTreeMap<SkillID, Skill>) -> Result<SkillByTags, String> {
    let (matched, unmatched) = group_skills_by_tags(&skills);
    for id in &unmatched {
        let skill = skills
            .get(id)
            .ok_or_else(|| format!("Skill ID '{}' not found in skills data.", id))?;
        return Err(format!(
            "Warning: Skill ID '{}' has unmatched tags: {:?}",
            id, skill.tags
        ));
    }
    return Ok(matched);
}

pub fn group_skills_by_tags(skills: &BTreeMap<SkillID, Skill>) -> (SkillByTags, Vec<SkillID>) {
    let mut matched = SkillByTags::new();
    let mut unmatched = Vec::new();
    for (id, skill) in skills {
        let tags: Option<Vec<Tag>> = EXCLUSIVE_TAG_GROUPS
            .iter()
            .map(|g| g.iter().find(|t| skill.tags.contains(t)).cloned())
            .collect();
        match tags {
            Some(t) => matched.entry(t).or_default().push(id.clone()),
            None => unmatched.push(id.clone()),
        }
    }
    (matched, unmatched)
}

pub mod skill_by_tags_key_map {
    use super::*;

    pub fn serialize<S>(map: &SkillByTags, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let string_map: BTreeMap<String, &Vec<SkillID>> = map
            .iter()
            .map(|(k, v)| {
                let key_str = k
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join("-");
                (key_str, v)
            })
            .collect();
        string_map.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SkillByTags, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string_map: BTreeMap<String, Vec<SkillID>> =
            BTreeMap::<String, Vec<SkillID>>::deserialize(deserializer)?;
        string_map
            .into_iter()
            .map(|(k, v)| {
                let tag_strs: Vec<&str> = k.split('-').collect();
                if tag_strs.len() != EXCLUSIVE_TAG_GROUPS.len() {
                    return Err(serde::de::Error::custom(format!(
                        "Key must have {} tags: {}",
                        EXCLUSIVE_TAG_GROUPS.len(),
                        k
                    )));
                }
                let tags: Vec<Tag> = tag_strs
                    .iter()
                    .map(|s| Tag::from_str(s).map_err(serde::de::Error::custom))
                    .collect::<Result<_, _>>()?;
                Ok((tags, v))
            })
            .collect()
    }
}

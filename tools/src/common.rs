use eframe::egui;
use egui::{RichText, Ui};
use rfd::FileDialog;
use serde::{Serialize, de::DeserializeOwned};
use std::fs;
use std::io::{self, Error, ErrorKind};
use std::path::{Path, PathBuf};

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

pub fn show_file_menu<T: FileOperator<PathBuf> + Default>(ui: &mut Ui, t: &mut T) {
    egui::menu::menu_button(ui, "檔案", |ui| {
        if ui.button("新增").clicked() {
            *t = T::default();
            t.set_status("已建立新檔案".to_string(), false);
            ui.close_menu();
        }

        if ui.button("開啟...").clicked() {
            if let Some(path) = FileDialog::new()
                .add_filter("TOML", &["toml"])
                .set_directory(".")
                .pick_file()
            {
                *t = T::default();
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

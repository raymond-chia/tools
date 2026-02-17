mod app;
mod constants;
mod editor_item;
#[macro_use]
mod editor_macros;
mod generic_editor;
mod generic_io;
mod tabs;
mod utils;

use app::EditorApp;
use constants::{
    APP_TITLE, FONT_FILE_PATH, FONT_NAME, FONT_SIZE_BODY, FONT_SIZE_BUTTON, FONT_SIZE_HEADING,
    FONT_SIZE_MONOSPACE, FONT_SIZE_SMALL, STROKE_WIDTH,
};
use std::sync::Arc;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_maximized(true),
        ..Default::default()
    };
    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            setup_visuals(&cc.egui_ctx);
            Ok(Box::new(EditorApp::new()))
        }),
    )
}

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    if let Ok(font_data) = std::fs::read(FONT_FILE_PATH) {
        fonts.font_data.insert(
            FONT_NAME.to_string(),
            Arc::new(egui::FontData::from_owned(font_data)),
        );

        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_insert_with(Vec::new)
            .insert(0, FONT_NAME.to_string());
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_insert_with(Vec::new)
            .insert(0, FONT_NAME.to_string());
    }

    ctx.set_fonts(fonts);

    let mut style = (*ctx.style()).clone();
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(FONT_SIZE_HEADING, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(FONT_SIZE_BODY, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        egui::FontId::new(FONT_SIZE_MONOSPACE, egui::FontFamily::Monospace),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(FONT_SIZE_BUTTON, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::new(FONT_SIZE_SMALL, egui::FontFamily::Proportional),
    );

    ctx.set_style(style);
}

fn setup_visuals(ctx: &egui::Context) {
    let mut visuals = ctx.style().visuals.clone();

    // Checkbox 背景色
    // 會影響 Checkbox 以外的 UI
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(0, 0, 0); // 未勾選
    visuals.widgets.inactive.fg_stroke.width = STROKE_WIDTH; // 勾勾

    ctx.set_visuals(visuals);
}

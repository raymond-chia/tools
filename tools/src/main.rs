mod dialogs;
mod skills;

use dialogs::DialogsEditor;
use eframe::{Frame, egui};
use egui::{FontData, FontDefinitions, FontFamily, Ui};
use skills::SkillsEditor;

/// 編輯器模式
#[derive(PartialEq)]
enum EditorMode {
    Skills,
    Dialogs,
}

/// 主應用程序狀態
struct EditorApp {
    editor_mode: EditorMode,
    skills_editor: SkillsEditor,
    dialogs_editor: DialogsEditor,
}

impl EditorApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // 設定字體以支援繁體中文
        let mut fonts = FontDefinitions::default();

        // 在 Windows 中，使用系統已安裝的中文字體
        // 微軟正黑體是 Windows 中常見的繁體中文字體
        match std::fs::read("C:\\Windows\\Fonts\\msjh.ttc") {
            Ok(font_data) => {
                fonts
                    .font_data
                    .insert("msyh".to_owned(), FontData::from_owned(font_data).into());

                // 將中文字體添加到 Proportional 字體族中的首位
                fonts
                    .families
                    .get_mut(&FontFamily::Proportional)
                    .unwrap()
                    .insert(0, "msyh".to_owned());
            }
            Err(err) => {
                println!("無法載入中文字體: {}", err);
                // 這裡可以加載備用字體或繼續使用預設字體
            }
        }

        // 設置字體
        cc.egui_ctx.set_fonts(fonts);

        // 設定初始字型大小和樣式
        let mut style = (*cc.egui_ctx.style()).clone();
        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::new(24.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Heading,
            egui::FontId::new(32.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::new(24.0, egui::FontFamily::Proportional),
        );
        cc.egui_ctx.set_style(style);

        Self {
            editor_mode: EditorMode::Skills, // 默認為技能編輯器
            skills_editor: SkillsEditor::new(cc),
            dialogs_editor: DialogsEditor::new(cc),
        }
    }

    fn show_mode_selector(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.editor_mode == EditorMode::Skills, "技能編輯器")
                .clicked()
            {
                self.editor_mode = EditorMode::Skills;
            }
            if ui
                .selectable_label(self.editor_mode == EditorMode::Dialogs, "劇情編輯器")
                .clicked()
            {
                self.editor_mode = EditorMode::Dialogs;
            }
        });
    }
}

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut Frame) {
        egui::TopBottomPanel::top("mode_selector").show(ctx, |ui| {
            self.show_mode_selector(ui);
        });

        // 根據當前模式顯示相應的編輯器
        match self.editor_mode {
            EditorMode::Skills => {
                self.skills_editor.update(ctx, frame);
            }
            EditorMode::Dialogs => {
                self.dialogs_editor.update(ctx, frame);
            }
        }
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1024.0, 768.0]),
        ..Default::default()
    };

    eframe::run_native(
        "遊戲編輯器",
        options,
        Box::new(|cc| Ok(Box::new(EditorApp::new(cc)))),
    )
}

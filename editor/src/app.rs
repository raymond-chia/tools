use crate::state::EditorApp;
use crate::ui::UiRenderer;

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.render_top_bar(ctx);
        self.render_page_content(ctx);
    }
}

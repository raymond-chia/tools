use crate::state::{EditorApp, Page};

pub trait UiRenderer {
    fn render_top_bar(&mut self, ctx: &egui::Context);
    fn render_page_content(&mut self, ctx: &egui::Context);
}

impl UiRenderer for EditorApp {
    fn render_top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                render_page_button(
                    ui,
                    "戰場編輯",
                    Page::BattlefieldEditor,
                    &mut self.state.current_page,
                );
                render_page_button(
                    ui,
                    "物件編輯",
                    Page::ObjectEditor,
                    &mut self.state.current_page,
                );
            });
        });
    }

    fn render_page_content(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |_ui| {
            match self.state.current_page {
                Page::BattlefieldEditor => {
                    // 保持空的
                }
                Page::ObjectEditor => {
                    // 保持空的
                }
            }
        });
    }
}

fn render_page_button(ui: &mut egui::Ui, label: &str, page: Page, current_page: &mut Page) {
    let is_current = *current_page == page;
    if ui.selectable_label(is_current, label).clicked() {
        *current_page = page;
    }
}

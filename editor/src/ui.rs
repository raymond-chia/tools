use crate::state::{EditorApp, Page};

pub trait UiRenderer {
    fn render_top_bar(&mut self, ctx: &egui::Context);
    fn render_page_content(&mut self, ctx: &egui::Context);
    fn render_message_window(&mut self, ctx: &egui::Context);
}

impl UiRenderer for EditorApp {
    fn render_top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                render_page_button(
                    ui,
                    "戰場編輯",
                    Page::BattlefieldEditor,
                    &mut self.current_page,
                );
                render_page_button(ui, "物件編輯", Page::ObjectEditor, &mut self.current_page);

                ui.separator();
                if ui.button("訊息").clicked() {
                    self.message.visible = !self.message.visible;
                }
            });
        });
    }

    fn render_page_content(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| match self.current_page {
            Page::BattlefieldEditor => {}
            Page::ObjectEditor => {
                if ui.button("載入").clicked() {
                    match std::fs::read_to_string("ignore-data/object.toml") {
                        Ok(_) => {
                            self.message.text = "載入成功".to_string();
                            self.message.visible = true;
                        }
                        Err(e) => {
                            self.message.text = format!("讀取失敗：{}", e);
                            self.message.visible = true;
                        }
                    }
                }

                if ui.button("儲存").clicked() {
                    match std::fs::write("ignore-data/object.toml", "") {
                        Ok(()) => {
                            self.message.text = "儲存成功".to_string();
                            self.message.visible = true;
                        }
                        Err(e) => {
                            self.message.text = format!("儲存失敗：{}", e);
                            self.message.visible = true;
                        }
                    }
                }
            }
        });
    }

    fn render_message_window(&mut self, ctx: &egui::Context) {
        if self.message.visible {
            egui::Window::new("訊息")
                .open(&mut self.message.visible)
                .show(ctx, |ui| {
                    ui.label(&self.message.text);
                });
        }
    }
}

fn render_page_button(ui: &mut egui::Ui, label: &str, page: Page, current_page: &mut Page) {
    let is_current = *current_page == page;
    if ui.selectable_label(is_current, label).clicked() {
        *current_page = page;
    }
}

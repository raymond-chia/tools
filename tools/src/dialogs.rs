use crate::common::{FileOperator, from_file, show_file_menu, to_file};
use dialogs_lib::{Node, Pos, Script};
use eframe::{Frame, egui};
use std::path::PathBuf;

pub struct DialogsEditor {
    script: Script,
    selected_node: Option<String>,
    camera_offset: egui::Vec2,              // 攝影機平移量
    camera_zoom: f32,                       // 攝影機縮放比例
    has_unsaved_changes_flag: bool,         // 追蹤是否有未保存的變動
    current_file_path: Option<PathBuf>,     // 目前檔案路徑
    status_message: Option<(String, bool)>, // 狀態訊息 (訊息, 是否為錯誤)
}

impl Default for DialogsEditor {
    fn default() -> Self {
        Self {
            script: Script::default(),
            selected_node: None,
            camera_offset: egui::vec2(0.0, 0.0),
            camera_zoom: 1.0,
            has_unsaved_changes_flag: false,
            current_file_path: None,
            status_message: None,
        }
    }
}

fn convert_to_pos(value: &egui::Pos2) -> Pos {
    return Pos {
        x: value.x,
        y: value.y,
    };
}

fn convert_to_egui_pos(value: &Pos) -> egui::Pos2 {
    return egui::Pos2::new(value.x, value.y);
}

impl FileOperator<PathBuf> for DialogsEditor {
    fn current_file_path(&self) -> Option<PathBuf> {
        return self.current_file_path.clone();
    }

    fn load_file(&mut self, path: PathBuf) {
        self.load_file(path);
    }

    fn save_file(&mut self, path: PathBuf) {
        self.save_file(path);
    }

    fn set_status(&mut self, status: String, is_error: bool) {
        self.set_status(status, is_error);
    }
}

impl DialogsEditor {
    // 檢查是否有未保存的變動
    pub fn has_unsaved_changes(&self) -> bool {
        self.has_unsaved_changes_flag
    }

    // 從檔案載入對話腳本
    fn load_file(&mut self, path: PathBuf) {
        match from_file(&path) {
            Ok(script) => {
                let current_file_path = Some(path);
                *self = Self {
                    script,
                    current_file_path,
                    ..Default::default()
                };
                self.set_status(format!("成功載入檔案"), false);
            }
            Err(err) => {
                self.set_status(format!("載入檔案失敗: {}", err), true);
            }
        }
    }

    // 儲存到檔案
    fn save_file(&mut self, path: PathBuf) {
        match to_file(&path, &self.script) {
            Ok(_) => {
                self.current_file_path = Some(path);
                self.set_status(format!("成功儲存檔案"), false);
                self.has_unsaved_changes_flag = false;
            }
            Err(err) => {
                self.set_status(format!("儲存檔案失敗: {}", err), true);
            }
        }
    }

    // 設定狀態訊息
    fn set_status(&mut self, message: String, is_error: bool) {
        self.status_message = Some((message, is_error));
    }

    // 顯示檔案選單
    fn show_file_menu(&mut self, ui: &mut egui::Ui) {
        return show_file_menu(ui, self);
    }

    // 將節點的世界坐標轉為螢幕坐標
    fn world_to_screen(&self, world_pos: egui::Pos2) -> egui::Pos2 {
        (world_pos - self.camera_offset) * self.camera_zoom
    }

    // 將螢幕坐標轉為世界坐標
    fn screen_to_world(&self, screen_pos: egui::Pos2) -> egui::Pos2 {
        screen_pos / self.camera_zoom + self.camera_offset
    }

    // 顯示狀態訊息
    fn show_status_message(&mut self, ctx: &egui::Context) {
        if let Some((message, is_error)) = &self.status_message {
            let color = if *is_error {
                egui::Color32::RED
            } else {
                egui::Color32::GREEN
            };

            egui::TopBottomPanel::bottom("status_panel").show(ctx, |ui| {
                ui.label(egui::RichText::new(message).color(color));
            });
        }
    }

    pub fn update(&mut self, ctx: &egui::Context, _frame: &Frame) {
        // 頂部面板：顯示檔案選單
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            self.show_file_menu(ui);
        });

        // 側邊欄：顯示選中節點的詳細內容
        egui::SidePanel::right("details_panel")
            .resizable(true)
            .min_width(400.0)
            .show(ctx, |ui| {
                // 顯示詳細資訊
                ui.heading("節點詳情");
                if let Some(node_id) = &self.selected_node {
                    if let Some(node) = self.script.nodes.get(node_id) {
                        match node {
                            Node::Dialogue {
                                dialogues,
                                actions,
                                next_node,
                                pos: _,
                            } => {
                                ui.label(format!("類型: {}", node.to_string()));
                                ui.label(format!("ID: {}", node_id));
                                for dialogue in dialogues {
                                    ui.label(format!("<{}>: {}", dialogue.speaker, dialogue.text));
                                }
                                if let Some(actions) = actions {
                                    ui.label("動作:");
                                    for action in actions {
                                        ui.label(format!(
                                            "函數: {}, 參數: {:?}",
                                            action.function, action.params
                                        ));
                                    }
                                }
                                ui.label(format!("下一個節點: {}", next_node));
                            }
                            Node::Option { options, pos: _ } => {
                                ui.label(format!("類型: {}", node.to_string()));
                                ui.label(format!("ID: {}", node_id));
                                for option in options {
                                    ui.label(format!("選項: {}", option.text));
                                    ui.label(format!("下一個節點: {}", option.next_node));
                                    if let Some(conditions) = &option.conditions {
                                        ui.label("條件:");
                                        for cond in conditions {
                                            ui.label(format!(
                                                "函數: {}, 參數: {:?}",
                                                cond.function, cond.params
                                            ));
                                        }
                                    }
                                    if let Some(actions) = &option.actions {
                                        ui.label("動作:");
                                        for action in actions {
                                            ui.label(format!(
                                                "函數: {}, 參數: {:?}",
                                                action.function, action.params
                                            ));
                                        }
                                    }
                                }
                            }
                            Node::Battle { outcomes, pos: _ } => {
                                ui.label(format!("類型: {}", node.to_string()));
                                ui.label(format!("ID: {}", node_id));
                                for outcome in outcomes {
                                    ui.label(format!("結果: {}", outcome.result));
                                    ui.label(format!("下一個節點: {}", outcome.next_node));
                                    if let Some(conditions) = &outcome.conditions {
                                        ui.label("條件:");
                                        for cond in conditions {
                                            ui.label(format!(
                                                "函數: {}, 參數: {:?}",
                                                cond.function, cond.params
                                            ));
                                        }
                                    }
                                    if let Some(actions) = &outcome.actions {
                                        ui.label("動作:");
                                        for action in actions {
                                            ui.label(format!(
                                                "函數: {}, 參數: {:?}",
                                                action.function, action.params
                                            ));
                                        }
                                    }
                                }
                            }
                            Node::Condition { conditions, pos: _ } => {
                                ui.label(format!("類型: {}", node.to_string()));
                                ui.label(format!("ID: {}", node_id));
                                for cond in conditions {
                                    ui.label(format!(
                                        "函數: {}, 參數: {:?}",
                                        cond.function, cond.params
                                    ));
                                    ui.label(format!("下一個節點: {}", cond.next_node));
                                }
                            }
                            Node::End { pos: _ } => {
                                ui.label(format!("節點 ID: {}", node_id));
                                ui.label(format!("類型: {}", node.to_string()));
                            }
                        }
                    }
                } else {
                    ui.label("未選中節點");
                }
            });

        // 顯示狀態訊息
        self.show_status_message(ctx);

        // 主畫布：顯示節點和連線
        egui::CentralPanel::default().show(ctx, |ui| {
            let node_size = egui::vec2(100.0 * self.camera_zoom, 50.0 * self.camera_zoom);

            // 處理攝影機移動（右鍵拖曳）
            if ui.input(|i| i.pointer.secondary_down()) {
                self.camera_offset -= ui.input(|i| i.pointer.delta()) / self.camera_zoom;
            }

            // 處理縮放（滾輪）
            if ui.input(|i| i.raw_scroll_delta.y) != 0.0 {
                self.camera_zoom *= 1.0 + ui.input(|i| i.raw_scroll_delta.y) * 0.001;
                self.camera_zoom = self.camera_zoom.clamp(0.1, 2.0); // 限制縮放範圍

                // 調整 offset 以保持縮放中心
                if let Some(mouse_pos) = ui.input(|i| i.pointer.latest_pos()) {
                    let world_mouse = self.screen_to_world(mouse_pos);
                    self.camera_offset = world_mouse - (mouse_pos / self.camera_zoom);
                }
            }

            // 第1階段：收集所有節點資訊，準備繪製和交互
            // 預先計算節點的位置和矩形區域
            let mut node_connections = Vec::new();
            let mut node_data = Vec::new();

            // 收集連線數據
            for (_, node) in &self.script.nodes {
                let pos = convert_to_egui_pos(node.pos());
                let source_pos = self.world_to_screen(pos);
                let source_center = source_pos + node_size / 2.0;

                let next_nodes: Vec<&str> = match node {
                    Node::Dialogue { next_node, .. } => vec![next_node],
                    Node::Option { options, .. } => {
                        options.iter().map(|o| o.next_node.as_ref()).collect()
                    }
                    Node::Battle { outcomes, .. } => {
                        outcomes.iter().map(|o| o.next_node.as_ref()).collect()
                    }
                    Node::Condition { conditions, .. } => {
                        conditions.iter().map(|c| c.next_node.as_ref()).collect()
                    }
                    Node::End { .. } => Vec::new(),
                };

                for next_node_id in next_nodes {
                    if let Some(node) = self.script.nodes.get(next_node_id) {
                        let pos = convert_to_egui_pos(node.pos());
                        let target_pos = self.world_to_screen(pos);
                        let target_center = target_pos + node_size / 2.0;

                        node_connections.push((source_center, target_center));
                    }
                }
            }

            // 收集節點資訊
            for (node_id, node) in &self.script.nodes {
                let pos = convert_to_egui_pos(node.pos());
                let screen_pos = self.world_to_screen(pos);
                let node_rect = egui::Rect::from_min_size(screen_pos, node_size);
                let is_selected = self.selected_node.as_ref() == Some(node_id);
                let node_type = self.script.nodes.get(node_id).unwrap().to_string();

                node_data.push((node_id.clone(), node_rect, is_selected, node_type));
            }

            // 第2階段：處理用戶交互
            let mut node_actions = Vec::new();
            let mut clicked_node = None;

            for (node_id, node_rect, _, _) in &node_data {
                let response = ui.allocate_rect(*node_rect, egui::Sense::click_and_drag());

                // 確保只有在左鍵按下的情況下才能拖曳
                if response.dragged() && ui.input(|i| i.pointer.primary_down()) {
                    let delta = response.drag_delta();
                    let world_delta = delta / self.camera_zoom;
                    node_actions.push((node_id.clone(), world_delta));
                }

                if response.clicked() {
                    clicked_node = Some(node_id.clone());
                }
            }

            // 第3階段：繪製所有元素
            let painter = ui.painter();

            // 繪製連線
            for (start, end) in node_connections {
                painter.line_segment(
                    [start, end],
                    egui::Stroke::new(2.0, egui::Color32::LIGHT_BLUE),
                );

                // 計算箭頭
                let direction = (end - start).normalized();
                let arrow_size = 10.0; // 箭頭大小

                // 箭頭尖端位置（比終點略短一點）
                let arrow_tip = start + (end - start) / 2.0;

                // 計算箭頭的兩個翼點
                let perpendicular = egui::vec2(-direction.y, direction.x) * arrow_size;
                let left_wing = arrow_tip - direction * arrow_size + perpendicular;
                let right_wing = arrow_tip - direction * arrow_size - perpendicular;

                // 繪製箭頭（填充三角形）
                let points = vec![arrow_tip, left_wing, right_wing, arrow_tip];
                for i in 0..points.len() - 1 {
                    painter.line_segment(
                        [points[i], points[i + 1]],
                        egui::Stroke::new(2.0, egui::Color32::LIGHT_BLUE),
                    );
                }
            }

            // 繪製節點
            for (node_id, node_rect, is_selected, node_type) in &node_data {
                let color = if *is_selected {
                    egui::Color32::DARK_RED
                } else {
                    egui::Color32::DARK_GREEN
                };

                // 繪製節點背景
                painter.rect_filled(*node_rect, 0.0, color);

                // 繪製邊框
                let min = node_rect.min;
                let max = node_rect.max;
                painter.line_segment(
                    [egui::pos2(min.x, min.y), egui::pos2(max.x, min.y)],
                    egui::Stroke::new(1.0, egui::Color32::LIGHT_GRAY),
                );
                painter.line_segment(
                    [egui::pos2(max.x, min.y), egui::pos2(max.x, max.y)],
                    egui::Stroke::new(1.0, egui::Color32::LIGHT_GRAY),
                );
                painter.line_segment(
                    [egui::pos2(max.x, max.y), egui::pos2(min.x, max.y)],
                    egui::Stroke::new(1.0, egui::Color32::LIGHT_GRAY),
                );
                painter.line_segment(
                    [egui::pos2(min.x, max.y), egui::pos2(min.x, min.y)],
                    egui::Stroke::new(1.0, egui::Color32::LIGHT_GRAY),
                );

                // 繪製節點文字
                let label_pos = node_rect.min + node_size / 2.0;

                painter.text(
                    label_pos,
                    egui::Align2::CENTER_CENTER,
                    format!("<{}>\n{}", node_type, node_id),
                    egui::FontId::proportional(14.0 * self.camera_zoom),
                    egui::Color32::WHITE,
                );
            }

            // 第4階段：更新狀態
            // 更新選中節點
            if let Some(node_id) = clicked_node {
                self.selected_node = Some(node_id);
            }

            // 應用節點移動
            for (node_id, world_delta) in node_actions {
                if let Some(node) = self.script.nodes.get_mut(&node_id) {
                    let pos = convert_to_egui_pos(node.pos()) + world_delta;
                    let pos = convert_to_pos(&pos);
                    node.set_pos(pos);
                    self.has_unsaved_changes_flag = true;
                }
            }
        });
    }
}

use crate::common::*;
use dialogs_lib::{Node, Pos, Script};
use eframe::{Frame, egui};
use egui::ScrollArea;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str::FromStr;
use strum::IntoEnumIterator;

const LEFT_SIDE_PANEL_WIDTH: f32 = 400.0; // 側邊面板的寬度

#[derive(Debug, Default)]
pub struct DialogsEditor {
    script: Script,
    has_unsaved_changes_flag: bool,         // 追蹤是否有未保存的變動
    current_file_path: Option<PathBuf>,     // 目前檔案路徑
    status_message: Option<(String, bool)>, // 狀態訊息 (訊息, 是否為錯誤)
    //
    camera: Camera2D,
    //
    adding_node: bool, // 是否正在添加節點
    selected_node: Option<String>,
    temp_node_name: String, // 新節點的名稱
}

impl crate::common::New for DialogsEditor {
    fn new() -> Self {
        Self::new()
    }
}

impl DialogsEditor {
    pub fn new() -> Self {
        let mut result = Self::default();
        result.reload();
        return result;
    }

    /// 重新載入固定對話檔案（DIALOGS_FILE），失敗時保留原資料並回傳錯誤
    pub fn reload(&mut self) {
        self.load_file(dialogs_file());
    }

    /// 儲存對話資料到固定檔案（DIALOGS_FILE），失敗時回傳錯誤
    pub fn save(&mut self) {
        self.save_file(dialogs_file());
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
    pub fn update(&mut self, ctx: &egui::Context, _: &mut Frame) {
        // 頂部面板：顯示檔案選單
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                self.show_file_menu(ui);
                ui.separator();
                if ui.button("重新載入").clicked() {
                    self.reload();
                }
                if ui.button("儲存").clicked() {
                    self.save();
                }

                egui::menu::menu_button(ui, "節點", |ui| {
                    if ui.button("添加").clicked() {
                        self.adding_node = true;
                        self.temp_node_name = String::new();
                    }
                });
                egui::menu::menu_button(ui, "刪除節點", |ui| {
                    if ui.button("刪除").clicked() {
                        let Some(node_id) = self.selected_node.clone() else {
                            self.set_status(format!("請先選擇一個節點"), true);
                            return;
                        };
                        self.script.nodes.remove(&node_id);
                        self.selected_node = None;
                        self.has_unsaved_changes_flag = true;
                        self.set_status(format!("已刪除節點: {}", node_id), false);
                    }
                });
            });
        });

        // 側邊欄：顯示選中節點的詳細內容
        // 先產生 right panel, 以免 central panel 偵測到 right panel 滑鼠
        self.right_panel(ctx);

        // 主畫布：顯示節點和連線
        self.central_panel(ctx);

        // 顯示狀態訊息
        self.show_status_message(ctx);

        // 顯示添加節點和編輯節點的視窗
        if self.adding_node {
            self.show_add_node_window(ctx);
        }
    }

    fn central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let node_size = egui::vec2(100.0 * self.camera.zoom, 50.0 * self.camera.zoom);

            // 處理攝影機移動與縮放
            self.camera.handle_pan_zoom(ui);
            self.camera.handle_keyboard_zoom(ui);

            // 第1階段：收集所有節點資訊，準備繪製和交互
            // 預先計算節點的位置和矩形區域
            let mut node_connections = Vec::new();
            let mut node_data = Vec::new();

            // 收集連線數據
            let mut invalid_connection = None;
            for (node_id, node) in &self.script.nodes {
                let pos = convert_to_egui_pos(&node.pos());
                let source_pos = self.camera.world_to_screen(pos);
                let source_center = source_pos + node_size / 2.0;

                let next_nodes: Vec<&str> = match node {
                    Node::Dialogue { next_node, .. } => vec![next_node],
                    Node::Option { options, .. } => {
                        options.iter().map(|o| o.next_node.as_ref()).collect()
                    }
                    Node::Battle { results, .. } => {
                        results.iter().map(|o| o.next_node.as_ref()).collect()
                    }
                    Node::Condition { conditions, .. } => {
                        conditions.iter().map(|c| c.next_node.as_ref()).collect()
                    }
                    Node::End { .. } => Vec::new(),
                };

                for next_node_id in next_nodes {
                    let Some(node) = self.script.nodes.get(next_node_id) else {
                        invalid_connection = Some((node_id.clone(), next_node_id.to_string()));
                        continue;
                    };
                    let pos = convert_to_egui_pos(&node.pos());
                    let target_pos = self.camera.world_to_screen(pos);
                    let target_center = target_pos + node_size / 2.0;
                    node_connections.push((source_center, target_center));
                }
            }
            if let Some((node_id, next_node_id)) = invalid_connection {
                self.set_status(
                    format!("非法 next node id: {:?} -> {:?}", node_id, next_node_id),
                    true,
                );
            } else if let Some(message) = &self.status_message {
                if message.0.starts_with("非法 next node id") {
                    self.set_status(String::new(), false);
                }
            }

            // 收集節點資訊
            for (node_id, node) in &self.script.nodes {
                let pos = convert_to_egui_pos(&node.pos());
                let screen_pos = self.camera.world_to_screen(pos);
                let node_rect = egui::Rect::from_min_size(screen_pos, node_size);
                let is_selected = self.selected_node.as_ref() == Some(node_id);
                let node_type = self
                    .script
                    .nodes
                    .get(node_id)
                    .expect("nodes in race condition")
                    .to_string();

                node_data.push((node_id.clone(), node_rect, is_selected, node_type));
            }

            // 第2階段：處理用戶交互
            let mut node_action = None;
            let mut clicked_node = None;

            for (node_id, node_rect, _, _) in &node_data {
                let response = ui.allocate_rect(*node_rect, egui::Sense::click_and_drag());

                // 確保只有在左鍵按下的情況下才能拖曳
                if response.dragged() && ui.input(|i| i.pointer.primary_down()) {
                    let delta = response.drag_delta();
                    let world_delta = delta / self.camera.zoom;
                    node_action = Some((node_id.clone(), world_delta));
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
                    egui::FontId::proportional(14.0 * self.camera.zoom),
                    egui::Color32::WHITE,
                );
            }

            // 第4階段：更新狀態
            // 更新選中節點
            if let Some(node_id) = clicked_node {
                self.selected_node = Some(node_id.clone());
                self.temp_node_name = node_id;
            }

            // 應用節點移動
            if let Some((node_id, world_delta)) = node_action {
                if let Some(node) = self.script.nodes.get_mut(&node_id) {
                    let pos = convert_to_egui_pos(&node.pos()) + world_delta;
                    let pos = convert_to_pos(&pos);
                    node.set_pos(pos);
                    self.has_unsaved_changes_flag = true;
                }
            }
        });
    }

    fn right_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("details_panel")
            .min_width(LEFT_SIDE_PANEL_WIDTH)
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    // 顯示詳細資訊
                    ui.heading("節點詳情");

                    let Some(node_id) = &self.selected_node else {
                        ui.label("未選中節點");
                        return;
                    };
                    let node_id = node_id.clone();
                    let Some(_) = self.script.nodes.get(&node_id) else {
                        ui.label("節點不在設定檔案中或已被刪除");
                        return;
                    };

                    // 顯示節點 ID 和名稱編輯框
                    ui.horizontal(|ui| {
                        ui.label(format!("節點 ID: {}", &node_id));
                        ui.text_edit_singleline(&mut self.temp_node_name);
                        if ui.button("儲存").clicked() {
                            if self.temp_node_name.is_empty() {
                                self.set_status(format!("節點名稱不能為空"), true);
                                return;
                            }
                            if self.script.nodes.contains_key(&self.temp_node_name) {
                                self.set_status(format!("節點名稱已存在"), true);
                                return;
                            }
                            // 更新節點 ID
                            let node = self.script.nodes.remove(&node_id).unwrap();
                            self.script.nodes.insert(self.temp_node_name.clone(), node);
                            self.selected_node = Some(self.temp_node_name.clone());
                            self.has_unsaved_changes_flag = true;
                            self.set_status(
                                format!("已更新節點 ID 為: {}", self.temp_node_name),
                                false,
                            );
                        }
                    });
                    let Some(node_id) = &self.selected_node else {
                        ui.label("未選中節點");
                        return;
                    };
                    let node_id = node_id.clone();

                    // 顯示節點類型選擇器
                    let Some(node) = self.script.nodes.get(&node_id) else {
                        ui.label("節點不在設定檔案中或已被刪除");
                        return;
                    };
                    let node_type = node.to_string();
                    let pos = node.pos();
                    ui.label(format!("當前類型: {}", node_type.clone()));
                    ui.horizontal(|ui| {
                        ui.label("更改節點類型為：");
                        let mut changed_type = false;
                        let mut current_type = node_type.clone();

                        egui::ComboBox::from_id_salt("node_type_selector")
                            .selected_text(node_type.clone())
                            .show_ui(ui, |ui| {
                                for node_type in Node::iter() {
                                    let node_type = node_type.to_string();
                                    changed_type |= ui
                                        .selectable_value(
                                            &mut current_type,
                                            node_type.clone(),
                                            node_type,
                                        )
                                        .clicked();
                                }
                            });

                        if changed_type {
                            let mut node = Node::from_str(&current_type).unwrap();
                            node.set_pos(pos);

                            // 更新節點
                            self.script.nodes.insert(node_id.clone(), node);
                            self.has_unsaved_changes_flag = true;
                            self.set_status(format!("已更新節點類型為: {}", current_type), false);
                        }
                    });

                    ui.separator();

                    // 根據節點類型顯示與編輯特定屬性
                    match self.script.nodes.get_mut(&node_id) {
                        Some(Node::Dialogue {
                            dialogues,
                            actions,
                            next_node,
                            pos: _,
                        }) => {
                            // 顯示與編輯對話節點
                            ui.label("對話內容:");

                            // 顯示現有對話
                            let mut to_remove_idx = None;
                            for (idx, dialogue) in dialogues.iter_mut().enumerate() {
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        // 編輯發言者
                                        ui.label("發言者:");
                                        if ui.text_edit_singleline(&mut dialogue.speaker).changed()
                                        {
                                            self.has_unsaved_changes_flag = true;
                                        }

                                        // 刪除按鈕
                                        if ui.button("刪除").clicked() {
                                            to_remove_idx = Some(idx);
                                            self.has_unsaved_changes_flag = true;
                                        }
                                    });

                                    // 編輯對話文本
                                    if ui.text_edit_multiline(&mut dialogue.text).changed() {
                                        self.has_unsaved_changes_flag = true;
                                    }
                                });
                            }

                            // 刪除對話
                            if let Some(idx) = to_remove_idx {
                                dialogues.remove(idx);
                            }

                            // 添加新對話按鈕
                            if ui.button("添加對話").clicked() {
                                dialogues.push(dialogs_lib::DialogueEntry {
                                    speaker: "".to_string(),
                                    text: "".to_string(),
                                });
                                self.has_unsaved_changes_flag = true;
                            }

                            ui.separator();

                            // 編輯下一個節點
                            ui.horizontal(|ui| {
                                ui.label("下一個節點:");
                                if ui.text_edit_singleline(next_node).changed() {
                                    self.has_unsaved_changes_flag = true;
                                }
                            });

                            ui.separator();

                            // 顯示與編輯動作
                            ui.label("動作:");
                            if let Some(action_list) = actions {
                                let mut to_remove_action = None;
                                for (idx, action) in action_list.iter_mut().enumerate() {
                                    ui.group(|ui| {
                                        ui.horizontal(|ui| {
                                            // 編輯函數名稱
                                            ui.label("函數:");
                                            if ui
                                                .text_edit_singleline(&mut action.function)
                                                .changed()
                                            {
                                                self.has_unsaved_changes_flag = true;
                                            }

                                            // 刪除動作按鈕
                                            if ui.button("刪除").clicked() {
                                                to_remove_action = Some(idx);
                                                self.has_unsaved_changes_flag = true;
                                            }
                                        });

                                        // 顯示參數
                                        ui.label(format!("參數: {:?}", action.params));
                                    });
                                }

                                // 刪除動作
                                if let Some(idx) = to_remove_action {
                                    action_list.remove(idx);
                                }

                                // 添加新動作按鈕
                                if ui.button("添加動作").clicked() {
                                    action_list.push(dialogs_lib::Action {
                                        function: "new_function".to_string(),
                                        params: BTreeMap::new(),
                                    });
                                    self.has_unsaved_changes_flag = true;
                                }
                            } else {
                                // 如果沒有動作，顯示添加按鈕
                                if ui.button("添加動作").clicked() {
                                    *actions = Some(vec![dialogs_lib::Action {
                                        function: "new_function".to_string(),
                                        params: BTreeMap::new(),
                                    }]);
                                    self.has_unsaved_changes_flag = true;
                                }
                            }
                        }
                        Some(Node::Option { options, pos: _ }) => {
                            // 顯示與編輯選項節點
                            ui.label("選項:");

                            // 顯示現有選項
                            let mut to_remove_idx = None;
                            for (idx, option) in options.iter_mut().enumerate() {
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(format!("選項 {}:", idx + 1));

                                        // 刪除按鈕
                                        if ui.button("刪除").clicked() {
                                            to_remove_idx = Some(idx);
                                            self.has_unsaved_changes_flag = true;
                                        }
                                    });

                                    // 編輯選項文本
                                    ui.horizontal(|ui| {
                                        ui.label("文本:");
                                        if ui.text_edit_singleline(&mut option.text).changed() {
                                            self.has_unsaved_changes_flag = true;
                                        }
                                    });

                                    // 編輯下一個節點
                                    ui.horizontal(|ui| {
                                        ui.label("下一個節點:");
                                        if ui.text_edit_singleline(&mut option.next_node).changed()
                                        {
                                            self.has_unsaved_changes_flag = true;
                                        }
                                    });
                                });
                            }

                            // 刪除選項
                            if let Some(idx) = to_remove_idx {
                                options.remove(idx);
                            }

                            // 添加新選項按鈕
                            if ui.button("添加選項").clicked() {
                                options.push(dialogs_lib::OptionEntry {
                                    text: "New option text".to_string(),
                                    next_node: "end".to_string(),
                                    conditions: None,
                                    actions: None,
                                });
                                self.has_unsaved_changes_flag = true;
                            }
                        }
                        Some(Node::Battle { results, pos: _ }) => {
                            // 顯示與編輯戰鬥節點
                            ui.label("戰鬥結果:");

                            // 顯示現有結果
                            let mut to_remove_idx = None;
                            for (idx, result) in results.iter_mut().enumerate() {
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(format!("結果 {}:", idx + 1));

                                        // 刪除按鈕
                                        if ui.button("刪除").clicked() {
                                            to_remove_idx = Some(idx);
                                            self.has_unsaved_changes_flag = true;
                                        }
                                    });

                                    // 編輯結果文本
                                    ui.horizontal(|ui| {
                                        ui.label("結果:");
                                        if ui.text_edit_singleline(&mut result.result).changed() {
                                            self.has_unsaved_changes_flag = true;
                                        }
                                    });

                                    // 編輯下一個節點
                                    ui.horizontal(|ui| {
                                        ui.label("下一個節點:");
                                        if ui.text_edit_singleline(&mut result.next_node).changed()
                                        {
                                            self.has_unsaved_changes_flag = true;
                                        }
                                    });
                                });
                            }

                            // 刪除結果
                            if let Some(idx) = to_remove_idx {
                                results.remove(idx);
                            }

                            // 添加新結果按鈕
                            if ui.button("添加戰鬥結果").clicked() {
                                results.push(dialogs_lib::BattleResult {
                                    result: "New result".to_string(),
                                    next_node: "end".to_string(),
                                    conditions: None,
                                    actions: None,
                                });
                                self.has_unsaved_changes_flag = true;
                            }
                        }
                        Some(Node::Condition { conditions, pos: _ }) => {
                            // 顯示與編輯條件節點
                            ui.label("條件:");

                            // 顯示現有條件
                            let mut to_remove_idx = None;
                            for (idx, cond) in conditions.iter_mut().enumerate() {
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(format!("條件 {}:", idx + 1));

                                        // 刪除按鈕
                                        if ui.button("刪除").clicked() {
                                            to_remove_idx = Some(idx);
                                            self.has_unsaved_changes_flag = true;
                                        }
                                    });

                                    // 編輯函數名稱
                                    ui.horizontal(|ui| {
                                        ui.label("函數:");
                                        if ui.text_edit_singleline(&mut cond.function).changed() {
                                            self.has_unsaved_changes_flag = true;
                                        }
                                    });

                                    // 顯示參數
                                    ui.label(format!("參數: {:?}", cond.params));

                                    // 編輯下一個節點
                                    ui.horizontal(|ui| {
                                        ui.label("下一個節點:");
                                        if ui.text_edit_singleline(&mut cond.next_node).changed() {
                                            self.has_unsaved_changes_flag = true;
                                        }
                                    });
                                });
                            }

                            // 刪除條件
                            if let Some(idx) = to_remove_idx {
                                conditions.remove(idx);
                            }

                            // 添加新條件按鈕
                            if ui.button("添加條件").clicked() {
                                conditions.push(dialogs_lib::ConditionNodeEntry {
                                    function: "new_condition".to_string(),
                                    params: BTreeMap::new(),
                                    next_node: "end".to_string(),
                                });
                                self.has_unsaved_changes_flag = true;
                            }
                        }
                        Some(Node::End { pos: _ }) => {
                            ui.label("結束節點無需編輯任何內容。");
                        }
                        None => {
                            ui.label("節點不存在或已被刪除。");
                        }
                    }
                });
            });
    }

    // 顯示添加節點的視窗
    fn show_add_node_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("添加新節點")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("請輸入新節點的名稱：");
                ui.text_edit_singleline(&mut self.temp_node_name);

                ui.horizontal(|ui| {
                    if ui.button("確認").clicked() {
                        if self.temp_node_name.is_empty() {
                            self.set_status(format!("節點名稱不能為空"), true);
                            return;
                        }
                        if self.script.nodes.contains_key(&self.temp_node_name) {
                            self.set_status(format!("節點名稱已存在"), true);
                            return;
                        }
                        // 建立新節點
                        let pos = ctx.screen_rect().center();
                        let pos = self.camera.offset
                            + egui::vec2(pos.x - LEFT_SIDE_PANEL_WIDTH, pos.y) / self.camera.zoom;
                        let pos = Pos { x: pos.x, y: pos.y };
                        let node = Node::End { pos };

                        self.script.nodes.insert(self.temp_node_name.clone(), node);
                        self.selected_node = Some(self.temp_node_name.clone());
                        self.adding_node = false;
                        self.has_unsaved_changes_flag = true;
                        self.set_status(format!("已添加新節點"), false);
                    }

                    if ui.button("取消").clicked() {
                        self.adding_node = false;
                        self.temp_node_name = String::new();
                    }
                });
            });
    }

    // 顯示檔案選單
    fn show_file_menu(&mut self, ui: &mut egui::Ui) {
        return show_file_menu(ui, self);
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
        // 將節點轉換為有序的 BTreeMap 並按鍵（node_id）排序
        let nodes = self.script.nodes.clone().into_iter().collect();
        #[derive(Debug, Deserialize, Serialize, Default)]
        struct SortedScript {
            function_signatures: Vec<String>,
            nodes: BTreeMap<String, Node>,
        }
        let sorted = SortedScript {
            function_signatures: self.script.function_signatures.clone(),
            nodes,
        };

        // 使用排序後的 Script 物件進行儲存
        match to_file(&path, &sorted) {
            Ok(_) => {
                self.current_file_path = Some(path);
                self.has_unsaved_changes_flag = false;
                self.set_status(format!("成功儲存檔案"), false);
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

    // 顯示狀態訊息
    fn show_status_message(&mut self, ctx: &egui::Context) {
        if let Some((message, is_error)) = &self.status_message {
            show_status_message(ctx, message, *is_error);
        }
    }

    // 檢查是否有未保存的變動
    pub fn has_unsaved_changes(&self) -> bool {
        self.has_unsaved_changes_flag
    }
}

use dialogs_lib::{
    Action, ConditionNodeEntry, DialogueEntry, Node, OptionEntry, Outcome, Pos, Script,
};
use eframe::{Frame, egui};
use std::collections::HashMap;
use toml;

// 節點的 UI 狀態
#[derive(Clone, Debug)]
struct NodeState {
    pos: egui::Pos2,
}

pub struct DialogsEditor {
    script: Script,
    node_states: HashMap<String, NodeState>,
    selected_node: Option<String>,
    camera_offset: egui::Vec2,      // 攝影機平移量
    camera_zoom: f32,               // 攝影機縮放比例
    has_unsaved_changes_flag: bool, // 追蹤是否有未保存的變動
}

impl DialogsEditor {
    pub fn new() -> Self {
        // 模擬 TOML 數據（實際應用中應從文件讀取）
        let toml_str = r#"
            function_signature = [
                "check_item_quantity(item_id: string, operator: string, value: int) -> boolean",
                "modify_item_quantity(item_id: string, operation: string, value: int) -> boolean",
                "modify_character(character_id: string, operation: string) -> boolean",
                "always_true() -> boolean"
            ]

            [node.dialogue_1]
            type = "dialogue"
            dialogues = [
                { speaker = "NPC_001", text = "歡迎來到村莊！這是一把鑰匙，拿去吧！" },
                { speaker = "Player", text = "謝謝，我會好好使用它。" }
            ]
            actions = [
                { function = "modify_item_quantity", params = { item_id = "key_001", operation = "+", value = 1 } }
            ]
            next_node = "option_1"
            pos = { x = 100.0, y = 100.0 }

            [node.option_1]
            type = "option"
            options = [
                { text = "接受任務", next_node = "dialogue_2" },
                { text = "拒絕任務", next_node = "dialogue_3" },
                { text = "詢問更多資訊", next_node = "dialogue_4", conditions = [{ function = "check_item_quantity", params = { item_id = "map_001", operator = ">", value = 0 } }], actions = [{ function = "modify_item_quantity", params = { item_id = "map_001", operation = "-", value = 1 } }] }
            ]
            pos = { x = 250.0, y = 100.0 }

            [node.dialogue_2]
            type = "dialogue"
            dialogues = [
                { speaker = "NPC_001", text = "太好了！戰士艾倫將加入你的隊伍，一起擊敗怪獸吧！" }
            ]
            actions = [
                { function = "modify_character", params = { character_id = "char_001", operation = "+" } }
            ]
            next_node = "battle_1"
            pos = { x = 400.0, y = 100.0 }

            [node.dialogue_3]
            type = "dialogue"
            dialogues = [
                { speaker = "NPC_001", text = "沒關係，也許下次你會改變主意。" }
            ]
            next_node = "end"
            pos = { x = 250.0, y = 200.0 }

            [node.dialogue_4]
            type = "dialogue"
            dialogues = [
                { speaker = "NPC_001", text = "這個村莊正面臨怪獸的威脅，我們需要勇者幫忙！" }
            ]
            next_node = "option_1"
            pos = { x = 250.0, y = 300.0 }

            [node.battle_1]
            type = "battle"
            outcomes = [
                { result = "victory", next_node = "dialogue_5", actions = [{ function = "modify_item_quantity", params = { item_id = "map_001", operation = "+", value = 2 } }] },
                { result = "defeat", next_node = "game_over" },
                { result = "escape", next_node = "dialogue_6", conditions = [{ function = "check_item_quantity", params = { item_id = "escape_orb", operator = "=", value = 1 } }], actions = [{ function = "modify_item_quantity", params = { item_id = "escape_orb", operation = "-", value = 1 } }] }
            ]
            pos = { x = 550.0, y = 100.0 }

            [node.dialogue_5]
            type = "dialogue"
            dialogues = [
                { speaker = "NPC_001", text = "你擊敗了怪獸！村莊安全了！" }
            ]
            next_node = "end"
            pos = { x = 700.0, y = 100.0 }

            [node.dialogue_6]
            type = "dialogue"
            dialogues = [
                { speaker = "Player", text = "我成功逃脫了戰鬥，但得小心行事。" }
            ]
            next_node = "option_2"
            pos = { x = 550.0, y = 200.0 }

            [node.game_over]
            type = "dialogue"
            dialogues = [
                { speaker = "System", text = "遊戲結束！你被擊敗了。" }
            ]
            next_node = "end"
            pos = { x = 550.0, y = 300.0 }

            [node.option_2]
            type = "option"
            options = [
                { text = "繼續探索", next_node = "condition_1" },
                { text = "返回村莊", next_node = "dialogue_1", conditions = [{ function = "check_item_quantity", params = { item_id = "village_pass", operator = ">=", value = 1 } }], actions = [{ function = "modify_character", params = { character_id = "char_001", operation = "-" } }] }
            ]
            pos = { x = 700.0, y = 200.0 }

            [node.condition_1]
            type = "condition"
            conditions = [
                { function = "check_item_quantity", params = { item_id = "key_001", operator = ">", value = 0 }, next_node = "dialogue_7" },
                { function = "check_item_quantity", params = { item_id = "map_001", operator = ">=", value = 2 }, next_node = "dialogue_8" },
                { function = "always_true", params = {}, next_node = "dialogue_9" }
            ]
            pos = { x = 850.0, y = 200.0 }

            [node.dialogue_7]
            type = "dialogue"
            dialogues = [
                { speaker = "NPC_002", text = "你有鑰匙！可以進入寶藏房間。" }
            ]
            next_node = "end"
            pos = { x = 1000.0, y = 100.0 }

            [node.dialogue_8]
            type = "dialogue"
            dialogues = [
                { speaker = "NPC_002", text = "你有足夠的地圖！可以找到隱藏路徑。" }
            ]
            next_node = "end"
            pos = { x = 1000.0, y = 200.0 }

            [node.dialogue_9]
            type = "dialogue"
            dialogues = [
                { speaker = "NPC_002", text = "你需要鑰匙或足夠的地圖才能繼續。" }
            ]
            next_node = "end"
            pos = { x = 1000.0, y = 300.0 }

            [node.end]
            type = "end"
            pos = { x = 1150.0, y = 200.0 }
        "#;

        let script: Script = toml::from_str(toml_str).expect("Failed to parse TOML");
        let mut node_states = HashMap::new();
        for (node_id, node) in &script.node {
            node_states.insert(
                node_id.clone(),
                NodeState {
                    pos: egui::pos2(node.pos().x, node.pos().y),
                },
            );
        }
        Self {
            script,
            node_states,
            selected_node: None,
            camera_offset: egui::vec2(0.0, 0.0),
            camera_zoom: 1.0,
            has_unsaved_changes_flag: false,
        }
    }

    // 檢查是否有未保存的變動
    pub fn has_unsaved_changes(&self) -> bool {
        self.has_unsaved_changes_flag
    }

    // 儲存 script 回 TOML
    fn save_to_toml(&self) -> String {
        toml::to_string(&self.script).expect("Failed to serialize to TOML")
    }

    // 將節點的世界坐標轉為螢幕坐標
    fn world_to_screen(&self, world_pos: egui::Pos2) -> egui::Pos2 {
        (world_pos - self.camera_offset) * self.camera_zoom
    }

    // 將螢幕坐標轉為世界坐標
    fn screen_to_world(&self, screen_pos: egui::Pos2) -> egui::Pos2 {
        screen_pos / self.camera_zoom + self.camera_offset
    }

    pub fn update(&mut self, ctx: &egui::Context, _frame: &Frame) {
        // 側邊欄：顯示選中節點的詳細內容
        egui::SidePanel::right("details").show(ctx, |ui| {
            ui.heading("節點詳情");
            if let Some(node_id) = &self.selected_node {
                if let Some(node) = self.script.node.get(node_id) {
                    match node {
                        Node::Dialogue {
                            dialogues,
                            actions,
                            next_node,
                            pos,
                        } => {
                            ui.label(format!("節點 ID: {}", node_id));
                            ui.label(format!("類型: {}", node.to_string()));
                            ui.label(format!("位置: ({}, {})", pos.x, pos.y));
                            for dialogue in dialogues {
                                ui.label(format!("說者: {} - {}", dialogue.speaker, dialogue.text));
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
                        Node::Option { options, pos } => {
                            ui.label(format!("節點 ID: {}", node_id));
                            ui.label(format!("類型: {}", node.to_string()));
                            ui.label(format!("位置: ({}, {})", pos.x, pos.y));
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
                        Node::Battle { outcomes, pos } => {
                            ui.label(format!("節點 ID: {}", node_id));
                            ui.label(format!("類型: {}", node.to_string()));
                            ui.label(format!("位置: ({}, {})", pos.x, pos.y));
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
                        Node::Condition { conditions, pos } => {
                            ui.label(format!("節點 ID: {}", node_id));
                            ui.label(format!("類型: {}", node.to_string()));
                            ui.label(format!("位置: ({}, {})", pos.x, pos.y));
                            for cond in conditions {
                                ui.label(format!(
                                    "函數: {}, 參數: {:?}",
                                    cond.function, cond.params
                                ));
                                ui.label(format!("下一個節點: {}", cond.next_node));
                            }
                        }
                        Node::End { pos } => {
                            ui.label(format!("節點 ID: {}", node_id));
                            ui.label(format!("類型: {}", node.to_string()));
                            ui.label(format!("位置: ({}, {})", pos.x, pos.y));
                        }
                    }
                }
            } else {
                ui.label("未選中節點");
            }

            // 儲存按鈕
            if ui.button("儲存為 TOML").clicked() {
                let toml_output = self.save_to_toml();
                println!("TOML 輸出:\n{}", toml_output);
                self.has_unsaved_changes_flag = false;
            }
        });

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
            for (node_id, node) in &self.script.node {
                let source_state = self.node_states.get(node_id).unwrap();
                let source_pos = self.world_to_screen(source_state.pos);
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
                    if let Some(target_state) = self.node_states.get(next_node_id) {
                        let target_pos = self.world_to_screen(target_state.pos);
                        let target_center = target_pos + node_size / 2.0;

                        node_connections.push((source_center, target_center));
                    }
                }
            }

            // 收集節點資訊
            for (node_id, state) in &self.node_states {
                let screen_pos = self.world_to_screen(state.pos);
                let node_rect = egui::Rect::from_min_size(screen_pos, node_size);
                let is_selected = self.selected_node.as_ref() == Some(node_id);
                let node_type = self.script.node.get(node_id).unwrap().to_string();

                node_data.push((node_id.clone(), node_rect, is_selected, node_type));
            }

            // 第2階段：處理用戶交互
            let mut node_actions = Vec::new();
            let mut clicked_node = None;

            for (node_id, node_rect, _, _) in &node_data {
                let response = ui.allocate_rect(*node_rect, egui::Sense::click_and_drag());

                if response.dragged() {
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
            }

            // 繪製節點
            for (node_id, node_rect, is_selected, node_type) in &node_data {
                let color = if *is_selected {
                    egui::Color32::LIGHT_GREEN
                } else {
                    egui::Color32::LIGHT_GRAY
                };

                // 繪製節點背景
                painter.rect_filled(*node_rect, 0.0, color);

                // 繪製邊框
                let min = node_rect.min;
                let max = node_rect.max;
                painter.line_segment(
                    [egui::pos2(min.x, min.y), egui::pos2(max.x, min.y)],
                    egui::Stroke::new(1.0, egui::Color32::BLACK),
                );
                painter.line_segment(
                    [egui::pos2(max.x, min.y), egui::pos2(max.x, max.y)],
                    egui::Stroke::new(1.0, egui::Color32::BLACK),
                );
                painter.line_segment(
                    [egui::pos2(max.x, max.y), egui::pos2(min.x, max.y)],
                    egui::Stroke::new(1.0, egui::Color32::BLACK),
                );
                painter.line_segment(
                    [egui::pos2(min.x, max.y), egui::pos2(min.x, min.y)],
                    egui::Stroke::new(1.0, egui::Color32::BLACK),
                );

                // 繪製節點文字
                let label_pos = node_rect.min + node_size / 2.0;

                painter.text(
                    label_pos,
                    egui::Align2::CENTER_CENTER,
                    format!("{} ({})", node_id, node_type),
                    egui::FontId::proportional(14.0 * self.camera_zoom),
                    egui::Color32::BLACK,
                );
            }

            // 第4階段：更新狀態
            // 更新選中節點
            if let Some(node_id) = clicked_node {
                self.selected_node = Some(node_id);
            }

            // 應用節點移動
            for (node_id, world_delta) in node_actions {
                if let Some(state) = self.node_states.get_mut(&node_id) {
                    state.pos = state.pos + world_delta;

                    // 更新節點在 script 中的位置
                    if let Some(node) = self.script.node.get_mut(&node_id) {
                        node.set_pos(Pos {
                            x: state.pos.x,
                            y: state.pos.y,
                        });
                        self.has_unsaved_changes_flag = true;
                    }
                }
            }
        });
    }
}

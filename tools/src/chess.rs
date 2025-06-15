use crate::common::{
    Camera2D, FileOperator, from_file, show_file_menu, show_status_message, to_file,
};
use chess_lib::{
    BattleObjectiveType, Battlefield, BattlefieldObject, Cell, PLAYER_TEAM, Pos, Team, Terrain,
    Unit,
};
use eframe::{Frame, egui};
use egui::{Button, Color32, Rect, Stroke};
use rand::{Rng, distributions::Alphanumeric};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

const DEPLOYMENT_CELL_SIZE: f32 = 0.5;
const OBJECT_CELL_SIZE: f32 = 0.2;
const UNIT_CELL_SIZE: f32 = 0.3;

// ===== 擴展 TerrainType 功能 =====

trait TerrainTypeExt {
    /// 獲取地形的顯示顏色
    fn color(&self) -> Color32;

    /// 獲取地形的顯示名稱
    fn name(&self) -> &'static str;
}

impl TerrainTypeExt for Terrain {
    fn color(&self) -> Color32 {
        match self {
            Terrain::Plain => Color32::from_rgb(220, 220, 170), // 淺黃色
            Terrain::Hill => Color32::from_rgb(190, 170, 130),  // 棕黃色
            Terrain::Mountain => Color32::from_rgb(150, 140, 130), // 灰棕色
            Terrain::Forest => Color32::from_rgb(100, 170, 100), // 綠色
            Terrain::ShallowWater => Color32::from_rgb(150, 200, 255), // 淺藍色
            Terrain::DeepWater => Color32::from_rgb(100, 150, 255), // 深藍色
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Terrain::Plain => "平原",
            Terrain::Hill => "丘陵",
            Terrain::Mountain => "山地",
            Terrain::Forest => "森林",
            Terrain::ShallowWater => "淺水",
            Terrain::DeepWater => "深水",
        }
    }
}

// ===== 擴展 BattlefieldObject 功能 =====

trait BattlefieldObjectExt {
    /// 獲取物件的顯示名稱
    fn name(&self) -> &'static str;
}

impl BattlefieldObjectExt for BattlefieldObject {
    fn name(&self) -> &'static str {
        match self {
            BattlefieldObject::Wall => "牆壁",
            BattlefieldObject::Tent1 { .. } => "帳篷(1格)",
            BattlefieldObject::Tent9 { .. } => "帳篷(9格)",
        }
    }
}

// ===== 擴展 Battlefield 功能 =====

trait BattlefieldEditorExt {
    fn set_deployable(&mut self, pos: Pos, deployable: bool) -> bool;
    fn resize(&mut self, new_width: usize, new_height: usize);
    fn add_team(&mut self, team_id: &str) -> bool;
    fn remove_team(&mut self, team_id: &str) -> bool;
}

impl BattlefieldEditorExt for Battlefield {
    fn set_deployable(&mut self, pos: Pos, deployable: bool) -> bool {
        if !self.is_valid_position(pos) {
            return false;
        }

        if deployable {
            self.deployable_positions.insert(pos);
        } else {
            self.deployable_positions.remove(&pos);
        }

        true
    }

    fn resize(&mut self, new_width: usize, new_height: usize) {
        // 調整每一行的寬度
        for row in &mut self.grid {
            if row.len() < new_width {
                // 如果新寬度更大，添加新格子
                row.resize_with(new_width, Cell::default);
            } else if row.len() > new_width {
                // 如果新寬度更小，刪除多餘格子
                row.truncate(new_width);
            }
        }

        // 調整高度
        if self.grid.len() < new_height {
            // 如果新高度更大，添加新行
            let width = if self.grid.is_empty() {
                new_width
            } else {
                self.grid[0].len()
            };
            self.grid.resize_with(new_height, || {
                let mut row = Vec::with_capacity(width);
                row.resize_with(width, Cell::default);
                row
            });
        } else if self.grid.len() > new_height {
            // 如果新高度更小，刪除多餘行
            self.grid.truncate(new_height);
        }

        // 移除超出範圍的部署位置
        self.deployable_positions
            .retain(|pos| pos.x < new_width && pos.y < new_height);
    }

    fn add_team(&mut self, team_id: &str) -> bool {
        // 檢查是否已存在同ID的隊伍
        if self.teams.contains_key(team_id) {
            return false;
        }

        self.teams.insert(
            team_id.to_string(),
            Team {
                id: team_id.to_string(),
                color: chess_lib::Color {
                    r: 0,
                    g: 100,
                    b: 255,
                },
            },
        );
        true
    }

    fn remove_team(&mut self, team_id: &str) -> bool {
        self.teams.remove(team_id).is_some()
    }
}

/// 戰場數據集
#[derive(Debug, Clone)]
struct BattlefieldData {
    battlefields: HashMap<String, Battlefield>,
}

impl BattlefieldData {
    fn from_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let battlefields = from_file(path)?;
        Ok(Self { battlefields })
    }

    /// 修改戰場 id
    fn rename_battlefield(&mut self, old_id: &str, new_id: &str) -> Result<(), String> {
        if !self.battlefields.contains_key(old_id) {
            return Err("找不到指定的戰場".to_string());
        }
        if self.battlefields.contains_key(new_id) {
            return Err("新戰場 ID 已存在".to_string());
        }
        let mut battlefield = self.battlefields.remove(old_id).unwrap();
        battlefield.id = new_id.to_string();
        self.battlefields.insert(new_id.to_string(), battlefield);
        Ok(())
    }

    fn save_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        to_file(path, &self.battlefields)
    }

    /// 新增戰場
    fn create_battlefield(
        &mut self,
        battlefield_id: &str,
        width: usize,
        height: usize,
    ) -> Result<(), String> {
        if self.battlefields.contains_key(battlefield_id) {
            return Err("戰場 ID 已存在".to_string());
        }
        self.battlefields.insert(
            battlefield_id.to_string(),
            Battlefield::new(battlefield_id, width, height),
        );
        Ok(())
    }

    /// 刪除戰場
    fn delete_battlefield(&mut self, battlefield_id: &str) -> Result<(), String> {
        if !self.battlefields.contains_key(battlefield_id) {
            return Err("找不到指定的戰場".to_string());
        }
        self.battlefields.remove(battlefield_id);
        Ok(())
    }
}

// ===== 編輯器實現 =====

/// 編輯模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
enum EditMode {
    Terrain,    // 地形編輯
    Object,     // 物件編輯
    Deployment, // 部署區域設定
    Unit,       // 單位放置
    Objective,  // 目標設定
    Simulation, // 模擬戰鬥
}

impl EditMode {
    fn name(&self) -> &'static str {
        match self {
            EditMode::Terrain => "地形",
            EditMode::Object => "物件",
            EditMode::Deployment => "部署區域",
            EditMode::Unit => "單位",
            EditMode::Objective => "目標",
            EditMode::Simulation => "模擬",
        }
    }
}

/// 確認操作類型
#[derive(Debug, Clone)]
enum ConfirmationAction {
    None,
    DeleteBattlefield(String),
    ResizeBattlefield { width: usize, height: usize },
}

// 添加一個枚舉來跟蹤UI按鈕操作，避免直接在閉包中修改自身
#[derive(Debug, Clone)]
enum TeamAction {
    None,
    AddTeam(String, String),    // battlefield_id, team_id
    RemoveTeam(String, String), // battlefield_id, team_id
}

/// 戰棋編輯器
pub struct ChessEditor {
    battlefield_data: BattlefieldData,
    has_unsaved_changes_flag: bool,
    current_file_path: Option<PathBuf>,
    status_message: Option<(String, bool)>, // message, is_error

    // 編輯器狀態
    selected_battlefield: Option<String>,
    editing_battlefield_id: Option<String>, // 雙擊進入編輯狀態的 id
    editing_battlefield_id_value: String,   // 編輯框內容
    new_battlefield_id: String,
    new_battlefield_width: usize,
    new_battlefield_height: usize,

    // 編輯工具
    edit_mode: EditMode,
    selected_terrain: Terrain,
    selected_object: Option<BattlefieldObject>,
    new_unit_type: String,
    durability: i32, // 預設耐久度

    // 隊伍管理
    selected_team_id: String,
    editing_team_id: Option<(String, String)>, // (battlefield_id, team_id)
    editing_team_id_value: String,
    new_team_id: String,
    team_action: TeamAction,

    // 目標編輯
    selected_objective: Option<String>,

    // 視野狀態
    camera: Camera2D,

    // 地形塗刷狀態
    last_painted_pos: Option<Pos>,

    simulation_selected_unit: Option<Pos>,
    simulation_battle: Option<chess_lib::Battle>,
    simulation_battlefield: Option<Battlefield>,

    // 對話框
    show_confirmation_dialog: bool,
    confirmation_action: ConfirmationAction,
    show_resize_dialog: bool,
    resize_width: usize,
    resize_height: usize,
}

impl Default for ChessEditor {
    fn default() -> Self {
        Self {
            battlefield_data: BattlefieldData {
                battlefields: HashMap::new(),
            },
            has_unsaved_changes_flag: false,
            current_file_path: None,
            status_message: None,

            selected_battlefield: None,
            editing_battlefield_id: None,
            editing_battlefield_id_value: String::new(),
            new_battlefield_id: String::new(),
            new_battlefield_width: 10,
            new_battlefield_height: 10,

            edit_mode: EditMode::Terrain,
            selected_terrain: Terrain::Plain,
            selected_object: None,
            new_unit_type: String::new(),
            durability: 10, // 預設耐久度

            selected_team_id: PLAYER_TEAM.to_string(),
            editing_team_id: None,
            editing_team_id_value: String::new(),
            new_team_id: String::new(),
            team_action: TeamAction::None,

            selected_objective: None,

            camera: Camera2D::default(),

            last_painted_pos: None,

            simulation_selected_unit: None,
            simulation_battle: None,
            simulation_battlefield: None,

            show_confirmation_dialog: false,
            confirmation_action: ConfirmationAction::None,
            show_resize_dialog: false,
            resize_width: 10,
            resize_height: 10,
        }
    }
}

impl FileOperator<PathBuf> for ChessEditor {
    fn current_file_path(&self) -> Option<PathBuf> {
        self.current_file_path.clone()
    }

    fn load_file(&mut self, path: PathBuf) {
        match BattlefieldData::from_file(&path) {
            Ok(battlefield_data) => {
                let current_file_path = Some(path);
                *self = Self {
                    battlefield_data,
                    current_file_path,
                    ..Default::default()
                };
                self.set_status(format!("成功載入戰場檔案"), false);
            }
            Err(err) => {
                self.set_status(format!("載入戰場檔案失敗: {}", err), true);
            }
        }
    }

    fn save_file(&mut self, path: PathBuf) {
        match self.battlefield_data.save_to_file(&path) {
            Ok(_) => {
                self.current_file_path = Some(path);
                self.has_unsaved_changes_flag = false;
                self.set_status(format!("成功儲存戰場檔案"), false);
            }
            Err(err) => {
                self.set_status(format!("儲存戰場檔案失敗: {}", err), true);
            }
        }
    }

    fn set_status(&mut self, status: String, is_error: bool) {
        self.status_message = Some((status, is_error));
    }
}

impl ChessEditor {
    /// 產生在 battlefield 下唯一的 unit id
    fn generate_unique_unit_id(battlefield: &Battlefield, unit_type: &str) -> String {
        let mut rng = rand::thread_rng();
        loop {
            let rand_str: String = (&mut rng)
                .sample_iter(&Alphanumeric)
                .take(6)
                .map(char::from)
                .collect();
            let id = format!("{unit_type}-{rand_str}");
            if !battlefield.unit_id_to_unit.contains_key(&id) {
                return id;
            }
        }
    }
    /// 檢查是否有未保存的變動
    pub fn has_unsaved_changes(&self) -> bool {
        self.has_unsaved_changes_flag
    }

    /// 顯示狀態信息
    fn show_status_message(&mut self, ctx: &egui::Context) {
        if let Some((message, is_error)) = &self.status_message {
            show_status_message(ctx, message, *is_error);
        }
    }

    /// 創建新戰場
    fn create_battlefield(&mut self) {
        if self.new_battlefield_id.is_empty() {
            self.set_status("戰場 ID 不能為空".to_string(), true);
            return;
        }

        match self.battlefield_data.create_battlefield(
            &self.new_battlefield_id,
            self.new_battlefield_width,
            self.new_battlefield_height,
        ) {
            Ok(_) => {
                // 創建後直接選中這個戰場
                self.selected_battlefield = Some(self.new_battlefield_id.clone());
                self.new_battlefield_id.clear();
                self.has_unsaved_changes_flag = true;
                self.set_status(format!("成功創建戰場"), false);
            }
            Err(err) => {
                self.set_status(err, true);
            }
        }
    }

    /// 顯示戰場列表
    fn show_battlefield_list(&mut self, ui: &mut egui::Ui) {
        ui.heading("戰場列表");

        ui.horizontal(|ui| {
            ui.label("新增戰場 ID:");
            ui.text_edit_singleline(&mut self.new_battlefield_id);
        });

        ui.horizontal(|ui| {
            ui.label("寬度:");
            ui.add(egui::DragValue::new(&mut self.new_battlefield_width).range(5..=50));
            ui.label("高度:");
            ui.add(egui::DragValue::new(&mut self.new_battlefield_height).range(5..=50));
        });

        if ui.button("新增戰場").clicked() {
            self.create_battlefield();
        }

        ui.add_space(10.0);
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            // 收集所有戰場 ID 並按字母順序排序
            let mut battlefield_ids = self
                .battlefield_data
                .battlefields
                .keys()
                .cloned()
                .collect::<Vec<_>>();
            battlefield_ids.sort();

            // 顯示排序後的戰場列表
            for battlefield_id in battlefield_ids {
                let selected =
                    self.selected_battlefield.as_deref() == Some(battlefield_id.as_str());

                // 雙擊進入編輯狀態
                if self.editing_battlefield_id.as_deref() == Some(battlefield_id.as_str()) {
                    // 編輯狀態
                    let response = ui.text_edit_singleline(&mut self.editing_battlefield_id_value);
                    if response.lost_focus() && ui.input(|i| !i.pointer.any_down()) {
                        let new_id = self.editing_battlefield_id_value.trim();
                        let old_id = &battlefield_id;
                        if !new_id.is_empty() && new_id != old_id {
                            match self.battlefield_data.rename_battlefield(old_id, new_id) {
                                Ok(_) => {
                                    self.selected_battlefield = Some(new_id.to_string());
                                    self.has_unsaved_changes_flag = true;
                                    self.set_status("成功修改戰場 ID".to_string(), false);
                                }
                                Err(err) => {
                                    self.set_status(err, true);
                                }
                            }
                        }
                        self.editing_battlefield_id = None;
                    }
                } else {
                    // 顯示按鈕，支援雙擊
                    let button = Button::new(battlefield_id.as_str())
                        .fill(if selected {
                            ui.style().visuals.selection.bg_fill
                        } else {
                            ui.style().visuals.widgets.noninteractive.bg_fill
                        })
                        .min_size(egui::vec2(ui.available_width(), 0.0));
                    let response = ui.add(button);
                    if response.clicked() {
                        self.selected_battlefield = Some(battlefield_id.clone());
                    }
                    if response.double_clicked() {
                        self.editing_battlefield_id = Some(battlefield_id.clone());
                        self.editing_battlefield_id_value = battlefield_id.clone();
                    }
                }
            }
        });

        ui.separator();

        // 操作按鈕
        if let Some(battlefield_id) = self.selected_battlefield.clone() {
            ui.horizontal(|ui| {
                if ui.button("刪除戰場").clicked() {
                    self.confirmation_action =
                        ConfirmationAction::DeleteBattlefield(battlefield_id.clone());
                    self.show_confirmation_dialog = true;
                }

                if ui.button("調整大小").clicked() {
                    if let Some(battlefield) =
                        self.battlefield_data.battlefields.get(&battlefield_id)
                    {
                        self.resize_width = battlefield.width();
                        self.resize_height = battlefield.height();
                        self.show_resize_dialog = true;
                    }
                }
            });
        }
    }

    /// 顯示編輯模式選擇器
    fn show_edit_mode_selector(&mut self, ui: &mut egui::Ui) {
        ui.heading("編輯工具");

        ui.horizontal_wrapped(|ui| {
            for mode in EditMode::iter() {
                if ui
                    .selectable_label(self.edit_mode == mode, mode.name())
                    .clicked()
                {
                    // 若從 Simulation 切換到其他模式，清空 simulation_battlefield/battle
                    if self.edit_mode == EditMode::Simulation && mode != EditMode::Simulation {
                        self.simulation_battlefield = None;
                        self.simulation_battle = None;
                        self.simulation_selected_unit = None;
                    }
                    self.edit_mode = mode;
                }
            }
        });

        ui.add_space(10.0);
        ui.separator();

        // 根據當前編輯模式顯示相應的工具面板
        match self.edit_mode {
            EditMode::Terrain => self.show_terrain_tools(ui),
            EditMode::Object => self.show_object_tools(ui),
            EditMode::Deployment => self.show_deployment_tools(ui),
            EditMode::Unit => self.show_unit_tools(ui),
            EditMode::Objective => self.show_objective_tools(ui),
            EditMode::Simulation => self.show_simulation_tools(ui),
        }
    }

    /// 顯示模擬戰鬥頁面
    fn show_simulation_tools(&mut self, ui: &mut egui::Ui) {
        ui.heading("模擬戰鬥");
        if let Some(battle) = self.simulation_battle.as_ref() {
            ui.label(format!("目前行動單位: {}", battle.active_unit_id));
            if ui.button("下一單位").clicked() {
                if let Some(battle) = self.simulation_battle.as_mut() {
                    let prev_round = battle.round;
                    battle.end_turn();
                    self.simulation_selected_unit = None;
                    if battle.round > prev_round {
                        self.set_status("已進入下一輪".to_string(), false);
                    }
                }
            }
        }
    }

    /// 顯示地形編輯工具
    fn show_terrain_tools(&mut self, ui: &mut egui::Ui) {
        ui.heading("地形編輯");

        egui::Grid::new("terrain_grid").show(ui, |ui| {
            for (i, terrain) in Terrain::iter().enumerate() {
                let selected = self.selected_terrain == terrain;
                let button = egui::Button::new(
                    egui::RichText::new(terrain.name())
                        .color(Color32::BLACK) // 設置文字顏色為黑色，避免與背景混淆
                        .strong(),
                )
                .fill(terrain.color())
                .stroke(if selected {
                    Stroke::new(2.0, Color32::WHITE)
                } else {
                    Stroke::new(1.0, Color32::BLACK)
                });

                if ui.add(button).clicked() {
                    self.selected_terrain = terrain;
                }

                // 每行顯示3個選項
                if i % 3 == 2 {
                    ui.end_row();
                }
            }
        });

        ui.add_space(10.0);
        ui.label("點擊網格修改地形");
    }

    /// 顯示物件編輯工具
    fn show_object_tools(&mut self, ui: &mut egui::Ui) {
        ui.heading("物件編輯");

        // 耐久度設定
        ui.horizontal(|ui| {
            ui.label("耐久度:");
            ui.add(egui::DragValue::new(&mut self.durability).range(-1..=999));
        });

        // 顯示"無"選項
        if ui
            .selectable_label(self.selected_object.is_none(), "無")
            .clicked()
        {
            self.selected_object = None;
        }

        // 顯示其他物件選項
        for object in BattlefieldObject::iter() {
            let selected = match self.selected_object {
                Some(obj) => obj == object,
                None => false,
            };

            if ui.selectable_label(selected, object.name()).clicked() {
                self.selected_object = Some(object);
            }
        }

        ui.add_space(10.0);
        ui.label("點擊網格放置物件");
    }

    /// 顯示部署區域設定工具
    fn show_deployment_tools(&mut self, ui: &mut egui::Ui) {
        ui.heading("部署區域設定");

        ui.label("點擊網格設定/取消部署區域");

        ui.add_space(10.0);
        ui.separator();

        if let Some(battlefield_id) = &self.selected_battlefield {
            if let Some(battlefield) = self.battlefield_data.battlefields.get(battlefield_id) {
                ui.label(format!(
                    "已設定 {} 個部署格子",
                    battlefield.deployable_positions.len()
                ));
            }
        } else {
            ui.label("請先選擇一個戰場");
        }
    }

    /// 顯示單位編輯工具
    fn show_unit_tools(&mut self, ui: &mut egui::Ui) {
        ui.heading("單位編輯");

        ui.horizontal(|ui| {
            ui.label("單位種類:");
            ui.text_edit_singleline(&mut self.new_unit_type);
        });

        // 取得目前戰場的隊伍列表
        let teams = self
            .selected_battlefield
            .as_ref()
            .and_then(|battlefield_id| {
                self.battlefield_data
                    .battlefields
                    .get(battlefield_id)
                    .map(|bf| bf.teams.keys().cloned().collect::<Vec<_>>())
            });

        ui.add_space(5.0);

        match teams {
            Some(team_ids) if !team_ids.is_empty() => {
                ui.horizontal(|ui| {
                    ui.label("所屬隊伍:");
                    egui::ComboBox::from_id_salt("unit_team_select")
                        .selected_text(&self.selected_team_id)
                        .show_ui(ui, |ui| {
                            for team_id in &team_ids {
                                ui.selectable_value(
                                    &mut self.selected_team_id,
                                    (*team_id).to_string(),
                                    team_id,
                                );
                            }
                        });
                });
            }
            _ => {
                ui.label("請先新增隊伍，才能設定單位所屬隊伍");
            }
        }

        ui.label("點擊網格放置/移除單位");
        ui.label("(清空種類後點擊可移除單位)");
    }

    /// 顯示目標設定工具
    fn show_objective_tools(&mut self, ui: &mut egui::Ui) {
        ui.heading("目標設定");

        if let Some(battlefield_id) = &self.selected_battlefield {
            if let Some(battlefield) = self.battlefield_data.battlefields.get(battlefield_id) {
                match &battlefield.objectives {
                    BattleObjectiveType::Alternative { objectives } => {
                        ui.label("選擇性目標 (完成其中一個即可)");

                        if objectives.is_empty() {
                            ui.label("尚未設定目標");
                        }

                        // 顯示目標列表
                        for (id, _) in objectives {
                            ui.label(id);
                        }

                        // 簡化：暫時只提示功能尚未完全實現
                        ui.add_space(5.0);
                        ui.label("(目標編輯功能尚未完全實現)");
                    }
                    _ => {
                        ui.label("目標類型有誤，應為選擇性目標");
                    }
                }
            }
        } else {
            ui.label("請先選擇一個戰場");
        }
    }

    /// 顯示隊伍管理界面 - 使用組合方式實現，避免自我借用的問題
    fn show_team_management(&mut self, ui: &mut egui::Ui) {
        ui.heading("隊伍管理");

        // 1. 首先收集我們需要的信息
        let battlefield_id_opt = self.selected_battlefield.clone();
        let teams_opt = battlefield_id_opt.as_ref().and_then(|id| {
            self.battlefield_data
                .battlefields
                .get(id)
                .map(|bf| bf.teams.values().cloned().collect::<Vec<_>>())
        });

        // 2. 在UI中顯示這些信息和控制項
        if let (Some(battlefield_id), Some(teams)) = (battlefield_id_opt, teams_opt) {
            // 顯示新增隊伍界面
            ui.horizontal(|ui| {
                ui.label("新增隊伍 ID:");
                ui.text_edit_singleline(&mut self.new_team_id);

                if ui.button("新增").clicked() && !self.new_team_id.is_empty() {
                    // 這裡只記錄操作，實際執行放在update函數中
                    self.team_action =
                        TeamAction::AddTeam(battlefield_id.clone(), self.new_team_id.clone());
                }
            });

            ui.separator();

            // 顯示隊伍列表
            ui.label("現有隊伍:");

            for team in teams {
                ui.horizontal(|ui| {
                    // 雙擊隊伍 id 進入編輯狀態
                    if self.editing_team_id.as_ref()
                        == Some(&(battlefield_id.clone(), team.id.clone()))
                    {
                        let response = ui.text_edit_singleline(&mut self.editing_team_id_value);
                        if response.lost_focus() && ui.input(|i| !i.pointer.any_down()) {
                            let new_id = self.editing_team_id_value.trim();
                            let old_id = &team.id;
                            if !new_id.is_empty() && new_id != old_id {
                                if let Some(battlefield) =
                                    self.battlefield_data.battlefields.get_mut(&battlefield_id)
                                {
                                    if !battlefield.teams.contains_key(new_id) {
                                        if let Some(mut t) = battlefield.teams.remove(old_id) {
                                            t.id = new_id.to_string();
                                            battlefield.teams.insert(new_id.to_string(), t);
                                            // 同步更新所有單位的 team_id
                                            for unit in battlefield.unit_id_to_unit.values_mut() {
                                                if unit.team_id == *old_id {
                                                    unit.team_id = new_id.to_string();
                                                }
                                            }
                                            // 同步更新目前選擇的隊伍（新增單位時預設隊伍）
                                            if self.selected_team_id == *old_id {
                                                self.selected_team_id = new_id.to_string();
                                            }
                                            self.has_unsaved_changes_flag = true;
                                        }
                                    }
                                }
                            }
                            self.editing_team_id = None;
                        }
                    } else {
                        let label = egui::Label::new(&team.id).sense(egui::Sense::click());
                        let response = ui.add(label);
                        if response.double_clicked() {
                            self.editing_team_id = Some((battlefield_id.clone(), team.id.clone()));
                            self.editing_team_id_value = team.id.clone();
                        }
                    }

                    // 顏色選擇器
                    let mut color =
                        egui::Color32::from_rgb(team.color.r, team.color.g, team.color.b);
                    if ui.color_edit_button_srgba(&mut color).changed() {
                        if let Some(battlefield) =
                            self.battlefield_data.battlefields.get_mut(&battlefield_id)
                        {
                            // 直接覆蓋 team
                            let mut new_team = team.clone();
                            new_team.color = chess_lib::Color {
                                r: color.r(),
                                g: color.g(),
                                b: color.b(),
                            };
                            battlefield.teams.insert(new_team.id.clone(), new_team);
                            self.has_unsaved_changes_flag = true;
                        }
                    }

                    if team.id != PLAYER_TEAM && ui.button("刪除").clicked() {
                        // 同樣，記錄操作以便稍後執行
                        self.team_action =
                            TeamAction::RemoveTeam(battlefield_id.clone(), team.id.clone());
                    }
                });
            }
        } else {
            ui.label("請先選擇一個戰場");
        }
    }

    // 處理團隊操作
    fn process_team_action(&mut self) {
        match &self.team_action {
            TeamAction::None => {}
            TeamAction::AddTeam(battlefield_id, team_id) => {
                if let Some(battlefield) =
                    self.battlefield_data.battlefields.get_mut(battlefield_id)
                {
                    if battlefield.add_team(team_id) {
                        self.has_unsaved_changes_flag = true;
                        self.new_team_id.clear();
                    } else {
                        self.set_status("隊伍 ID 已存在".to_string(), true);
                    }
                }
            }
            TeamAction::RemoveTeam(battlefield_id, team_id) => {
                if let Some(battlefield) =
                    self.battlefield_data.battlefields.get_mut(battlefield_id)
                {
                    if battlefield.remove_team(team_id) {
                        self.has_unsaved_changes_flag = true;
                    }
                }
            }
        }
        // 重設團隊操作
        self.team_action = TeamAction::None;
    }

    /// 顯示戰場編輯區域
    fn show_battlefield_editor(&mut self, ui: &mut egui::Ui) {
        if self.selected_battlefield.is_none() {
            self.handle_no_battlefield_selected(ui);
            return;
        }

        if self.edit_mode == EditMode::Simulation {
            self.handle_simulation_mode(ui);
            return;
        }

        if let Some((width, height, grid, deployable_positions)) =
            self.get_battlefield_render_data()
        {
            self.draw_battlefield_area(ui, width, height, &grid, &deployable_positions);
        } else {
            ui.heading("戰場編輯器");
            ui.label("選中的戰場不存在");
        }
    }

    fn handle_no_battlefield_selected(&self, ui: &mut egui::Ui) {
        ui.heading("戰場編輯器");
        ui.label("選擇或創建一個戰場開始編輯");
    }

    fn handle_simulation_mode(&mut self, ui: &mut egui::Ui) {
        // 初始化 simulation_battle
        if self.simulation_battle.is_none() {
            if let Some(battlefield_id) = &self.selected_battlefield {
                let Some(bf) = self.battlefield_data.battlefields.get(battlefield_id) else {
                    self.set_status("選中的戰場不存在".to_string(), true);
                    return;
                };
                let unit_ids: Vec<String> = bf.unit_id_to_unit.keys().cloned().collect();
                let result = chess_lib::Battle::default().start(unit_ids);
                let battle = match result {
                    Ok(battle) => battle,
                    Err(msg) => {
                        self.set_status(msg, true);
                        return;
                    }
                };
                self.simulation_battlefield = Some(bf.clone());
                self.simulation_battle = Some(battle);
            }
        }

        // 顯示戰場
        if let Some((width, height, grid, deployable_positions)) =
            self.get_battlefield_render_data()
        {
            self.draw_battlefield_area(ui, width, height, &grid, &deployable_positions);
        } else {
            ui.heading("戰場編輯器");
            ui.label("選中的戰場不存在");
        }
    }

    fn get_battlefield_render_data(
        &self,
    ) -> Option<(
        usize,
        usize,
        Vec<Vec<Cell>>,
        std::collections::BTreeSet<Pos>,
    )> {
        let battlefield = if self.edit_mode == EditMode::Simulation {
            self.simulation_battlefield.as_ref()
        } else {
            let battlefield_id = self.selected_battlefield.as_ref().unwrap();
            self.battlefield_data.battlefields.get(battlefield_id)
        };
        battlefield.map(|bf| {
            (
                bf.width(),
                bf.height(),
                bf.grid.clone(),
                bf.deployable_positions.clone(),
            )
        })
    }

    fn draw_battlefield_area(
        &mut self,
        ui: &mut egui::Ui,
        width: usize,
        height: usize,
        grid: &Vec<Vec<Cell>>,
        deployable_positions: &std::collections::BTreeSet<Pos>,
    ) {
        let battlefield_id = self.selected_battlefield.clone().unwrap();
        let cell_size = 40.0;

        // 顯示網格和交互區域
        let (rect, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());

        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return;
        }

        // 處理相機縮放和平移
        self.camera.handle_pan_zoom(ui);

        self.handle_cell_operation(&battlefield_id, ui, rect, width, height, cell_size);

        let painter = ui.painter();

        // 收集所有格子要繪製的文字
        let mut cell_texts: Vec<(egui::Pos2, String)> = Vec::new();

        self.draw_cells(
            &battlefield_id,
            &painter,
            rect,
            width,
            height,
            grid,
            deployable_positions,
            cell_size,
            &mut cell_texts,
        );

        self.draw_grid_lines(&painter, rect, width, height, cell_size);

        self.highlight_hovered_cell(&painter, ui, rect, width, height, cell_size);

        // 呼叫獨立函式繪製所有單位種類文字
        Self::draw_cell_texts(&painter, &cell_texts);
    }

    /// 統一繪製所有單位種類文字（加描邊/陰影）
    fn draw_cell_texts(painter: &egui::Painter, cell_texts: &[(egui::Pos2, String)]) {
        for (pos, text) in cell_texts {
            let font = egui::FontId::proportional(14.0);
            // 黑色陰影/描邊（四個方向）
            let outline_offsets = [
                egui::vec2(-1.0, 0.0),
                egui::vec2(1.0, 0.0),
                egui::vec2(0.0, -1.0),
                egui::vec2(0.0, 1.0),
            ];
            for offset in &outline_offsets {
                painter.text(
                    *pos + *offset,
                    egui::Align2::CENTER_CENTER,
                    text,
                    font.clone(),
                    Color32::BLACK,
                );
            }
            // 主文字（白色）
            painter.text(
                *pos,
                egui::Align2::CENTER_CENTER,
                text,
                font,
                Color32::WHITE,
            );
        }
    }

    fn handle_cell_operation(
        &mut self,
        battlefield_id: &str,
        ui: &mut egui::Ui,
        rect: Rect,
        width: usize,
        height: usize,
        cell_size: f32,
    ) {
        // 處理網格交互
        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
            if rect.contains(pointer_pos) {
                // 將鼠標位置轉換為網格座標
                // 先將螢幕座標轉為世界座標
                let world_pos = self.camera.screen_to_world(pointer_pos);
                // 再計算網格索引
                let x = ((world_pos.x - rect.min.x) / cell_size) as usize;
                let y = ((world_pos.y - rect.min.y) / cell_size) as usize;

                if x >= width || y >= height {
                    return;
                }

                let pos = Pos { x, y };

                // 處理鼠標點擊或拖曳
                let input = ui.ctx().input(|i| i.clone());
                if input.pointer.primary_clicked() || input.pointer.primary_down() {
                    // Simulation 模式下單位選取與移動
                    if self.edit_mode == EditMode::Simulation && input.pointer.primary_clicked() {
                        self.handle_simulation_unit_interaction(pos);
                    } else {
                        self.set_cell(battlefield_id, pos);
                    }
                } else {
                    self.last_painted_pos = None;
                }
            }
        }
    }

    fn draw_cells(
        &self,
        battlefield_id: &str,
        painter: &egui::Painter,
        rect: Rect,
        width: usize,
        height: usize,
        grid: &Vec<Vec<Cell>>,
        deployable_positions: &std::collections::BTreeSet<Pos>,
        cell_size: f32,
        cell_texts: &mut Vec<(egui::Pos2, String)>,
    ) {
        // 繪製網格
        for y in 0..height {
            for x in 0..width {
                let cell = &grid[y][x];
                let pos = Pos { x, y };

                let cell_rect = Rect::from_min_size(
                    self.camera.world_to_screen(egui::pos2(
                        rect.min.x + x as f32 * cell_size,
                        rect.min.y + y as f32 * cell_size,
                    )),
                    egui::vec2(cell_size, cell_size) * self.camera.zoom,
                );

                // 繪製地形
                painter.rect_filled(cell_rect, 0.0, cell.terrain.color());

                // 繪製物件
                if let Some(object) = &cell.object {
                    match object {
                        BattlefieldObject::Wall => {
                            painter.rect_filled(
                                cell_rect.shrink(cell_size * OBJECT_CELL_SIZE),
                                0.0,
                                Color32::DARK_GRAY,
                            );
                        }
                        BattlefieldObject::Tent1 { durability } => {
                            painter.rect_filled(
                                cell_rect.shrink(cell_size * OBJECT_CELL_SIZE),
                                0.0,
                                Color32::from_rgb(200, 180, 120),
                            );
                            cell_texts
                                .push((cell_rect.center(), format!("Tent1\n({})", durability)));
                        }
                        BattlefieldObject::Tent9 {
                            durability,
                            rel_pos,
                        } => {
                            painter.rect_filled(
                                cell_rect.shrink(cell_size * OBJECT_CELL_SIZE),
                                0.0,
                                Color32::from_rgb(180, 140, 80),
                            );
                            cell_texts.push((
                                cell_rect.center(),
                                format!("Tent9\n({},{},{})", rel_pos.x, rel_pos.y, durability),
                            ));
                        }
                    }
                }

                // 繪製部署區域
                if deployable_positions.contains(&pos) {
                    let inner_rect = cell_rect.shrink(cell_size * DEPLOYMENT_CELL_SIZE);
                    painter.rect_filled(
                        inner_rect,
                        2.0,
                        Color32::from_rgba_premultiplied(0, 255, 0, 60),
                    );
                }

                // 繪製單位
                if let Some(unit_id) = &cell.unit_id {
                    let (team_color, unit_type) = self
                        .battlefield_data
                        .battlefields
                        .get(battlefield_id)
                        .map(|bf| {
                            return bf
                                .unit_id_to_unit
                                .get(unit_id)
                                .map(|unit| {
                                    bf.teams
                                        .get(&unit.team_id)
                                        .map(|team| {
                                            (
                                                Color32::from_rgb(
                                                    team.color.r,
                                                    team.color.g,
                                                    team.color.b,
                                                ),
                                                unit.unit_type.clone(),
                                            )
                                        })
                                        .unwrap_or((Color32::GRAY, "?".to_string()))
                                })
                                .unwrap_or((Color32::GRAY, "?".to_string()));
                        })
                        .unwrap_or((Color32::GRAY, "?".to_string()));

                    painter.circle_filled(
                        cell_rect.center(),
                        cell_size * UNIT_CELL_SIZE,
                        team_color,
                    );

                    // 收集單位種類文字資訊，稍後統一繪製
                    cell_texts.push((cell_rect.center(), unit_type));
                }
            }
        }
    }

    fn draw_grid_lines(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        width: usize,
        height: usize,
        cell_size: f32,
    ) {
        let grid_color = Color32::from_gray(100);

        for y in 0..=height {
            let y_pos = rect.min.y + y as f32 * cell_size;
            let start = self.camera.world_to_screen(egui::pos2(rect.min.x, y_pos));
            let end = self
                .camera
                .world_to_screen(egui::pos2(rect.min.x + width as f32 * cell_size, y_pos));
            painter.line_segment([start, end], Stroke::new(1.0, grid_color));
        }

        for x in 0..=width {
            let x_pos = rect.min.x + x as f32 * cell_size;
            let start = self.camera.world_to_screen(egui::pos2(x_pos, rect.min.y));
            let end = self
                .camera
                .world_to_screen(egui::pos2(x_pos, rect.min.y + height as f32 * cell_size));
            painter.line_segment([start, end], Stroke::new(1.0, grid_color));
        }
    }

    fn highlight_hovered_cell(
        &self,
        painter: &egui::Painter,
        ui: &egui::Ui,
        rect: Rect,
        width: usize,
        height: usize,
        cell_size: f32,
    ) {
        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
            if rect.contains(pointer_pos) {
                // 先將螢幕座標轉為世界座標
                let world_pos = self.camera.screen_to_world(pointer_pos);
                // 再計算網格索引
                let grid_x = ((world_pos.x - rect.min.x) / cell_size) as usize;
                let grid_y = ((world_pos.y - rect.min.y) / cell_size) as usize;

                if grid_x >= width || grid_y >= height {
                    return;
                }

                let cell_rect = Rect::from_min_size(
                    self.camera.world_to_screen(egui::pos2(
                        rect.min.x + grid_x as f32 * cell_size,
                        rect.min.y + grid_y as f32 * cell_size,
                    )),
                    egui::vec2(cell_size, cell_size) * self.camera.zoom,
                );

                painter.rect_filled(
                    cell_rect,
                    0.0,
                    Color32::from_rgba_premultiplied(255, 255, 0, 40),
                );
            }
        }
    }

    /// 顯示確認對話框
    fn show_confirmation_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_confirmation_dialog {
            return;
        }

        let mut open = self.show_confirmation_dialog;
        let title = "確認";
        let message = match &self.confirmation_action {
            ConfirmationAction::None => "確定要執行此操作嗎？",
            ConfirmationAction::DeleteBattlefield(_) => "確定要刪除此戰場嗎？",
            ConfirmationAction::ResizeBattlefield { .. } => {
                "確定要調整戰場大小嗎？這可能會導致部分數據丟失。"
            }
        };

        let mut confirm_clicked = false;
        let mut cancel_clicked = false;
        let action_clone = self.confirmation_action.clone();

        egui::Window::new(title)
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(message);
                ui.horizontal(|ui| {
                    confirm_clicked = ui.button("確定").clicked();
                    cancel_clicked = ui.button("取消").clicked();
                });
            });

        // 在閉包外處理按鈕事件
        if confirm_clicked {
            match action_clone {
                ConfirmationAction::DeleteBattlefield(battlefield_id) => {
                    if let Err(err) = self.battlefield_data.delete_battlefield(&battlefield_id) {
                        self.set_status(err, true);
                    } else {
                        self.has_unsaved_changes_flag = true;
                        self.set_status("成功刪除戰場".to_string(), false);
                        self.selected_battlefield = None;
                    }
                }
                ConfirmationAction::ResizeBattlefield { width, height } => {
                    if let Some(battlefield_id) = &self.selected_battlefield {
                        if let Some(battlefield) =
                            self.battlefield_data.battlefields.get_mut(battlefield_id)
                        {
                            battlefield.resize(width, height);
                            self.has_unsaved_changes_flag = true;
                            self.set_status(
                                format!("已調整戰場大小為 {}x{}", width, height),
                                false,
                            );
                        }
                    }
                }
                _ => {}
            }
            open = false;
        }

        if cancel_clicked {
            open = false;
        }

        self.show_confirmation_dialog = open;
    }

    /// 顯示調整戰場大小對話框
    fn show_resize_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_resize_dialog {
            return;
        }

        let mut open = self.show_resize_dialog;
        let mut resize_clicked = false;
        let mut cancel_clicked = false;

        egui::Window::new("調整戰場大小")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("寬度:");
                    ui.add(egui::DragValue::new(&mut self.resize_width).range(5..=50));
                });

                ui.horizontal(|ui| {
                    ui.label("高度:");
                    ui.add(egui::DragValue::new(&mut self.resize_height).range(5..=50));
                });

                ui.horizontal(|ui| {
                    resize_clicked = ui.button("調整").clicked();
                    cancel_clicked = ui.button("取消").clicked();
                });
            });

        // 在閉包外處理按鈕事件
        if resize_clicked {
            self.confirmation_action = ConfirmationAction::ResizeBattlefield {
                width: self.resize_width,
                height: self.resize_height,
            };
            self.show_confirmation_dialog = true;
            open = false;
        }

        if cancel_clicked {
            open = false;
        }

        self.show_resize_dialog = open;
    }

    fn set_cell(&mut self, battlefield_id: &str, pos: Pos) {
        let edit_mode = self.edit_mode;
        let selected_terrain = self.selected_terrain;
        let selected_object = self.selected_object;
        let new_unit_type = self.new_unit_type.clone();
        let durability = self.durability;

        let battlefield = self
            .battlefield_data
            .battlefields
            .get_mut(battlefield_id)
            .unwrap();
        match edit_mode {
            EditMode::Terrain => {
                if self.last_painted_pos != Some(pos) {
                    if battlefield.set_terrain(pos, selected_terrain) {
                        self.has_unsaved_changes_flag = true;
                    }
                    self.last_painted_pos = Some(pos);
                }
            }
            EditMode::Object => {
                if self.last_painted_pos != Some(pos) {
                    match selected_object {
                        Some(BattlefieldObject::Wall) => {
                            if battlefield.grid[pos.y][pos.x].object.is_none() {
                                if battlefield.set_object(pos, selected_object) {
                                    self.has_unsaved_changes_flag = true;
                                }
                            }
                        }
                        Some(BattlefieldObject::Tent1 { .. }) => {
                            if battlefield.grid[pos.y][pos.x].object.is_none() {
                                if battlefield
                                    .set_object(pos, Some(BattlefieldObject::Tent1 { durability }))
                                {
                                    self.has_unsaved_changes_flag = true;
                                }
                            }
                        }
                        Some(BattlefieldObject::Tent9 { .. }) => {
                            // 先檢查 3x3 區塊
                            match Self::check_tent9(battlefield, pos) {
                                Ok(()) => {
                                    Self::place_tent9(battlefield, pos, durability);
                                    self.has_unsaved_changes_flag = true;
                                }
                                Err(msg) => {
                                    self.set_status(msg, true);
                                }
                            }
                        }
                        None => {
                            if battlefield.set_object(pos, None) {
                                self.has_unsaved_changes_flag = true;
                            }
                        }
                    }
                    self.last_painted_pos = Some(pos);
                }
            }
            EditMode::Deployment => {
                if self.last_painted_pos != Some(pos) {
                    let is_deployable = battlefield.is_deployable(pos);
                    if battlefield.set_deployable(pos, !is_deployable) {
                        self.has_unsaved_changes_flag = true;
                    }
                    self.last_painted_pos = Some(pos);
                }
            }
            EditMode::Unit => {
                if self.last_painted_pos != Some(pos) {
                    let unit = if !new_unit_type.is_empty() {
                        Some(Unit {
                            id: Self::generate_unique_unit_id(battlefield, &new_unit_type),
                            unit_type: new_unit_type.clone(),
                            team_id: self.selected_team_id.clone(),
                        })
                    } else {
                        None
                    };

                    if battlefield.set_unit(pos, unit) {
                        self.has_unsaved_changes_flag = true;
                    }
                    self.last_painted_pos = Some(pos);
                }
            }
            EditMode::Objective | EditMode::Simulation => {
                // 不處理
            }
        }
    }

    /// 檢查 Tent9 是否可放置於指定位置
    fn check_tent9(battlefield: &Battlefield, pos: Pos) -> Result<(), String> {
        for dy in 0..3 {
            for dx in 0..3 {
                let px = pos.x as isize + dx - 1;
                let py = pos.y as isize + dy - 1;
                if px < 0 || py < 0 {
                    return Err("帳篷超出邊界".to_string());
                }
                let p = Pos {
                    x: px as usize,
                    y: py as usize,
                };
                if !battlefield.is_valid_position(p) {
                    return Err("帳篷超出戰場範圍".to_string());
                }
                if battlefield.grid[p.y][p.x].object.is_some() {
                    return Err("帳篷區域已有其他物件".to_string());
                }
            }
        }
        Ok(())
    }

    /// 實際放置 Tent9（假設已通過檢查）
    fn place_tent9(battlefield: &mut Battlefield, pos: Pos, durability: i32) {
        for dy in 0..3 {
            for dx in 0..3 {
                let px = pos.x + dx - 1;
                let py = pos.y + dy - 1;
                let p = Pos { x: px, y: py };
                battlefield.set_object(
                    p,
                    Some(BattlefieldObject::Tent9 {
                        durability,
                        rel_pos: Pos { x: dx, y: dy },
                    }),
                );
            }
        }
    }

    /// 單位選取與移動的互動邏輯（呼叫 battle.rs 通用邏輯）
    fn handle_simulation_unit_interaction(&mut self, pos: Pos) {
        let move_range = 3;
        if let (Some(battle), Some(battlefield)) = (
            self.simulation_battle.as_mut(),
            self.simulation_battlefield.as_mut(),
        ) {
            let result = battle.click_battlefield(
                battlefield,
                self.simulation_selected_unit,
                pos,
                move_range,
            );
            // 根據訊息內容決定是否要清除 simulation_selected_unit
            match result {
                Ok(chess_lib::ValidResult::PickMinion) => {
                    self.set_status("選取手下".to_string(), false);
                    self.simulation_selected_unit = Some(pos);
                }
                Ok(chess_lib::ValidResult::FirstMovement) => {
                    // 第一階段移動成功，保持選取
                    self.set_status("一階段移動".to_string(), false);
                }
                Ok(chess_lib::ValidResult::SecondMovement) => {
                    // 第二階段移動成功，清除選取
                    self.set_status("二階段移動".to_string(), false);
                }
                Err(err) => {
                    self.set_status(err.to_string(), true);
                    self.simulation_selected_unit = None;
                }
            }
        }
    }
}

impl eframe::App for ChessEditor {
    fn update(&mut self, ctx: &egui::Context, _: &mut Frame) {
        // 首先處理之前記錄的團隊操作
        self.process_team_action();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                show_file_menu(ui, self);
            });
        });

        egui::SidePanel::left("battlefield_list_panel")
            .default_width(200.0)
            .show(ctx, |ui| {
                self.show_battlefield_list(ui);
            });

        egui::SidePanel::right("tools_panel")
            .default_width(200.0)
            .show(ctx, |ui| {
                self.show_edit_mode_selector(ui);

                ui.add_space(20.0);
                ui.separator();

                self.show_team_management(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_battlefield_editor(ui);
        });

        self.show_confirmation_dialog(ctx);
        self.show_resize_dialog(ctx);
        self.show_status_message(ctx);
    }
}

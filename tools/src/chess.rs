use crate::common::{FileOperator, from_file, show_file_menu, show_status_message, to_file};
use chess_lib::{
    BattleObjectiveType, Battlefield, BattlefieldObject, Cell, Pos, Team, Terrain, Unit,
};
use eframe::{Frame, egui};
use egui::{Button, Color32, Rect, Stroke, Vec2};
use rand::{Rng, distributions::Alphanumeric};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

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
        }
    }
}

// ===== 擴展 Battlefield 功能 =====

trait BattlefieldEditorExt {
    fn set_deployable(&mut self, pos: &Pos, deployable: bool) -> bool;
    fn resize(&mut self, new_width: usize, new_height: usize);
    fn add_team(&mut self, team_id: &str) -> bool;
    fn remove_team(&mut self, team_id: &str) -> bool;
}

impl BattlefieldEditorExt for Battlefield {
    fn set_deployable(&mut self, pos: &Pos, deployable: bool) -> bool {
        if !self.is_valid_position(pos) {
            return false;
        }

        if deployable {
            self.deployable_positions.insert(*pos);
        } else {
            self.deployable_positions.remove(pos);
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
}

impl EditMode {
    fn name(&self) -> &'static str {
        match self {
            EditMode::Terrain => "地形",
            EditMode::Object => "物件",
            EditMode::Deployment => "部署區域",
            EditMode::Unit => "單位",
            EditMode::Objective => "目標",
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
    new_battlefield_id: String,
    new_battlefield_width: usize,
    new_battlefield_height: usize,

    // 編輯工具
    edit_mode: EditMode,
    selected_terrain: Terrain,
    selected_object: Option<BattlefieldObject>,
    new_unit_type: String,

    // 隊伍管理
    new_team_id: String,
    team_action: TeamAction,

    // 單位編輯 - 新增選擇隊伍
    selected_team_id: String,

    // 目標編輯
    selected_objective: Option<String>,

    // 顯示設置
    cell_size: f32,

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
            new_battlefield_id: String::new(),
            new_battlefield_width: 10,
            new_battlefield_height: 10,

            edit_mode: EditMode::Terrain,
            selected_terrain: Terrain::Plain,
            selected_object: None,
            new_unit_type: String::new(),

            new_team_id: String::new(),
            team_action: TeamAction::None,

            // 預設隊伍為 "player"
            selected_team_id: "player".to_string(),

            selected_objective: None,

            cell_size: 40.0,

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
            if !battlefield.unit_id_to_team.contains_key(&id) {
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
            let mut battlefield_ids: Vec<_> = self.battlefield_data.battlefields.keys().collect();
            battlefield_ids.sort();

            // 顯示排序後的戰場列表
            for battlefield_id in battlefield_ids {
                let selected = self.selected_battlefield.as_ref() == Some(battlefield_id);

                let button = Button::new(battlefield_id)
                    .fill(if selected {
                        ui.style().visuals.selection.bg_fill
                    } else {
                        ui.style().visuals.widgets.noninteractive.bg_fill
                    })
                    .min_size(egui::vec2(ui.available_width(), 0.0));

                if ui.add(button).clicked() {
                    // 點擊就直接切換戰場
                    self.selected_battlefield = Some(battlefield_id.clone());
                }
            }
        });

        ui.separator();

        // 操作按鈕
        if self.selected_battlefield.is_some() {
            ui.horizontal(|ui| {
                if ui.button("刪除戰場").clicked() {
                    let battlefield_id = self.selected_battlefield.clone().unwrap();
                    self.confirmation_action =
                        ConfirmationAction::DeleteBattlefield(battlefield_id);
                    self.show_confirmation_dialog = true;
                }

                if ui.button("調整大小").clicked() {
                    if let Some(battlefield_id) = &self.selected_battlefield {
                        if let Some(battlefield) =
                            self.battlefield_data.battlefields.get(battlefield_id)
                        {
                            self.resize_width = battlefield.width();
                            self.resize_height = battlefield.height();
                            self.show_resize_dialog = true;
                        }
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
        let new_team_id = self.new_team_id.clone();

        // 2. 在UI中顯示這些信息和控制項
        if let (Some(battlefield_id), Some(teams)) = (battlefield_id_opt, teams_opt) {
            // 顯示新增隊伍界面
            ui.horizontal(|ui| {
                ui.label("新增隊伍 ID:");
                let mut team_id = new_team_id.clone();
                ui.text_edit_singleline(&mut team_id);
                self.new_team_id = team_id;

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
                    ui.label(&team.id);

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

                    if team.id != "player" && ui.button("刪除").clicked() {
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
            ui.heading("戰場編輯器");
            ui.label("選擇或創建一個戰場開始編輯");
            return;
        }

        let battlefield_id = self.selected_battlefield.clone().unwrap();
        let edit_mode = self.edit_mode;
        let selected_terrain = self.selected_terrain;
        let selected_object = self.selected_object;
        let new_unit_type = self.new_unit_type.clone();
        let cell_size = self.cell_size;

        // 處理滑鼠滾輪事件調整格子大小
        let scroll_delta = ui.ctx().input(|i| i.raw_scroll_delta.y);
        if scroll_delta.abs() > 0.0 {
            // 根據滾輪滾動的方向調整格子大小
            let new_size = (self.cell_size - scroll_delta * 0.1).clamp(20.0, 60.0);
            if (new_size - self.cell_size).abs() > 0.1 {
                self.cell_size = new_size;
            }
        }

        // 首先獲取戰場信息
        let battlefield_option =
            self.battlefield_data
                .battlefields
                .get(&battlefield_id)
                .map(|bf| {
                    (
                        bf.width(),
                        bf.height(),
                        bf.grid.clone(),
                        bf.deployable_positions.clone(),
                    )
                });

        if let Some((width, height, grid, deployable_positions)) = battlefield_option {
            ui.heading(format!("編輯戰場: {}", battlefield_id));

            // 顯示當前格子大小
            ui.label(format!("格子大小: {:.1} (滑鼠滾輪可調整)", cell_size));

            // 顯示網格和交互區域
            let (rect, _) = ui.allocate_exact_size(
                Vec2::new(width as f32 * cell_size, height as f32 * cell_size),
                egui::Sense::click_and_drag(),
            );

            if rect.width() > 0.0 && rect.height() > 0.0 {
                let painter = ui.painter();

                // 處理網格交互
                if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                    if rect.contains(pointer_pos) {
                        // 將鼠標位置轉換為網格坐標
                        let grid_x = ((pointer_pos.x - rect.min.x) / cell_size) as usize;
                        let grid_y = ((pointer_pos.y - rect.min.y) / cell_size) as usize;

                        if grid_x < width && grid_y < height {
                            let pos = Pos {
                                x: grid_x,
                                y: grid_y,
                            };

                            // 處理鼠標點擊
                            if ui.ctx().input(|i| i.pointer.primary_clicked()) {
                                let battlefield = self
                                    .battlefield_data
                                    .battlefields
                                    .get_mut(&battlefield_id)
                                    .unwrap();
                                match edit_mode {
                                    EditMode::Terrain => {
                                        if battlefield.set_terrain(&pos, selected_terrain) {
                                            self.has_unsaved_changes_flag = true;
                                        }
                                    }
                                    EditMode::Object => {
                                        if battlefield.set_object(&pos, selected_object) {
                                            self.has_unsaved_changes_flag = true;
                                        }
                                    }
                                    EditMode::Deployment => {
                                        let is_deployable = battlefield.is_deployable(&pos);
                                        if battlefield.set_deployable(&pos, !is_deployable) {
                                            self.has_unsaved_changes_flag = true;
                                        }
                                    }
                                    EditMode::Unit => {
                                        let unit = if !new_unit_type.is_empty() {
                                            Some(Unit {
                                                id: Self::generate_unique_unit_id(
                                                    battlefield,
                                                    &new_unit_type,
                                                ),
                                                unit_type: new_unit_type.clone(),
                                                team_id: self.selected_team_id.clone(),
                                            })
                                        } else {
                                            None
                                        };

                                        if battlefield.set_unit_and_team(&pos, unit) {
                                            self.has_unsaved_changes_flag = true;
                                        }
                                    }
                                    EditMode::Objective => {
                                        // 目標編輯暫時不實現，可以後續擴展
                                    }
                                }
                            }
                        }
                    }
                }

                // 繪製網格
                for y in 0..height {
                    for x in 0..width {
                        let cell = &grid[y][x];
                        let pos = Pos { x, y };

                        let cell_rect = Rect::from_min_size(
                            egui::pos2(
                                rect.min.x + x as f32 * cell_size,
                                rect.min.y + y as f32 * cell_size,
                            ),
                            egui::vec2(cell_size, cell_size),
                        );

                        // 繪製地形
                        painter.rect_filled(cell_rect, 0.0, cell.terrain.color());

                        // 繪製物件
                        if let Some(object) = &cell.object {
                            match object {
                                BattlefieldObject::Wall => {
                                    painter.rect_filled(
                                        cell_rect.shrink(cell_size * 0.2),
                                        0.0,
                                        Color32::DARK_GRAY,
                                    );
                                }
                            }
                        }

                        // 繪製部署區域標記
                        if deployable_positions.contains(&pos) {
                            // 用填充矩形的方式繪製部署區域標記
                            let inner_rect = cell_rect.shrink(cell_size * 0.3);
                            painter.rect_filled(
                                inner_rect,
                                2.0,
                                Color32::from_rgba_premultiplied(0, 255, 0, 60),
                            );
                        }

                        // 繪製單位
                        if let Some(unit_id) = &cell.unit_id {
                            // 根據 unit_id 查 team_id，再查 team.color
                            let (team_color, unit_type) = self
                                .battlefield_data
                                .battlefields
                                .get(&battlefield_id)
                                .map(|bf| {
                                    let team_color = bf
                                        .get_unit_team(unit_id)
                                        .and_then(|team_id| {
                                            bf.teams.get(team_id).map(|t| {
                                                Color32::from_rgb(t.color.r, t.color.g, t.color.b)
                                            })
                                        })
                                        .unwrap_or(Color32::GRAY);

                                    let unit_type = bf
                                        .unit_id_to_team
                                        .get(unit_id)
                                        .map(|unit| unit.unit_type.clone())
                                        .unwrap_or_else(|| "?".to_string());

                                    (team_color, unit_type)
                                })
                                .unwrap_or((Color32::GRAY, "?".to_string()));

                            painter.circle_filled(cell_rect.center(), cell_size * 0.3, team_color);

                            // 顯示單位種類
                            painter.text(
                                cell_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                unit_type,
                                egui::FontId::proportional(14.0),
                                Color32::WHITE,
                            );
                        }
                    }
                }

                // 繪製水平和垂直線，使用矩形填充而不是線段來避免問題
                let grid_color = Color32::from_gray(100);

                // 繪製水平線 (用細長矩形代替)
                for y in 0..=height {
                    let y_pos = rect.min.y + y as f32 * cell_size;
                    painter.rect_filled(
                        Rect::from_min_max(
                            egui::pos2(rect.min.x, y_pos),
                            egui::pos2(rect.min.x + width as f32 * cell_size, y_pos + 1.0),
                        ),
                        0.0,
                        grid_color,
                    );
                }

                // 繪製垂直線 (用細長矩形代替)
                for x in 0..=width {
                    let x_pos = rect.min.x + x as f32 * cell_size;
                    painter.rect_filled(
                        Rect::from_min_max(
                            egui::pos2(x_pos, rect.min.y),
                            egui::pos2(x_pos + 1.0, rect.min.y + height as f32 * cell_size),
                        ),
                        0.0,
                        grid_color,
                    );
                }

                // 將指針位置轉換為網格坐標
                if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                    if rect.contains(pointer_pos) {
                        let grid_x = ((pointer_pos.x - rect.min.x) / cell_size) as usize;
                        let grid_y = ((pointer_pos.y - rect.min.y) / cell_size) as usize;

                        if grid_x < width && grid_y < height {
                            // 高亮顯示當前單元格
                            let cell_rect = Rect::from_min_size(
                                egui::pos2(
                                    rect.min.x + grid_x as f32 * cell_size,
                                    rect.min.y + grid_y as f32 * cell_size,
                                ),
                                egui::vec2(cell_size, cell_size),
                            );

                            // 簡單畫一個半透明矩形表示選中
                            painter.rect_filled(
                                cell_rect,
                                0.0,
                                Color32::from_rgba_premultiplied(255, 255, 0, 40),
                            );
                        }
                    }
                }
            }
        } else {
            ui.heading("戰場編輯器");
            ui.label("選中的戰場不存在");
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

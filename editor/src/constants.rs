pub(crate) const APP_TITLE: &str = "編輯器";
pub(crate) const FONT_FILE_PATH: &str = "fonts/NotoSans.ttf";
pub(crate) const FONT_NAME: &str = "NotoSans";

// 字體大小
pub(crate) const FONT_SIZE_HEADING: f32 = 40.0;
pub(crate) const FONT_SIZE_BODY: f32 = 32.0;
pub(crate) const FONT_SIZE_MONOSPACE: f32 = 32.0;
pub(crate) const FONT_SIZE_BUTTON: f32 = 24.0;
pub(crate) const FONT_SIZE_SMALL: f32 = 24.0;

// UI 間距
pub(crate) const SPACING_SMALL: f32 = 5.0;
pub(crate) const SPACING_MEDIUM: f32 = 10.0;

// UI 尺寸
pub(crate) const LIST_PANEL_WIDTH: f32 = 300.0;
pub(crate) const LIST_PANEL_MIN_HEIGHT: f32 = 300.0;
pub(crate) const STROKE_WIDTH: f32 = 3.0;

pub(crate) const TURN_PANEL_WIDTH: f32 = 200.0;
pub(crate) const BOTTOM_PANEL_HEIGHT: f32 = 70.0;
pub(crate) const BOTTOM_PANEL_BUTTON_WIDTH: f32 = 130.0;

// UI 數值
pub(crate) const DRAG_VALUE_SPEED: f64 = 1.0;

// 檔案相關
pub(crate) const DATA_DIRECTORY_PATH: &str = "ignore-data/";
pub(crate) const FILE_EXTENSION_TOML: &str = ".toml";

// 編輯器相關
pub(crate) const COPY_SUFFIX: &str = "-copy";

// 關卡編輯器 - 清除選項
pub(crate) const CLEAR_LABEL: &str = "── 清除 ──";

// 關卡編輯器 - 戰場預覽
pub(crate) const BATTLEFIELD_CELL_SIZE: f32 = 36.0;
pub(crate) const BATTLEFIELD_GRID_SPACING: f32 = 2.0;
pub(crate) const BATTLEFIELD_TEXT_SIZE: f32 = 14.0;

// 關卡編輯器 - 戰場預覽 - 顏色
pub(crate) const BATTLEFIELD_COLOR_DEPLOYMENT: egui::Color32 = egui::Color32::LIGHT_GREEN;
pub(crate) const BATTLEFIELD_COLOR_UNIT: egui::Color32 = egui::Color32::DARK_GRAY;
pub(crate) const BATTLEFIELD_COLOR_OBJECT: egui::Color32 = egui::Color32::GRAY;
pub(crate) const BATTLEFIELD_COLOR_EMPTY: egui::Color32 = egui::Color32::DARK_GREEN;
pub(crate) const BATTLEFIELD_COLOR_HIGHLIGHT: egui::Color32 = egui::Color32::YELLOW;
// 關卡編輯器 - 戰場預覽 - 技能相關顏色
pub(crate) const BATTLEFIELD_COLOR_SKILL_RED: egui::Color32 =
    egui::Color32::from_rgb(255, 100, 100);
pub(crate) const BATTLEFIELD_COLOR_SKILL_PICKED: egui::Color32 =
    egui::Color32::from_rgb(255, 160, 40);
// 關卡編輯器 - 戰鬥 - 技能目標數
pub(crate) const SINGLE_TARGET_COUNT: usize = 1;
// 關卡編輯器 - 戰場預覽 - 移動相關顏色
pub(crate) const BATTLEFIELD_COLOR_MOVE_1MOV: egui::Color32 =
    egui::Color32::from_rgb(100, 150, 255);
pub(crate) const BATTLEFIELD_COLOR_MOVE_2MOV: egui::Color32 =
    egui::Color32::from_rgb(100, 100, 200);
pub(crate) const BATTLEFIELD_COLOR_MOVE_PATH: egui::Color32 =
    egui::Color32::from_rgb(255, 200, 100);

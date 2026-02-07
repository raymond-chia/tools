pub const APP_TITLE: &str = "編輯器";
pub const FONT_FILE_PATH: &str = "fonts/NotoSans.ttf";
pub const FONT_NAME: &str = "NotoSans";

// 字體大小
pub const FONT_SIZE_HEADING: f32 = 40.0;
pub const FONT_SIZE_BODY: f32 = 32.0;
pub const FONT_SIZE_MONOSPACE: f32 = 32.0;
pub const FONT_SIZE_BUTTON: f32 = 24.0;
pub const FONT_SIZE_SMALL: f32 = 24.0;

// UI 間距
pub const SPACING_SMALL: f32 = 5.0;
pub const SPACING_MEDIUM: f32 = 10.0;

// UI 尺寸
pub const LIST_PANEL_WIDTH: f32 = 300.0;

// UI 數值
pub const DRAG_VALUE_SPEED: f64 = 1.0;

// 檔案相關
pub const DATA_DIRECTORY_PATH: &str = "ignore-data/";
pub const FILE_EXTENSION_TOML: &str = ".toml";

// 編輯器相關
pub const COPY_SUFFIX: &str = "-copy";

// 技能編輯器 - 機制類型
pub const MECHANIC_TYPE_HITBASED: &str = "HitBased";
pub const MECHANIC_TYPE_DCBASED: &str = "DcBased";
pub const MECHANIC_TYPE_GUARANTEED: &str = "Guaranteed";

// 技能編輯器 - 目標模式
pub const TARGET_MODE_SINGLETARGET: &str = "SingleTarget";
pub const TARGET_MODE_MULTITARGET: &str = "MultiTarget";
pub const TARGET_MODE_AREA: &str = "Area";

// 技能編輯器 - AOE 形狀
pub const AOE_SHAPE_DIAMOND: &str = "Diamond";
pub const AOE_SHAPE_CROSS: &str = "Cross";
pub const AOE_SHAPE_LINE: &str = "Line";
pub const AOE_SHAPE_RECTANGLE: &str = "Rectangle";

// 關卡編輯器 - 戰場預覽
pub const BATTLEFIELD_CELL_SIZE: f32 = 36.0;
pub const BATTLEFIELD_GRID_SPACING: f32 = 2.0;
pub const BATTLEFIELD_TEXT_SIZE: f32 = 14.0;

// 關卡編輯器 - 戰場預覽 - 顏色
pub const BATTLEFIELD_COLOR_PLAYER: egui::Color32 = egui::Color32::LIGHT_BLUE;
pub const BATTLEFIELD_COLOR_ENEMY: egui::Color32 = egui::Color32::LIGHT_RED;
pub const BATTLEFIELD_COLOR_OBJECT: egui::Color32 = egui::Color32::GRAY;
pub const BATTLEFIELD_COLOR_EMPTY: egui::Color32 = egui::Color32::DARK_GREEN;
pub const BATTLEFIELD_COLOR_DRAG_HIGHLIGHT: egui::Color32 = egui::Color32::YELLOW;
pub const BATTLEFIELD_DRAG_STROKE_WIDTH: f32 = 3.0;

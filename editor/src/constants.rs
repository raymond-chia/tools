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
pub const LIST_PANEL_MIN_HEIGHT: f32 = 300.0;
pub const STROKE_WIDTH: f32 = 3.0;

// UI 數值
pub const DRAG_VALUE_SPEED: f64 = 1.0;

// 檔案相關
pub const DATA_DIRECTORY_PATH: &str = "ignore-data/";
pub const FILE_EXTENSION_TOML: &str = ".toml";

// 編輯器相關
pub const COPY_SUFFIX: &str = "-copy";

// 關卡編輯器 - ComboBox 尺寸
pub const COMBOBOX_MIN_WIDTH: f32 = 250.0;
pub const COMBOBOX_MIN_HEIGHT: f32 = 10000.0;

// 關卡編輯器 - 戰場預覽
pub const BATTLEFIELD_CELL_SIZE: f32 = 36.0;
pub const BATTLEFIELD_GRID_SPACING: f32 = 2.0;
pub const BATTLEFIELD_TEXT_SIZE: f32 = 14.0;

// 關卡編輯器 - 戰場預覽 - 顏色
pub const BATTLEFIELD_COLOR_DEPLOYMENT: egui::Color32 = egui::Color32::LIGHT_GREEN;
pub const BATTLEFIELD_COLOR_UNIT: egui::Color32 = egui::Color32::DARK_GRAY;
pub const BATTLEFIELD_COLOR_OBJECT: egui::Color32 = egui::Color32::GRAY;
pub const BATTLEFIELD_COLOR_EMPTY: egui::Color32 = egui::Color32::DARK_GREEN;
pub const BATTLEFIELD_COLOR_HIGHLIGHT: egui::Color32 = egui::Color32::YELLOW;

use egui::{FontData, FontDefinitions, FontFamily};
use noise::{NoiseFn, Perlin};

/// 地形分級（依據高度）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerrainType {
    DeepWater,    // 深水
    ShallowWater, // 淺水
    WadingZone,   // 可涉水通過
    Plain,        // 平原
    Hill,         // 丘陵
    Mountain,     // 山地
    HighMountain, // 高山
}

/// 柯本氣候分類
#[derive(Debug)]
pub enum KoppenClimate {
    Af,  // 熱帶雨林
    Aw,  // 熱帶乾濕季氣候
    BWh, // 熱帶沙漠氣候
    BWk, // 溫帶沙漠氣候
    Cw,  // 溫帶海洋性季風氣候
    Df,  // 暖溫帶濕潤氣候
    ET,  // 苔原氣候
    EF,  // 極地冰原氣候
}

/// 人類攜帶氧氣筒最深抵達 332 米
const LOWEST_HUMAN_REACHABLE: i32 = -300;
const LOWEST_WADING_ZONE: i32 = -1;
const SEA_LEVEL: i32 = 0;
const LOWEST_HILL: i32 = 300;
const LOWEST_MOUNTAIN: i32 = 600;
const HIGHEST_HUMAN_REACHABLE: i32 = 4000;
const HIGHEST_MOUNTAIN: i32 = 8000;

/// 圖顯示縮放倍率（畫素放大用）
const MAP_POINT_SIZE: f32 = 3.0;

/// 多層 Perlin 噪聲高度產生器
/// 使用三層獨立 Perlin 噪聲（低、中、高頻）搭配權重，產生更自然的海島地形
pub struct HeightGenerator {
    /// 低頻層（大尺度地形：海洋盆地、大陸）
    perlin_low: Perlin,
    /// 中頻層（中尺度地形：島嶼、山脈）
    perlin_mid: Perlin,
    /// 高頻層（小尺度細節：山峰紋理）
    perlin_high: Perlin,

    /// 低頻層的縮放係數，數值越大地形越平坦
    low_scale: f64,
    /// 中頻層的縮放係數
    mid_scale: f64,
    /// 高頻層的縮放係數
    high_scale: f64,

    /// 低頻層的權重（0-100，使用 u16 表示）
    low_weight: u16,
    /// 中頻層的權重（0-100）
    mid_weight: u16,
    /// 高頻層的權重（0-100）
    high_weight: u16,
}

impl HeightGenerator {
    /// 建立新的高度產生器
    ///
    /// # 參數
    /// - `seed`: 隨機種子（用於初始化所有 Perlin 層）
    /// - `low_scale`: 低頻層縮放（推薦 150-250）
    /// - `mid_scale`: 中頻層縮放（推薦 40-80）
    /// - `high_scale`: 高頻層縮放（推薦 8-15）
    /// - `low_weight`: 低頻權重（0-100，推薦 50-70）
    /// - `mid_weight`: 中頻權重（0-100，推薦 20-40）
    /// - `high_weight`: 高頻權重（0-100，推薦 5-15）
    pub fn new(
        seed: u32,
        low_scale: f64,
        mid_scale: f64,
        high_scale: f64,
        low_weight: u16,
        mid_weight: u16,
        high_weight: u16,
    ) -> Self {
        // 使用不同種子初始化各層，避免三層完全相同
        let perlin_low = Perlin::new(seed);
        let perlin_mid = Perlin::new(seed.wrapping_add(1));
        let perlin_high = Perlin::new(seed.wrapping_add(2));

        Self {
            perlin_low,
            perlin_mid,
            perlin_high,
            low_scale,
            mid_scale,
            high_scale,
            low_weight,
            mid_weight,
            high_weight,
        }
    }

    /// 取得 (x, y) 座標的高度值，範圍為 0.0 ~ 1.0
    ///
    /// 使用加權疊加三層噪聲：
    /// 高度 = 低頻 × low_weight + 中頻 × mid_weight + 高頻 × high_weight
    /// 計算指定座標的高度值，允許對低頻層（大尺度地形）套用遮罩。
    ///
    /// - `x`, `y`：地圖座標。
    /// - `low_mask`：低頻層遮罩（0.0~1.0，通常依距離中心遞減，僅影響大尺度地形分布）。
    ///
    /// 僅低頻層乘以遮罩，確保島嶼集中於中央但不強制圓形或中心最高。
    /// 中頻與高頻層維持原噪聲，保留細節與隨機性。
    /// 回傳值範圍為 0.0~1.0。
    pub fn get_height(&self, x: f64, y: f64, low_mask: f64) -> f64 {
        let low = ((self
            .perlin_low
            .get([x / self.low_scale, y / self.low_scale])
            + 1.0)
            * 0.5)
            .clamp(0.0, 1.0)
            * low_mask;
        let mid = ((self
            .perlin_mid
            .get([x / self.mid_scale, y / self.mid_scale])
            + 1.0)
            * 0.5)
            .clamp(0.0, 1.0);
        let high = ((self
            .perlin_high
            .get([x / self.high_scale, y / self.high_scale])
            + 1.0)
            * 0.5)
            .clamp(0.0, 1.0);

        let total_weight = (self.low_weight + self.mid_weight + self.high_weight) as f64;
        let total_weight = total_weight.max(1.0);
        let combined = (low * self.low_weight as f64
            + mid * self.mid_weight as f64
            + high * self.high_weight as f64)
            / total_weight;

        combined.clamp(0.0, 1.0)
    }
}

/// egui App：高度地圖視覺化
#[derive(PartialEq)]
pub enum HeightMapTab {
    Noise,
    LandWater,
    Terrain,
}

pub struct HeightMapApp {
    /// 當前顯示的 tab
    tab: HeightMapTab,

    /// 隨機種子
    seed: u32,
    /// 最低高度
    min_height: i32,
    /// 最高高度
    max_height: i32,

    /// 低頻層縮放
    low_scale: f64,
    /// 中頻層縮放
    mid_scale: f64,
    /// 高頻層縮放
    high_scale: f64,
    /// 低頻權重（0-100）
    low_weight: u16,
    /// 中頻權重（0-100）
    mid_weight: u16,
    /// 高頻權重（0-100）
    high_weight: u16,
    /// 是否啟用中央遮罩（讓島嶼集中於中央）
    is_center_mask_enabled: bool,

    /// 地圖寬度
    width: usize,
    /// 地圖高度
    height: usize,

    /// 原始 Perlin 噪聲高度（0~1）
    noise_heights: Vec<f64>,
    /// 實際高度
    terrain_heights: Vec<f64>,

    /// 使用者選取的點 (x, y)
    selected: Option<(usize, usize)>,
}

impl Default for HeightMapApp {
    fn default() -> Self {
        let width = 375;
        let height = 240;

        let mut app = Self {
            tab: HeightMapTab::Noise,
            seed: 0,
            min_height: -2000,
            max_height: HIGHEST_MOUNTAIN,
            low_scale: 80.0,
            mid_scale: 50.0,
            high_scale: 10.0,
            low_weight: 80,
            mid_weight: 15,
            high_weight: 5,
            is_center_mask_enabled: true,
            width,
            height,
            noise_heights: vec![0.0; width * height],
            terrain_heights: vec![0.0; width * height],
            selected: None,
        };
        app.regenerate();
        app
    }
}

impl HeightMapApp {
    fn get(&self, vec: &Vec<f64>, x: usize, y: usize) -> Option<f64> {
        if x < self.width && y < self.height {
            Some(vec[y * self.width + x])
        } else {
            None
        }
    }

    /// 計算中央遮罩值（可模組化擴充）
    fn center_mask(&self, x: f64, y: f64) -> f64 {
        if !self.is_center_mask_enabled {
            return 1.0;
        }
        let cx = self.width as f64 / 2.0;
        let cy = self.height as f64 / 2.0;
        let r = cx.min(cy) * 0.8;
        let p = 2.0; // 遮罩指數，2.0 為平滑過渡
        let dx = x - cx;
        let dy = y - cy;
        let d = (dx * dx + dy * dy).sqrt();
        (1.0 - (d / r).powf(p)).clamp(0.0, 1.0)
    }

    /// 重新產生高度圖資料
    fn regenerate(&mut self) {
        let generator = HeightGenerator::new(
            self.seed,
            self.low_scale,
            self.mid_scale,
            self.high_scale,
            self.low_weight,
            self.mid_weight,
            self.high_weight,
        );

        // 產生 noise heights（0~1）
        self.noise_heights = (0..self.width * self.height)
            .map(|i| {
                let x = (i % self.width) as f64;
                let y = (i / self.width) as f64;
                let mask = self.center_mask(x, y);
                generator.get_height(x, y, mask)
            })
            .collect();

        // 產生實際高度 vec（根據 min_height/max_height）
        self.terrain_heights = self
            .noise_heights
            .iter()
            .map(|&h| self.to_real_height(h))
            .collect();
    }

    fn is_land(&self, noise: f64) -> bool {
        self.to_real_height(noise) >= SEA_LEVEL as f64
    }

    /// 將 0.0~1.0 的高度轉換為真實高度
    fn to_real_height(&self, h: f64) -> f64 {
        h * (self.max_height - self.min_height) as f64 + self.min_height as f64
    }
}

impl eframe::App for HeightMapApp {
    /// 更新 UI，每一幀都會呼叫
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("left_panel").show(ctx, |ui| {
            ui.heading("參數調整");
            egui::ScrollArea::vertical().show(ui, |ui| {
                let regen = self.ui_parameter_controls(ui);
                if regen {
                    self.noise_heights = vec![0.0; self.width * self.height];
                    self.terrain_heights = vec![0.0; self.width * self.height];
                    self.regenerate();
                    self.selected = None;
                }
            });
        });

        // 右側顯示選取點資訊
        egui::SidePanel::right("right_panel").show(ctx, |ui| {
            self.ui_selected_info(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, HeightMapTab::Noise, "噪音");
                ui.selectable_value(&mut self.tab, HeightMapTab::LandWater, "海陸");
                ui.selectable_value(&mut self.tab, HeightMapTab::Terrain, "地形");
            });
            match self.tab {
                HeightMapTab::Noise => {
                    // 顯示高度圖並取得互動回應
                    let response = self.ui_heightmap_display(ui);
                    // 處理點擊選取
                    self.handle_selection(&response);
                }
                HeightMapTab::LandWater => {
                    let response = self.ui_landwater_display(ui);
                    self.handle_selection(&response);
                }
                HeightMapTab::Terrain => {
                    let response = self.ui_terrain_display(ui);
                    self.handle_selection(&response);
                }
            }
        });
    }
}

impl HeightMapApp {
    /// 參數調整 UI，回傳是否有變動需重生地圖
    fn ui_parameter_controls(&mut self, ui: &mut egui::Ui) -> bool {
        let mut regen = false;

        ui.label("Width:");
        regen |= ui
            .add(egui::Slider::new(&mut self.width, 16..=512))
            .changed();
        ui.label("Height:");
        regen |= ui
            .add(egui::Slider::new(&mut self.height, 16..=256))
            .changed();

        ui.separator();

        ui.label("Seed:");
        regen |= ui.add(egui::DragValue::new(&mut self.seed)).changed();

        ui.separator();

        ui.label("最低高度:");
        regen |= ui
            .add(egui::DragValue::new(&mut self.min_height).speed(10))
            .changed();
        ui.label("最高高度:");
        regen |= ui
            .add(egui::DragValue::new(&mut self.max_height).speed(10))
            .changed();

        ui.separator();

        ui.label("低頻層\n(大尺度地形):");
        ui.label("Scale:");
        regen |= ui
            .add(egui::Slider::new(&mut self.low_scale, 80.0..=340.0).step_by(20.0))
            .changed();
        ui.label("Weight:");
        regen |= ui
            .add(egui::Slider::new(&mut self.low_weight, 0..=100).step_by(5.0))
            .changed();
        regen |= ui
            .checkbox(&mut self.is_center_mask_enabled, "集中於中央")
            .changed();

        ui.label("中頻層\n(島嶼):");
        ui.label("Scale:");
        regen |= ui
            .add(egui::Slider::new(&mut self.mid_scale, 30.0..=130.0).step_by(10.0))
            .changed();
        ui.label("Weight:");
        regen |= ui
            .add(egui::Slider::new(&mut self.mid_weight, 0..=100).step_by(5.0))
            .changed();

        ui.label("高頻層\n(細節):");
        ui.label("Scale:");
        regen |= ui
            .add(egui::Slider::new(&mut self.high_scale, 10.0..=50.0).step_by(5.0))
            .changed();
        ui.label("Weight:");
        regen |= ui
            .add(egui::Slider::new(&mut self.high_weight, 0..=100).step_by(5.0))
            .changed();

        regen
    }

    fn ui_display(
        &self,
        ui: &mut egui::Ui,
        map_name: &str,
        image: egui::ColorImage,
    ) -> egui::Response {
        let texture = ui
            .ctx()
            .load_texture(map_name, image, egui::TextureOptions::NEAREST);

        let img_size = [
            self.width as f32 * MAP_POINT_SIZE,
            self.height as f32 * MAP_POINT_SIZE,
        ];
        ui.add(
            egui::Image::new(&texture)
                .fit_to_exact_size(img_size.into())
                // 支援點擊選取
                .sense(egui::Sense::click()),
        )
    }

    fn ui_heightmap_display(&self, ui: &mut egui::Ui) -> egui::Response {
        let image = egui::ColorImage::from_rgb(
            [self.width, self.height],
            &self
                .noise_heights
                .iter()
                .flat_map(|&h| {
                    let v = (h * 255.0).clamp(0.0, 255.0) as u8;
                    vec![v, v, v]
                })
                .collect::<Vec<_>>(),
        );
        self.ui_display(ui, "noisemap", image)
    }

    fn ui_landwater_display(&self, ui: &mut egui::Ui) -> egui::Response {
        let image = egui::ColorImage::from_rgb(
            [self.width, self.height],
            &self
                .noise_heights
                .iter()
                .flat_map(|&h| {
                    if self.is_land(h) {
                        TerrainType::terrain_type_to_color(TerrainType::Mountain)
                    } else {
                        TerrainType::terrain_type_to_color(TerrainType::ShallowWater)
                    }
                })
                .collect::<Vec<_>>(),
        );
        self.ui_display(ui, "landwatermap", image)
    }

    fn ui_terrain_display(&self, ui: &mut egui::Ui) -> egui::Response {
        let image = egui::ColorImage::from_rgb(
            [self.width, self.height],
            &self
                .terrain_heights
                .iter()
                .flat_map(|&h| {
                    let terrain = TerrainType::height_to_terrain_type(h);
                    TerrainType::terrain_type_to_color(terrain)
                })
                .collect::<Vec<_>>(),
        );
        self.ui_display(ui, "terrainmap", image)
    }

    /// 處理高度圖點擊選取，更新 self.selected
    fn handle_selection(&mut self, response: &egui::Response) {
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let px = ((pos.x - response.rect.left()) / MAP_POINT_SIZE).floor() as usize;
                let py = ((pos.y - response.rect.top()) / MAP_POINT_SIZE).floor() as usize;
                if px < self.width && py < self.height {
                    self.selected = Some((px, py));
                }
            }
        }
    }

    /// 顯示目前選取點的資訊
    fn ui_selected_info(&self, ui: &mut egui::Ui) {
        if let Some((x, y)) = self.selected {
            let noise = self
                .get(&self.noise_heights, x, y)
                .expect("程式碼邏輯問題: 非法座標");
            ui.label(format!("選取座標: ({}, {})，數值: {:.3}", x, y, noise));
        } else {
            ui.label("尚未選取任何點");
        }
    }
}

impl TerrainType {
    /// 將實際高度對應到地形分級
    fn height_to_terrain_type(height: f64) -> Self {
        if height < LOWEST_HUMAN_REACHABLE as f64 {
            Self::DeepWater
        } else if height < LOWEST_WADING_ZONE as f64 {
            Self::ShallowWater
        } else if height < SEA_LEVEL as f64 {
            Self::WadingZone
        } else if height < LOWEST_HILL as f64 {
            Self::Plain
        } else if height < LOWEST_MOUNTAIN as f64 {
            Self::Hill
        } else if height < HIGHEST_HUMAN_REACHABLE as f64 {
            Self::Mountain
        } else {
            Self::HighMountain
        }
    }

    /// 地形分級對應顏色（RGB）
    fn terrain_type_to_color(terrain: Self) -> [u8; 3] {
        match terrain {
            Self::DeepWater => [0x00, 0x00, 0x99],    // #009
            Self::ShallowWater => [0x00, 0x00, 0xFF], // #00F
            Self::WadingZone => [0x00, 0xFF, 0xFF],   // #0FF
            Self::Plain => [0x00, 0xFF, 0x00],        // #0F0
            Self::Hill => [0xAA, 0xAA, 0x00],         // #AA0
            Self::Mountain => [0xAA, 0x66, 0x00],     // #A60
            Self::HighMountain => [0xFF, 0xFF, 0xFF], // #FFF
        }
    }
}

/// 主程式進入點，啟動 egui 視窗
fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native(
        "高度地圖產生器",
        options,
        Box::new(|cc| {
            // 參考 editor/src/main.rs，設定中文字型
            let mut fonts = FontDefinitions::default();
            match std::fs::read("../../fonts/NotoSans.ttf") {
                Ok(font_data) => {
                    fonts.font_data.insert(
                        "noto_sans".to_owned(),
                        FontData::from_owned(font_data).into(),
                    );
                    fonts
                        .families
                        .get_mut(&FontFamily::Proportional)
                        .unwrap()
                        .insert(0, "noto_sans".to_owned());
                }
                Err(err) => {
                    println!("無法載入中文字體: {}", err);
                }
            }
            cc.egui_ctx.set_fonts(fonts);

            // 設定初始字型大小和樣式
            let mut style = (*cc.egui_ctx.style()).clone();
            style.text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(24.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Heading,
                egui::FontId::new(32.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Button,
                egui::FontId::new(24.0, egui::FontFamily::Proportional),
            );
            cc.egui_ctx.set_style(style);

            Ok(Box::new(HeightMapApp::default()))
        }),
    )
}

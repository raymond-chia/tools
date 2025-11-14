use egui::{FontData, FontDefinitions, FontFamily};
use noise::{NoiseFn, Simplex};

/** 地形分級（依高度） */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerrainType {
    DeepWater,
    ShallowWater,
    WadingZone, // 可涉水通過
    Plain,
    Hill,
    Mountain,
    HighMountain,
}

/** 柯本氣候分類 */
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

/** 地圖顯示縮放倍率 */
const MAP_POINT_SIZE: f32 = 2.0;

/// 多層噪聲高度產生器
/// 使用三層獨立噪聲（低、中、高頻）搭配權重，產生更自然的海島地形
pub struct HeightGenerator {
    /// 低頻層（大尺度地形：海洋盆地、大陸）
    noise_low: Simplex,
    /// 中頻層（中尺度地形：島嶼、山脈）
    noise_mid: Simplex,
    /// 高頻層（小尺度細節：山峰紋理）
    noise_high: Simplex,

    /// 數值越大地形越平坦
    low_scale: f64,
    mid_scale: f64,
    high_scale: f64,

    /// 權重（0-100）
    low_weight: u16,
    mid_weight: u16,
    high_weight: u16,
}

impl HeightGenerator {
    /// 建立新的高度產生器
    pub fn new(
        seed: u32,
        low_scale: f64,
        mid_scale: f64,
        high_scale: f64,
        low_weight: u16,
        mid_weight: u16,
        high_weight: u16,
    ) -> Self {
        // 各層使用不同種子
        let noise_low = Simplex::new(seed);
        let noise_mid = Simplex::new(seed.wrapping_add(1));
        let noise_high = Simplex::new(seed.wrapping_add(2));

        Self {
            noise_low,
            noise_mid,
            noise_high,
            low_scale,
            mid_scale,
            high_scale,
            low_weight,
            mid_weight,
            high_weight,
        }
    }

    /// 取得 (x, y) 座標的高度值（0.0~1.0），僅低頻層可套用遮罩。
    /// low_masks: 遮罩函數陣列，依序作用於低頻層。
    pub fn get_height(&self, x: f64, y: f64, low_masks: &[&dyn Fn(f64) -> f64]) -> f64 {
        let mut low = ((self.noise_low.get([x / self.low_scale, y / self.low_scale]) + 1.0) * 0.5)
            .clamp(0.0, 1.0);
        for mask_fn in low_masks {
            low = mask_fn(low);
        }
        let mid = ((self.noise_mid.get([x / self.mid_scale, y / self.mid_scale]) + 1.0) * 0.5)
            .clamp(0.0, 1.0);
        let high = ((self
            .noise_high
            .get([x / self.high_scale, y / self.high_scale])
            + 1.0)
            * 0.5)
            .clamp(0.0, 1.0);

        let total_weight = (self.low_weight + self.mid_weight + self.high_weight) as f64;
        let total_weight = total_weight.max(1.0); // 避免除以零
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
    /// 地表高度上限
    surface_height_limit: i32,

    /// 地圖寬度
    width: usize,
    /// 地圖高度
    height: usize,

    /// 原始噪聲高度（0~1）
    noise_heights: Vec<f64>,
    /// 實際高度
    real_heights: Vec<i32>,

    /// 使用者選取的點 (x, y)
    selected: Option<(usize, usize)>,
}

impl Default for HeightMapApp {
    fn default() -> Self {
        let width = 560;
        let height = 360;

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
            surface_height_limit: HIGHEST_MOUNTAIN,
            width,
            height,
            noise_heights: vec![0.0; width * height],
            real_heights: vec![0; width * height],
            selected: None,
        };
        app.regenerate();
        app
    }
}

impl HeightMapApp {
    fn get<T: Copy>(&self, vec: &Vec<T>, x: usize, y: usize) -> Option<T> {
        if x < self.width && y < self.height {
            Some(vec[y * self.width + x])
        } else {
            None
        }
    }

    /// 產生中心遮罩函數（僅處理 noise，依是否啟用自動判斷）
    fn center_mask_fn(&self, x: f64, y: f64) -> impl Fn(f64) -> f64 {
        let enabled = self.is_center_mask_enabled;
        let cx = self.width as f64 / 2.0;
        let cy = self.height as f64 / 2.0;
        let r = cx.min(cy) * 0.8;
        let p = 2.0;
        let dx = x - cx;
        let dy = y - cy;
        let d = (dx * dx + dy * dy).sqrt();
        let mask = (1.0 - (d / r).powf(p)).clamp(0.0, 1.0);
        move |noise| {
            if !enabled {
                return noise;
            }
            noise * mask
        }
    }

    /// 重新產生高度圖
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
                let center_mask = self.center_mask_fn(x, y);
                let masks: Vec<&dyn Fn(f64) -> f64> = vec![&center_mask];
                generator.get_height(x, y, &masks)
            })
            .collect();

        // 產生實際高度 vec（根據 min_height/max_height）
        self.real_heights = self
            .noise_heights
            .iter()
            .map(|&h| self.to_real_height(h).min(self.surface_height_limit))
            .collect();
    }

    fn to_real_height(&self, noise: f64) -> i32 {
        let h = noise * (self.max_height - self.min_height) as f64;
        let h = h as i32;
        h + self.min_height
    }

    fn is_land(&self, h: i32) -> bool {
        h >= SEA_LEVEL
    }
}

impl eframe::App for HeightMapApp {
    /// 每幀更新 UI
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("left_panel").show(ctx, |ui| {
            ui.heading("參數調整");
            egui::ScrollArea::vertical().show(ui, |ui| {
                let regen = self.ui_parameter_controls(ui);
                if regen {
                    self.noise_heights = vec![0.0; self.width * self.height];
                    self.real_heights = vec![0; self.width * self.height];
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
        let mut changed = false;
        ui.label("Width:");
        changed |= ui
            .add(egui::Slider::new(&mut self.width, 16..=560))
            .changed();
        ui.label("Height:");
        changed |= ui
            .add(egui::Slider::new(&mut self.height, 16..=360))
            .changed();

        ui.separator();

        ui.label("Seed:");
        ui.horizontal(|ui| {
            if ui.button("-").clicked() {
                // 防止 seed 溢位
                if self.seed > 0 {
                    self.seed -= 1;
                    changed = true;
                }
            }
            changed |= ui.add(egui::DragValue::new(&mut self.seed)).changed();
            if ui.button("+").clicked() {
                // 防止溢位到 u32::MAX
                if self.seed < u32::MAX {
                    self.seed += 1;
                    changed = true;
                }
            }
        });

        egui::CollapsingHeader::new("參數調整")
            .default_open(true)
            .show(ui, |ui| {
                ui.label("最低高度:");
                changed |= ui
                    .add(egui::DragValue::new(&mut self.min_height).speed(10))
                    .changed();
                ui.label("最高高度:");
                changed |= ui
                    .add(egui::DragValue::new(&mut self.max_height).speed(10))
                    .changed();

                ui.separator();

                ui.label("低頻層\n(大尺度地形):");
                ui.label("Scale:");
                changed |= ui
                    .add(egui::Slider::new(&mut self.low_scale, 80.0..=340.0).step_by(20.0))
                    .changed();
                ui.label("Weight:");
                changed |= ui
                    .add(egui::Slider::new(&mut self.low_weight, 0..=100).step_by(5.0))
                    .changed();
                changed |= ui
                    .checkbox(&mut self.is_center_mask_enabled, "集中於中央")
                    .changed();

                ui.label("中頻層\n(島嶼):");
                ui.label("Scale:");
                changed |= ui
                    .add(egui::Slider::new(&mut self.mid_scale, 30.0..=130.0).step_by(10.0))
                    .changed();
                ui.label("Weight:");
                changed |= ui
                    .add(egui::Slider::new(&mut self.mid_weight, 0..=100).step_by(5.0))
                    .changed();

                ui.label("高頻層\n(細節):");
                ui.label("Scale:");
                changed |= ui
                    .add(egui::Slider::new(&mut self.high_scale, 10.0..=50.0).step_by(5.0))
                    .changed();
                ui.label("Weight:");
                changed |= ui
                    .add(egui::Slider::new(&mut self.high_weight, 0..=100).step_by(5.0))
                    .changed();

                ui.separator();

                ui.label("地表高度上限:");
                changed |= ui
                    .add(
                        egui::Slider::new(
                            &mut self.surface_height_limit,
                            self.min_height..=self.max_height,
                        )
                        .step_by(1.0),
                    )
                    .changed();
            });
        changed
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
                .real_heights
                .iter()
                .flat_map(|&h| {
                    if self.is_land(h) {
                        TerrainType::terrain_type_to_color(TerrainType::Plain)
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
                .real_heights
                .iter()
                .flat_map(|&h| {
                    let terrain = TerrainType::height_to_terrain_type(h);
                    TerrainType::terrain_type_to_color(terrain)
                })
                .collect::<Vec<_>>(),
        );
        self.ui_display(ui, "terrainmap", image)
    }

    /// 處理地圖點擊選取
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

    /// 顯示選取點資訊
    fn ui_selected_info(&self, ui: &mut egui::Ui) {
        if let Some((x, y)) = self.selected {
            let noise = self
                .get(&self.noise_heights, x, y)
                .expect("程式碼邏輯問題: 非法座標");
            let height = self
                .get(&self.real_heights, x, y)
                .expect("程式碼邏輯問題: 非法座標");
            ui.label(format!("選取座標: ({}, {})", x, y));
            ui.label(format!("雜訊值: {:.3}", noise));
            ui.label(format!("實際高度: {:.1} 米", height));
        } else {
            ui.label("尚未選取任何點");
        }
    }
}

impl TerrainType {
    /// 實際高度對應地形分級
    fn height_to_terrain_type(height: i32) -> Self {
        if height < LOWEST_HUMAN_REACHABLE {
            Self::DeepWater
        } else if height < LOWEST_WADING_ZONE {
            Self::ShallowWater
        } else if height < SEA_LEVEL {
            Self::WadingZone
        } else if height < LOWEST_HILL {
            Self::Plain
        } else if height < LOWEST_MOUNTAIN {
            Self::Hill
        } else if height < HIGHEST_HUMAN_REACHABLE {
            Self::Mountain
        } else {
            Self::HighMountain
        }
    }

    /// 地形分級對應顏色
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

/** 主程式進入點 */
fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native(
        "高度地圖產生器",
        options,
        Box::new(|cc| {
            // 設定中文字型
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

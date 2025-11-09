use egui::{FontData, FontDefinitions, FontFamily};
use noise::{Fbm, NoiseFn, Perlin};
use rand::Rng;
use serde::{Deserialize, Serialize};

/// 柯本氣候分類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KoppenClimate {
    Af,  // 熱帶雨林
    Am,  // 熱帶季風
    Aw,  // 熱帶草原
    BWh, // 熱帶沙漠
    BWk, // 溫帶沙漠
    BSh, // 熱帶半乾旱
    BSk, // 溫帶半乾旱
    Csa, // 地中海式溫暖夏
    Csb, // 地中海式溫和夏
    Cwa, // 副熱帶季風溫暖夏
    Cwb, // 副熱帶季風溫和夏
    Cfa, // 副熱帶濕潤溫暖夏
    Cfb, // 海洋性溫和夏
    Dfa, // 大陸性溫暖夏
    Dfb, // 大陸性溫和夏
    Dfc, // 大陸性涼夏
    Dfd, // 大陸性極端涼夏
    Dwa, // 大陸性季風溫暖夏
    Dwb, // 大陸性季風溫和夏
    Dwc, // 大陸性季風涼夏
    Dwd, // 大陸性季風極端涼夏
    ET,  // 凍原
    EF,  // 冰川
}

/// 地圖格子
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tile {
    pub height: f64,        // 高度，0-1
    pub temperature: f64,   // 溫度，攝氏度
    pub precipitation: f64, // 降雨量，mm/年
    pub climate: KoppenClimate,
    pub is_ocean: bool, // 是否為海洋
}

/// 地圖
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Map {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<Tile>,
    pub min_lat: f64, // 最小緯度
    pub max_lat: f64, // 最大緯度
}

const OCEAN_HEIGHT: f64 = 0.35;
const MOUNTAIN_HEIGHT: f64 = 0.7;

impl Map {
    pub fn new(width: usize, height: usize, min_lat: f64, max_lat: f64) -> Self {
        Self {
            width,
            height,
            tiles: vec![
                Tile {
                    height: 0.0,
                    temperature: 0.0,
                    precipitation: 0.0,
                    climate: KoppenClimate::Af,
                    is_ocean: false,
                };
                width * height
            ],
            min_lat,
            max_lat,
        }
    }

    pub fn get(&self, x: usize, y: usize) -> &Tile {
        &self.tiles[y * self.width + x]
    }

    pub fn get_mut(&mut self, x: usize, y: usize) -> &mut Tile {
        &mut self.tiles[y * self.width + x]
    }

    /// 產生高度
    pub fn generate_height(&mut self, seed: u32) {
        let fbm = Fbm::<Perlin>::new(seed);
        for y in 0..self.height {
            for x in 0..self.width {
                let nx = x as f64 / self.width as f64 * 4.0;
                let ny = y as f64 / self.height as f64 * 4.0;
                let value = fbm.get([nx, ny]);
                let height = (value + 1.0) / 2.0; // 0-1
                let tile = self.get_mut(x, y);
                tile.height = height;
                tile.is_ocean = height < OCEAN_HEIGHT;
            }
        }
    }

    /// 計算緯度
    /// 計算緯度：y=0 為地圖上方（最大緯度），y=height-1 為地圖下方（最小緯度）
    fn latitude(&self, y: usize) -> f64 {
        let lat_range = self.max_lat - self.min_lat;
        self.max_lat - (y as f64 / (self.height - 1) as f64) * lat_range
    }

    /// 計算溫度
    pub fn generate_temperature(&mut self) {
        for y in 0..self.height {
            let lat = self.latitude(y);
            // 簡單線性緯度溫度模型：
            // 30.0 表示赤道基準溫度（攝氏 30 度）
            // 50.0 表示從赤道到極地（90 度）溫度遞減的幅度（總共降 50 度）
            // 當 lat = 0（赤道）=> base_temp = 30.0
            // 當 lat = ±90（極地）=> base_temp = -20.0
            let base_temp = 30.0 - (lat.abs() / 90.0) * 50.0;
            for x in 0..self.width {
                let height_effect = -self.get(x, y).height * 20.0; // 高度降低溫度
                let temp = base_temp + height_effect;
                self.get_mut(x, y).temperature = temp;
            }
        }
    }

    /// 計算降雨
    pub fn generate_precipitation(&mut self) {
        // 簡單實作：距離海洋遠近
        // 假設左邊和右邊是海洋
        for y in 0..self.height {
            for x in 0..self.width {
                let dist_to_left = x as f64;
                let dist_to_right = (self.width - 1 - x) as f64;
                let min_dist = dist_to_left.min(dist_to_right);
                // 山脈阻擋：如果高度 > 0.7，阻擋降雨
                let mut blocked = false;
                if x < self.width / 2 {
                    // 從左邊來
                    for i in 0..x {
                        if self.get(i, y).height > MOUNTAIN_HEIGHT {
                            blocked = true;
                            break;
                        }
                    }
                } else {
                    // 從右邊來
                    for i in (x + 1)..self.width {
                        if self.get(i, y).height > MOUNTAIN_HEIGHT {
                            blocked = true;
                            break;
                        }
                    }
                }
                let precip = if blocked {
                    500.0
                } else {
                    2000.0 - min_dist * 50.0
                };
                self.get_mut(x, y).precipitation = precip.max(0.0);
            }
        }
    }

    /// 分類氣候
    pub fn classify_climate(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                let tile = self.get_mut(x, y);
                let temp = tile.temperature;
                let precip = tile.precipitation;

                // 簡單柯本分類
                if temp < -10.0 {
                    tile.climate = KoppenClimate::EF;
                } else if temp < 0.0 {
                    tile.climate = KoppenClimate::ET;
                } else if temp < 10.0 {
                    if precip < 250.0 {
                        tile.climate = KoppenClimate::BWk;
                    } else {
                        tile.climate = KoppenClimate::Dfc;
                    }
                } else if temp < 20.0 {
                    if precip < 500.0 {
                        tile.climate = KoppenClimate::BSk;
                    } else if precip < 1000.0 {
                        tile.climate = KoppenClimate::Cfb;
                    } else {
                        tile.climate = KoppenClimate::Cfa;
                    }
                } else {
                    if precip < 500.0 {
                        tile.climate = KoppenClimate::BWh;
                    } else if precip < 1500.0 {
                        tile.climate = KoppenClimate::Aw;
                    } else {
                        tile.climate = KoppenClimate::Af;
                    }
                }
            }
        }
    }

    /// 產生完整地圖
    pub fn generate(&mut self, seed: u32) {
        self.generate_height(seed);
        self.generate_temperature();
        self.generate_precipitation();
        self.classify_climate();
    }
}

/// 視圖模式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    Koppen,
    Height,
    Temperature,
    Precipitation,
}

/// 應用
#[derive(Debug)]
pub struct MapGeneratorApp {
    pub map: Map,
    pub seed: u32,
    pub view_mode: ViewMode,
    pub selected_tile: Option<(usize, usize)>,
    pub min_lat_input: String,
    pub max_lat_input: String,
    pub width_input: String,
    pub height_input: String,
}

impl MapGeneratorApp {
    pub fn new() -> Self {
        let width = 100;
        let height = 100;
        let min_lat = -60.0;
        let max_lat = 60.0;
        let mut map = Map::new(width, height, min_lat, max_lat);
        let seed = rand::rng().random::<u32>();
        map.generate(seed);
        Self {
            map,
            seed,
            view_mode: ViewMode::Koppen,
            selected_tile: None,
            min_lat_input: format!("{:.1}", min_lat),
            max_lat_input: format!("{:.1}", max_lat),
            width_input: format!("{}", width),
            height_input: format!("{}", height),
        }
    }

    /// 隨機產生新 seed
    pub fn randomize_seed(&mut self) {
        self.seed = rand::rng().random::<u32>();
        self.map.generate(self.seed);
        self.selected_tile = None;
    }

    /// 僅根據新設定重建地圖（同一 seed 下可切換不同緯度）
    pub fn regenerate_with_current_seed(&mut self) {
        let width_res = self.width_input.parse::<usize>();
        let height_res = self.height_input.parse::<usize>();
        let min_lat_res = self.min_lat_input.parse::<f64>();
        let max_lat_res = self.max_lat_input.parse::<f64>();

        if let (Ok(w), Ok(h), Ok(min_lat), Ok(max_lat)) =
            (width_res, height_res, min_lat_res, max_lat_res)
        {
            if w > 0 && w <= 500 && h > 0 && h <= 500 && min_lat < max_lat {
                self.map = Map::new(w, h, min_lat, max_lat);
                self.map.generate(self.seed);
                self.selected_tile = None;
            }
        }
    }

    /// 更新設定
    pub fn update_settings(&mut self) {
        let mut changed = false;
        let mut new_width = self.map.width;
        let mut new_height = self.map.height;
        let mut new_min_lat = self.map.min_lat;
        let mut new_max_lat = self.map.max_lat;

        // 先全部 parse，再一起比較
        let width_res = self.width_input.parse::<usize>();
        let height_res = self.height_input.parse::<usize>();
        let min_lat_res = self.min_lat_input.parse::<f64>();
        let max_lat_res = self.max_lat_input.parse::<f64>();

        if let (Ok(w), Ok(h), Ok(min_lat), Ok(max_lat)) =
            (width_res, height_res, min_lat_res, max_lat_res)
        {
            if w > 0 && w <= 500 && h > 0 && h <= 500 && min_lat < max_lat {
                new_width = w;
                new_height = h;
                new_min_lat = min_lat;
                new_max_lat = max_lat;
                changed = true;
            }
        }

        if changed {
            self.map = Map::new(new_width, new_height, new_min_lat, new_max_lat);
            self.map.generate(self.seed);
            self.selected_tile = None;
        }
    }

    /// 獲取顏色
    fn get_color(&self, x: usize, y: usize) -> egui::Color32 {
        let tile = self.map.get(x, y);
        if tile.is_ocean {
            return egui::Color32::from_rgb(40, 100, 200); // 海洋藍
        }
        match self.view_mode {
            ViewMode::Koppen => {
                // 柯本氣候分類顏色
                match tile.climate {
                    KoppenClimate::Af => egui::Color32::from_rgb(0, 120, 0), // 熱帶雨林 深綠
                    KoppenClimate::Am => egui::Color32::from_rgb(0, 180, 80), // 熱帶季風 淺綠
                    KoppenClimate::Aw => egui::Color32::from_rgb(200, 220, 60), // 熱帶草原 黃綠
                    KoppenClimate::BWh => egui::Color32::from_rgb(230, 200, 80), // 熱帶沙漠 黃
                    KoppenClimate::BWk => egui::Color32::from_rgb(220, 180, 120), // 溫帶沙漠 淺黃
                    KoppenClimate::BSh => egui::Color32::from_rgb(240, 220, 120), // 熱帶半乾旱
                    KoppenClimate::BSk => egui::Color32::from_rgb(200, 200, 140), // 溫帶半乾旱
                    KoppenClimate::Csa => egui::Color32::from_rgb(255, 160, 80), // 地中海式溫暖夏
                    KoppenClimate::Csb => egui::Color32::from_rgb(255, 200, 120), // 地中海式溫和夏
                    KoppenClimate::Cwa => egui::Color32::from_rgb(120, 200, 255), // 副熱帶季風溫暖夏
                    KoppenClimate::Cwb => egui::Color32::from_rgb(160, 220, 255), // 副熱帶季風溫和夏
                    KoppenClimate::Cfa => egui::Color32::from_rgb(80, 180, 255), // 副熱帶濕潤溫暖夏
                    KoppenClimate::Cfb => egui::Color32::from_rgb(120, 200, 220), // 海洋性溫和夏
                    KoppenClimate::Dfa => egui::Color32::from_rgb(255, 120, 120), // 大陸性溫暖夏
                    KoppenClimate::Dfb => egui::Color32::from_rgb(255, 180, 180), // 大陸性溫和夏
                    KoppenClimate::Dfc => egui::Color32::from_rgb(200, 200, 255), // 大陸性涼夏
                    KoppenClimate::Dfd => egui::Color32::from_rgb(180, 180, 255), // 大陸性極端涼夏
                    KoppenClimate::Dwa => egui::Color32::from_rgb(255, 160, 200), // 大陸性季風溫暖夏
                    KoppenClimate::Dwb => egui::Color32::from_rgb(255, 200, 220), // 大陸性季風溫和夏
                    KoppenClimate::Dwc => egui::Color32::from_rgb(200, 220, 255), // 大陸性季風涼夏
                    KoppenClimate::Dwd => egui::Color32::from_rgb(180, 200, 255), // 大陸性季風極端涼夏
                    KoppenClimate::ET => egui::Color32::from_rgb(180, 255, 255),  // 凍原
                    KoppenClimate::EF => egui::Color32::from_rgb(255, 255, 255),  // 冰川
                }
            }
            ViewMode::Height => {
                let h = tile.height as f32;
                egui::Color32::from_rgb((h * 255.0) as u8, (h * 255.0) as u8, (h * 255.0) as u8)
            }
            ViewMode::Temperature => {
                let t = ((tile.temperature + 50.0) / 100.0).clamp(0.0, 1.0) as f32;
                egui::Color32::from_rgb((t * 255.0) as u8, 0, ((1.0 - t) * 255.0) as u8)
            }
            ViewMode::Precipitation => {
                let p = (tile.precipitation / 2000.0).clamp(0.0, 1.0) as f32;
                egui::Color32::from_rgb(0, (p * 255.0) as u8, ((1.0 - p) * 255.0) as u8)
            }
        }
    }
}

impl eframe::App for MapGeneratorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 右上角浮動按鈕區
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("隨機 Seed").clicked() {
                    self.randomize_seed();
                }
                if ui.button("同 Seed").clicked() {
                    self.regenerate_with_current_seed();
                }
                ui.label(format!("Seed: {}", self.seed));
                ui.add_space(16.0);
                if ui.button("輸出 TOML").clicked() {
                    let toml = toml::to_string(&self.map).unwrap();
                    println!("{}", toml);
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("寬度:");
                ui.text_edit_singleline(&mut self.width_input);
                ui.label("高度:");
                ui.text_edit_singleline(&mut self.height_input);
                ui.label("最小緯度");
                ui.text_edit_singleline(&mut self.min_lat_input);
                ui.label("最大緯度");
                ui.text_edit_singleline(&mut self.max_lat_input);
                if ui.button("更新設定").clicked() {
                    self.update_settings();
                }
            });

            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.view_mode, ViewMode::Koppen, "柯本圖");
                ui.selectable_value(&mut self.view_mode, ViewMode::Height, "高度圖");
                ui.selectable_value(&mut self.view_mode, ViewMode::Temperature, "溫度圖");
                ui.selectable_value(&mut self.view_mode, ViewMode::Precipitation, "降水圖");
            });

            // 主區域分上下：上方地圖，下方格子資訊
            let info_height = 120.0;
            let map_size = egui::Vec2::new(600.0, 600.0 - info_height);
            let (rect, response) = ui.allocate_at_least(map_size, egui::Sense::click());
            let painter = ui.painter();

            let tile_size = map_size.x / self.map.width as f32;

            for y in 0..self.map.height {
                for x in 0..self.map.width {
                    let pos =
                        rect.min + egui::Vec2::new(x as f32 * tile_size, y as f32 * tile_size);
                    let tile_rect = egui::Rect::from_min_size(pos, egui::Vec2::splat(tile_size));
                    painter.rect_filled(tile_rect, 0.0, self.get_color(x, y));
                }
            }

            // 點擊檢測
            if response.clicked() {
                if let Some(pointer) = response.interact_pointer_pos() {
                    let pos = pointer - rect.min;
                    let x = (pos.x / tile_size) as usize;
                    let y = (pos.y / tile_size) as usize;
                    if x < self.map.width && y < self.map.height {
                        self.selected_tile = Some((x, y));
                    }
                }
            }

            // 下方資訊區塊
            ui.add_space(8.0);
            egui::Frame::none()
                .fill(ui.visuals().panel_fill) // 使用與面板相同的背景色
                .show(ui, |ui| {
                    ui.set_min_height(info_height - 8.0);
                    if let Some((x, y)) = self.selected_tile {
                        let tile = self.map.get(x, y);
                        let lat = self.map.latitude(y);
                        ui.horizontal(|ui| {
                            ui.label(format!("位置: ({}, {})", x, y));
                            ui.label(format!("緯度: {:.2}°", lat));
                            ui.label(format!("高度: {:.2}", tile.height));
                            ui.label(format!("溫度: {:.1}°C", tile.temperature));
                            ui.label(format!("降雨: {:.0} mm/年", tile.precipitation));
                            ui.label(format!("氣候: {:?}", tile.climate));
                            if tile.is_ocean {
                                ui.label("地形: 海洋");
                            } else {
                                ui.label("地形: 陸地");
                            }
                        });
                    } else {
                        ui.label("請點擊地圖格子以檢視詳細資訊");
                    }
                });

            // (已移至右上角浮動按鈕區)
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "奇幻戰棋地圖產生器",
        options,
        Box::new(|cc| {
            // 設定字體以支援繁體中文
            let mut fonts = FontDefinitions::default();

            // 嘗試載入專案內的中文字型（如 fonts/NotoSans.ttf）
            match std::fs::read("../../fonts/NotoSans.ttf") {
                Ok(font_data) => {
                    fonts.font_data.insert(
                        "noto_sans".to_owned(),
                        FontData::from_owned(font_data).into(),
                    );

                    // 將中文字體添加到 Proportional 字體族中的首位
                    fonts
                        .families
                        .get_mut(&FontFamily::Proportional)
                        .unwrap()
                        .insert(0, "noto_sans".to_owned());
                }
                Err(err) => {
                    println!("無法載入中文字體: {}", err);
                    // 這裡可以加載備用字體或繼續使用預設字體
                }
            }

            // 設置字體
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

            Ok(Box::new(MapGeneratorApp::new()))
        }),
    )
}

use dialogs_lib::{EventType, Scene};
use eframe::{Frame, egui};
use egui::{Button, RichText, ScrollArea, Separator, Ui};
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Error, ErrorKind};
use std::path::{Path, PathBuf};

/// 劇情資料集
#[derive(Debug, Clone, Deserialize, Serialize)]
struct StoryData {
    #[serde(default)]
    name: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    version: String,
    #[serde(flatten)]
    scenes: HashMap<String, Scene>,
}

impl StoryData {
    /// 從指定路徑載入 TOML 檔案
    fn from_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let content = fs::read_to_string(path)?;
        Self::from_toml_str(&content)
    }

    /// 從多個檔案載入劇情
    #[allow(dead_code)]
    fn load_from_directory<P: AsRef<Path>>(dir_path: P) -> io::Result<Self> {
        let mut story = StoryData::empty();

        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().map_or(false, |ext| ext == "toml") {
                let chapter_story = StoryData::from_file(&path)?;
                // 合併場景數據
                story.scenes.extend(chapter_story.scenes);

                // 若主數據還未設置名稱等屬性，從第一個檔案取得
                if story.name.is_empty() && !chapter_story.name.is_empty() {
                    story.name = chapter_story.name;
                    story.author = chapter_story.author;
                    story.version = chapter_story.version;
                }
            }
        }

        Ok(story)
    }

    /// 從 TOML 字串解析
    fn from_toml_str(content: &str) -> io::Result<Self> {
        toml::from_str(content)
            .map_err(|err| Error::new(ErrorKind::InvalidData, format!("解析 TOML 失敗: {}", err)))
    }

    /// 轉換為 TOML 格式
    fn to_toml(&self) -> io::Result<String> {
        toml::to_string_pretty(self)
            .map_err(|err| Error::new(ErrorKind::InvalidData, format!("序列化 TOML 失敗: {}", err)))
    }

    /// 寫入到檔案
    fn save_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let toml_content = self.to_toml()?;
        fs::write(path, toml_content)
    }

    /// 儲存為多個檔案 (按章節)
    #[allow(dead_code)]
    fn save_to_directory<P: AsRef<Path>>(&self, dir_path: P, prefix: &str) -> io::Result<()> {
        let dir_path = dir_path.as_ref();
        if !dir_path.exists() {
            fs::create_dir_all(dir_path)?;
        }

        // 章節分割策略 (這裡用簡單的前綴策略，實際可能需要更複雜的邏輯)
        // 假設場景ID格式為 "chapter1_scene1", "chapter1_scene2" 等
        let mut chapters: HashMap<String, HashMap<String, Scene>> = HashMap::new();

        for (scene_id, scene) in &self.scenes {
            let parts: Vec<&str> = scene_id.split('_').collect();
            if parts.len() >= 1 {
                let chapter = parts[0].to_string();
                chapters
                    .entry(chapter)
                    .or_insert_with(HashMap::new)
                    .insert(scene_id.clone(), scene.clone());
            } else {
                // 沒有明確章節的場景放入"misc"
                chapters
                    .entry("misc".to_string())
                    .or_insert_with(HashMap::new)
                    .insert(scene_id.clone(), scene.clone());
            }
        }

        // 儲存每個章節
        for (chapter, scenes) in chapters {
            let chapter_story = StoryData {
                name: self.name.clone(),
                author: self.author.clone(),
                version: self.version.clone(),
                scenes,
            };

            let filename = format!("{}_{}.toml", prefix, chapter);
            let file_path = dir_path.join(filename);
            chapter_story.save_to_file(file_path)?;
        }

        Ok(())
    }

    /// 新增場景
    fn create_scene(&mut self, scene_id: String) -> Result<(), String> {
        if self.scenes.contains_key(&scene_id) {
            return Err("場景 ID 已存在".to_string());
        }
        self.scenes.insert(scene_id, Scene::default());
        Ok(())
    }

    /// 更新場景
    fn update_scene(&mut self, scene_id: String, updated_scene: Scene) -> Result<(), String> {
        if let Some(scene) = self.scenes.get_mut(&scene_id) {
            *scene = updated_scene;
            Ok(())
        } else {
            Err(format!("找不到場景 ID: {}", scene_id))
        }
    }

    /// 刪除場景
    fn delete_scene(&mut self, scene_id: &str) -> Result<(), String> {
        if !self.scenes.contains_key(scene_id) {
            return Err("找不到指定的場景".to_string());
        }
        self.scenes.remove(scene_id);
        Ok(())
    }

    /// 檢查劇情完整性
    #[allow(dead_code)]
    fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // 檢查所有場景引用的next_scene是否存在
        for (scene_id, scene) in &self.scenes {
            // 檢查選項
            for option in &scene.options {
                if !self.scenes.contains_key(&option.next_scene) {
                    errors.push(format!(
                        "場景 '{}' 的選項 '{}' 引用了不存在的場景 '{}'",
                        scene_id, option.text, option.next_scene
                    ));
                }
            }

            // 檢查事件中的場景引用
            for event in &scene.events {
                match event {
                    EventType::Choice { next_scene_key, .. } => {
                        if !self.scenes.contains_key(next_scene_key) {
                            errors.push(format!(
                                "場景 '{}' 的選擇事件引用了不存在的場景 '{}'",
                                scene_id, next_scene_key
                            ));
                        }
                    }
                    EventType::Condition { next_scene, .. } => {
                        if !self.scenes.contains_key(next_scene) {
                            errors.push(format!(
                                "場景 '{}' 的條件事件引用了不存在的場景 '{}'",
                                scene_id, next_scene
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// 建立空的劇情資料集
    fn empty() -> Self {
        Self {
            name: String::new(),
            author: String::new(),
            version: String::new(),
            scenes: HashMap::new(),
        }
    }
}

/// 劇情編輯器
pub struct DialogsEditor {
    story_data: StoryData,
    current_file_path: Option<PathBuf>,
    new_scene_id: String,
    temp_scene: Option<(String, Scene)>,
    status_message: Option<(String, bool)>, // message, is_error
    show_confirmation_dialog: bool,
    confirmation_action: ConfirmationAction,
}

#[derive(Debug, Clone)]
enum ConfirmationAction {
    None,
    DeleteScene(String),
    // 其他操作可以在這裡添加
}

impl Default for DialogsEditor {
    fn default() -> Self {
        Self {
            story_data: StoryData::empty(),
            current_file_path: None,
            new_scene_id: String::new(),
            temp_scene: None,
            status_message: None,
            show_confirmation_dialog: false,
            confirmation_action: ConfirmationAction::None,
        }
    }
}

// 為用戶界面元素定義thread_local變量，避免使用static mut
thread_local! {
    static NEW_FLAG: RefCell<String> = RefCell::new(String::new());
    static NEW_VALUE: RefCell<bool> = RefCell::new(true);
    static NEW_PREREQ_FLAG: RefCell<String> = RefCell::new(String::new());
    static NEW_PREREQ_VALUE: RefCell<bool> = RefCell::new(true);
}

impl DialogsEditor {
    pub fn new(_: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    /// 檢查目前編輯中的場景是否有未保存的變動
    pub fn has_unsaved_changes(&self) -> bool {
        false
    }

    fn load_file(&mut self, path: PathBuf) {
        match StoryData::from_file(&path) {
            Ok(data) => {
                self.story_data = data;
                self.current_file_path = Some(path);
                self.temp_scene = None;
                self.set_status(format!("成功載入檔案"), false);
            }
            Err(err) => {
                self.set_status(format!("載入檔案失敗: {}", err), true);
            }
        }
    }

    fn save_file(&mut self, path: &Path) {
        match self.story_data.save_to_file(path) {
            Ok(_) => {
                self.current_file_path = Some(path.to_path_buf());
                self.set_status(format!("成功儲存檔案"), false);
            }
            Err(err) => {
                self.set_status(format!("儲存檔案失敗: {}", err), true);
            }
        }
    }

    fn set_status(&mut self, message: String, is_error: bool) {
        self.status_message = Some((message, is_error));
    }

    fn create_scene(&mut self) {
        if self.new_scene_id.is_empty() {
            self.set_status("場景 ID 不能為空".to_string(), true);
            return;
        }

        match self.story_data.create_scene(self.new_scene_id.clone()) {
            Ok(_) => {
                self.temp_scene = Some((
                    self.new_scene_id.clone(),
                    self.story_data
                        .scenes
                        .get(&self.new_scene_id)
                        .unwrap()
                        .clone(),
                ));
                self.new_scene_id.clear();
                self.set_status(format!("成功建立場景"), false);
            }
            Err(err) => {
                self.set_status(err, true);
            }
        }
    }

    fn show_file_menu(&mut self, ui: &mut Ui) {
        egui::menu::bar(ui, |ui| {
            egui::menu::menu_button(ui, "檔案", |ui| {
                if ui.button("新增").clicked() {
                    self.story_data = StoryData::empty();
                    self.current_file_path = None;
                    self.temp_scene = None;
                    self.set_status("已建立新檔案".to_string(), false);
                    ui.close_menu();
                }

                if ui.button("開啟...").clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("TOML", &["toml"])
                        .set_directory(".")
                        .pick_file()
                    {
                        self.load_file(path);
                    }
                    ui.close_menu();
                }

                if ui.button("儲存").clicked() {
                    let should_open_dialog = self.current_file_path.is_none();
                    if !should_open_dialog {
                        let path = self.current_file_path.as_ref().unwrap().clone();
                        self.save_file(&path);
                    } else {
                        if let Some(path) = FileDialog::new()
                            .add_filter("TOML", &["toml"])
                            .set_directory(".")
                            .save_file()
                        {
                            self.save_file(&path);
                        }
                    }
                    ui.close_menu();
                }

                if ui.button("另存為...").clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("TOML", &["toml"])
                        .set_directory(".")
                        .save_file()
                    {
                        self.save_file(&path);
                    }
                    ui.close_menu();
                }
            });
        });
    }

    fn show_scene_list(&mut self, ui: &mut Ui) {
        ui.heading("場景列表");

        ui.horizontal(|ui| {
            ui.label("新增場景 ID:");
            ui.text_edit_singleline(&mut self.new_scene_id);
            if ui.button("新增").clicked() {
                self.create_scene();
            }
        });

        ui.add_space(10.0);

        ScrollArea::vertical().show(ui, |ui| {
            // 收集所有場景 ID 並按字母順序排序
            let mut scene_ids: Vec<_> = self.story_data.scenes.keys().collect();
            scene_ids.sort(); // 按字母排序

            // 顯示排序後的場景列表
            for scene_id in scene_ids {
                let selected = self.temp_scene.as_ref().map(|(id, _)| id) == Some(scene_id);

                let button = Button::new(scene_id)
                    .fill(if selected {
                        ui.style().visuals.selection.bg_fill
                    } else {
                        ui.style().visuals.widgets.noninteractive.bg_fill
                    })
                    .min_size(egui::vec2(ui.available_width(), 0.0));

                if ui.add(button).clicked() {
                    let scene = self.story_data.scenes.get(scene_id).unwrap().clone();
                    self.temp_scene = Some((scene_id.clone(), scene));
                }
            }
        });
    }

    fn show_scene_editor(&mut self, ui: &mut Ui) {
        // 首先添加標題和按鈕（這些保持在固定位置）
        let mut save_clicked = false;
        let mut delete_clicked = false;

        if let Some((scene_id, scene)) = &mut self.temp_scene {
            ui.heading(format!("編輯場景: {}", scene_id));

            ui.horizontal(|ui| {
                save_clicked = ui.button("儲存變更").clicked();
                delete_clicked = ui.button("刪除場景").clicked();
            });

            ui.add_space(8.0);
            ui.add(Separator::default());

            // 計算 ScrollArea 的最大高度，為底部留出空間
            let available_height = ui.available_height();
            let scroll_height = available_height.max(100.0) - 40.0; // 為底部狀態欄保留空間

            // 添加可捲動區域，設定最大高度
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .max_height(scroll_height)
                .show(ui, |ui| {
                    // 基本場景屬性
                    ui.group(|ui| {
                        ui.heading("基本資訊");

                        // 場景類型
                        ui.horizontal(|ui| {
                            ui.label("場景類型:");
                            egui::ComboBox::from_id_salt("scene_type")
                                .selected_text(format!("{:?}", scene.scene_type))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut scene.scene_type,
                                        dialogs_lib::SceneType::Dialogue,
                                        "對話",
                                    );
                                    ui.selectable_value(
                                        &mut scene.scene_type,
                                        dialogs_lib::SceneType::Choice,
                                        "選項",
                                    );
                                    ui.selectable_value(
                                        &mut scene.scene_type,
                                        dialogs_lib::SceneType::Battle,
                                        "戰鬥",
                                    );
                                    ui.selectable_value(
                                        &mut scene.scene_type,
                                        dialogs_lib::SceneType::Ending,
                                        "結局",
                                    );
                                });
                        });

                        // 標題和描述
                        ui.horizontal(|ui| {
                            ui.label("標題:");
                            ui.text_edit_singleline(&mut scene.title);
                        });

                        ui.label("描述:");
                        ui.text_edit_multiline(&mut scene.description);
                    });

                    ui.add_space(10.0);

                    // 事件編輯區
                    ui.group(|ui| {
                        ui.heading("事件列表");

                        // 新增事件按鈕
                        if ui.button("新增事件").clicked() {
                            // 添加一個默認事件
                            scene.events.push(EventType::Dialogue {
                                speaker: None,
                                content: "新事件內容".to_string(),
                                portrait: None,
                            });
                        }

                        ui.add_space(5.0);

                        // 顯示現有事件列表
                        let mut event_to_remove = None;

                        for (index, event) in scene.events.iter_mut().enumerate() {
                            ui.push_id(format!("event_{}", index), |ui| {
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        // 顯示事件標識和刪除按鈕
                                        ui.label(format!("事件 #{}", index + 1));
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::RIGHT),
                                            |ui| {
                                                if ui.button("刪除").clicked() {
                                                    event_to_remove = Some(index);
                                                }
                                            },
                                        );
                                    });

                                    // 事件類型選擇器
                                    ui.horizontal(|ui| {
                                        ui.label("類型:");

                                        let current_type = match event {
                                            EventType::Dialogue { .. } => "對話",
                                            EventType::Choice { .. } => "選項",
                                            EventType::SetFlag { .. } => "設置旗標",
                                            EventType::Condition { .. } => "條件",
                                            EventType::PlaySound { .. } => "播放音效",
                                            EventType::ChangeItem { .. } => "物品變更",
                                        };

                                        egui::ComboBox::from_id_salt(format!(
                                            "event_type_{}",
                                            index
                                        ))
                                        .selected_text(current_type)
                                        .show_ui(
                                            ui,
                                            |ui| {
                                                let mut selected = false;

                                                if ui
                                                    .selectable_label(
                                                        matches!(event, EventType::Dialogue { .. }),
                                                        "對話",
                                                    )
                                                    .clicked()
                                                {
                                                    *event = EventType::Dialogue {
                                                        speaker: None,
                                                        content: String::new(),
                                                        portrait: None,
                                                    };
                                                    selected = true;
                                                }

                                                if ui
                                                    .selectable_label(
                                                        matches!(event, EventType::Choice { .. }),
                                                        "選項",
                                                    )
                                                    .clicked()
                                                {
                                                    *event = EventType::Choice {
                                                        prompt: String::new(),
                                                        next_scene_key: String::new(),
                                                    };
                                                    selected = true;
                                                }

                                                if ui
                                                    .selectable_label(
                                                        matches!(event, EventType::SetFlag { .. }),
                                                        "設置旗標",
                                                    )
                                                    .clicked()
                                                {
                                                    *event = EventType::SetFlag {
                                                        flag: String::new(),
                                                        value: true,
                                                    };
                                                    selected = true;
                                                }

                                                if ui
                                                    .selectable_label(
                                                        matches!(
                                                            event,
                                                            EventType::Condition { .. }
                                                        ),
                                                        "條件",
                                                    )
                                                    .clicked()
                                                {
                                                    *event = EventType::Condition {
                                                        flag: String::new(),
                                                        value: true,
                                                        next_scene: String::new(),
                                                    };
                                                    selected = true;
                                                }

                                                if ui
                                                    .selectable_label(
                                                        matches!(
                                                            event,
                                                            EventType::PlaySound { .. }
                                                        ),
                                                        "播放音效",
                                                    )
                                                    .clicked()
                                                {
                                                    *event = EventType::PlaySound {
                                                        sound_id: String::new(),
                                                    };
                                                    selected = true;
                                                }

                                                if ui
                                                    .selectable_label(
                                                        matches!(
                                                            event,
                                                            EventType::ChangeItem { .. }
                                                        ),
                                                        "物品變更",
                                                    )
                                                    .clicked()
                                                {
                                                    *event = EventType::ChangeItem {
                                                        item_id: String::new(),
                                                        quantity: 1,
                                                    };
                                                    selected = true;
                                                }

                                                selected
                                            },
                                        );
                                    });

                                    // 根據事件類型顯示相應的編輯控件
                                    match event {
                                        EventType::Dialogue {
                                            speaker,
                                            content,
                                            portrait,
                                        } => {
                                            ui.horizontal(|ui| {
                                                ui.label("說話者:");
                                                if let Some(s) = speaker {
                                                    ui.text_edit_singleline(s);
                                                    if ui.button("清除").clicked() {
                                                        *speaker = None;
                                                    }
                                                } else {
                                                    if ui.button("添加說話者").clicked() {
                                                        *speaker = Some(String::new());
                                                    }
                                                }
                                            });

                                            ui.label("對話內容:");
                                            ui.text_edit_multiline(content);

                                            ui.horizontal(|ui| {
                                                ui.label("頭像 (選填):");
                                                if let Some(port) = portrait {
                                                    ui.text_edit_singleline(port);
                                                    if ui.button("清除").clicked() {
                                                        *portrait = None;
                                                    }
                                                } else {
                                                    if ui.button("添加頭像").clicked() {
                                                        *portrait = Some(String::new());
                                                    }
                                                }
                                            });
                                        }
                                        EventType::Choice {
                                            prompt,
                                            next_scene_key,
                                        } => {
                                            ui.horizontal(|ui| {
                                                ui.label("提示文字:");
                                                ui.text_edit_singleline(prompt);
                                            });

                                            ui.horizontal(|ui| {
                                                ui.label("下一場景ID:");

                                                // 顯示場景列表下拉選單
                                                egui::ComboBox::from_id_salt(format!(
                                                    "choice_next_scene_{}",
                                                    index
                                                ))
                                                .selected_text(next_scene_key.clone())
                                                .show_ui(ui, |ui| {
                                                    // 僅顯示有效的場景ID
                                                    let mut scene_ids: Vec<_> =
                                                        self.story_data.scenes.keys().collect();
                                                    scene_ids.sort();

                                                    for scene_id in scene_ids {
                                                        if ui
                                                            .selectable_label(
                                                                next_scene_key == scene_id,
                                                                scene_id,
                                                            )
                                                            .clicked()
                                                        {
                                                            *next_scene_key = scene_id.clone();
                                                        }
                                                    }
                                                });

                                                ui.text_edit_singleline(next_scene_key);
                                            });
                                        }
                                        EventType::SetFlag { flag, value } => {
                                            ui.horizontal(|ui| {
                                                ui.label("旗標名稱:");
                                                ui.text_edit_singleline(flag);
                                            });

                                            ui.checkbox(value, "旗標值");
                                        }
                                        EventType::Condition {
                                            flag,
                                            value,
                                            next_scene,
                                        } => {
                                            ui.horizontal(|ui| {
                                                ui.label("條件旗標:");
                                                ui.text_edit_singleline(flag);
                                            });

                                            ui.checkbox(value, "期望值");

                                            ui.horizontal(|ui| {
                                                ui.label("符合時前往場景:");

                                                // 顯示場景列表下拉選單
                                                egui::ComboBox::from_id_salt(format!(
                                                    "condition_next_scene_{}",
                                                    index
                                                ))
                                                .selected_text(next_scene.clone())
                                                .show_ui(ui, |ui| {
                                                    let mut scene_ids: Vec<_> =
                                                        self.story_data.scenes.keys().collect();
                                                    scene_ids.sort();

                                                    for scene_id in scene_ids {
                                                        if ui
                                                            .selectable_label(
                                                                next_scene == scene_id,
                                                                scene_id,
                                                            )
                                                            .clicked()
                                                        {
                                                            *next_scene = scene_id.clone();
                                                        }
                                                    }
                                                });

                                                ui.text_edit_singleline(next_scene);
                                            });
                                        }
                                        EventType::PlaySound { sound_id } => {
                                            ui.horizontal(|ui| {
                                                ui.label("音效ID:");
                                                ui.text_edit_singleline(sound_id);
                                            });
                                        }
                                        EventType::ChangeItem { item_id, quantity } => {
                                            ui.horizontal(|ui| {
                                                ui.label("物品ID:");
                                                ui.text_edit_singleline(item_id);
                                            });

                                            ui.horizontal(|ui| {
                                                ui.label("變更數量:");
                                                ui.add(
                                                    egui::DragValue::new(quantity)
                                                        .speed(1)
                                                        .range(-100..=100),
                                                );
                                            });

                                            ui.label(if *quantity > 0 {
                                                "正數表示獲得物品"
                                            } else if *quantity < 0 {
                                                "負數表示失去物品"
                                            } else {
                                                "數量為零無效果"
                                            });
                                        }
                                    }
                                });
                            });

                            ui.add_space(5.0);
                        }

                        // 處理事件刪除
                        if let Some(index) = event_to_remove {
                            if index < scene.events.len() {
                                scene.events.remove(index);
                            }
                        }
                    });

                    ui.add_space(10.0);

                    // 選項編輯區
                    ui.group(|ui| {
                        ui.heading("選項列表");

                        // 新增選項按鈕
                        if ui.button("新增選項").clicked() {
                            scene.options.push(dialogs_lib::DialogOption {
                                text: "新選項".to_string(),
                                next_scene: "".to_string(),
                                condition_flags: HashMap::new(),
                            });
                        }

                        ui.add_space(5.0);

                        // 顯示現有選項列表
                        let mut option_to_remove = None;

                        for (index, option) in scene.options.iter_mut().enumerate() {
                            ui.push_id(format!("option_{}", index), |ui| {
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(format!("選項 #{}", index + 1));
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::RIGHT),
                                            |ui| {
                                                if ui.button("刪除").clicked() {
                                                    option_to_remove = Some(index);
                                                }
                                            },
                                        );
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("選項文字:");
                                        ui.text_edit_singleline(&mut option.text);
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("下一場景:");

                                        // 顯示場景列表下拉選單
                                        egui::ComboBox::from_id_salt(format!(
                                            "option_next_scene_{}",
                                            index
                                        ))
                                        .selected_text(option.next_scene.clone())
                                        .show_ui(
                                            ui,
                                            |ui| {
                                                let mut scene_ids: Vec<_> =
                                                    self.story_data.scenes.keys().collect();
                                                scene_ids.sort();

                                                for scene_id in scene_ids {
                                                    if ui
                                                        .selectable_label(
                                                            option.next_scene == *scene_id,
                                                            scene_id,
                                                        )
                                                        .clicked()
                                                    {
                                                        option.next_scene = scene_id.clone();
                                                    }
                                                }
                                            },
                                        );

                                        ui.text_edit_singleline(&mut option.next_scene);
                                    });

                                    // 條件旗標編輯
                                    ui.collapsing("條件旗標", |ui| {
                                        ui.label("選項顯示條件 (需滿足所有條件):");

                                        // 顯示現有條件
                                        let mut flag_to_remove = None;

                                        for (flag, value) in option.condition_flags.iter_mut() {
                                            ui.horizontal(|ui| {
                                                ui.label(flag);
                                                ui.checkbox(value, "");
                                                if ui.button("刪除").clicked() {
                                                    flag_to_remove = Some(flag.clone());
                                                }
                                            });
                                        }

                                        // 處理旗標刪除
                                        if let Some(flag) = flag_to_remove {
                                            option.condition_flags.remove(&flag);
                                        }

                                        // 添加新條件
                                        ui.add_space(5.0);
                                        ui.label("添加新條件:");

                                        let mut new_flag = String::new();
                                        NEW_FLAG.with(|f| new_flag = f.borrow().clone());
                                        let mut new_value = true;
                                        NEW_VALUE.with(|v| new_value = *v.borrow());

                                        ui.horizontal(|ui| {
                                            ui.label("旗標名稱:");
                                            if ui.text_edit_singleline(&mut new_flag).changed() {
                                                NEW_FLAG
                                                    .with(|f| *f.borrow_mut() = new_flag.clone());
                                            }

                                            if ui.checkbox(&mut new_value, "值").changed() {
                                                NEW_VALUE.with(|v| *v.borrow_mut() = new_value);
                                            }

                                            if ui.button("添加").clicked() && !new_flag.is_empty()
                                            {
                                                option
                                                    .condition_flags
                                                    .insert(new_flag.clone(), new_value);
                                                NEW_FLAG.with(|f| f.borrow_mut().clear());
                                            }
                                        });
                                    });
                                });
                            });

                            ui.add_space(5.0);
                        }

                        // 處理選項刪除
                        if let Some(index) = option_to_remove {
                            if index < scene.options.len() {
                                scene.options.remove(index);
                            }
                        }
                    });

                    ui.add_space(10.0);

                    // 前置條件編輯區
                    ui.group(|ui| {
                        ui.heading("場景前置條件");
                        ui.label("進入此場景需要滿足的條件:");

                        let mut prereq_to_remove = None;

                        for (flag, value) in scene.prerequisites.iter_mut() {
                            ui.horizontal(|ui| {
                                ui.label(flag);
                                ui.checkbox(value, "");
                                if ui.button("刪除").clicked() {
                                    prereq_to_remove = Some(flag.clone());
                                }
                            });
                        }

                        // 處理前置條件刪除
                        if let Some(flag) = prereq_to_remove {
                            scene.prerequisites.remove(&flag);
                        }

                        // 添加新前置條件
                        ui.add_space(5.0);
                        ui.label("添加新前置條件:");

                        let mut new_prereq_flag = String::new();
                        NEW_PREREQ_FLAG.with(|f| new_prereq_flag = f.borrow().clone());
                        let mut new_prereq_value = true;
                        NEW_PREREQ_VALUE.with(|v| new_prereq_value = *v.borrow());

                        ui.horizontal(|ui| {
                            ui.label("旗標名稱:");
                            if ui.text_edit_singleline(&mut new_prereq_flag).changed() {
                                NEW_PREREQ_FLAG.with(|f| *f.borrow_mut() = new_prereq_flag.clone());
                            }

                            if ui.checkbox(&mut new_prereq_value, "值").changed() {
                                NEW_PREREQ_VALUE.with(|v| *v.borrow_mut() = new_prereq_value);
                            }

                            if ui.button("添加").clicked() && !new_prereq_flag.is_empty() {
                                scene
                                    .prerequisites
                                    .insert(new_prereq_flag.clone(), new_prereq_value);
                                NEW_PREREQ_FLAG.with(|f| f.borrow_mut().clear());
                            }
                        });
                    });
                });
        } else {
            ui.heading("劇情編輯器");
            ui.label("選擇或建立一個場景開始編輯");
        }

        // 處理按鈕事件（在 ScrollArea 外部）
        // 克隆必要的數據以避免借用衝突
        let action = if save_clicked {
            if let Some((scene_id, scene)) = &self.temp_scene {
                let scene_id_clone = scene_id.clone();
                let scene_clone = scene.clone();

                match self.story_data.update_scene(scene_id_clone, scene_clone) {
                    Ok(_) => Some(("成功更新場景".to_string(), false)),
                    Err(err) => Some((err, true)),
                }
            } else {
                None
            }
        } else {
            None
        };

        // 應用 save 操作的結果
        if let Some((message, is_error)) = action {
            self.set_status(message, is_error);
        }

        // 處理刪除場景按鈕
        if delete_clicked && self.temp_scene.is_some() {
            let scene_id = self.temp_scene.as_ref().unwrap().0.clone();
            self.confirmation_action = ConfirmationAction::DeleteScene(scene_id);
            self.show_confirmation_dialog = true;
        }
    }

    fn show_confirmation_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_confirmation_dialog {
            return;
        }

        let mut open = self.show_confirmation_dialog;
        let title = "確認";
        let message = match &self.confirmation_action {
            ConfirmationAction::None => "確定要執行此操作嗎？",
            ConfirmationAction::DeleteScene(_) => "確定要刪除此場景嗎？",
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
                ConfirmationAction::DeleteScene(scene_id) => {
                    if let Err(err) = self.story_data.delete_scene(&scene_id) {
                        self.set_status(err, true);
                    } else {
                        self.set_status("成功刪除場景".to_string(), false);
                        self.temp_scene = None;
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

    fn show_status_message(&mut self, ctx: &egui::Context) {
        if let Some((message, is_error)) = &self.status_message {
            let color = if *is_error {
                egui::Color32::RED
            } else {
                egui::Color32::GREEN
            };

            egui::TopBottomPanel::bottom("status_panel").show(ctx, |ui| {
                ui.label(RichText::new(message).color(color));
            });
        }
    }
}

impl eframe::App for DialogsEditor {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            self.show_file_menu(ui);
        });

        egui::SidePanel::left("scenes_list_panel")
            .default_width(200.0)
            .show(ctx, |ui| {
                self.show_scene_list(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_scene_editor(ui);
        });

        self.show_confirmation_dialog(ctx);
        self.show_status_message(ctx);
    }
}

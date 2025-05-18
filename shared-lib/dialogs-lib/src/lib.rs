use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum_macros::{Display, EnumString};

/// 場景類型
#[derive(Debug, Clone, Deserialize, Serialize, EnumString, Display, PartialEq)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SceneType {
    Dialogue,
    Choice,
    Battle,
    Ending,
}

/// 事件類型
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum EventType {
    /// 角色對話
    Dialogue {
        speaker: Option<String>,
        content: String,
        portrait: Option<String>,
    },
    /// 選項提示
    Choice {
        prompt: String,
        next_scene_key: String,
    },
    /// 設置旗標
    SetFlag { flag: String, value: bool },
    /// 條件檢查
    Condition {
        flag: String,
        value: bool,
        next_scene: String,
    },
    /// 播放音效
    PlaySound { sound_id: String },
    /// 物品變更
    ChangeItem {
        item_id: String,
        quantity: i32, // 正數為獲得，負數為失去
    },
}

/// 選項結構
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DialogOption {
    pub text: String,
    pub next_scene: String,
    #[serde(default)]
    pub condition_flags: HashMap<String, bool>, // 需要滿足的旗標條件
}

/// 場景結構
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Scene {
    pub scene_type: SceneType,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub events: Vec<EventType>,
    #[serde(default)]
    pub options: Vec<DialogOption>,
    #[serde(default)]
    pub prerequisites: HashMap<String, bool>, // 進入場景需要的前置條件
}

/// 實作 Scene 的預設值
impl Default for Scene {
    fn default() -> Self {
        Self {
            scene_type: SceneType::Dialogue,
            title: String::new(),
            description: String::new(),
            events: vec![],
            options: vec![],
            prerequisites: HashMap::new(),
        }
    }
}

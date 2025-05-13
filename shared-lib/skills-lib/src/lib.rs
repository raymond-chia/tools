use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Error, ErrorKind};
use std::path::Path;
use strum_macros::{Display, EnumString};

#[derive(Debug, Clone, Deserialize, Serialize, EnumString, Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Tag {
    // 主動; 被動
    Active,
    Passive,
    // 範圍
    Single,
    Area,
    // 距離
    Melee,
    Ranged,
    // 特性
    Attack,
    Beneficial,
    BodyControl,
    MindControl,
    // 其他
    Magic,
    Heal,
    Fire,
}

#[derive(Debug, Clone, Deserialize, Serialize, EnumString, Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum TargetType {
    Caster,
    Ally,
    AllyExcludeCaster,
    Enemy,
    Any,
    AnyExcludeCaster,
}

#[derive(Debug, Clone, Deserialize, Serialize, EnumString, Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Shape {
    Point,
    Circle(usize),
    Rectangle(usize, usize),
    Line(usize),
    Cone(usize, f32),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Effect {
    Hp {
        target_type: TargetType,
        shape: Shape,
        value: i32,
    },
    Burn {
        target_type: TargetType,
        shape: Shape,
        duration: u16,
    },
}

/// 技能資料結構
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Skill {
    #[serde(default)]
    pub tags: Vec<Tag>,
    #[serde(default)]
    pub range: usize,
    #[serde(default)]
    pub cost: u16,
    #[serde(default)]
    pub hit_rate: Option<u16>,
    #[serde(default)]
    pub crit_rate: Option<u16>,
    #[serde(default)]
    pub effects: Vec<Effect>,
}

/// 實作 Skill 的預設值
impl Default for Skill {
    fn default() -> Self {
        Self {
            tags: vec![],
            range: 0,
            cost: 0,
            hit_rate: None,
            crit_rate: None,
            effects: vec![],
        }
    }
}

/// 技能資料集
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkillsData {
    #[serde(flatten)]
    pub skills: HashMap<String, Skill>,
}

impl SkillsData {
    /// 從指定路徑載入 TOML 檔案
    pub fn from_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let content = fs::read_to_string(path)?;
        Self::from_toml_str(&content)
    }

    /// 從 TOML 字串解析
    pub fn from_toml_str(content: &str) -> io::Result<Self> {
        let skills_map: HashMap<String, Skill> = toml::from_str(content).map_err(|err| {
            Error::new(ErrorKind::InvalidData, format!("解析 TOML 失敗: {}", err))
        })?;

        Ok(Self { skills: skills_map })
    }

    /// 轉換為 TOML 格式
    pub fn to_toml(&self) -> io::Result<String> {
        toml::to_string_pretty(&self.skills)
            .map_err(|err| Error::new(ErrorKind::InvalidData, format!("序列化 TOML 失敗: {}", err)))
    }

    /// 寫入到檔案
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let toml_content = self.to_toml()?;
        fs::write(path, toml_content)
    }

    /// 新增技能
    pub fn create_skill(&mut self, skill_id: String) -> Result<(), String> {
        if self.skills.contains_key(&skill_id) {
            return Err("技能 ID 已存在".to_string());
        }
        self.skills.insert(skill_id, Skill::default());
        Ok(())
    }

    /// 更新技能屬性
    pub fn update_skill(&mut self, skill_id: String, updated_skill: Skill) -> Result<(), String> {
        if let Some(skill) = self.skills.get_mut(&skill_id) {
            *skill = updated_skill;
            Ok(())
        } else {
            Err(format!("找不到技能 ID: {}", skill_id))
        }
    }

    /// 刪除技能
    pub fn delete_skill(&mut self, skill_id: &str) -> Result<(), String> {
        if !self.skills.contains_key(skill_id) {
            return Err("找不到指定的技能".to_string());
        }
        self.skills.remove(skill_id);
        Ok(())
    }

    /// 建立空的技能資料集
    pub fn empty() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }
}

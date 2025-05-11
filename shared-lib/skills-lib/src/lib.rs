use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Error, ErrorKind};
use std::path::Path;

/// 技能資料結構
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Skill {
    #[serde(default = "default_is_true")]
    pub is_active: bool,
    #[serde(default)]
    pub is_beneficial: bool,
}

fn default_is_true() -> bool {
    true
}

/// 實作 Skill 的預設值
impl Default for Skill {
    fn default() -> Self {
        Self {
            is_active: true,
            is_beneficial: false,
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
    pub fn update_skill(&mut self, skill_id: String, is_active: bool, is_beneficial: bool) {
        if let Some(skill) = self.skills.get_mut(&skill_id) {
            skill.is_active = is_active;
            skill.is_beneficial = is_beneficial;
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

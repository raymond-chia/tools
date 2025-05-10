use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Error, ErrorKind};
use std::path::Path;

/// 技能資料結構
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
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

    /// 取得單一技能
    pub fn get_skill(&self, skill_id: &str) -> Option<&Skill> {
        self.skills.get(skill_id)
    }

    /// 新增技能
    pub fn create_skill(
        &mut self,
        skill_id: String,
        name: String,
        description: String,
    ) -> Result<(), String> {
        if self.skills.contains_key(&skill_id) {
            return Err("技能 ID 已存在".to_string());
        }
        self.skills.insert(skill_id, Skill { name, description });
        Ok(())
    }

    /// 更新技能
    pub fn update_skill(&mut self, skill_id: String, name: String, description: String) {
        self.skills.insert(skill_id, Skill { name, description });
    }

    /// 刪除技能
    pub fn delete_skill(&mut self, skill_id: &str) -> Result<(), String> {
        if !self.skills.contains_key(skill_id) {
            return Err("找不到指定的技能".to_string());
        }
        self.skills.remove(skill_id);
        Ok(())
    }

    /// 檢查技能是否存在
    pub fn has_skill(&self, skill_id: &str) -> bool {
        self.skills.contains_key(skill_id)
    }

    /// 列出所有技能 ID
    pub fn skill_ids(&self) -> Vec<String> {
        self.skills.keys().cloned().collect()
    }

    /// 建立空的技能資料集
    pub fn empty() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_skills_data() {
        let data = SkillsData::empty();
        assert_eq!(data.skills.len(), 0);
    }
}

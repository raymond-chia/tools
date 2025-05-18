use serde::{Deserialize, Serialize};
use skills_lib::{Effect, Shape, Skill, Tag};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{self, Error, ErrorKind};
use std::path::Path;

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
        let sorted_skills: BTreeMap<_, _> = self.skills.clone().into_iter().collect();

        toml::to_string_pretty(&sorted_skills)
            .map_err(|err| Error::new(ErrorKind::InvalidData, format!("序列化 TOML 失敗: {}", err)))
    }

    /// 寫入到檔案
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let toml_content = self.to_toml()?;
        fs::write(path, toml_content)
    }

    /// 新增技能
    pub fn create_skill(&mut self, skill_id: &str) -> Result<(), String> {
        if self.skills.contains_key(skill_id) {
            return Err("技能 ID 已存在".to_string());
        }
        self.skills.insert(skill_id.to_string(), Skill::default());
        Ok(())
    }

    /// 更新技能屬性
    pub fn update_skill(&mut self, skill_id: &str, updated_skill: Skill) -> Result<(), String> {
        let Some(skill) = self.skills.get_mut(skill_id) else {
            return Err(format!("找不到技能 ID: {}", skill_id));
        };
        if let Err(e) = Self::sanity_check(&updated_skill) {
            return Err(format!("不合法的技能設定: {}", e));
        }
        *skill = updated_skill;
        Ok(())
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
    pub fn sanity_check(skill: &Skill) -> Result<(), String> {
        if skill.effects.len() == 0 {
            return Err("技能必須至少有一個效果".to_string());
        }
        if skill.range.0 > skill.range.1 {
            return Err("技能範圍的最小值不能大於最大值".to_string());
        }
        if skill.tags.contains(&Tag::Single) {
            match &skill.effects[0] {
                Effect::Hp { shape, .. } | Effect::Burn { shape, .. } => {
                    if shape != &Shape::Point {
                        return Err("單體技能的效果形狀必須是點".to_string());
                    }
                }
            }
        }
        if skill.tags.contains(&Tag::Area) {
            match &skill.effects[0] {
                Effect::Hp { shape, .. } | Effect::Burn { shape, .. } => match shape {
                    Shape::Point => {
                        return Err("範圍技能的效果形狀不能是點".to_string());
                    }
                    Shape::Circle(radius) => {
                        if *radius < 2 {
                            return Err("範圍技能的效果形狀半徑不能小於 2".to_string());
                        }
                    }
                    Shape::Rectangle(width, height) => {
                        if *width < 2 && *height < 2 {
                            return Err("範圍技能的效果形狀寬度和高度不能小於 2".to_string());
                        }
                    }
                    Shape::Line(length) => {
                        if *length < 2 {
                            return Err("範圍技能的效果形狀長度不能小於 2".to_string());
                        }
                    }
                    Shape::Cone(radius, angle) => {
                        if *radius < 2 && *angle < 90.0 {
                            return Err(
                                "範圍技能的效果形狀半徑不能小於 2 同時角度又小於 90".to_string()
                            );
                        }
                    }
                },
            }
        }
        if skill.tags.contains(&Tag::Caster) {
            match &skill.effects[0] {
                Effect::Hp { target_type, .. } | Effect::Burn { target_type, .. } => {
                    if target_type != &skills_lib::TargetType::Caster {
                        return Err("施法者技能的目標類型必須是施法者".to_string());
                    }
                }
            }
        }
        return Ok(());
    }
}

use serde::Serialize;
use skills_lib::{Skill, SkillsData};
use std::collections::HashMap;
use std::str::FromStr;

/// 前端的技能資料結構
#[derive(Serialize)]
struct SkillsResponse {
    skills: HashMap<String, Skill>,
    file_path: String,
}

#[tauri::command]
fn check_file(path: &str) -> Result<String, String> {
    let file_path = std::path::Path::new(path);

    if !file_path.exists() {
        return Err("找不到指定的檔案".into());
    }

    if file_path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
        return Err("請選擇 TOML 檔案".into());
    }

    Ok(path.to_string())
}

#[tauri::command]
fn load_skills(path: &str) -> Result<SkillsResponse, String> {
    let skills_data =
        SkillsData::from_file(path).map_err(|e| format!("載入技能檔案失敗: {}", e))?;

    Ok(SkillsResponse {
        skills: skills_data.skills,
        file_path: path.to_string(),
    })
}

use serde::Deserialize;
use skills_lib::{Effect, Tag, TargetType};

/// 從前端接收的技能編輯請求
#[derive(Deserialize)]
struct SkillUpdateRequest {
    tags: Vec<String>,
    range: usize,
    area: usize,
    cost: u16,
    hit_rate: Option<u16>,
    crit_rate: Option<u16>,
    effects: Vec<EffectRequest>,
}

/// 從前端接收的效果請求
#[derive(Deserialize)]
struct EffectRequest {
    r#type: String,
    target_type: Option<String>,
    value: Option<i32>,
    duration: Option<u16>,
}

#[tauri::command]
fn save_skill(path: &str, skill_id: &str, skill_data: SkillUpdateRequest) -> Result<(), String> {
    let mut skills_data =
        SkillsData::from_file(path).map_err(|e| format!("載入技能檔案失敗: {}", e))?;

    // 將字串標籤轉換為 Tag 枚舉
    let tags = skill_data
        .tags
        .iter()
        .filter_map(|tag_str| match tag_str.as_str() {
            "active" => Some(Tag::Active),
            "passive" => Some(Tag::Passive),
            "single" => Some(Tag::Single),
            "area" => Some(Tag::Area),
            "melee" => Some(Tag::Melee),
            "ranged" => Some(Tag::Ranged),
            "attack" => Some(Tag::Attack),
            "beneficial" => Some(Tag::Beneficial),
            "bodycontrol" => Some(Tag::BodyControl),
            "mindcontrol" => Some(Tag::MindControl),
            "magic" => Some(Tag::Magic),
            "heal" => Some(Tag::Heal),
            "fire" => Some(Tag::Fire),
            _ => None,
        })
        .collect();

    // 處理效果
    let effects = skill_data
        .effects
        .iter()
        .filter_map(|effect_req| match effect_req.r#type.as_str() {
            "hp" => {
                let target_type_str = effect_req.target_type.as_ref()?;
                let value = effect_req.value?;

                let target_type = TargetType::from_str(target_type_str).ok()?;

                Some(Effect::Hp { target_type, value })
            }
            "burn" => {
                let duration = effect_req.duration?;
                Some(Effect::Burn { duration })
            }
            _ => None,
        })
        .collect();

    // 創建更新後的技能
    let updated_skill = Skill {
        tags,
        range: skill_data.range,
        area: skill_data.area,
        cost: skill_data.cost,
        hit_rate: skill_data.hit_rate,
        crit_rate: skill_data.crit_rate,
        effects,
    };

    // 更新技能
    skills_data.update_skill(skill_id.to_string(), updated_skill)?;

    // 保存到檔案
    skills_data
        .save_to_file(path)
        .map_err(|e| format!("保存技能失敗: {}", e))?;

    Ok(())
}

#[tauri::command]
fn create_skill(path: &str, skill_id: &str) -> Result<(), String> {
    let mut skills_data =
        SkillsData::from_file(path).map_err(|e| format!("載入技能檔案失敗: {}", e))?;

    skills_data.create_skill(skill_id.to_string())?;

    skills_data
        .save_to_file(path)
        .map_err(|e| format!("保存技能失敗: {}", e))?;

    Ok(())
}

#[tauri::command]
fn delete_skill(path: &str, skill_id: &str) -> Result<(), String> {
    let mut skills_data =
        SkillsData::from_file(path).map_err(|e| format!("載入技能檔案失敗: {}", e))?;

    skills_data.delete_skill(skill_id)?;

    skills_data
        .save_to_file(path)
        .map_err(|e| format!("保存檔案失敗: {}", e))?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            check_file,
            load_skills,
            save_skill,
            create_skill,
            delete_skill
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

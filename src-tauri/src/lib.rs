use serde::Serialize;
use skills_lib::{Skill, SkillsData};
use std::collections::HashMap;
use std::path::Path;

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

#[tauri::command]
fn save_skill(
    path: &str,
    skill_id: &str,
    is_active: bool,
    is_beneficial: bool,
) -> Result<(), String> {
    let mut skills_data =
        SkillsData::from_file(path).map_err(|e| format!("載入技能檔案失敗: {}", e))?;

    skills_data.update_skill(skill_id.to_string(), is_active, is_beneficial);

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

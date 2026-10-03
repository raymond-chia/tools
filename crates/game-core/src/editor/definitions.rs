//! 編輯器的欄位修改、新增、複製、刪除與引用檢查；不參與遊戲執行期規則運算。
use super::EditorError;
use crate::authoring::{Definitions, Map, UnitType};
use crate::gameplay_config::DEFAULT_GROUND_TERRAIN;
use crate::{Game, SkillDef, SkillEffect, TerrainEntryRule, TerrainLayer, TerrainTypeDef};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 作者輸入的資料操作；引用與合法性由編輯器集中處理。
/// ID 建立後不可更動，因此不需要更新既有引用；刪除前仍須檢查是否被引用。
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum DefinitionEdit {
    SkillEffectOptions,
    UpdateField {
        category: DefinitionCategory,
        id: String,
        key: String,
        value: serde_json::Value,
    },
    ChangeSkillEffect {
        id: String,
        #[serde(flatten)]
        effect: EffectSelection,
    },
    Add {
        category: DefinitionCategory,
        id: String,
    },
    Duplicate {
        category: DefinitionCategory,
        id: String,
        source_id: String,
    },
    CheckRemove {
        category: DefinitionCategory,
        id: String,
    },
    Remove {
        category: DefinitionCategory,
        id: String,
    },
}

// 作者只選擇效果及必要的地形；效果欄位與初始值由核心建立。
#[derive(Deserialize)]
#[serde(tag = "effect", rename_all = "snake_case")]
enum EffectSelection {
    Attack,
    Push,
    Mire { terrain: String },
    Heal,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DefinitionCategory {
    UnitTypes,
    Skills,
    TerrainTypes,
}

#[derive(Serialize)]
struct EditedDocuments {
    definitions: Definitions,
}

pub fn edit_definition_from_json(
    definitions: &str,
    maps: &str,
    command: &str,
) -> Result<String, EditorError> {
    let mut definitions: Definitions = serde_json::from_str(definitions)
        .map_err(|e| EditorError::input("definitions_json_parse", e.to_string()))?;
    let maps: BTreeMap<String, Map> = serde_json::from_str(maps)
        .map_err(|e| EditorError::input("map_json_parse", e.to_string()))?;
    let command: DefinitionEdit = serde_json::from_str(command)
        .map_err(|e| EditorError::operation("invalid_command", &e.to_string(), Vec::new()))?;
    match command {
        DefinitionEdit::SkillEffectOptions => {
            let mut terrain_ids: Vec<_> = definitions
                .terrain_types
                .iter()
                .filter(|(_, terrain)| terrain.layer == TerrainLayer::Overlay)
                .map(|(id, _)| id.clone())
                .collect();
            terrain_ids.sort();
            return Ok(serde_json::json!({"terrain_ids": terrain_ids}).to_string());
        }
        DefinitionEdit::UpdateField {
            category,
            id,
            key,
            value,
        } => match category {
            DefinitionCategory::UnitTypes => {
                let entry = definitions
                    .unit_types
                    .iter_mut()
                    .find(|entry| entry.id == id)
                    .ok_or_else(|| EditorError::operation("not_found", &id, Vec::new()))?;
                update_field(entry, &key, value)?;
            }
            DefinitionCategory::Skills => {
                let entry = definitions
                    .skills
                    .iter_mut()
                    .find(|entry| entry.id == id)
                    .ok_or_else(|| EditorError::operation("not_found", &id, Vec::new()))?;
                update_field(entry, &key, value)?;
            }
            DefinitionCategory::TerrainTypes => {
                let entry = definitions
                    .terrain_types
                    .get_mut(&id)
                    .ok_or_else(|| EditorError::operation("not_found", &id, Vec::new()))?;
                update_field(entry, &key, value)?;
            }
        },
        DefinitionEdit::ChangeSkillEffect { id, effect } => {
            let skill = definitions
                .skills
                .iter_mut()
                .find(|skill| skill.id == id)
                .ok_or_else(|| EditorError::operation("not_found", &id, Vec::new()))?;
            skill.effect = match effect {
                EffectSelection::Attack => SkillEffect::Attack {
                    attack_bonus: 0,
                    power_bonus: 0,
                },
                EffectSelection::Push => SkillEffect::Push {
                    attack_bonus: 0,
                    power_bonus: 0,
                },
                EffectSelection::Mire { terrain } => SkillEffect::Mire {
                    terrain,
                    duration: 2,
                },
                EffectSelection::Heal => SkillEffect::Heal { power_bonus: 0 },
            };
        }
        DefinitionEdit::Add { category, id } => {
            validate_new_id(&definitions, category, &id)?;
            match category {
                DefinitionCategory::UnitTypes => definitions.unit_types.push(UnitType {
                    id,
                    visual: "fighter.svg".to_owned(),
                    width: 1,
                    height: 1,
                    hp: 10,
                    movement: 5,
                    initiative: 0,
                    dodge: 2,
                    block: 2,
                    attack: 3,
                    power: 3,
                    skills: Vec::new(),
                }),
                DefinitionCategory::Skills => definitions.skills.push(SkillDef {
                    id,
                    ranged: false,
                    min_range: 1,
                    max_range: 1,
                    effect: SkillEffect::Attack {
                        attack_bonus: 0,
                        power_bonus: 0,
                    },
                }),
                DefinitionCategory::TerrainTypes => {
                    definitions.terrain_types.insert(
                        id,
                        TerrainTypeDef {
                            visual: "plain".to_owned(),
                            layer: TerrainLayer::Overlay,
                            entry_rule: TerrainEntryRule::Walkable,
                            damage: 0,
                            extra_movement_cost: 0,
                            dodge_penalty: 0,
                            block_penalty: 0,
                        },
                    );
                }
            }
        }
        DefinitionEdit::Duplicate {
            category,
            id,
            source_id,
        } => {
            validate_new_id(&definitions, category, &id)?;
            if !contains_id(&definitions, category, &source_id) {
                return Err(EditorError::operation("not_found", &source_id, Vec::new()));
            }
            // ID 建立後不可更動；複製只設定新資料的 ID，因此不需要更新既有引用。
            match category {
                DefinitionCategory::UnitTypes => {
                    let mut unit = definitions
                        .unit_types
                        .iter()
                        .find(|unit| unit.id == source_id)
                        .expect("已確認複製來源的單位 ID 存在")
                        .clone();
                    unit.id = id;
                    definitions.unit_types.push(unit);
                }
                DefinitionCategory::Skills => {
                    let mut skill = definitions
                        .skills
                        .iter()
                        .find(|skill| skill.id == source_id)
                        .expect("已確認複製來源的技能 ID 存在")
                        .clone();
                    skill.id = id;
                    definitions.skills.push(skill);
                }
                DefinitionCategory::TerrainTypes => {
                    let terrain = definitions
                        .terrain_types
                        .get(&source_id)
                        .expect("已確認複製來源的地形 ID 存在")
                        .clone();
                    definitions.terrain_types.insert(id, terrain);
                }
            }
        }
        DefinitionEdit::CheckRemove { category, id } => {
            check_removal(&definitions, &maps, category, &id)?;
            return Ok(serde_json::json!({"can_remove": true}).to_string());
        }
        DefinitionEdit::Remove { category, id } => {
            check_removal(&definitions, &maps, category, &id)?;
            match category {
                DefinitionCategory::UnitTypes => {
                    definitions.unit_types.retain(|unit| unit.id != id)
                }
                DefinitionCategory::Skills => definitions.skills.retain(|skill| skill.id != id),
                DefinitionCategory::TerrainTypes => {
                    definitions.terrain_types.remove(&id);
                }
            }
        }
    }

    for (path, map) in &maps {
        Game::from_authoring(definitions.clone(), map.clone()).map_err(|error| {
            EditorError::Map {
                path: path.clone(),
                error,
            }
        })?;
    }
    serde_json::to_string(&EditedDocuments { definitions })
        .map_err(|e| EditorError::input("json_serialize", e.to_string()))
}

// 檢查與實際刪除共用同一份引用判斷；預檢不修改文件。
fn check_removal(
    definitions: &Definitions,
    maps: &BTreeMap<String, Map>,
    category: DefinitionCategory,
    id: &str,
) -> Result<(), EditorError> {
    validate_existing_id(definitions, category, id)?;
    let mut references = Vec::new();
    match category {
        DefinitionCategory::UnitTypes => {
            for (path, map) in maps {
                if map.units.iter().any(|unit| unit.unit_type == id) {
                    references.push(("maps", path.clone()));
                }
            }
        }
        DefinitionCategory::Skills => {
            for unit in &definitions.unit_types {
                if unit.skills.iter().any(|skill| skill == id) {
                    references.push(("unit_types", unit.id.clone()));
                }
            }
        }
        DefinitionCategory::TerrainTypes => {
            for skill in &definitions.skills {
                if matches!(&skill.effect, SkillEffect::Mire { terrain, duration: _ } if terrain == id)
                {
                    references.push(("skills", skill.id.clone()));
                }
            }
            for (path, map) in maps {
                if map.terrains.iter().any(|terrain| terrain.kind == id) {
                    references.push(("maps", path.clone()));
                }
            }
        }
    }
    if !references.is_empty() {
        return Err(EditorError::operation("referenced", id, references));
    }
    Ok(())
}

fn contains_id(definitions: &Definitions, category: DefinitionCategory, id: &str) -> bool {
    match category {
        DefinitionCategory::UnitTypes => definitions.unit_types.iter().any(|unit| unit.id == id),
        DefinitionCategory::Skills => definitions.skills.iter().any(|skill| skill.id == id),
        DefinitionCategory::TerrainTypes => definitions.terrain_types.contains_key(id),
    }
}

fn validate_new_id(
    definitions: &Definitions,
    category: DefinitionCategory,
    id: &str,
) -> Result<(), EditorError> {
    if id.trim().is_empty() {
        return Err(EditorError::operation("empty_id", id, Vec::new()));
    }
    if contains_id(definitions, category, id) {
        return Err(EditorError::operation("duplicate_id", id, Vec::new()));
    }
    Ok(())
}

fn validate_existing_id(
    definitions: &Definitions,
    category: DefinitionCategory,
    id: &str,
) -> Result<(), EditorError> {
    if !contains_id(definitions, category, id) {
        return Err(EditorError::operation("not_found", id, Vec::new()));
    }
    if matches!(category, DefinitionCategory::TerrainTypes) && id == DEFAULT_GROUND_TERRAIN {
        return Err(EditorError::operation("default_ground", id, Vec::new()));
    }
    Ok(())
}

// 欄位型別沿用共用作者資料格式，避免編輯器另維護一套欄位與型別表。
fn update_field<T: Serialize + serde::de::DeserializeOwned>(
    entry: &mut T,
    key: &str,
    value: serde_json::Value,
) -> Result<(), EditorError> {
    let mut document = serde_json::to_value(&*entry)
        .map_err(|e| EditorError::input("json_serialize", e.to_string()))?;
    if key == "id" || key == "effect" || document.get(key).is_none() {
        return Err(EditorError::operation("invalid_field", key, Vec::new()));
    }
    document[key] = value;
    *entry = serde_json::from_value(document)
        .map_err(|e| EditorError::input("definitions_json_parse", e.to_string()))?;
    Ok(())
}

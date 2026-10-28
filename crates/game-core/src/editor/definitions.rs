//! 編輯器的欄位修改、新增、複製、刪除與引用檢查；不參與遊戲執行期規則運算。
use super::EditorError;
use crate::authoring::{Definitions, EquipmentDef, EquipmentSlot, Map, UnitType, validation};
use crate::gameplay_config::DEFAULT_GROUND_TERRAIN;
use crate::{
    ActiveSkillDef, ActiveSkillEffect, PassiveSkillDef, PassiveSkillEffect, TerrainEntryRule,
    TerrainLayer, TerrainTypeDef,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 作者輸入的資料操作；引用與合法性由編輯器集中處理。
/// ID 建立後不可更動，因此不需要更新既有引用；刪除前仍須檢查是否被引用。
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum DefinitionEdit {
    SkillEffectOptions,
    UpdateDefaultPassiveSkills {
        skills: Vec<String>,
    },
    Move {
        category: DefinitionCategory,
        id: String,
        offset: i32,
    },
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
    Sweep,
    MoveThenAttack,
    Attack,
    Loot,
    Push,
    Mire { terrain: String },
    Heal,
    Flanking,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DefinitionCategory {
    Equipment,
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
    let mut definitions: Definitions = super::json::from_str(definitions)
        .map_err(|e| EditorError::conversion("definitions_json_parse", e.to_string()))?;
    let maps: BTreeMap<String, Map> = super::json::from_str(maps)
        .map_err(|e| EditorError::conversion("map_json_parse", e.to_string()))?;
    let command: DefinitionEdit = super::json::from_str(command)
        .map_err(|e| EditorError::operation("invalid_command", &e.to_string(), Vec::new()))?;
    match command {
        DefinitionEdit::UpdateDefaultPassiveSkills { skills } => {
            definitions.default_passive_skills = skills;
        }
        DefinitionEdit::SkillEffectOptions => {
            let terrain_ids: Vec<_> = definitions
                .terrain_types
                .iter()
                .filter(|terrain| terrain.layer == TerrainLayer::Overlay)
                .map(|terrain| terrain.id.clone())
                .collect();
            let mut terrains: Vec<_> = definitions.terrain_types.iter().collect();
            terrains.sort_by_key(|terrain| terrain.layer != TerrainLayer::Ground);
            let terrain_entries: Vec<_> = terrains
                .iter()
                .map(|terrain| {
                    serde_json::json!({
                        "id": terrain.id,
                        "move_targets": terrains.iter()
                            .filter(|target| target.layer == terrain.layer)
                            .map(|target| &target.id)
                            .collect::<Vec<_>>(),
                    })
                })
                .collect();
            let equipment = crate::equipment::definitions(definitions.equipment.clone())?;
            let equipment_choices: BTreeMap<_, _> = definitions
                .unit_types
                .iter()
                .map(|unit| (unit.id.clone(), equipment_choices(unit, &equipment)))
                .collect();
            let mut active_skills: Vec<_> = definitions.active_skills.iter().collect();
            active_skills.sort_by_key(|skill| is_magical_effect(&skill.effect));
            let active_skill_ids: Vec<_> = active_skills.iter().map(|skill| &skill.id).collect();
            let passive_skill_ids: Vec<_> = definitions
                .passive_skills
                .iter()
                .map(|passive| &passive.id)
                .collect();
            // 技能頁依一般主動、魔法、被動分組；拖曳只調整同組內的順序。
            let skill_entries: Vec<_> = active_skills
                .iter()
                .map(|skill| {
                    let move_targets: Vec<_> = active_skills
                        .iter()
                        .filter(|target| {
                            is_magical_effect(&target.effect) == is_magical_effect(&skill.effect)
                        })
                        .map(|target| &target.id)
                        .collect();
                    serde_json::json!({"id": skill.id, "move_targets": move_targets})
                })
                .chain(
                    passive_skill_ids
                        .iter()
                        .map(|id| serde_json::json!({"id": id, "move_targets": passive_skill_ids})),
                )
                .collect();
            return Ok(serde_json::json!({
                "terrain_ids": terrain_ids,
                "terrain_entries": terrain_entries,
                "skill_entries": skill_entries,
                "equipment_choices": equipment_choices,
                "active_skill_ids": active_skill_ids,
                "passive_skill_ids": passive_skill_ids,
            })
            .to_string());
        }
        DefinitionEdit::Move {
            category,
            id,
            offset,
        } => match category {
            DefinitionCategory::Equipment => {
                move_entry(&mut definitions.equipment, &id, offset, |_, _| true)?
            }
            DefinitionCategory::UnitTypes => {
                move_entry(&mut definitions.unit_types, &id, offset, |_, _| true)?
            }
            DefinitionCategory::Skills => {
                if position(&definitions.active_skills, &id).is_some() {
                    // 與畫面使用相同分組，讓拖曳位移對應正確的資料位置。
                    definitions
                        .active_skills
                        .sort_by_key(|skill| is_magical_effect(&skill.effect));
                    move_entry(&mut definitions.active_skills, &id, offset, |a, b| {
                        is_magical_effect(&a.effect) == is_magical_effect(&b.effect)
                    })?
                } else {
                    move_entry(&mut definitions.passive_skills, &id, offset, |_, _| true)?
                }
            }
            DefinitionCategory::TerrainTypes => {
                // 地形固定依圖層分組；拖曳只調整同圖層內的順序。
                definitions
                    .terrain_types
                    .sort_by_key(|terrain| terrain.layer != TerrainLayer::Ground);
                move_entry(&mut definitions.terrain_types, &id, offset, |a, b| {
                    a.layer == b.layer
                })?
            }
        },
        DefinitionEdit::UpdateField {
            category,
            id,
            key,
            value,
        } => match category {
            DefinitionCategory::Equipment => {
                update_entry(&mut definitions.equipment, &id, &key, value)?
            }
            DefinitionCategory::UnitTypes => {
                update_entry(&mut definitions.unit_types, &id, &key, value)?
            }
            DefinitionCategory::Skills => {
                if position(&definitions.active_skills, &id).is_some() {
                    update_entry(&mut definitions.active_skills, &id, &key, value)?
                } else {
                    update_entry(&mut definitions.passive_skills, &id, &key, value)?
                }
            }
            DefinitionCategory::TerrainTypes => {
                update_entry(&mut definitions.terrain_types, &id, &key, value)?
            }
        },
        DefinitionEdit::ChangeSkillEffect { id, effect } => {
            let active = position(&definitions.active_skills, &id);
            let passive = position(&definitions.passive_skills, &id);
            if active.is_none() && passive.is_none() {
                return Err(EditorError::operation("not_found", &id, Vec::new()));
            }
            // 主被動互換時移到另一組清單末端；主動技能之間切換保留射程。
            let active_effect = match effect {
                EffectSelection::Attack => Some(ActiveSkillEffect::Attack {
                    power_source: crate::PowerSource::Physical,
                    ranged: false,
                    hit_bonus: 0,
                    power_bonus: 0,
                }),
                EffectSelection::Sweep => Some(ActiveSkillEffect::Sweep),
                EffectSelection::MoveThenAttack => Some(ActiveSkillEffect::MoveThenAttack),
                EffectSelection::Loot => Some(ActiveSkillEffect::Loot),
                EffectSelection::Push => Some(ActiveSkillEffect::Push {
                    ranged: false,
                    hit_bonus: 0,
                }),
                EffectSelection::Mire { terrain } => Some(ActiveSkillEffect::Mire {
                    terrain,
                    duration: crate::model::default_duration(),
                }),
                EffectSelection::Heal => Some(ActiveSkillEffect::Heal {
                    power_source: crate::PowerSource::Magical,
                    power_bonus: 0,
                }),
                EffectSelection::Flanking => None,
            };
            match (active_effect, active, passive) {
                (Some(effect), Some(index), _) => definitions.active_skills[index].effect = effect,
                (Some(effect), None, _) => {
                    remove_entry(&mut definitions.passive_skills, &id);
                    definitions.active_skills.push(ActiveSkillDef {
                        id,
                        min_range: crate::model::default_one(),
                        max_range: crate::model::default_one(),
                        effect,
                    });
                }
                (None, _, Some(index)) => {
                    definitions.passive_skills[index].effect =
                        PassiveSkillEffect::Flanking { hit_bonus: 0 }
                }
                (None, _, None) => {
                    remove_entry(&mut definitions.active_skills, &id);
                    definitions.passive_skills.push(PassiveSkillDef {
                        id,
                        effect: PassiveSkillEffect::Flanking { hit_bonus: 0 },
                    });
                }
            }
        }
        DefinitionEdit::Add { category, id } => {
            validate_new_id(&definitions, category, &id)?;
            match category {
                DefinitionCategory::Equipment => definitions.equipment.push(EquipmentDef {
                    id,
                    slot: EquipmentSlot::default(),
                    hp: 0,
                    physical_power: 0,
                    magical_power: 0,
                    block: 0,
                    block_reduction: 0,
                }),
                DefinitionCategory::UnitTypes => definitions.unit_types.push(UnitType {
                    ai_profile: definitions
                        .ai_profiles
                        .first()
                        .ok_or_else(|| {
                            EditorError::operation(
                                "invalid_command",
                                "新增單位前必須先定義 AI 傾向",
                                Vec::new(),
                            )
                        })?
                        .id
                        .clone(),
                    id,
                    visual: String::new(),
                    width: crate::model::default_one(),
                    height: crate::model::default_one(),
                    hp: crate::model::default_one(),
                    movement: 0,
                    initiative: 0,
                    dodge: 0,
                    hit: 0,
                    physical_power: 0,
                    magical_power: 0,
                    main_hand: String::new(),
                    off_hand: String::new(),
                    armor: String::new(),
                    accessory: String::new(),
                    active_skills: Vec::new(),
                    passive_skills: Vec::new(),
                }),
                DefinitionCategory::Skills => definitions.active_skills.push(ActiveSkillDef {
                    id,
                    min_range: crate::model::default_one(),
                    max_range: crate::model::default_one(),
                    effect: ActiveSkillEffect::Attack {
                        power_source: crate::PowerSource::default(),
                        ranged: false,
                        hit_bonus: 0,
                        power_bonus: 0,
                    },
                }),
                DefinitionCategory::TerrainTypes => {
                    definitions.terrain_types.push(TerrainTypeDef {
                        blocks_sight: false,
                        id,
                        layer: TerrainLayer::default(),
                        entry_rule: TerrainEntryRule::default(),
                        damage: 0,
                        extra_movement_cost: 0,
                        dodge_penalty: 0,
                        block_penalty: 0,
                    });
                }
            }
        }
        DefinitionEdit::Duplicate {
            category,
            id,
            source_id,
        } => {
            validate_new_id(&definitions, category, &id)?;
            // ID 建立後不可更動；複製只設定新資料的 ID，因此不需要更新既有引用。
            match category {
                DefinitionCategory::Equipment => {
                    duplicate_entry(&mut definitions.equipment, id, &source_id)?
                }
                DefinitionCategory::UnitTypes => {
                    duplicate_entry(&mut definitions.unit_types, id, &source_id)?
                }
                DefinitionCategory::Skills => {
                    if position(&definitions.active_skills, &source_id).is_some() {
                        duplicate_entry(&mut definitions.active_skills, id, &source_id)?
                    } else {
                        duplicate_entry(&mut definitions.passive_skills, id, &source_id)?
                    }
                }
                DefinitionCategory::TerrainTypes => {
                    duplicate_entry(&mut definitions.terrain_types, id, &source_id)?
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
                DefinitionCategory::Equipment => remove_entry(&mut definitions.equipment, &id),
                DefinitionCategory::UnitTypes => remove_entry(&mut definitions.unit_types, &id),
                DefinitionCategory::Skills => {
                    remove_entry(&mut definitions.active_skills, &id);
                    remove_entry(&mut definitions.passive_skills, &id);
                }
                DefinitionCategory::TerrainTypes => {
                    remove_entry(&mut definitions.terrain_types, &id)
                }
            }
        }
    }

    if maps.is_empty() {
        return Err(EditorError::operation("missing_map", "", Vec::new()));
    }
    // 定義只準備一次；所有地圖共用同一份結果，錯誤仍標示對應地圖。
    let prepared =
        validation::prepare_definitions(definitions.clone()).map_err(|error| EditorError::Map {
            path: maps.keys().next().expect("已確認至少有一張地圖").clone(),
            error,
        })?;
    for (path, map) in &maps {
        validation::validate_map_with_definitions(&prepared, map).map_err(|error| {
            EditorError::Map {
                path: path.clone(),
                error,
            }
        })?;
    }
    serde_json::to_string(&EditedDocuments { definitions })
        .map_err(|e| EditorError::conversion("json_serialize", e.to_string()))
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
        DefinitionCategory::Equipment => {
            for unit in &definitions.unit_types {
                if crate::equipment::equipment_slots(unit)
                    .iter()
                    .any(|(_, _, entry)| *entry == id)
                {
                    references.push(("unit_types", unit.id.clone()));
                }
            }
        }
        DefinitionCategory::UnitTypes => {
            for (path, map) in maps {
                if map.units.iter().any(|unit| unit.unit_type == id) {
                    references.push(("maps", path.clone()));
                }
            }
        }
        DefinitionCategory::Skills => {
            if definitions
                .default_passive_skills
                .iter()
                .any(|skill| skill == id)
            {
                references.push(("defaults", "default_passive_skills".into()));
            }
            for unit in &definitions.unit_types {
                if unit
                    .active_skills
                    .iter()
                    .chain(unit.passive_skills.iter())
                    .any(|skill| skill == id)
                {
                    references.push(("unit_types", unit.id.clone()));
                }
            }
        }
        DefinitionCategory::TerrainTypes => {
            for skill in &definitions.active_skills {
                if matches!(&skill.effect, ActiveSkillEffect::Mire { terrain, duration: _ } if terrain == id)
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
        DefinitionCategory::Equipment => position(&definitions.equipment, id).is_some(),
        DefinitionCategory::UnitTypes => position(&definitions.unit_types, id).is_some(),
        DefinitionCategory::Skills => {
            position(&definitions.active_skills, id).is_some()
                || position(&definitions.passive_skills, id).is_some()
        }
        DefinitionCategory::TerrainTypes => position(&definitions.terrain_types, id).is_some(),
    }
}

/// 編輯器以 ID 操作的作者資料。
trait Identified: Clone {
    fn id(&self) -> &str;
    fn set_id(&mut self, id: String);
}

impl Identified for EquipmentDef {
    fn id(&self) -> &str {
        &self.id
    }
    fn set_id(&mut self, id: String) {
        self.id = id;
    }
}

impl Identified for UnitType {
    fn id(&self) -> &str {
        &self.id
    }
    fn set_id(&mut self, id: String) {
        self.id = id;
    }
}

impl Identified for ActiveSkillDef {
    fn id(&self) -> &str {
        &self.id
    }
    fn set_id(&mut self, id: String) {
        self.id = id;
    }
}

impl Identified for PassiveSkillDef {
    fn id(&self) -> &str {
        &self.id
    }
    fn set_id(&mut self, id: String) {
        self.id = id;
    }
}

impl Identified for TerrainTypeDef {
    fn id(&self) -> &str {
        &self.id
    }
    fn set_id(&mut self, id: String) {
        self.id = id;
    }
}

/// 編輯器的魔法分組集中在此，供清單顯示與拖曳共用。
fn is_magical_effect(effect: &ActiveSkillEffect) -> bool {
    match effect {
        ActiveSkillEffect::Attack { power_source, .. }
        | ActiveSkillEffect::Heal { power_source, .. } => {
            *power_source == crate::PowerSource::Magical
        }
        ActiveSkillEffect::Mire { .. } => true,
        ActiveSkillEffect::Sweep
        | ActiveSkillEffect::Loot
        | ActiveSkillEffect::MoveThenAttack
        | ActiveSkillEffect::Push { .. } => false,
    }
}

fn position<T: Identified>(entries: &[T], id: &str) -> Option<usize> {
    entries.iter().position(|entry| entry.id() == id)
}

fn existing_position<T: Identified>(entries: &[T], id: &str) -> Result<usize, EditorError> {
    position(entries, id).ok_or_else(|| EditorError::operation("not_found", id, Vec::new()))
}

/// 只能移動到 same_group 判定為同組的位置。
fn move_entry<T: Identified>(
    entries: &mut Vec<T>,
    id: &str,
    offset: i32,
    same_group: impl Fn(&T, &T) -> bool,
) -> Result<(), EditorError> {
    let invalid = || EditorError::operation("invalid_command", id, Vec::new());
    if offset == 0 {
        return Err(invalid());
    }
    let index = existing_position(entries, id)?;
    let target = usize::try_from(index as i64 + i64::from(offset))
        .ok()
        .filter(|target| *target < entries.len())
        .ok_or_else(invalid)?;
    if !same_group(&entries[index], &entries[target]) {
        return Err(invalid());
    }
    let entry = entries.remove(index);
    entries.insert(target, entry);
    Ok(())
}

fn update_entry<T: Identified + Serialize + serde::de::DeserializeOwned>(
    entries: &mut [T],
    id: &str,
    key: &str,
    value: serde_json::Value,
) -> Result<(), EditorError> {
    let index = existing_position(entries, id)?;
    update_field(&mut entries[index], key, value)
}

/// 複本插在來源之後。
fn duplicate_entry<T: Identified>(
    entries: &mut Vec<T>,
    id: String,
    source_id: &str,
) -> Result<(), EditorError> {
    let index = existing_position(entries, source_id)?;
    let mut entry = entries[index].clone();
    entry.set_id(id);
    entries.insert(index + 1, entry);
    Ok(())
}

fn remove_entry<T: Identified>(entries: &mut Vec<T>, id: &str) {
    entries.retain(|entry| entry.id() != id);
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
        .map_err(|e| EditorError::conversion("json_serialize", e.to_string()))?;
    if key == "id" || key == "effect" || document.get(key).is_none() {
        return Err(EditorError::operation("invalid_field", key, Vec::new()));
    }
    document[key] = value;
    *entry = super::json::from_value(document)
        .map_err(|e| EditorError::conversion("definitions_json_parse", e.to_string()))?;
    Ok(())
}

/// 編輯器只呈現核心決定的合法配裝選項，不在 Godot 重建手數與欄位規則。
fn equipment_choices(
    unit: &UnitType,
    entries: &std::collections::HashMap<String, EquipmentDef>,
) -> BTreeMap<&'static str, Vec<String>> {
    let mut candidates: Vec<_> = entries.keys().cloned().collect();
    candidates.sort();
    candidates.insert(0, String::new());
    let slots = crate::equipment::equipment_slots(unit);
    slots
        .iter()
        .enumerate()
        .map(|(index, (key, _, _))| {
            let choices = candidates
                .iter()
                .filter(|id| {
                    let mut candidate = slots;
                    candidate[index].2 = id;
                    crate::equipment::resolve_slots(unit, candidate, entries).is_ok()
                })
                .cloned()
                .collect();
            (*key, choices)
        })
        .collect()
}

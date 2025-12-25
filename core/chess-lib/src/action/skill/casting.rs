//! 施法流程控制模組
//!
//! 負責技能施放的流程控制，包括：
//! - 施法前提驗證
//! - MP 消耗
//! - 狀態效果檢查
//! - 效果應用協調

use crate::*;
use skills_lib::*;
use std::collections::BTreeMap;

use super::effect_application::{apply_effect_to_pos, apply_effects_to_empty_tile};
use super::hit_resolution::{AttackResult, SaveResult, calc_hit_result, calc_save_result};
use super::targeting::{is_able_to_cast, is_targeting_valid_target};

/// 驗證施法前提條件：施法者存在、能施法、技能選擇、目標有效
/// 返回技能 ID（克隆）
pub(super) fn validate_skill_casting(
    board: &Board,
    skills: &BTreeMap<SkillID, Skill>,
    caster: UnitID,
    selected_skill: &Option<SkillID>,
    target: Pos,
) -> Result<SkillID, Error> {
    let func = "validate_skill_casting";

    // 施放前必須找到 unit，否則不能施放技能
    let unit = board
        .units
        .get(&caster)
        .ok_or_else(|| Error::NoActingUnit {
            func,
            unit_id: caster,
        })?;
    is_able_to_cast(unit).map_err(|e| Error::Wrap {
        func,
        source: Box::new(e),
    })?;

    let skill_id = selected_skill
        .as_ref()
        .ok_or_else(|| Error::NoSkillSelected { func })?;
    let skill = skills.get(skill_id).ok_or_else(|| Error::SkillNotFound {
        func,
        skill_id: skill_id.clone(),
    })?;

    // 只判斷第一個 effect 的 target_type
    is_targeting_valid_target(board, skill_id, skill, caster, target).or_else(|err| {
        Err(Error::Wrap {
            func,
            source: Box::new(err),
        })
    })?;

    Ok(skill_id.clone())
}

/// 魔力消耗檢查與扣除
pub(super) fn consume_skill_mp(
    board: &mut Board,
    caster: UnitID,
    skill_id: &SkillID,
    skill: &Skill,
) -> Result<(), Error> {
    let func = "consume_skill_mp";

    // 魔力消耗檢查與扣除
    if skill.cost < 0 {
        let unit = match board.units.get_mut(&caster) {
            Some(unit) => unit,
            None => {
                return Err(Error::NoActingUnit {
                    func,
                    unit_id: caster,
                });
            }
        };
        let mp = unit.mp + skill.cost;
        if mp < 0 {
            return Err(Error::NotEnoughMp {
                func,
                unit_type: unit.unit_template_type.clone(),
                skill_id: skill_id.clone(),
                mp: unit.mp,
                cost: skill.cost,
            });
        }
        unit.mp = mp;
    }

    Ok(())
}

/// 檢查施法者的狀態效果是否阻止技能施放（例如 Silence 阻止 Magical 技能）
pub(super) fn check_status_effects_block_skill(
    board: &Board,
    caster: UnitID,
    skill_id: &SkillID,
    skill: &Skill,
) -> Result<(), Error> {
    let func = "check_status_effects_block_skill";

    // 檢查施法者是否被 Silence（阻止 Magical 技能）
    let caster_unit = board
        .units
        .get(&caster)
        .ok_or_else(|| Error::NoActingUnit {
            func,
            unit_id: caster,
        })?;
    for effect in &caster_unit.status_effects {
        if let Effect::Silence { .. } = effect {
            if skill.tags.contains(&Tag::Magical) {
                return Err(Error::StatusEffectBlocksSkill {
                    func,
                    effect: effect.clone(),
                    skill_id: skill_id.clone(),
                });
            }
        }
    }

    Ok(())
}

/// 應用技能效果到影響區域
/// 根據技能是否有命中數值，採用不同的應用邏輯
pub(super) fn apply_skill_to_area(
    board: &mut Board,
    skills: &BTreeMap<SkillID, Skill>,
    caster: UnitID,
    caster_pos: Pos,
    skill_id: &SkillID,
    skill: &Skill,
    affect_area: Vec<Pos>,
    target: Pos,
) -> Result<Vec<String>, Error> {
    let func = "apply_skill_to_area";

    let mut msgs = vec![format!("{} 在 ({}, {}) 施放", skill_id, target.x, target.y)];

    // 命中機制最終設計摘要如下：
    // 1. 技能命中數值（accuracy）僅計算一次，並套用於所有目標（僅計算命中數值，不進行閃避或格擋判定）。
    // 2. 檢查每個目標是否為技能效果的合法套用對象（敵軍、友軍、自己等）。
    // 3. 僅對符合效果目標的單位，進行閃避與格擋的判定。
    // 4. 若目標被「閃避」則不套用效果；若為「命中」或「格擋」則都會套用效果，但格擋可影響效果強度（如減傷）。
    // 5. 命中結果（命中、閃避、格擋）皆可顯示訊息。
    match skill.accuracy {
        None => {
            // 無命中數值，所有格子直接套用效果（無爆擊）
            for pos in affect_area {
                for effect in &skill.effects {
                    // 只對有單位的目標進行豁免判定
                    let save_result = match board.pos_to_unit(pos) {
                        Some(target_id) => {
                            calc_save_result(board, skills, caster, target_id, skill, effect)
                                .map_err(|e| Error::Wrap {
                                    func,
                                    source: Box::new(e),
                                })?
                        }
                        None => SaveResult::NoSave,
                    };
                    if let Some(msg) = apply_effect_to_pos(
                        board,
                        effect,
                        caster_pos,
                        pos,
                        AttackResult::NoAttack,
                        save_result,
                    ) {
                        msgs.push(msg);
                    }
                }
            }
        }
        Some(skill_accuracy) => {
            // 預先計算施法者的 accuracy 加成
            let caster_unit = board
                .units
                .get(&caster)
                .ok_or_else(|| Error::NoActingUnit {
                    func,
                    unit_id: caster,
                })?;

            let caster_accuracy = unit::skills_to_accuracy(caster_unit.skills.iter(), skills)
                .map_err(|e| Error::Wrap {
                    func,
                    source: Box::new(e),
                })?;

            // 將技能 accuracy 與施法者 accuracy 加總
            let total_accuracy = skill_accuracy + caster_accuracy;

            msgs.extend(calc_hit_result(
                board,
                (caster, caster_pos),
                skills,
                skill,
                affect_area,
                total_accuracy,
            )?);
        }
    }

    Ok(msgs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, HashMap};

    fn prepare_test_board(
        pos: Pos,
        extra_unit_pos: Option<Vec<Pos>>,
    ) -> (Board, UnitID, BTreeMap<SkillID, Skill>) {
        let data = include_str!("../../../tests/unit.json");
        let v: serde_json::Value = serde_json::from_str(data).unwrap();
        let template: UnitTemplate = serde_json::from_value(v["UnitTemplate"].clone()).unwrap();
        let marker: UnitMarker = serde_json::from_value(v["UnitMarker"].clone()).unwrap();
        let team: Team = serde_json::from_value(v["Team"].clone()).unwrap();
        let teams = HashMap::from([(team.id.clone(), team.clone())]);
        let skills = {
            let slash_data = include_str!("../../../tests/skill_slash.json");
            let slash_skill: Skill = serde_json::from_str(slash_data).unwrap();
            let shoot_data = include_str!("../../../tests/skill_shoot.json");
            let shoot_skill: Skill = serde_json::from_str(shoot_data).unwrap();
            let splash_data = include_str!("../../../tests/skill_splash.json");
            let splash_skill: Skill = serde_json::from_str(splash_data).unwrap();
            BTreeMap::from([
                ("shoot".to_string(), shoot_skill),
                ("slash".to_string(), slash_skill),
                ("splash".to_string(), splash_skill),
            ])
        };
        let template = {
            let mut template = template;
            template.skills = skills.iter().map(|(id, _)| id.clone()).collect();
            template
        };
        let unit = Unit::from_template(&marker, &template, &skills).unwrap();
        let unit_id = unit.id;

        let mut unit_map = UnitMap::default();
        unit_map.insert(unit_id, pos);
        let mut units = HashMap::from([(unit_id, unit)]);

        if let Some(pos_list) = extra_unit_pos {
            let mut next_id = unit_id;
            for p in pos_list {
                next_id += 1;
                let extra_template = template.clone();
                let mut extra_unit =
                    Unit::from_template(&marker, &extra_template, &skills).unwrap();
                extra_unit.id = next_id;
                unit_map.insert(extra_unit.id, p);
                units.insert(extra_unit.id, extra_unit);
            }
        }

        let board = Board {
            tiles: vec![vec![Tile::default(); 10]; 10],
            teams,
            unit_map,
            units,
        };
        (board, unit_id, skills)
    }

    #[test]
    fn test_consume_skill_mp_sufficient() {
        let (mut board, unit_id, skills) = prepare_test_board(Pos { x: 1, y: 1 }, None);

        // 設置 MP
        board.units.get_mut(&unit_id).unwrap().mp = 50;

        // 創建消耗 MP 的技能
        let skill = Skill {
            tags: BTreeSet::new(),
            range: (0, 1),
            cost: -10, // 消耗 10 MP
            accuracy: None,
            crit_rate: None,
            effects: vec![],
        };

        let result = consume_skill_mp(&mut board, unit_id, &"test".to_string(), &skill);
        assert!(result.is_ok());
        assert_eq!(board.units.get(&unit_id).unwrap().mp, 40);
    }

    #[test]
    fn test_consume_skill_mp_insufficient() {
        let (mut board, unit_id, skills) = prepare_test_board(Pos { x: 1, y: 1 }, None);

        // 設置 MP 不足
        board.units.get_mut(&unit_id).unwrap().mp = 5;

        let skill = Skill {
            tags: BTreeSet::new(),
            range: (0, 1),
            cost: -10,
            accuracy: None,
            crit_rate: None,
            effects: vec![],
        };

        let result = consume_skill_mp(&mut board, unit_id, &"test".to_string(), &skill);
        assert!(matches!(result, Err(Error::NotEnoughMp { .. })));
        // MP 不應該被扣除
        assert_eq!(board.units.get(&unit_id).unwrap().mp, 5);
    }

    #[test]
    fn test_check_silence_blocks_magical_skill() {
        let (mut board, unit_id, skills) = prepare_test_board(Pos { x: 1, y: 1 }, None);

        // 添加 Silence 狀態
        board
            .units
            .get_mut(&unit_id)
            .unwrap()
            .status_effects
            .push(Effect::Silence {
                target_type: TargetType::Enemy,
                shape: Shape::Point,
                duration: 2,
                save_type: SaveType::Will,
            });

        // 創建 Magical 技能
        let skill = Skill {
            tags: BTreeSet::from([Tag::Magical]),
            range: (1, 1),
            cost: 0,
            accuracy: Some(100),
            crit_rate: None,
            effects: vec![],
        };

        let result =
            check_status_effects_block_skill(&board, unit_id, &"magic_skill".to_string(), &skill);
        assert!(matches!(result, Err(Error::StatusEffectBlocksSkill { .. })));
    }

    #[test]
    fn test_silence_allows_physical_skill() {
        let (mut board, unit_id, skills) = prepare_test_board(Pos { x: 1, y: 1 }, None);

        // 添加 Silence 狀態
        board
            .units
            .get_mut(&unit_id)
            .unwrap()
            .status_effects
            .push(Effect::Silence {
                target_type: TargetType::Enemy,
                shape: Shape::Point,
                duration: 2,
                save_type: SaveType::Will,
            });

        // 創建 Physical 技能（沒有 Magical tag）
        let skill = Skill {
            tags: BTreeSet::from([Tag::Physical]),
            range: (1, 1),
            cost: 0,
            accuracy: Some(100),
            crit_rate: None,
            effects: vec![],
        };

        let result = check_status_effects_block_skill(
            &board,
            unit_id,
            &"physical_skill".to_string(),
            &skill,
        );
        assert!(result.is_ok());
    }
}

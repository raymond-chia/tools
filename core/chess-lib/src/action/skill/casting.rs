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

use super::effect_application::apply_effect_to_pos;
use super::hit_resolution::{AttackResult, SaveResult, calc_hit_result, calc_save_result};
use super::targeting::{
    calc_shape_area, is_able_to_act, is_in_skill_range_manhattan, is_targeting_valid_target,
};

/// 取得施法者位置
///
/// # 參數
/// - `board`: 棋盤狀態
/// - `caster_id`: 施法者 ID
///
/// # 返回值
/// - Ok(Pos): 施法者位置
/// - Err(Error): 施法者不在棋盤上
pub(in crate::action) fn get_caster_pos(board: &Board, caster_id: UnitID) -> Result<Pos, Error> {
    let func = "get_caster_pos";

    board.unit_to_pos(caster_id).ok_or(Error::NoActingUnit {
        func,
        unit_id: caster_id,
    })
}

/// 計算技能影響區域（含範圍檢查與視線檢查）
///
/// # 參數
/// - `board`: 棋盤狀態
/// - `skills`: 技能資料表（用於檢查 Sense 能力）
/// - `caster_id`: 施法者 ID
/// - `skill_id`: 技能 ID（用於錯誤訊息）
/// - `skill`: 技能引用
/// - `caster_pos`: 施法者位置
/// - `target_pos`: 目標位置
///
/// # 返回值
/// - Ok(Vec<Pos>): 影響區域座標列表
/// - Err(Error): 目標超出範圍、視線被阻擋或影響區域為空
pub(in crate::action) fn calc_skill_affect_area(
    board: &Board,
    skills: &BTreeMap<SkillID, Skill>,
    skill_id: &SkillID,
    skill: &Skill,
    (caster_id, caster_pos): (UnitID, Pos),
    // logical_pos 不管是否重疊
    (actual_pos, logical_pos): (Pos, Pos),
) -> Result<Vec<Pos>, Error> {
    let func = "calc_skill_affect_area";

    // 判斷 logical_pos 是否在技能 range 內
    if !is_in_skill_range_manhattan(skill.range, caster_pos, logical_pos) {
        return Err(Error::SkillOutOfRange {
            func,
            skill_id: skill_id.clone(),
            caster_pos,
            target_pos: logical_pos,
            range: skill.range,
        });
    }

    // 檢查視線：是否能看到目標位置
    match board.can_see_target((caster_id, caster_pos), logical_pos, skills) {
        Ok(true) => {} // 可以看到，繼續
        Ok(false) => {
            return Err(Error::SkillOutOfRange {
                func,
                skill_id: skill_id.clone(),
                caster_pos,
                target_pos: logical_pos,
                range: skill.range,
            });
        }
        Err(e) => return Err(e).wrap_context(func),
    }

    // 取得技能範圍形狀（僅取第一個 effect 的 shape）
    let shape = skill
        .effects
        .first()
        .ok_or_else(|| Error::InvalidSkill {
            func,
            skill_id: skill_id.clone(),
        })?
        .shape();

    // 計算範圍
    let affect_area = calc_shape_area(board, shape, caster_pos, actual_pos);

    if affect_area.is_empty() {
        return Err(Error::SkillAffectEmpty {
            func,
            skill_id: skill_id.clone(),
            pos: actual_pos,
        });
    }

    Ok(affect_area)
}

/// 施放技能核心邏輯（供 execute_action 和 execute_reaction 共用）
///
/// # 參數
/// - `board`: 棋盤狀態
/// - `skills`: 技能資料表
/// - `caster_id`: 施法者 ID
/// - `skill_id`: 技能 ID（已驗證）
/// - `target_pos`: 目標位置
///
/// # 返回值
/// - Ok(Vec<String>): 技能效果訊息
/// - Err(Error): 施放失敗
///
/// # 流程
/// 1. 取得技能引用
/// 2. 取得施法者位置
/// 3. 計算影響區域（含範圍檢查）
/// 4. 消耗 MP
/// 5. 應用技能效果
///
/// # 注意
/// 此函數不包含資源消耗（action/reaction），由調用者負責
pub(in crate::action) fn cast_skill_internal(
    board: &mut Board,
    battle: &mut Battle,
    skills: &BTreeMap<SkillID, Skill>,
    caster_id: UnitID,
    skill_id: &SkillID,
    (actual_pos, logical_pos): (Pos, Pos),
) -> Result<Vec<String>, Error> {
    let func = "cast_skill_internal";

    // 1. 取得技能引用
    let skill = skills.get(skill_id).ok_or_else(|| Error::SkillNotFound {
        func,
        skill_id: skill_id.clone(),
    })?;

    // 2. 取得施法者位置
    let caster_pos = get_caster_pos(board, caster_id).wrap_context(func)?;

    // 3. 計算影響區域（含範圍檢查與視線檢查）
    let affect_area = calc_skill_affect_area(
        board,
        skills,
        skill_id,
        skill,
        (caster_id, caster_pos),
        (actual_pos, logical_pos),
    )
    .wrap_context(func)?;

    // 4. 消耗 MP
    consume_skill_mp(board, caster_id, skill_id, skill).wrap_context(func)?;

    // 5. 應用技能效果
    let msgs = apply_skill_to_area(
        board,
        battle,
        skills,
        caster_id,
        caster_pos,
        skill_id,
        skill,
        affect_area,
        actual_pos,
    )
    .wrap_context(func)?;

    Ok(msgs)
}

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
    let unit = board.units.get(&caster).ok_or(Error::NoActingUnit {
        func,
        unit_id: caster,
    })?;
    is_able_to_act(unit).wrap_context(func)?;

    let skill_id = selected_skill
        .as_ref()
        .ok_or(Error::NoSkillSelected { func })?;
    let skill = skills.get(skill_id).ok_or(Error::SkillNotFound {
        func,
        skill_id: skill_id.clone(),
    })?;

    // 只判斷第一個 effect 的 target_type
    is_targeting_valid_target(board, skill_id, skill, caster, target).wrap_context(func)?;

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
        let unit = board.units.get_mut(&caster).ok_or(Error::NoActingUnit {
            func,
            unit_id: caster,
        })?;

        if unit.mp + skill.cost < 0 {
            return Err(Error::NotEnoughMp {
                func,
                unit_type: unit.unit_template_type.clone(),
                skill_id: skill_id.clone(),
                mp: unit.mp,
                cost: skill.cost,
            });
        }
        unit.mp += skill.cost;
    }

    Ok(())
}

/// 應用技能效果到影響區域
/// 根據技能是否有命中數值，採用不同的應用邏輯
pub(super) fn apply_skill_to_area(
    board: &mut Board,
    battle: &mut Battle,
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

    // 命中機制設計：
    // - 無 accuracy：所有目標直接套用效果（可進行豁免判定）
    // - 有 accuracy：計算命中數值後，對每個目標進行閃避/格擋判定
    //   格擋時仍會套用效果，但可減免傷害；閃避時不套用效果
    match skill.accuracy {
        None => {
            // 無命中數值，所有格子直接套用效果（無爆擊）
            for &pos in &affect_area {
                for effect in &skill.effects {
                    // 只對有單位的目標進行豁免判定
                    let save_result = match board.pos_to_unit(pos) {
                        Some(target_id) => {
                            calc_save_result(board, skills, caster, target_id, skill, effect)
                                .wrap_context(func)?
                        }
                        None => SaveResult::NoSave,
                    };
                    if let Some(msg) = apply_effect_to_pos(
                        board,
                        battle,
                        (caster, caster_pos),
                        pos,
                        effect,
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
            let caster_unit = board.units.get(&caster).ok_or(Error::NoActingUnit {
                func,
                unit_id: caster,
            })?;

            let caster_accuracy =
                unit::skills_to_accuracy(caster_unit.skills.iter(), skills).wrap_context(func)?;

            let total_accuracy = skill_accuracy + caster_accuracy;

            msgs.extend(
                calc_hit_result(
                    board,
                    battle,
                    (caster, caster_pos),
                    skills,
                    skill,
                    &affect_area,
                    total_accuracy,
                )
                .wrap_context(func)?,
            );
        }
    }

    // 根據技能 Tag 執行點燃/熄滅
    if skill.tags.contains(&Tag::Ignite) {
        for &pos in &affect_area {
            let count = board.object_map.ignite_objects_at(pos);
            if count > 0 {
                msgs.push(format!("在 ({}, {}) 點燃了 {} 個物件", pos.x, pos.y, count));
            }
        }
    }
    if skill.tags.contains(&Tag::Extinguish) {
        for &pos in &affect_area {
            let count = board.object_map.extinguish_objects_at(pos);
            if count > 0 {
                msgs.push(format!("在 ({}, {}) 熄滅了 {} 個物件", pos.x, pos.y, count));
            }
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
            ambient_light: LightLevel::default(),
            object_map: ObjectMap::default(),
        };
        (board, unit_id, skills)
    }

    #[test]
    fn test_consume_skill_mp_sufficient() {
        let (mut board, unit_id, _skills) = prepare_test_board(Pos { x: 1, y: 1 }, None);

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
        let (mut board, unit_id, _skills) = prepare_test_board(Pos { x: 1, y: 1 }, None);

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
}

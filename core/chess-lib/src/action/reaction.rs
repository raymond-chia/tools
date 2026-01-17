//! action/reaction.rs：
//! - Reaction 系統核心邏輯
//! - 負責查找符合條件的 reaction 技能
//! - 檢查觸發條件與次數限制
use crate::action::movement::get_adjacent_positions;
use crate::action::skill::{cast_skill_internal, is_targeting_valid_target};
use crate::*;
use skills_lib::*;
use std::collections::{BTreeMap, BTreeSet};

/// 根據 TriggeredSkill 查找符合條件的技能
///
/// # 參數
/// - `triggered_skill`: 被觸發的技能（SkillId 或 Tag）
/// - `unit_skills`: 單位擁有的技能 ID 集合
/// - `all_skills`: 全局技能表
///
/// # 前提條件
/// - `unit_skills` 中的所有技能都必須存在於 `all_skills` 中（在 Unit::from_template 時已驗證）
///
/// # 返回值
/// - Ok(Vec<SkillID>): 符合條件的技能 ID 列表
/// - Err(Error): 單位不擁有指定的技能
pub fn find_reaction_skills(
    triggered_skill: &TriggeredSkill,
    unit_skills: &BTreeSet<String>,
    all_skills: &BTreeMap<SkillID, Skill>,
) -> Result<Vec<SkillID>, Error> {
    let func = "find_reaction_skills";

    match triggered_skill {
        TriggeredSkill::SkillId { id } => {
            // 檢查單位是否擁有該技能
            if !unit_skills.contains(id) {
                return Err(Error::SkillNotFound {
                    func,
                    skill_id: id.clone(),
                });
            }
            Ok(vec![id.clone()])
        }
        TriggeredSkill::Tag { tag } => {
            // 查找單位擁有的所有具有該 tag 的技能
            let mut matching_skills = Vec::new();
            for skill_id in unit_skills {
                let skill =
                    all_skills
                        .get(skill_id.as_str())
                        .ok_or_else(|| Error::SkillNotFound {
                            func,
                            skill_id: skill_id.clone(),
                        })?;

                if skill.tags.contains(tag) {
                    matching_skills.push(skill_id.clone());
                }
            }

            // 如果沒有找到任何符合 tag 的技能，返回錯誤
            if matching_skills.is_empty() {
                return Err(Error::SkillNotFound {
                    func,
                    skill_id: format!("tag:{:?}", tag),
                });
            }

            Ok(matching_skills)
        }
    }
}

/// 檢查單位是否可以觸發 reaction
///
/// # 參數
/// - `unit`: 要檢查的單位
///
/// # 返回值
/// - Ok(()): 可以觸發（次數未用盡）
/// - Err(Error): 不可觸發（次數已用盡）
pub fn is_able_to_react(unit: &Unit) -> Result<(), Error> {
    let func = "is_able_to_react";

    if unit.reactions_used_this_turn >= unit.max_reactions_per_turn {
        return Err(Error::NotEnoughAP { func });
    }
    Ok(())
}

/// 消耗一次 reaction 次數
///
/// # 參數
/// - `unit`: 要消耗次數的單位
///
/// # 返回值
/// - Ok(()): 成功消耗
/// - Err(Error): 沒有可用的 reaction 次數
///
/// # 使用場景
/// 在執行 reaction 技能後調用此函數來消耗次數
pub fn consume_reaction(unit: &mut Unit) -> Result<(), Error> {
    let func = "consume_reaction";

    is_able_to_react(unit).wrap_context(func)?;

    unit.reactions_used_this_turn += 1;
    Ok(())
}

/// 簡化的 reaction 資訊（單一單位）
#[derive(Debug, Clone, PartialEq)]
pub struct ReactionInfo {
    pub triggered_skill: TriggeredSkill, // 被觸發的技能來源
    pub available_skills: Vec<SkillID>,  // 可用的技能列表
}

/// 完整的 pending reaction（包含觸發者資訊）
#[derive(Debug, Clone, PartialEq)]
pub struct PendingReaction {
    pub reactor_id: UnitID,       // 觸發 reaction 的單位 ID
    pub reactor_pos: Pos,         // 觸發 reaction 的單位位置
    pub trigger: ReactionTrigger, // 觸發條件類型
    pub info: ReactionInfo,       // reaction 詳細資訊
}

/// 檢查單一單位的 reactions（公開供測試使用）
///
/// # 參數
/// - `unit`: 要檢測的單位
/// - `trigger_type`: 觸發條件類型（OnMove 或 OnAttacked）
/// - `all_skills`: 全局技能表
///
/// # 返回值
/// - 可觸發的 reactions 列表（可能為空）
///
/// # 實作說明
/// - 直接從 unit.skills 檢查被動技能中的 Effect::Reaction
/// - 暫不支援從 status_effects 檢查（buff 產生的臨時 Reaction）
pub fn check_unit_reactions(
    unit: &Unit,
    trigger_type: ReactionTrigger,
    all_skills: &BTreeMap<SkillID, Skill>,
) -> Result<Vec<ReactionInfo>, Error> {
    let func = "check_unit_reactions";

    // 檢查是否還有可用的 reaction 次數
    if is_able_to_react(unit).is_err() {
        return Ok(Vec::new());
    }

    let mut reactions = Vec::new();

    // 遍歷單位擁有的所有技能，檢查是否有匹配的 Reaction effect
    for skill_id in &unit.skills {
        let skill = all_skills
            .get(skill_id.as_str())
            .ok_or_else(|| Error::SkillNotFound {
                func,
                skill_id: skill_id.clone(),
            })?;

        // 檢查技能的所有效果
        for effect in &skill.effects {
            if let Effect::Reaction {
                trigger,
                triggered_skill,
                ..
            } = effect
            {
                // 只處理匹配的觸發類型
                if *trigger != trigger_type {
                    continue;
                }

                // 查找可用的技能
                let available_skills =
                    find_reaction_skills(triggered_skill, &unit.skills, all_skills)
                        .map_err(|e| match e {
                            Error::SkillNotFound { skill_id, func } => Error::SkillNotFoundInUnit {
                                func, // 保留原始 func name
                                unit_id: unit.id,
                                unit_type: unit.unit_template_type.clone(),
                                skill_id,
                            },
                            other => other,
                        })
                        .wrap_context(func)?;

                reactions.push(ReactionInfo {
                    triggered_skill: triggered_skill.clone(),
                    available_skills,
                });
            }
        }
    }

    Ok(reactions)
}

/// 檢查多個單位的 reactions（公開 API）
///
/// # 參數
/// - `units`: 要檢查的單位列表（單位引用 + 位置）
/// - `trigger_type`: 觸發條件類型（OnMove 或 OnAttacked）
/// - `all_skills`: 全局技能表
///
/// # 返回值
/// - 所有可觸發的 reactions 列表（可能為空）
pub fn check_reactions(
    units: &[(&Unit, Pos)],
    trigger_type: ReactionTrigger,
    all_skills: &BTreeMap<SkillID, Skill>,
) -> Result<Vec<PendingReaction>, Error> {
    let func = "check_reactions";
    let mut all_pending = Vec::new();

    for &(unit, pos) in units {
        // 檢查該單位的 reactions
        let reactions =
            check_unit_reactions(unit, trigger_type.clone(), all_skills).wrap_context(func)?;

        // 將 ReactionInfo 包裝成 PendingReaction
        for info in reactions {
            all_pending.push(PendingReaction {
                reactor_id: unit.id,
                reactor_pos: pos,
                trigger: trigger_type.clone(),
                info,
            });
        }
    }

    Ok(all_pending)
}

/// 檢查移動離開某個位置時觸發的 reactions（借機攻擊）
///
/// # 參數
/// - `board`: 棋盤狀態
/// - `mover`: (移動的單位, 離開的位置)
/// - `all_skills`: 全局技能表
///
/// # 返回值
/// - Ok(Vec<PendingReaction>): 被觸發的 reactions 列表（可能為空）
/// - Err(Error): 查找過程中的錯誤
///
/// # 實作說明
/// 1. 檢查離開位置相鄰（上下左右）格子的敵方單位
/// 2. 調用 check_reactions 查找有 OnMove trigger 的 reactions
pub fn check_move_reactions(
    board: &Board,
    mover: (&Unit, Pos),
    all_skills: &BTreeMap<SkillID, Skill>,
) -> Result<Vec<PendingReaction>, Error> {
    let func = "check_move_reactions";
    let (mover_unit, from_pos) = mover;

    // 收集相鄰的敵方單位
    let mut adjacent_enemies = Vec::new();
    for pos in get_adjacent_positions(from_pos) {
        if let Some(unit_id) = board.pos_to_unit(pos) {
            if let Some(unit) = board.units.get(&unit_id) {
                // 只檢查敵方單位
                if unit.team != mover_unit.team {
                    adjacent_enemies.push((unit, pos));
                }
            }
        }
    }

    // 檢查這些敵方單位的 OnMove reactions
    check_reactions(&adjacent_enemies, ReactionTrigger::OnMove, all_skills).wrap_context(func)
}

/// 執行 reaction（施放技能並消耗 reaction 次數）
///
/// # 參數
/// - `board`: 棋盤狀態
/// - `skills`: 技能資料表
/// - `reactor_id`: 觸發 reaction 的單位 ID
/// - `skill_id`: 要施放的技能 ID
/// - `target_pos`: 目標位置
///
/// # 返回值
/// - Ok(Vec<String>): 技能效果訊息
/// - Err(Error): 施放失敗
///
/// # 流程
/// 1. 驗證技能存在
/// 2. 驗證單位可以 react（檢查 reaction counter）
/// 3. 驗證目標有效（TargetType 檢查）
/// 4. 施放技能（使用共用邏輯）
/// 5. 消耗 reaction 次數
pub fn execute_reaction(
    board: &mut Board,
    battle: &mut Battle,
    skills: &BTreeMap<SkillID, Skill>,
    reactor_id: UnitID,
    skill_id: &SkillID,
    (target_actual_pos, target_logical_pos): (Pos, Pos),
) -> Result<Vec<String>, Error> {
    let func = "execute_reaction";

    // 1. 驗證技能存在
    let skill = skills.get(skill_id).ok_or_else(|| Error::SkillNotFound {
        func,
        skill_id: skill_id.clone(),
    })?;

    // 2. 驗證單位可以 react（檢查 reaction counter）
    let reactor = board.units.get(&reactor_id).ok_or(Error::NoActingUnit {
        func,
        unit_id: reactor_id,
    })?;
    is_able_to_react(reactor).wrap_context(func)?;

    // 3. 驗證目標有效（TargetType 檢查）
    is_targeting_valid_target(board, skill_id, skill, reactor_id, target_logical_pos)
        .wrap_context(func)?;

    // 4. 施放技能（共用邏輯，這是反應）
    let msgs = cast_skill_internal(
        board,
        battle,
        skills,
        reactor_id,
        skill_id,
        (target_actual_pos, target_logical_pos),
        true, // is_reaction
    )
    .wrap_context(func)?;

    // 5. 消耗 reaction 次數
    let reactor = board
        .units
        .get_mut(&reactor_id)
        .ok_or(Error::NoActingUnit {
            func,
            unit_id: reactor_id,
        })?;
    consume_reaction(reactor).wrap_context(func)?;

    Ok(msgs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_reaction_skills_by_id() {
        let mut all_skills = BTreeMap::new();
        let skill = Skill::default();
        all_skills.insert("basic_attack".to_string(), skill);

        let mut unit_skills = BTreeSet::new();
        unit_skills.insert("basic_attack".to_string());

        let source = TriggeredSkill::SkillId {
            id: "basic_attack".to_string(),
        };

        let result = find_reaction_skills(&source, &unit_skills, &all_skills).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "basic_attack");
    }

    #[test]
    fn test_find_reaction_skills_by_id_not_owned() {
        let mut all_skills = BTreeMap::new();
        let skill = Skill::default();
        all_skills.insert("basic_attack".to_string(), skill);

        let unit_skills = BTreeSet::new(); // 單位沒有該技能

        let source = TriggeredSkill::SkillId {
            id: "basic_attack".to_string(),
        };

        let result = find_reaction_skills(&source, &unit_skills, &all_skills);
        match result {
            Err(Error::SkillNotFound { skill_id, .. }) => {
                assert_eq!(skill_id, "basic_attack");
            }
            _ => panic!("應該返回 SkillNotFound 錯誤"),
        }
    }

    #[test]
    fn test_find_reaction_skills_by_tag() {
        let mut all_skills = BTreeMap::new();

        let mut skill1 = Skill::default();
        skill1.tags = vec![Tag::Physical].into_iter().collect();
        all_skills.insert("basic_attack".to_string(), skill1);

        let mut skill2 = Skill::default();
        skill2.tags = vec![Tag::Physical].into_iter().collect();
        all_skills.insert("heavy_strike".to_string(), skill2);

        let mut skill3 = Skill::default();
        skill3.tags = vec![Tag::Fire].into_iter().collect();
        all_skills.insert("fireball".to_string(), skill3);

        let mut unit_skills = BTreeSet::new();
        unit_skills.insert("basic_attack".to_string());
        unit_skills.insert("heavy_strike".to_string());
        unit_skills.insert("fireball".to_string());

        let source = TriggeredSkill::Tag { tag: Tag::Physical };

        let result = find_reaction_skills(&source, &unit_skills, &all_skills).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.contains(&"basic_attack".to_string()));
        assert!(result.contains(&"heavy_strike".to_string()));
    }

    #[test]
    fn test_find_reaction_skills_by_tag_no_match() {
        let mut all_skills = BTreeMap::new();

        let mut skill1 = Skill::default();
        skill1.tags = vec![Tag::Fire].into_iter().collect();
        all_skills.insert("fireball".to_string(), skill1);

        let mut unit_skills = BTreeSet::new();
        unit_skills.insert("fireball".to_string());

        let source = TriggeredSkill::Tag { tag: Tag::Physical };

        let result = find_reaction_skills(&source, &unit_skills, &all_skills);
        match result {
            Err(Error::SkillNotFound { skill_id, .. }) => {
                assert!(skill_id.contains("Physical"));
            }
            _ => panic!("應該返回 SkillNotFound 錯誤"),
        }
    }

    #[test]
    fn test_is_able_to_react() {
        let unit = Unit {
            id: 1,
            unit_template_type: "test".to_string(),
            team: "t1".to_string(),
            moved: 0,
            move_points: 10,
            has_cast_skill_this_turn: false,
            hp: 100,
            max_hp: 100,
            mp: 50,
            max_mp: 50,
            skills: BTreeSet::new(),
            status_effects: Vec::new(),
            max_reactions_per_turn: 1,
            reactions_used_this_turn: 0,
        };

        assert!(is_able_to_react(&unit).is_ok());
    }

    #[test]
    fn test_is_able_to_react_exhausted() {
        let unit = Unit {
            id: 1,
            unit_template_type: "test".to_string(),
            team: "t1".to_string(),
            moved: 0,
            move_points: 10,
            has_cast_skill_this_turn: false,
            hp: 100,
            max_hp: 100,
            mp: 50,
            max_mp: 50,
            skills: BTreeSet::new(),
            status_effects: Vec::new(),
            max_reactions_per_turn: 1,
            reactions_used_this_turn: 1, // 已用盡
        };

        assert!(is_able_to_react(&unit).is_err());
    }

    #[test]
    fn test_consume_reaction_success() {
        let mut unit = Unit {
            id: 1,
            unit_template_type: "test".to_string(),
            team: "t1".to_string(),
            moved: 0,
            move_points: 10,
            has_cast_skill_this_turn: false,
            hp: 100,
            max_hp: 100,
            mp: 50,
            max_mp: 50,
            skills: BTreeSet::new(),
            status_effects: Vec::new(),
            max_reactions_per_turn: 2,
            reactions_used_this_turn: 0,
        };

        // 第一次消耗
        assert!(consume_reaction(&mut unit).is_ok());
        assert_eq!(unit.reactions_used_this_turn, 1);

        // 第二次消耗
        assert!(consume_reaction(&mut unit).is_ok());
        assert_eq!(unit.reactions_used_this_turn, 2);
    }

    #[test]
    fn test_consume_reaction_exhausted() {
        let mut unit = Unit {
            id: 1,
            unit_template_type: "test".to_string(),
            team: "t1".to_string(),
            moved: 0,
            move_points: 10,
            has_cast_skill_this_turn: false,
            hp: 100,
            max_hp: 100,
            mp: 50,
            max_mp: 50,
            skills: BTreeSet::new(),
            status_effects: Vec::new(),
            max_reactions_per_turn: 1,
            reactions_used_this_turn: 1, // 已用盡
        };

        // 應該返回錯誤
        let result = consume_reaction(&mut unit);
        assert!(result.is_err());
        // 次數不應該增加
        assert_eq!(unit.reactions_used_this_turn, 1);
    }

    #[test]
    fn test_consume_reaction_until_exhausted() {
        let mut unit = Unit {
            id: 1,
            unit_template_type: "test".to_string(),
            team: "t1".to_string(),
            moved: 0,
            move_points: 10,
            has_cast_skill_this_turn: false,
            hp: 100,
            max_hp: 100,
            mp: 50,
            max_mp: 50,
            skills: BTreeSet::new(),
            status_effects: Vec::new(),
            max_reactions_per_turn: 3,
            reactions_used_this_turn: 0,
        };

        // 消耗 3 次應該成功
        for i in 0..3 {
            assert!(consume_reaction(&mut unit).is_ok());
            assert_eq!(unit.reactions_used_this_turn, i + 1);
        }

        // 第 4 次應該失敗
        assert!(consume_reaction(&mut unit).is_err());
        assert_eq!(unit.reactions_used_this_turn, 3);
    }

    // check_move_reactions 測試輔助函數
    fn create_test_unit(id: UnitID, team: &str, pos: Pos) -> (Unit, Pos) {
        let unit = Unit {
            id,
            unit_template_type: "test".to_string(),
            team: team.to_string(),
            moved: 0,
            move_points: 10,
            has_cast_skill_this_turn: false,
            hp: 100,
            max_hp: 100,
            mp: 50,
            max_mp: 50,
            skills: BTreeSet::new(),
            status_effects: Vec::new(),
            max_reactions_per_turn: 0,
            reactions_used_this_turn: 0,
        };
        (unit, pos)
    }

    fn create_test_board_for_move_reactions() -> (Board, UnitID, UnitID) {
        use std::collections::HashMap;

        // 創建 3x3 棋盤
        let tiles = vec![
            vec![
                Tile {
                    terrain: Terrain::Plain,
                };
                3
            ];
            3
        ];

        // 創建單位：玩家在 (1,1)，敵人在 (2,1)
        let mover_pos = Pos { x: 1, y: 1 };
        let enemy_pos = Pos { x: 2, y: 1 };

        let (mover, _) = create_test_unit(1, "player", mover_pos);
        let (enemy, _) = create_test_unit(2, "enemy", enemy_pos);

        let mut units = HashMap::new();
        units.insert(mover.id, mover);
        units.insert(enemy.id, enemy);

        let mut unit_map = UnitMap::default();
        unit_map.insert(1, mover_pos);
        unit_map.insert(2, enemy_pos);

        let board = Board {
            tiles,
            teams: HashMap::new(),
            units,
            unit_map,
            ambient_light: LightLevel::default(),
            object_map: ObjectMap::default(),
        };

        (board, 1, 2)
    }

    #[test]
    fn test_check_move_reactions_no_adjacent_enemies() {
        // 測試：沒有相鄰敵人時，不觸發任何 reactions
        let (board, mover_id, _) = create_test_board_for_move_reactions();
        let skills_map = BTreeMap::new();

        // 測試從棋盤角落移動（沒有相鄰敵人）
        let mover = board.units.get(&mover_id).unwrap();
        let from_pos = Pos { x: 0, y: 0 };

        let reactions = check_move_reactions(&board, (mover, from_pos), &skills_map).unwrap();
        assert_eq!(reactions.len(), 0, "沒有相鄰敵人應該不觸發 reactions");
    }

    #[test]
    fn test_check_move_reactions_with_adjacent_enemy() {
        // 測試：有相鄰敵人且敵人有 OnMove reaction 時，應觸發 reactions
        let (mut board, mover_id, enemy_id) = create_test_board_for_move_reactions();

        // 創建帶有 OnMove reaction 的技能（觸發特定技能 ID）
        let reaction_skill = Skill {
            effects: vec![Effect::Reaction {
                target_type: TargetType::Enemy,
                shape: Shape::Point,
                trigger: ReactionTrigger::OnMove,
                triggered_skill: TriggeredSkill::SkillId {
                    id: "basic_attack".to_string(),
                },
                duration: -1, // 永久效果
            }],
            ..Default::default()
        };

        let basic_attack = Skill {
            effects: vec![Effect::Hp {
                target_type: TargetType::Enemy,
                shape: Shape::Point,
                value: -10,
            }],
            ..Default::default()
        };

        let mut skills_map = BTreeMap::new();
        skills_map.insert("opportunity_attack".to_string(), reaction_skill);
        skills_map.insert("basic_attack".to_string(), basic_attack);

        // 給敵方單位添加 reaction 技能和攻擊技能
        if let Some(enemy_unit) = board.units.get_mut(&enemy_id) {
            enemy_unit.skills.insert("opportunity_attack".to_string());
            enemy_unit.skills.insert("basic_attack".to_string());
            enemy_unit.max_reactions_per_turn = 1; // 允許 1 次 reaction
            enemy_unit.reactions_used_this_turn = 0;
        }

        let mover = board.units.get(&mover_id).unwrap();
        let from_pos = Pos { x: 1, y: 1 }; // 離開 (1,1)，相鄰 (2,1) 的敵人

        let reactions = check_move_reactions(&board, (mover, from_pos), &skills_map).unwrap();

        assert_eq!(reactions.len(), 1, "相鄰敵人有 reaction 應該觸發 1 個");
        assert_eq!(reactions[0].reactor_id, enemy_id);
        assert_eq!(reactions[0].reactor_pos, Pos { x: 2, y: 1 });
        assert_eq!(reactions[0].trigger, ReactionTrigger::OnMove);
        assert_eq!(reactions[0].info.available_skills.len(), 1);
        assert!(
            reactions[0]
                .info
                .available_skills
                .contains(&"basic_attack".to_string())
        );
    }

    #[test]
    fn test_check_move_reactions_ignores_allies() {
        // 測試：相鄰的友方單位不會觸發 reactions
        use std::collections::HashMap;

        let tiles = vec![
            vec![
                Tile {
                    terrain: Terrain::Plain,
                };
                3
            ];
            3
        ];

        // 創建單位：兩個都是同隊的
        let (mover, _) = create_test_unit(1, "player", Pos { x: 1, y: 1 });
        let (ally, _) = create_test_unit(2, "player", Pos { x: 2, y: 1 }); // 同隊

        let mut units = HashMap::new();
        units.insert(mover.id, mover);
        units.insert(ally.id, ally);

        let mut unit_map = UnitMap::default();
        unit_map.insert(1, Pos { x: 1, y: 1 });
        unit_map.insert(2, Pos { x: 2, y: 1 });

        let mut board = Board {
            tiles,
            teams: HashMap::new(),
            units,
            unit_map,
            ambient_light: LightLevel::default(),
            object_map: ObjectMap::default(),
        };

        // 給友方單位添加 reaction 技能（應該被忽略）
        if let Some(ally_unit) = board.units.get_mut(&2) {
            ally_unit.skills.insert("opportunity_attack".to_string());
            ally_unit.skills.insert("basic_attack".to_string());
            ally_unit.max_reactions_per_turn = 1;
            ally_unit.reactions_used_this_turn = 0;
        }

        let skills_map = BTreeMap::new();
        let mover = board.units.get(&1).unwrap();
        let from_pos = Pos { x: 1, y: 1 };

        let reactions = check_move_reactions(&board, (mover, from_pos), &skills_map).unwrap();
        assert_eq!(reactions.len(), 0, "友方單位不應觸發 reactions");
    }

    #[test]
    fn test_check_move_reactions_enemy_without_reaction() {
        // 測試：相鄰敵人沒有 OnMove reaction 時，不觸發
        let (board, mover_id, _enemy_id) = create_test_board_for_move_reactions();
        let skills_map = BTreeMap::new(); // 沒有 reaction 技能

        let mover = board.units.get(&mover_id).unwrap();
        let from_pos = Pos { x: 1, y: 1 };

        let reactions = check_move_reactions(&board, (mover, from_pos), &skills_map).unwrap();
        assert_eq!(reactions.len(), 0, "敵人沒有 reaction 技能應該不觸發");
    }
}

//! 技能選擇與施放模組
//!
//! 提供 SkillSelection 結構，作為技能系統的主要入口點

use crate::*;
use skills_lib::*;
use std::collections::BTreeMap;

use super::casting::{calc_skill_affect_area, cast_skill_internal, validate_skill_casting};
use super::targeting::{consume_action, is_able_to_act};

/// 技能選擇資料結構
#[derive(Debug, Clone, Default)]
pub struct SkillSelection {
    /// 當前選擇的技能 ID
    pub selected_skill: Option<SkillID>,
}

impl SkillSelection {
    /// 設定目前選擇的技能
    pub fn select_skill(&mut self, skill_id: Option<SkillID>) {
        self.selected_skill = skill_id;
    }

    /// 執行 action（施放技能並消耗 action 點數）
    pub fn execute_action(
        &self,
        board: &mut Board,
        skills: &BTreeMap<SkillID, Skill>,
        caster: UnitID,
        target: Pos,
    ) -> Result<Vec<String>, Error> {
        let func = "SkillSelection::execute_action";

        // 1. 驗證施法前提條件（技能選擇 + TargetType）
        let skill_id = validate_skill_casting(board, skills, caster, &self.selected_skill, target)
            .wrap_context(func)?;

        // 2. 施放技能（共用邏輯）
        let msgs = cast_skill_internal(board, skills, caster, &skill_id, (target, target))
            .wrap_context(func)?;

        // 3. 消耗 action
        let caster = board.units.get_mut(&caster).ok_or(Error::NoActingUnit {
            func,
            unit_id: caster,
        })?;
        consume_action(caster).wrap_context(func)?;

        Ok(msgs)
    }

    /// 預覽技能範圍
    /// 根據目前選擇的技能與棋盤狀態，計算技能可作用的座標列表
    /// - board: 棋盤狀態
    /// - skills: 技能資料表
    /// - caster_pos: 施法者座標
    /// - to: 指向格
    ///
    /// 回傳：技能可作用範圍的座標 Vec<Pos>
    pub fn skill_affect_area(
        &self,
        board: &Board,
        skills: &BTreeMap<SkillID, Skill>,
        caster_pos: Pos,
        to: Pos,
    ) -> Vec<Pos> {
        // 取得技能
        let skill_id = match &self.selected_skill {
            Some(id) => id,
            None => return vec![],
        };
        let skill = match skills.get(skill_id) {
            Some(s) => s,
            None => return vec![],
        };
        // 取得單位物件，檢查是否能施法
        let caster_id = match board.pos_to_unit(caster_pos) {
            Some(id) => id,
            None => return vec![],
        };
        let caster = match board.units.get(&caster_id) {
            Some(u) => u,
            None => return vec![],
        };
        if is_able_to_act(caster).is_err() {
            return vec![];
        }

        // 使用細粒度函數計算影響區域（失敗時返回空 vec 用於 UI 預覽）
        calc_skill_affect_area(board, skill_id, skill, caster_pos, (to, to))
            .unwrap_or_else(|_| vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

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
            light_sources: Vec::new(),
        };
        (board, unit_id, skills)
    }

    #[test]
    fn test_select_skill() {
        let mut sel = SkillSelection::default();
        assert_eq!(sel.selected_skill, None);
        sel.select_skill(Some("1".to_string()));
        assert_eq!(sel.selected_skill, Some("1".to_string()));
        sel.select_skill(None);
        assert_eq!(sel.selected_skill, None);
    }

    #[test]
    fn test_execute_action_basic() {
        let (mut board, unit_id, skills) =
            prepare_test_board(Pos { x: 1, y: 1 }, Some(vec![Pos { x: 1, y: 2 }]));
        let target_unit_id = unit_id + 1;

        // 設置不同隊伍
        board.units.get_mut(&target_unit_id).unwrap().team = "enemy".to_string();

        let mut sel = SkillSelection::default();
        sel.select_skill(Some("slash".to_string()));

        let target = Pos { x: 1, y: 2 };
        let result = sel.execute_action(&mut board, &skills, unit_id, target);

        assert!(result.is_ok());
        let msgs = result.unwrap();
        assert!(msgs.iter().any(|m| m.contains("slash 在 (1, 2) 施放")));

        // 檢查 has_cast_skill_this_turn 被設置
        assert!(board.units.get(&unit_id).unwrap().has_cast_skill_this_turn);
    }

    #[test]
    fn test_execute_action_no_selection() {
        let (mut board, unit_id, skills) = prepare_test_board(Pos { x: 1, y: 1 }, None);

        let sel = SkillSelection::default();
        let target = Pos { x: 1, y: 2 };
        let result = sel.execute_action(&mut board, &skills, unit_id, target);

        // 錯誤被包裝在 Error::Wrap 中
        assert!(matches!(result, Err(Error::Wrap { .. })));
    }

    #[test]
    fn test_execute_action_twice() {
        let (mut board, unit_id, skills) =
            prepare_test_board(Pos { x: 1, y: 1 }, Some(vec![Pos { x: 1, y: 2 }]));
        let target_unit_id = unit_id + 1;
        board.units.get_mut(&target_unit_id).unwrap().team = "enemy".to_string();

        let mut sel = SkillSelection::default();
        sel.select_skill(Some("slash".to_string()));

        let target = Pos { x: 1, y: 2 };

        // 第一次施放成功
        let result1 = sel.execute_action(&mut board, &skills, unit_id, target);
        assert!(result1.is_ok());

        // 第二次施放失敗（已經施放過）
        let result2 = sel.execute_action(&mut board, &skills, unit_id, target);
        // 可能因為目標已死亡而失敗，或因為已施放過而失敗
        assert!(result2.is_err(), "Expected error, got: {:?}", result2);
    }

    #[test]
    fn test_skill_affect_area() {
        let (board, _unit_id, skills) = prepare_test_board(Pos { x: 1, y: 1 }, None);

        let mut sel = SkillSelection::default();
        sel.select_skill(Some("slash".to_string()));

        let caster_pos = Pos { x: 1, y: 1 };
        let target = Pos { x: 1, y: 2 };

        let area = sel.skill_affect_area(&board, &skills, caster_pos, target);

        // slash 是 Point shape，應該只有目標位置
        assert_eq!(area.len(), 1);
        assert_eq!(area[0], target);
    }
}

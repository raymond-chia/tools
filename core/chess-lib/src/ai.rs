use crate::*;
use skills_lib::*;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct Tendency {
    pub weights: Weights,
    pub positioning_preference: PositionPreference,
}

#[derive(Debug)]
pub struct Weights {
    pub attack: AIScore,
    pub support: AIScore,
}

#[derive(Debug)]
pub enum PositionPreference {
    Frontline,
    Flexible,
    Backline,
}

#[derive(Debug)]
pub enum Action {
    Move {
        path: Vec<Pos>,
    },
    MoveAndUseSkill {
        path: Vec<Pos>,
        skill_id: SkillID,
        target: Pos,
        target_units: Vec<UnitID>,
    },
}

#[derive(Debug)]
pub struct ScoredAction {
    pub action: Action,
    pub score: AIScore,
    pub reason: String, // for debug purpose
}

#[derive(Debug)]
pub struct Config {
    pub tendencies: BTreeMap<UnitTemplateType, Tendency>,
}

pub fn decide_action(
    board: &Board,
    skills: &BTreeMap<SkillID, Skill>,
    config: &Config,
    unit_id: UnitID,
) -> Result<ScoredAction, Error> {
    let func = "decide_action";

    let unit = board
        .units
        .get(&unit_id)
        .ok_or(Error::NoActingUnit { func, unit_id })?;
    let from = board
        .unit_to_pos(unit_id)
        .ok_or(Error::NoActingUnit { func, unit_id })?;

    // 取得 tendency
    let tendency =
        config
            .tendencies
            .get(&unit.unit_template_type)
            .ok_or(Error::MissingUnitTemplate {
                func,
                template_type: unit.unit_template_type.clone(),
            })?;

    // 取得所有可移動格與路徑
    let movable = movable_area(board, from, skills);

    let mut actions = Vec::new();
    // 產生所有 Move 行動（不移動也算一種）
    for (&to, _) in &movable {
        let path = match reconstruct_path(&movable, from, to) {
            Err(_) => continue,
            Ok(p) => p,
        };
        actions.push(Action::Move { path });
    }

    // 產生所有 MoveAndUseSkill 行動（先移動，再產生能施放的 cast_area）
    for (&new_from, &(cost, _)) in &movable {
        let path = match reconstruct_path(&movable, from, new_from) {
            Err(_) => continue,
            Ok(p) => p,
        };
        // 本回合二階段移動後，不能施放技能
        if cost + unit.moved > unit.move_points {
            continue;
        }
        for skill_id in &unit.skills {
            let skill = match skills.get(skill_id) {
                None => continue,
                Some(s) => s,
            };
            // 技能施放範圍（以 move_to 為基準）
            let cast_area = skill_casting_area(board, new_from, skill.range);
            // 只取第一個 effect 的 shape
            let shape = skill
                .effects
                .get(0)
                .ok_or(Error::InvalidSkill {
                    func,
                    skill_id: skill_id.clone(),
                })?
                .shape();
            for target in cast_area {
                // 直接用 move_to 當 from 計算範圍
                let affect_area = calc_shape_area(board, &shape, new_from, target);
                if affect_area.is_empty() {
                    continue;
                }
                // 取得技能目標單位
                let mut target_units = Vec::new();
                for p in &affect_area {
                    if let Some(uid) = board.pos_to_unit(*p) {
                        target_units.push(uid);
                    }
                }
                actions.push(Action::MoveAndUseSkill {
                    path: path.clone(),
                    skill_id: skill_id.clone(),
                    target,
                    target_units,
                });
            }
        }
    }

    // 評分所有行動
    let mut scored: Vec<ScoredAction> = actions
        .into_iter()
        .map(|action| score(tendency, action))
        .collect();

    // 按分數排序，取最高分
    scored.sort_by(|a, b| b.score.cmp(&a.score));
    let best = scored
        .into_iter()
        .next()
        .unwrap_or_else(|| score(tendency, Action::Move { path: vec![from] }));
    Ok(best)
}

use inner::*;
mod inner {
    use super::*;

    pub fn score(tendency: &Tendency, action: Action) -> ScoredAction {
        ScoredAction {
            action,
            score: 0,
            reason: String::new(),
        }
    }
}

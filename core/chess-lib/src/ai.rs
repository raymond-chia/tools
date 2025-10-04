//! ai.rs：
//! - 負責 AI 行為決策、行動評分、傾向資料結構與自動化行為流程。
//! - 不負責戰鬥流程、單位屬性計算或棋盤資料結構。
//! - 僅與 AI 決策、行動選擇、評分相關的邏輯應放於此。
use crate::*;
use serde::Deserialize;
use skills_lib::*;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct AIConfig {
    pub tendencies: BTreeMap<UnitTemplateType, Tendency>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct Tendency {
    pub weights: Weights,
    pub positioning_preference: PositionPreference,
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct Weights {
    pub attack: AIScore,
    pub support: AIScore,
    /// score_move 最近敵人距離權重
    pub nearest_enemy: AIScore,
    /// score_move 距離基準分，可 data-driven 設定
    pub distance_base: AIScore,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub enum PositionPreference {
    Frontline,
    #[default]
    Flexible,
    Backline,
}

#[derive(Debug)]
pub enum Action {
    Idle,
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

pub fn decide_action(
    board: &Board,
    skills: &BTreeMap<SkillID, Skill>,
    config: &AIConfig,
    unit_id: UnitID,
) -> Result<ScoredAction, Error> {
    let scored = score_actions(board, skills, config, unit_id)?;
    let best = scored.into_iter().next().unwrap_or_else(|| ScoredAction {
        action: Action::Idle,
        score: 0.0,
        reason: format!("no valid action"),
    });
    Ok(best)
}

pub fn score_actions(
    board: &Board,
    skills: &BTreeMap<SkillID, Skill>,
    config: &AIConfig,
    unit_id: UnitID,
) -> Result<Vec<ScoredAction>, Error> {
    let func = "score_actions";

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

    let mut actions = vec![Action::Idle];
    // 產生所有 Move 行動（不移動也算一種）
    for (&to, _) in &movable {
        if from == to {
            // 已經包含在 idle
            continue;
        }
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
    let scored: Result<Vec<ScoredAction>, Error> = actions
        .into_iter()
        .map(|action| score(board, tendency, unit_id, action))
        .collect();
    let mut scored = scored?;

    // 按分數排序，取最高分
    scored.sort_by(|a, b| b.score.total_cmp(&a.score));
    Ok(scored)
}

use inner::*;
mod inner {
    use super::*;

    pub fn score(
        board: &Board,
        tendency: &Tendency,
        unit_id: UnitID,
        action: Action,
    ) -> Result<ScoredAction, Error> {
        let func = "ai.score";

        // 取得所有敵對單位位置
        let actor_team = &board
            .units
            .get(&unit_id)
            .ok_or(Error::NoActingUnit { func, unit_id })?
            .team;
        let enemy_positions: Vec<Pos> = board
            .units
            .iter()
            .filter(|(_, u)| &u.team != actor_team)
            .filter_map(|(id, _)| board.unit_to_pos(*id))
            .collect();
        match action {
            Action::Idle => Ok(ScoredAction {
                action,
                score: 0.0,
                reason: format!("idle action"),
            }),
            Action::Move { path } => score_move(tendency, enemy_positions, path),
            Action::MoveAndUseSkill {
                path,
                skill_id,
                target,
                target_units,
            } => {
                let ally_positions: Vec<Pos> = board
                    .units
                    .iter()
                    .filter(|(_, u)| &u.team == actor_team)
                    .filter_map(|(id, _)| board.unit_to_pos(*id))
                    .collect();
                score_move_and_use_skill(
                    tendency,
                    ally_positions,
                    enemy_positions,
                    path,
                    skill_id,
                    target,
                    target_units,
                )
            }
        }
    }

    mod inner {}

    fn score_move(
        tendency: &Tendency,
        enemy_positions: Vec<Pos>,
        path: Vec<Pos>,
    ) -> Result<ScoredAction, Error> {
        let func = "ai.score_move";
        let actor_pos = *path.last().ok_or(Error::InvalidParameter { func })?;

        // 計算與最近敵人距離與平均距離
        let mut min_dist: AIScore = 99.0;
        let mut sum_dist: AIScore = 0.0;
        let mut cnt: AIScore = 0.0;
        for enemy_pos in &enemy_positions {
            let d = manhattan_distance(actor_pos, *enemy_pos) as AIScore;
            if d < min_dist {
                min_dist = d;
            }
            sum_dist += d;
            cnt += 1.0;
        }
        let avg_dist: AIScore = if cnt > 0.0 { sum_dist / cnt } else { 0.0 };

        // 位置分數根據 tendency.positioning_preference 與 weights 權重
        let w = &tendency.weights;
        let mut score: AIScore = 0.0;
        match tendency.positioning_preference {
            PositionPreference::Frontline => {
                // 越靠近敵人越高分
                score += w.nearest_enemy * (w.distance_base - min_dist);
            }
            PositionPreference::Backline => {
                // 越遠離敵人越高分
                score += w.nearest_enemy * min_dist;
            }
            PositionPreference::Flexible => {
                // 可自訂，預設不加分
            }
        }

        let reason = format!(
            "最近敵人距離: {:.2}, 平均敵人距離: {:.2}, 分數: {:.2} (權重: nearest {:.2}, 偏好 {:?})",
            min_dist, avg_dist, score, w.nearest_enemy, tendency.positioning_preference
        );

        Ok(ScoredAction {
            action: Action::Move { path },
            score,
            reason,
        })
    }

    fn score_move_and_use_skill(
        tendency: &Tendency,
        ally_positions: Vec<Pos>,
        enemy_positions: Vec<Pos>,
        path: Vec<Pos>,
        skill_id: SkillID,
        target: Pos,
        target_units: Vec<UnitID>,
    ) -> Result<ScoredAction, Error> {
        let func = "ai.score_move_and_use_skill";

        Ok(ScoredAction {
            action: Action::Idle,
            score: -1.0,
            reason: format!("not implemented"),
        })
    }

    fn manhattan_distance(a: Pos, b: Pos) -> usize {
        ((a.x as isize - b.x as isize).abs() + (a.y as isize - b.y as isize).abs()) as usize
    }
}

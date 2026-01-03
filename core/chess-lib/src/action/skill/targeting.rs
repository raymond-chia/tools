//! 目標選擇與範圍計算模組
//!
//! 負責技能的目標驗證、施放範圍計算、形狀計算等功能

use crate::*;
use skills_lib::*;
use std::collections::BTreeMap;
use std::f64::consts::PI;

/// 計算單位周邊區域的技能施放範圍（根據 range，不含 shape）
/// - board: 棋盤物件
/// - from: 施放者座標
/// - range: (min_range, max_range) 技能 range 設定
/// - skills: 技能定義表（用於檢查 Sense 能力）
///
/// # Returns
/// 所有可施放座標（已過濾視線檢查）
pub fn skill_casting_area(
    board: &Board,
    active_unit_pos: Pos,
    range: (usize, usize),
    skills: &BTreeMap<SkillID, Skill>,
) -> Vec<Pos> {
    let active_unit_id = match board.pos_to_unit(active_unit_pos) {
        Some(id) => id,
        None => return vec![],
    };
    let active_unit = match board.units.get(&active_unit_id) {
        Some(u) => u,
        None => return vec![],
    };
    if is_able_to_act(active_unit).is_err() {
        return vec![];
    }

    let mut area = Vec::new();
    let max_range = range.1 as isize;
    for dy in -max_range..=max_range {
        for dx in -max_range..=max_range {
            let x = active_unit_pos.x as isize + dx;
            let y = active_unit_pos.y as isize + dy;
            if x < 0 || y < 0 {
                continue;
            }
            let (x, y) = (x as usize, y as usize);
            let target = Pos { x, y };
            if !is_in_skill_range_manhattan(range, active_unit_pos, target) {
                continue;
            }
            if board.get_tile(target).is_none() {
                continue;
            }

            // 檢查視線：是否能看到目標位置
            match board.can_see_target((active_unit_id, active_unit_pos), target, skills) {
                Ok(true) => area.push(target),
                _ => continue, // 看不到或出錯，跳過此位置
            }
        }
    }
    area
}

/// 判定現在狀態能否使用任何技能
pub fn is_able_to_act(unit: &Unit) -> Result<(), Error> {
    let func = "is_able_to_act";

    if unit.has_cast_skill_this_turn {
        return Err(Error::NotEnoughAP { func });
    }
    if unit.moved > unit.move_points {
        return Err(Error::NotEnoughAP { func });
    }
    Ok(())
}

/// 消耗 action 點數（設置施法標記）
///
/// # 參數
/// - `unit`: 要消耗 action 的單位
///
/// # 返回值
/// - Ok(()): 成功消耗
/// - Err(Error): 沒有可用的 action（已施放過技能或移動點數用盡）
///
/// # 使用場景
/// 在執行主動技能後調用此函數來消耗 action
pub fn consume_action(unit: &mut Unit) -> Result<(), Error> {
    let func = "consume_action";

    is_able_to_act(unit).wrap_context(func)?;

    unit.has_cast_skill_this_turn = true;
    Ok(())
}

/// 計算技能形狀範圍
/// board: 棋盤物件, shape: 技能形狀, from: 施放者座標, to: 目標座標
/// 回傳：座標列表
/// 只給本檔案和 ai.rs 使用
pub fn calc_shape_area(board: &Board, shape: &Shape, from: Pos, to: Pos) -> Vec<Pos> {
    match shape {
        Shape::Point => vec![to],
        Shape::Circle(r) => {
            let r = *r as isize; // 起點也算在半徑內
            let r2 = r * r;
            (-r..=r)
                .flat_map(|dx| (-r..=r).map(move |dy| (dx, dy)))
                .filter_map(|(dx, dy)| {
                    if dx * dx + dy * dy > r2 {
                        return None;
                    }
                    let x = to.x as isize + dx;
                    let y = to.y as isize + dy;
                    if x < 0 || y < 0 {
                        return None;
                    }
                    let (x, y) = (x as usize, y as usize);
                    let target = Pos { x, y };
                    board.get_tile(target).map(|_| target)
                })
                .collect()
        }
        Shape::Line(len) => {
            // https://en.wikipedia.org/wiki/Bresenham%27s_line_algorithm
            if from == to {
                return vec![];
            }
            let len = len + 1; // 不計算起點
            bresenham_line(from, to, len, |pos| board.get_tile(pos).is_some())
                .into_iter()
                .filter(|pos| *pos != from)
                .collect()
        }
        Shape::Cone(len, degree) => {
            if from == to {
                return vec![];
            }
            let len = *len as isize;
            let len2 = len * len;
            let degree = *degree as f64;
            let from_x = from.x as isize;
            let from_y = from.y as isize;
            let dx = (to.x as isize - from_x) as f64;
            let dy = (to.y as isize - from_y) as f64;
            let target_theta = dy.atan2(dx);
            let half_theta = degree.to_radians() / 2.0;

            let mut area = vec![];
            // 只在 from 附近搜尋半徑 len
            for x in (from_x - len)..=(from_x + len) {
                for y in (from_y - len)..=(from_y + len) {
                    if x < 0 || y < 0 {
                        continue;
                    }
                    let pos = Pos {
                        x: x as usize,
                        y: y as usize,
                    };
                    if board.get_tile(pos).is_none() {
                        continue;
                    }
                    let dx = x - from_x;
                    let dy = y - from_y;
                    let distance2 = dx * dx + dy * dy;
                    // 超出範圍 或 排除自身
                    if distance2 > len2 || distance2 == 0 {
                        continue;
                    }
                    // 格子的方向
                    let point_theta = (dy as f64).atan2(dx as f64);
                    // 角度差（方向無關正負、取最短和 2π補正）
                    let delta = point_theta - target_theta;
                    // 把 delta 調整到 [-PI, PI]
                    let delta = clamp_pi(delta);
                    if delta.abs() > half_theta {
                        continue; // 超出角度範圍
                    }
                    area.push(pos);
                }
            }
            area
        }
    }
}

/// 驗證目標是否符合技能的目標類型要求
pub fn is_targeting_valid_target(
    board: &Board,
    skill_id: &str,
    skill: &Skill,
    caster_id: UnitID,
    target: Pos,
) -> Result<(), Error> {
    let func = "is_targeting_valid_target";

    // 只檢查第一個效果
    let first_effect = skill.effects.first().ok_or_else(|| Error::InvalidSkill {
        func,
        skill_id: skill_id.to_string(),
    })?;
    // 如果技能只需要瞄準位置，則直接通過
    if !first_effect.is_targeting_unit() {
        return Ok(());
    }
    // 檢查陣營是否符合技能效果目標
    let caster_unit = board.units.get(&caster_id).ok_or(Error::NoActingUnit {
        func,
        unit_id: caster_id,
    })?;
    let target_id = board.pos_to_unit(target).ok_or(Error::SkillTargetNoUnit {
        func,
        skill_id: skill_id.to_string(),
        pos: target,
    })?;
    let target_unit = board
        .units
        .get(&target_id)
        .ok_or(Error::SkillTargetNoUnit {
            func,
            skill_id: skill_id.to_string(),
            pos: target,
        })?;
    let effect_target_type = first_effect.target_type();
    match effect_target_type {
        TargetType::Caster => {
            if caster_id != target_id {
                return Err(Error::SkillAffectWrongUnit {
                    func,
                    skill_id: skill_id.to_string(),
                    detail: "skill only affect caster".to_string(),
                });
            }
        }
        TargetType::Ally => {
            if caster_unit.team != target_unit.team {
                return Err(Error::SkillAffectWrongUnit {
                    func,
                    skill_id: skill_id.to_string(),
                    detail: format!(
                        "skill only affect ally: caster team {}, target team {}",
                        caster_unit.team, target_unit.team
                    ),
                });
            }
        }
        TargetType::AllyExcludeCaster => {
            if caster_id == target_id {
                return Err(Error::SkillAffectWrongUnit {
                    func,
                    skill_id: skill_id.to_string(),
                    detail: "skill only affect ally `exclude caster`".to_string(),
                });
            } else if caster_unit.team != target_unit.team {
                return Err(Error::SkillAffectWrongUnit {
                    func,
                    skill_id: skill_id.to_string(),
                    detail: format!(
                        "skill only affect `ally` exclude caster: caster team {}, target team {}",
                        caster_unit.team, target_unit.team
                    ),
                });
            }
        }
        TargetType::Enemy => {
            if caster_unit.team == target_unit.team {
                return Err(Error::SkillAffectWrongUnit {
                    func,
                    skill_id: skill_id.to_string(),
                    detail: format!(
                        "skill only affect enemy: caster team {}, target team {}",
                        caster_unit.team, target_unit.team
                    ),
                });
            }
        }
        TargetType::AnyUnit => {} // 任何單位都可
        TargetType::Any => {
            return Err(Error::InvalidImplementation {
                func,
                detail: "any target should not reach here".to_string(),
            });
        }
    }
    Ok(())
}

/// 判斷目標是否在技能範圍內（曼哈頓距離）
/// - range: (min_range, max_range) 技能 range 設定
/// - skill: 技能資料
/// - from: 施放者座標
/// - to: 目標座標
///
/// # Returns
/// 是否符合技能距離限制
pub fn is_in_skill_range_manhattan(range: (usize, usize), from: Pos, to: Pos) -> bool {
    let dx = (from.x as isize - to.x as isize).abs();
    let dy = (from.y as isize - to.y as isize).abs();
    let dist = (dx + dy) as usize;

    let (min_range, max_range) = range;
    min_range <= dist && dist <= max_range
}

/// 計算方向（使用曼哈頓主方向：abs(dx)>=abs(dy) -> 水平，否則垂直）
pub fn calc_direction_manhattan(from: Pos, to: Pos) -> (isize, isize) {
    let dx = to.x as isize - from.x as isize;
    let dy = to.y as isize - from.y as isize;
    if dx.abs() >= dy.abs() {
        (dx.signum(), 0)
    } else {
        (0, dy.signum())
    }
}

/// 將角度限制在 [-PI, PI] 範圍內
pub fn clamp_pi(mut rad: f64) -> f64 {
    while rad < -PI {
        rad += PI * 2.0;
    }
    while rad > PI {
        rad -= PI * 2.0;
    }
    rad
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet, HashMap};

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
    fn test_skill_casting_area() {
        let (board, _, skills) = prepare_test_board(Pos { x: 1, y: 1 }, None);

        let set = |v: &[Pos]| BTreeSet::from_iter(v.iter().copied());

        let test_data = [
            (
                (0, 1),
                Pos { x: 1, y: 1 },
                set(&[
                    Pos { x: 0, y: 1 },
                    Pos { x: 1, y: 0 },
                    Pos { x: 1, y: 1 },
                    Pos { x: 2, y: 1 },
                    Pos { x: 1, y: 2 },
                ]),
            ),
            (
                (1, 1),
                Pos { x: 1, y: 1 },
                set(&[
                    Pos { x: 0, y: 1 },
                    Pos { x: 1, y: 0 },
                    Pos { x: 2, y: 1 },
                    Pos { x: 1, y: 2 },
                ]),
            ),
            (
                (1, 2),
                Pos { x: 1, y: 1 },
                set(&[
                    Pos { x: 0, y: 0 },
                    Pos { x: 0, y: 1 },
                    Pos { x: 0, y: 2 },
                    Pos { x: 1, y: 0 },
                    Pos { x: 1, y: 2 },
                    Pos { x: 1, y: 3 },
                    Pos { x: 2, y: 0 },
                    Pos { x: 2, y: 1 },
                    Pos { x: 2, y: 2 },
                    Pos { x: 3, y: 1 },
                ]),
            ),
        ];

        for (range, active_unit_pos, expected) in test_data {
            let area = skill_casting_area(&board, active_unit_pos, range, &skills)
                .into_iter()
                .collect::<BTreeSet<_>>();
            assert_eq!(area, expected);
        }
    }

    #[test]
    fn test_is_able_to_act() {
        let (mut board, unit_id, _skills) = prepare_test_board(Pos { x: 0, y: 0 }, None);
        let unit = board.units.get_mut(&unit_id).unwrap();

        // 正常可施放
        unit.has_cast_skill_this_turn = false;
        unit.moved = 0;
        unit.move_points = 2;
        assert!(is_able_to_act(unit).is_ok());

        // 已施放過技能
        unit.has_cast_skill_this_turn = true;
        unit.moved = 0;
        assert!(matches!(
            is_able_to_act(unit),
            Err(Error::NotEnoughAP { .. })
        ));

        // 移動超過點數
        unit.has_cast_skill_this_turn = false;
        unit.moved = 3;
        unit.move_points = 2;
        assert!(matches!(
            is_able_to_act(unit),
            Err(Error::NotEnoughAP { .. })
        ));
    }

    #[test]
    fn test_is_targeting_valid_target_error_cases() {
        // 準備棋盤、單位、技能
        let (mut board, unit_id, skills) =
            prepare_test_board(Pos { x: 1, y: 1 }, Some(vec![Pos { x: 1, y: 2 }]));
        let target_unit_id = unit_id + 1;
        let mut skill = skills.get("slash").unwrap().clone();

        // 取得原始隊伍
        let orig_team = board.units.get(&unit_id).unwrap().team.clone();

        // Caster: 目標不是自己
        board.units.get_mut(&unit_id).unwrap().team = orig_team.clone();
        board.units.get_mut(&target_unit_id).unwrap().team = orig_team.clone();
        skill.effects[0] = Effect::Hp {
            value: 0,
            target_type: TargetType::Caster,
            shape: Shape::Point,
        };
        let err = is_targeting_valid_target(&board, "slash", &skill, unit_id, Pos { x: 1, y: 2 });
        assert!(matches!(err, Err(Error::SkillAffectWrongUnit { .. })));
        // Caster: 目標是自己（合法）
        skill.effects[0] = Effect::Hp {
            value: 0,
            target_type: TargetType::Caster,
            shape: Shape::Point,
        };
        let ok = is_targeting_valid_target(&board, "slash", &skill, unit_id, Pos { x: 1, y: 1 });
        assert!(ok.is_ok());

        // Ally: 目標不同隊伍
        board.units.get_mut(&unit_id).unwrap().team = orig_team.clone();
        board.units.get_mut(&target_unit_id).unwrap().team = "other".to_string();
        skill.effects[0] = Effect::Hp {
            value: 0,
            target_type: TargetType::Ally,
            shape: Shape::Point,
        };
        let err = is_targeting_valid_target(&board, "slash", &skill, unit_id, Pos { x: 1, y: 2 });
        assert!(matches!(err, Err(Error::SkillAffectWrongUnit { .. })));
        // Ally: 目標同隊（合法）
        board.units.get_mut(&unit_id).unwrap().team = orig_team.clone();
        board.units.get_mut(&target_unit_id).unwrap().team = orig_team.clone();
        skill.effects[0] = Effect::Hp {
            value: 0,
            target_type: TargetType::Ally,
            shape: Shape::Point,
        };
        let ok = is_targeting_valid_target(&board, "slash", &skill, unit_id, Pos { x: 1, y: 2 });
        assert!(ok.is_ok());

        // AllyExcludeCaster: 目標是自己
        board.units.get_mut(&unit_id).unwrap().team = orig_team.clone();
        board.units.get_mut(&target_unit_id).unwrap().team = orig_team.clone();
        skill.effects[0] = Effect::Hp {
            value: 0,
            target_type: TargetType::AllyExcludeCaster,
            shape: Shape::Point,
        };
        let err = is_targeting_valid_target(&board, "slash", &skill, unit_id, Pos { x: 1, y: 1 });
        assert!(matches!(err, Err(Error::SkillAffectWrongUnit { .. })));
        // AllyExcludeCaster: 目標同隊他人（合法）
        board.units.get_mut(&unit_id).unwrap().team = orig_team.clone();
        board.units.get_mut(&target_unit_id).unwrap().team = orig_team.clone();
        skill.effects[0] = Effect::Hp {
            value: 0,
            target_type: TargetType::AllyExcludeCaster,
            shape: Shape::Point,
        };
        let ok = is_targeting_valid_target(&board, "slash", &skill, unit_id, Pos { x: 1, y: 2 });
        assert!(ok.is_ok());

        // Enemy: 目標同隊
        board.units.get_mut(&unit_id).unwrap().team = orig_team.clone();
        board.units.get_mut(&target_unit_id).unwrap().team = orig_team.clone();
        skill.effects[0] = Effect::Hp {
            value: 0,
            target_type: TargetType::Enemy,
            shape: Shape::Point,
        };
        let err = is_targeting_valid_target(&board, "slash", &skill, unit_id, Pos { x: 1, y: 2 });
        assert!(matches!(err, Err(Error::SkillAffectWrongUnit { .. })));
        // Enemy: 目標不同隊伍（合法）
        board.units.get_mut(&unit_id).unwrap().team = orig_team.clone();
        board.units.get_mut(&target_unit_id).unwrap().team = "other".to_string();
        skill.effects[0] = Effect::Hp {
            value: 0,
            target_type: TargetType::Enemy,
            shape: Shape::Point,
        };
        let ok = is_targeting_valid_target(&board, "slash", &skill, unit_id, Pos { x: 1, y: 2 });
        assert!(ok.is_ok());
        // 還原隊伍
        board.units.get_mut(&target_unit_id).unwrap().team = orig_team.clone();

        // Any: 不會進入錯誤分支，is_targeting_unit() 為 false，會直接 Ok(())
        skill.effects[0] = Effect::Hp {
            value: 0,
            target_type: TargetType::Any,
            shape: Shape::Point,
        };
        let ok = is_targeting_valid_target(&board, "slash", &skill, unit_id, Pos { x: 1, y: 2 });
        assert!(ok.is_ok());
    }
}

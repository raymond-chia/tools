//! skill.rs：
//! - 負責技能效果、技能施放與解析邏輯。
//! - 僅處理技能本身，不負責戰鬥流程、AI 決策或棋盤初始化。
//! - 技能相關的資料結構與輔助函式應集中於此。
use crate::*;
use rand::Rng;
use skills_lib::*;
use std::{collections::BTreeMap, f64::consts::PI};

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

    /// 施放技能主流程
    pub fn cast_skill(
        &self,
        board: &mut Board,
        skills: &BTreeMap<SkillID, Skill>,
        caster: UnitID,
        target: Pos,
    ) -> Result<Vec<String>, Error> {
        let func = "SkillSelection::cast_skill";

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
        let skill_id = self
            .selected_skill
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

        let affect_area = self.skill_affect_area(board, skills, caster, target);
        if affect_area.is_empty() {
            return Err(Error::SkillAffectEmpty {
                func,
                skill_id: skill_id.clone(),
                pos: target,
            });
        }
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
        let mut msgs = vec![format!("{} 在 ({}, {}) 施放", skill_id, target.x, target.y)];

        // 命中機制最終設計摘要如下：
        // 1. 技能命中數值（accuracy）僅計算一次，並套用於所有目標（僅計算命中數值，不進行閃避或格擋判定）。
        // 2. 檢查每個目標是否為技能效果的合法套用對象（敵軍、友軍、自己等）。
        // 3. 僅對符合效果目標的單位，進行閃避與格擋的判定。
        // 4. 若目標被「閃避」則不套用效果；若為「命中」或「格擋」則都會套用效果，但格擋可影響效果強度（如減傷）。
        // 5. 命中結果（命中、閃避、格擋）皆可顯示訊息。
        match skill.accuracy {
            None => {
                // 無命中數值，所有格子直接套用效果
                for pos in affect_area {
                    for effect in &skill.effects {
                        if let Some(msg) = apply_effect_to_pos(board, effect, pos) {
                            msgs.push(msg);
                        }
                    }
                }
            }
            Some(accuracy) => {
                msgs.extend(calc_hit_result(
                    board,
                    skills,
                    skill,
                    affect_area,
                    accuracy,
                )?);
            }
        }

        if let Some(unit) = board.units.get_mut(&caster) {
            unit.has_cast_skill_this_turn = true;
        }
        Ok(msgs)
    }

    /// 預覽技能範圍
    /// 根據目前選擇的技能與棋盤狀態，計算技能可作用的座標列表
    /// - board: 棋盤狀態
    /// - skills: 技能資料表
    /// - unit_id: 行動單位 ID
    /// - to: 指向格
    ///
    /// 回傳：技能可作用範圍的座標 Vec<Pos>
    pub fn skill_affect_area(
        &self,
        board: &Board,
        skills: &BTreeMap<SkillID, Skill>,
        unit_id: UnitID,
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
        // 取得單位位置
        let from = match board.unit_to_pos(unit_id) {
            Some(p) => p,
            None => return vec![],
        };
        // 取得單位物件，檢查移動點數
        let unit = match board.units.get(&unit_id) {
            Some(u) => u,
            None => return vec![],
        };
        if is_able_to_cast(unit).is_err() {
            return vec![];
        }
        // 判斷 to 是否在技能 range 內，超過則不顯示範圍
        if !is_in_skill_range_manhattan(skill.range, from, to) {
            return vec![];
        }
        // 取得技能範圍形狀（僅取第一個 effect 的 shape）
        let shape = skill.effects.get(0).map(|e| e.shape());
        let shape = match shape {
            Some(s) => s,
            None => return vec![],
        };
        // 計算範圍
        calc_shape_area(board, shape, from, to)
    }
}

/// 計算單位周邊區域的技能施放範圍（根據 range，不含 shape）
/// - board: 棋盤物件
/// - from: 施放者座標
/// - range: (min_range, max_range) 技能 range 設定
/// 回傳：所有可施放座標
pub fn skill_casting_area(board: &Board, active_unit_pos: Pos, range: (usize, usize)) -> Vec<Pos> {
    let unit_id = match board.pos_to_unit(active_unit_pos) {
        Some(id) => id,
        None => return vec![],
    };
    let unit = match board.units.get(&unit_id) {
        Some(u) => u,
        None => return vec![],
    };
    if is_able_to_cast(unit).is_err() {
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
            area.push(target);
        }
    }
    area
}

/// 判定現在狀態能否使用任何技能
pub fn is_able_to_cast(unit: &Unit) -> Result<(), Error> {
    let func = "is_able_to_cast";

    if unit.has_cast_skill_this_turn {
        return Err(Error::NotEnoughAP { func });
    }
    if unit.moved > unit.move_points {
        return Err(Error::NotEnoughAP { func });
    }
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

use inner::*;
mod inner {
    use super::*;

    pub fn is_targeting_valid_target(
        board: &Board,
        skill_id: &str,
        skill: &Skill,
        caster_id: UnitID,
        target: Pos,
    ) -> Result<(), Error> {
        let func = "is_targeting_valid_target";

        // 只檢查第一個效果
        let first_effect = skill.effects.get(0).ok_or_else(|| Error::InvalidSkill {
            func,
            skill_id: skill_id.to_string(),
        })?;
        // 如果技能只需要瞄準位置，則直接通過
        if !first_effect.is_targeting_unit() {
            return Ok(());
        }
        // 檢查陣營是否符合技能效果目標
        let caster_unit = board
            .units
            .get(&caster_id)
            .ok_or_else(|| Error::NoActingUnit {
                func,
                unit_id: caster_id,
            })?;
        let target_id = board
            .pos_to_unit(target)
            .ok_or_else(|| Error::SkillTargetNoUnit {
                func,
                skill_id: skill_id.to_string(),
                pos: target,
            })?;
        let target_unit = board
            .units
            .get(&target_id)
            .ok_or_else(|| Error::SkillTargetNoUnit {
                func,
                skill_id: skill_id.to_string(),
                pos: target,
            })?;
        let effect_target_type = first_effect.target_type();
        let err_msg = match effect_target_type {
            TargetType::Caster => {
                if caster_id != target_id {
                    "skill only affect caster".to_string()
                } else {
                    "".to_string()
                }
            }
            TargetType::Ally => {
                if caster_unit.team != target_unit.team {
                    format!(
                        "skill only affect ally: caster team {}, target team {}",
                        caster_unit.team, target_unit.team
                    )
                } else {
                    "".to_string()
                }
            }
            TargetType::AllyExcludeCaster => {
                if caster_id == target_id {
                    "skill only affect ally `exclude caster`".to_string()
                } else if caster_unit.team != target_unit.team {
                    format!(
                        "skill only affect `ally` exclude caster: caster team {}, target team {}",
                        caster_unit.team, target_unit.team
                    )
                } else {
                    "".to_string()
                }
            }
            TargetType::Enemy => {
                if caster_unit.team == target_unit.team {
                    format!(
                        "skill only affect enemy: caster team {}, target team {}",
                        caster_unit.team, target_unit.team
                    )
                } else {
                    "".to_string()
                }
            }
            TargetType::AnyUnit => "".to_string(), // 任何單位都可
            TargetType::Any => {
                return Err(Error::InvalidImplementation {
                    func,
                    detail: "any target should not reach here".to_string(),
                });
            }
        };
        if err_msg.len() > 0 {
            return Err(Error::SkillAffectWrongUnit {
                func,
                skill_id: skill_id.to_string(),
                detail: err_msg,
            });
        }
        Ok(())
    }

    pub fn calc_hit_result(
        board: &mut Board,
        skills: &BTreeMap<SkillID, Skill>,
        skill: &Skill,
        affect_area: Vec<Pos>,
        accuracy: i32,
    ) -> Result<Vec<String>, Error> {
        let func = "calc_hit_result";

        // 有命中數值，進行命中機制（命中只算一次，閃避/格擋每目標）
        let mut rng = rand::rng();
        let hit_random = rng.random_range(1..100);
        let critical_failure = 5;
        let critical_success = 95;
        let hit_score = accuracy + hit_random;

        let mut msgs = vec![];

        for pos in affect_area {
            let unit_id = match board.pos_to_unit(pos) {
                // 無單位，直接套用效果
                None => {
                    for effect in &skill.effects {
                        if let Some(msg) = apply_effect_to_pos(board, effect, pos) {
                            msgs.push(format!("空地 {pos:?} 受到效果：{msg}"));
                        }
                    }
                    continue;
                }
                Some(unit_id) => unit_id,
            };
            let unit =
                board
                    .units
                    .get_mut(&unit_id)
                    .ok_or_else(|| Error::InvalidImplementation {
                        func,
                        detail: "unit not found".to_string(),
                    })?;
            let unit_type = unit.unit_template_type.clone();
            if hit_random <= critical_failure {
                // 完全閃避
                msgs.push(format!(
                    "亂數={hit_random} <= {critical_failure}%，單位 {unit_type} 完全閃避了攻擊！"
                ));
                continue;
            }
            if hit_random > critical_success {
                // 完全命中
                for effect in &skill.effects {
                    if let Some(msg) = apply_effect_to_pos(board, effect, pos) {
                        msgs.push(format!(
                            "亂數={hit_random} > {critical_success}%，單位 {unit_type} 被完全命中了：{msg}"
                        ));
                    }
                }
                continue;
            }
            let unit_skills: BTreeMap<_, _> = board
                .units
                .get(&unit_id)
                .map(|unit| {
                    unit.skills
                        .iter()
                        .filter_map(|skill_id| skills.get(skill_id).map(|s| (skill_id, s)))
                        .collect()
                })
                .unwrap_or_default();
            // 計算閃避值
            let evasion = crate::unit::skills_to_evasion(&unit_skills);
            // 閃避
            let evade_score = hit_score - evasion;
            if evade_score <= 0 {
                msgs.push(format!(
                    "單位 {unit_type} 閃避了攻擊！(accuracy={accuracy}, random={hit_random}, evade={evasion})",
                ));
                continue;
            }

            // 計算格擋值
            let block = crate::unit::skills_to_block(&unit_skills);
            let block_reduction = 1;
            let block_score = hit_score - block - evasion;
            // 格擋
            if block_score <= 0 {
                for effect in &skill.effects {
                    match effect {
                        Effect::Hp {
                            value,
                            target_type,
                            shape,
                        } => {
                            let mut value = *value;
                            if value < 0 {
                                value += block_reduction;
                            }
                            let effect = Effect::Hp {
                                value,
                                target_type: target_type.clone(),
                                shape: shape.clone(),
                            };
                            let old_hp = board
                                .units
                                .get_mut(&unit_id)
                                .ok_or_else(|| Error::InvalidImplementation {
                                    func,
                                    detail: format!("unit not found 2: {unit_type}"),
                                })?
                                .hp;
                            if let Some(msg) = apply_effect_to_pos(board, &effect, pos) {
                                let new_hp = board
                                    .units
                                    .get_mut(&unit_id)
                                    .ok_or_else(|| Error::InvalidImplementation {
                                        func,
                                        detail: format!("unit not found 3: {unit_type}"),
                                    })?
                                    .hp;
                                msgs.push(format!(
                                    "單位 {unit_type} 格擋攻擊！HP: {old_hp} → {new_hp} (accuracy={accuracy}, random={hit_random}, evade={evasion}, block={block})：{msg}",
                                ));
                            }
                        }
                        _ => {
                            if let Some(msg) = apply_effect_to_pos(board, effect, pos) {
                                msgs.push(format!(
                                    "單位 {unit_type} 格擋攻擊！(accuracy={accuracy}, random={hit_random}, evade={evasion}, block={block})。但是命中效果不受影響：{msg}",
                                ));
                            }
                        }
                    }
                }
                continue;
            }

            // 完全命中
            for effect in &skill.effects {
                if let Some(msg) = apply_effect_to_pos(board, effect, pos) {
                    msgs.push(format!(
                        "單位 {unit_type} 被完全命中 (accuracy={accuracy}, random={hit_random}, evade={evasion}, block={block})：{msg}"
                    ));
                }
            }
        }
        Ok(msgs)
    }

    /// 將單一效果套用到指定座標（單一 entry-point，方便後續擴充/重構）
    pub fn apply_effect_to_pos(board: &mut Board, effect: &Effect, pos: Pos) -> Option<String> {
        match effect {
            Effect::Hp { value, .. } => {
                if let Some(unit_id) = board.pos_to_unit(pos) {
                    if let Some(unit) = board.units.get_mut(&unit_id) {
                        let old_hp = unit.hp;
                        unit.hp += value;
                        if unit.hp > unit.max_hp {
                            unit.hp = unit.max_hp;
                        }
                        let new_hp = unit.hp;
                        return Some(format!(
                            "單位 {} HP: {old_hp} → {new_hp}",
                            &unit.unit_template_type,
                        ));
                    }
                }
                None
            }
            Effect::Mp { value, .. } => Some(format!("[未實作] Mp {value}",)),
            Effect::MaxHp {
                duration, value, ..
            } => Some(format!("[未實作] MaxHp {value}, 持續 {duration} 回合",)),
            Effect::MaxMp {
                duration, value, ..
            } => Some(format!("[未實作] MaxMp {value}, 持續 {duration} 回合",)),
            Effect::Initiative {
                duration, value, ..
            } => Some(format!("[未實作] Initiative {value}, 持續 {duration} 回合",)),
            Effect::Evasion {
                value, duration, ..
            } => Some(format!(
                "[未實作] Evasion 效果 +{value}%, 持續 {duration} 回合"
            )),
            Effect::Block {
                value, duration, ..
            } => Some(format!(
                "[未實作] Block 效果 +{value}%, 持續 {duration} 回合"
            )),
            Effect::MovePoints {
                value, duration, ..
            } => Some(format!("[未實作] 單位移動 {value}, 持續 {duration} 回合")),
            Effect::Burn { duration, .. } => {
                Some(format!("[未實作] Burn 效果, 持續 {duration} 回合",))
            }
            Effect::HitAndRun { .. } => Some(format!("[未實作] 打帶跑")),
        }
    }

    /// 判斷技能施放距離是否符合 range 設定
    /// - skill: 技能資料
    /// - from: 施放者座標
    /// - to: 目標座標
    /// 回傳：是否符合技能距離限制
    pub fn is_in_skill_range_manhattan(range: (usize, usize), from: Pos, to: Pos) -> bool {
        let dx = (from.x as isize - to.x as isize).abs();
        let dy = (from.y as isize - to.y as isize).abs();
        let dist = (dx + dy) as usize;

        let (min_range, max_range) = range;
        min_range <= dist && dist <= max_range
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, HashMap};

    fn prepare_test_board(
        pos: Pos,
        extra_unit_pos: Option<Vec<Pos>>,
    ) -> (Board, UnitID, BTreeMap<SkillID, Skill>) {
        let data = include_str!("../../tests/unit.json");
        let v: serde_json::Value = serde_json::from_str(data).unwrap();
        let template: UnitTemplate = serde_json::from_value(v["UnitTemplate"].clone()).unwrap();
        let marker: UnitMarker = serde_json::from_value(v["UnitMarker"].clone()).unwrap();
        let team: Team = serde_json::from_value(v["Team"].clone()).unwrap();
        let teams = HashMap::from([(team.id.clone(), team.clone())]);
        let skills = {
            let slash_data = include_str!("../../tests/skill_slash.json");
            let slash_skill: Skill = serde_json::from_str(slash_data).unwrap();
            let shoot_data = include_str!("../../tests/skill_shoot.json");
            let shoot_skill: Skill = serde_json::from_str(shoot_data).unwrap();
            let splash_data = include_str!("../../tests/skill_splash.json");
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
    fn test_select_skill() {
        let mut sel = SkillSelection::default();
        assert_eq!(sel.selected_skill, None);
        sel.select_skill(Some("1".to_string()));
        assert_eq!(sel.selected_skill, Some("1".to_string()));
        sel.select_skill(None);
        assert_eq!(sel.selected_skill, None);
    }

    #[test]
    fn test_cast_skill() {
        // 準備棋盤、單位、技能
        let (mut board, unit_id, skills) =
            prepare_test_board(Pos { x: 1, y: 1 }, Some(vec![Pos { x: 1, y: 3 }]));
        let target_unit_id = unit_id + 1;

        // 施放 shoot 技能到 (1,3)
        let mut sel = SkillSelection::default();
        sel.select_skill(Some("shoot".to_string()));
        let target = Pos { x: 1, y: 3 };

        // 施放前 HP
        let orig_hp = board.units.get(&target_unit_id).unwrap().hp;

        // 執行施放
        let msgs = sel
            .cast_skill(&mut board, &skills, unit_id, target)
            .unwrap();

        // 檢查訊息
        assert!(msgs.iter().any(|m| m.contains("shoot 在 (1, 3) 施放")));
        assert!(msgs.iter().any(|m| m.contains("HP:")));

        // 檢查 HP 變化
        let new_hp = board.units.get(&target_unit_id).unwrap().hp;
        assert_eq!(new_hp, orig_hp - 10);

        // 檢查 has_cast_skill_this_turn
        assert!(board.units.get(&unit_id).unwrap().has_cast_skill_this_turn);
    }

    #[test]
    fn test_cast_skill_non_exist_skill() {
        // 準備棋盤、單位、技能
        let (mut board, unit_id, skills) =
            prepare_test_board(Pos { x: 1, y: 1 }, Some(vec![Pos { x: 1, y: 3 }]));

        // 測試施放無效技能（未選擇技能）
        let target = Pos { x: 1, y: 3 };
        let sel_none = SkillSelection::default();
        let err = sel_none.cast_skill(&mut board, &skills, unit_id, target);
        assert!(err.is_err());
        let err = err.unwrap_err();
        assert!(matches!(err, Error::NoSkillSelected { .. }), "{err:?}");

        // 檢查 has_cast_skill_this_turn
        assert!(board.units.get(&unit_id).unwrap().has_cast_skill_this_turn == false);
    }

    #[test]
    fn test_cast_skill_once_per_turn() {
        // 準備棋盤、單位、技能
        let (mut board, unit_id, skills) =
            prepare_test_board(Pos { x: 2, y: 2 }, Some(vec![Pos { x: 2, y: 3 }]));

        // 第一次施放 shoot 技能到 (2,3)
        let mut sel = SkillSelection::default();
        sel.select_skill(Some("shoot".to_string()));
        let target = Pos { x: 2, y: 3 };
        let result = sel.cast_skill(&mut board, &skills, unit_id, target);
        assert!(result.is_ok());

        // 第二次同回合施放（應失敗）
        let mut sel2 = SkillSelection::default();
        sel2.select_skill(Some("shoot".to_string()));
        let result2 = sel2.cast_skill(&mut board, &skills, unit_id, target);
        assert!(result2.is_err());
        assert!(
            matches!(
                root_error(result2.as_ref().unwrap_err()),
                Error::NotEnoughAP { .. }
            ),
            "{:?}",
            result2
        );
    }

    #[test]
    fn test_cast_skill_moving_too_far() {
        // 準備棋盤、單位、技能
        let (mut board, unit_id, skills) =
            prepare_test_board(Pos { x: 2, y: 2 }, Some(vec![Pos { x: 2, y: 3 }]));
        let movement_point = board.units.get(&unit_id).unwrap().move_points;
        board.units.get_mut(&unit_id).unwrap().moved = movement_point + 1;

        let target = Pos { x: 2, y: 3 };

        let mut sel = SkillSelection::default();
        sel.select_skill(Some("shoot".to_string()));
        let result = sel.cast_skill(&mut board, &skills, unit_id, target);
        assert!(result.is_err(), "{:?}", &result);
    }

    #[test]
    fn test_cast_skill_no_target() {
        // 準備棋盤、單位、技能
        let (mut board, unit_id, skills) = prepare_test_board(Pos { x: 1, y: 1 }, None);

        // 選擇 shoot 技能
        let mut sel = SkillSelection::default();
        sel.select_skill(Some("shoot".to_string()));

        // 施放到沒有單位的格子
        let target = Pos { x: 2, y: 2 }; // 此格無 unit

        let result = sel.cast_skill(&mut board, &skills, unit_id, target);

        // 應回傳錯誤，且不應有施放技能訊息
        assert!(result.is_err());
        let err = result.unwrap_err();
        let inner_err = root_error(&err);
        assert!(
            matches!(inner_err, Error::SkillTargetNoUnit { .. }),
            "{:?}",
            inner_err
        );

        // 施法者狀態不變
        assert!(!board.units.get(&unit_id).unwrap().has_cast_skill_this_turn);
    }

    #[test]
    fn test_skill_affect_area() {
        let (board, unit_id, skills) = prepare_test_board(Pos { x: 1, y: 1 }, None);

        let set = |v: &[Pos]| BTreeSet::from_iter(v.into_iter().copied());

        // 測試
        let test_data = [
            ("slash", Pos { x: 1, y: 1 }, set(&[])), // 太近
            ("slash", Pos { x: 1, y: 2 }, set(&[Pos { x: 1, y: 2 }])),
            ("slash", Pos { x: 1, y: 3 }, set(&[])), // 太遠
            ("shoot", Pos { x: 1, y: 1 }, set(&[])), // 太近
            ("shoot", Pos { x: 1, y: 2 }, set(&[Pos { x: 1, y: 2 }])),
            ("shoot", Pos { x: 1, y: 3 }, set(&[Pos { x: 1, y: 3 }])),
            ("shoot", Pos { x: 1, y: 5 }, set(&[Pos { x: 1, y: 5 }])),
            ("shoot", Pos { x: 1, y: 6 }, set(&[])), // 太遠
            (
                "splash",
                Pos { x: 1, y: 1 },
                set(&[
                    Pos { x: 1, y: 0 },
                    Pos { x: 0, y: 1 },
                    Pos { x: 1, y: 1 },
                    Pos { x: 2, y: 1 },
                    Pos { x: 1, y: 2 },
                ]),
            ),
            (
                "splash",
                Pos { x: 1, y: 5 },
                set(&[
                    Pos { x: 1, y: 4 },
                    Pos { x: 0, y: 5 },
                    Pos { x: 1, y: 5 },
                    Pos { x: 2, y: 5 },
                    Pos { x: 1, y: 6 },
                ]),
            ),
            ("splash", Pos { x: 1, y: 6 }, set(&[])), // 太遠
            ("splash", Pos { x: 0, y: 5 }, set(&[])), // 太遠
            (
                "splash",
                Pos { x: 0, y: 4 },
                set(&[
                    Pos { x: 0, y: 3 },
                    Pos { x: 0, y: 4 },
                    Pos { x: 1, y: 4 },
                    Pos { x: 0, y: 5 },
                ]),
            ),
        ];
        for (skill_id, pos, expected) in test_data {
            let sel = SkillSelection {
                selected_skill: Some(skill_id.to_string()),
            };
            let area = sel
                .skill_affect_area(&board, &skills, unit_id, pos)
                .into_iter()
                .collect::<BTreeSet<_>>();
            assert_eq!(area, expected);
        }
    }

    #[test]
    fn test_skill_casting_area() {
        let (board, _, _) = prepare_test_board(Pos { x: 1, y: 1 }, None);

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
            let area = skill_casting_area(&board, active_unit_pos, range)
                .into_iter()
                .collect::<BTreeSet<_>>();
            assert_eq!(area, expected);
        }
    }

    #[test]
    fn test_is_able_to_cast() {
        let (mut board, unit_id, _skills) = prepare_test_board(Pos { x: 0, y: 0 }, None);
        let unit = board.units.get_mut(&unit_id).unwrap();

        // 正常可施放
        unit.has_cast_skill_this_turn = false;
        unit.moved = 0;
        unit.move_points = 2;
        assert!(is_able_to_cast(unit).is_ok());

        // 已施放過技能
        unit.has_cast_skill_this_turn = true;
        unit.moved = 0;
        assert!(matches!(
            is_able_to_cast(unit),
            Err(Error::NotEnoughAP { .. })
        ));

        // 移動超過點數
        unit.has_cast_skill_this_turn = false;
        unit.moved = 3;
        unit.move_points = 2;
        assert!(matches!(
            is_able_to_cast(unit),
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

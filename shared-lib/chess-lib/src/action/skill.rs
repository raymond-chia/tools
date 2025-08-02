use crate::*;
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
    /// 回傳 Ok(訊息列表) 或 Err(錯誤)
    pub fn cast_skill(
        &self,
        board: &mut Board,
        skills: &BTreeMap<SkillID, Skill>,
        caster: UnitID,
        target: Pos,
    ) -> Result<Vec<String>, String> {
        // 施放前必須找到 unit，否則不能施放技能
        let unit = board.units.get(&caster).ok_or("找不到施法者 unit")?;
        if unit.has_cast_skill_this_turn {
            return Err("本回合已施放過技能，無法再次施放".to_string());
        }
        let skill_id = self.selected_skill.as_ref().ok_or("未選擇技能")?;
        let skill = skills
            .get(skill_id)
            .ok_or(format!("技能 {} 不存在", skill_id))?;
        // 只判斷第一個 effect 的 target_type
        let need_unit = skill
            .effects
            .get(0)
            .map(|e| e.is_targeting_unit())
            .ok_or("技能沒有有效的 effect")?;
        if need_unit {
            let has_target_unit = board.pos_to_unit.get(&target).is_some();
            if !has_target_unit {
                return Err(format!(
                    "技能 {} 無法作用於 ({:?})，目標格必須有單位",
                    skill_id, target
                ));
            }
        }

        // skill_affect_area 會檢查移動距離
        let affect_area = self.skill_affect_area(board, skills, caster, target);
        if affect_area.is_empty() {
            return Err(format!("技能 {} 無法作用於 ({:?})", skill_id, target));
        }
        let mut msgs = vec![format!("{} 在 ({}, {}) 施放", skill_id, target.x, target.y)];
        for pos in affect_area {
            for effect in &skill.effects {
                match effect {
                    Effect::Hp { value, .. } => {
                        if let Some(unit_id) = board.pos_to_unit.get(&pos) {
                            if let Some(unit) = board.units.get_mut(unit_id) {
                                let old = unit.hp;
                                unit.hp += value;
                                if unit.hp > unit.max_hp {
                                    unit.hp = unit.max_hp;
                                }
                                msgs.push(format!("單位 {} HP: {} → {}", unit_id, old, unit.hp));
                            }
                        }
                    }
                    Effect::MaxHp { duration, .. } => {
                        msgs.push(format!("[未實作] MaxHp 效果: 持續 {} 回合", duration));
                    }
                    Effect::Burn { duration, .. } => {
                        msgs.push(format!("[未實作] Burn 效果: 持續 {} 回合", duration));
                    }
                    Effect::MovePoints { value, .. } => {
                        msgs.push(format!("[未實作] 單位移動 {value}"));
                    }
                    Effect::HitAndRun { .. } => {
                        msgs.push(format!("[未實作] 打帶跑"));
                    }
                }
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
        let from = match board.unit_pos(&unit_id) {
            Some(p) => p,
            None => return vec![],
        };
        // 取得單位物件，檢查移動點數
        let unit = match board.units.get(&unit_id) {
            Some(u) => u,
            None => return vec![],
        };
        if unit.moved > unit.move_points {
            // 單位移動太遠，無法再使用技能
            return vec![];
        }
        // 判斷 to 是否在技能 range 內，超過則不顯示範圍
        if !is_in_skill_range(skill.range, from, to) {
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
    let unit_id = match board.pos_to_unit.get(&active_unit_pos) {
        Some(id) => id,
        None => return vec![],
    };
    let unit = match board.units.get(unit_id) {
        Some(u) => u,
        None => return vec![],
    };
    if unit.moved > unit.move_points {
        // 單位移動太遠，無法再使用技能
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
            if !is_in_skill_range(range, active_unit_pos, target) {
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

use inner::*;
mod inner {
    use super::*;

    /// 計算技能形狀範圍
    /// board: 棋盤物件, shape: 技能形狀, from: 施放者座標, to: 目標座標
    /// 回傳：座標列表
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

    /// 判斷技能施放距離是否符合 range 設定
    /// - skill: 技能資料
    /// - from: 施放者座標
    /// - to: 目標座標
    /// 回傳：是否符合技能距離限制
    pub fn is_in_skill_range(range: (usize, usize), from: Pos, to: Pos) -> bool {
        let dx = from.x as isize - to.x as isize;
        let dy = from.y as isize - to.y as isize;
        let dist = dx * dx + dy * dy;
        let dist = dist as usize;

        let (min_range, max_range) = (range.0 * range.0, range.1 * range.1);
        min_range <= dist && dist <= max_range
    }

    mod inner {}

    /// 將角度限制在 [-PI, PI] 範圍內
    fn clamp_pi(mut rad: f64) -> f64 {
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

        let mut pos_to_unit = HashMap::from([(pos, unit_id)]);
        let mut units = HashMap::from([(unit_id, unit)]);

        if let Some(pos_list) = extra_unit_pos {
            let mut next_id = unit_id;
            for p in pos_list {
                next_id += 1;
                let extra_template = template.clone();
                let mut extra_unit =
                    Unit::from_template(&marker, &extra_template, &skills).unwrap();
                extra_unit.id = next_id;
                pos_to_unit.insert(p, extra_unit.id);
                units.insert(extra_unit.id, extra_unit);
            }
        }

        let board = Board {
            tiles: vec![vec![Tile::default(); 10]; 10],
            teams,
            pos_to_unit,
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
        assert!(err.contains("未選擇技能"), "{err:?}");

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
        assert!(result2.unwrap_err().contains("本回合已施放過技能"));
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
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("無法作用於"), "{:?}", err_msg);

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
}

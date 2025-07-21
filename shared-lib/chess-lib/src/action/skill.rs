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

    /// 預覽技能範圍
    /// 根據目前選擇的技能與棋盤狀態，計算技能可作用的座標列表
    /// - board: 棋盤狀態
    /// - unit_id: 行動單位 ID
    /// - skills: 技能資料表
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

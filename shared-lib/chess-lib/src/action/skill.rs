use crate::{board::Board, *};
use skills_lib::*;
use std::collections::BTreeMap;

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
        if !is_skill_in_range(skill.range, from, to) {
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
            let r = *r as isize - 1; // 半徑減 1，因為中心點也算一格
            (-r..=r)
                .flat_map(|dx| (-r..=r).map(move |dy| (dx, dy)))
                .filter_map(|(dx, dy)| {
                    if dx.abs() + dy.abs() > r {
                        return None;
                    }
                    let x = to.x as isize + dx;
                    let y = to.y as isize + dy;
                    if x < 0 || y < 0 {
                        return None;
                    }
                    let target = Pos {
                        x: x as usize,
                        y: y as usize,
                    };
                    board.get_tile(target).map(|_| target)
                })
                .collect()
        }
        Shape::Rectangle(w, h) => (0..*w)
            .flat_map(|dx| (0..*h).map(move |dy| (dx, dy)))
            .filter_map(|(dx, dy)| {
                let x = to.x + dx;
                let y = to.y + dy;
                let target = Pos { x, y };
                board.get_tile(target).map(|_| target)
            })
            .collect(),
        Shape::Line(len) => {
            if from == to {
                return vec![];
            }
            let dx = to.x as isize - from.x as isize;
            let dy = to.y as isize - from.y as isize;
            let dist = ((dx * dx + dy * dy) as f64).sqrt();
            let sx = dx as f64 / dist;
            let sy = dy as f64 / dist;
            let mut x = from.x as f64;
            let mut y = from.y as f64;
            (0..*len)
                .filter_map(|_| {
                    x += sx;
                    y += sy;
                    let xi = x.round() as isize;
                    let yi = y.round() as isize;
                    if xi < 0 || yi < 0 {
                        return None;
                    }
                    let target = Pos {
                        x: xi as usize,
                        y: yi as usize,
                    };
                    board.get_tile(target).map(|_| target)
                })
                .collect()
        }
        Shape::Cone(len, degree) => {
            if from == to {
                return vec![];
            }
            // 以 45 度為單位，計算主方向（8向）
            // 取得主方向（上下左右斜角）
            let dx = to.x as isize - from.x as isize;
            let dy = to.y as isize - from.y as isize;
            let dir_x = match dx {
                0 => 0,
                _ if dx > 0 => 1,
                _ => -1,
            };
            let dir_y = match dy {
                0 => 0,
                _ if dy > 0 => 1,
                _ => -1,
            };
            // TODO 確認是否正確: 錐形寬度：每 45 度為 1 格寬 ??
            let cone_width = (*degree as isize / 45).max(1);
            (1..=*len)
                .flat_map(|step| {
                    let cx = from.x as isize + dir_x * step as isize;
                    let cy = from.y as isize + dir_y * step as isize;
                    (-cone_width..=cone_width).map(move |w| {
                        // 展開方向：垂直於主方向
                        if dir_x == 0 {
                            (cx + w, cy)
                        } else if dir_y == 0 {
                            (cx, cy + w)
                        } else {
                            // 斜角時，展開兩側
                            (cx + w, cy + w)
                        }
                    })
                })
                .filter(|(tx, ty)| {
                    *tx >= 0
                        && *ty >= 0
                        && (*tx as usize) < board.width()
                        && (*ty as usize) < board.height()
                })
                .filter_map(|(tx, ty)| {
                    let target = Pos {
                        x: tx as usize,
                        y: ty as usize,
                    };
                    board.get_tile(target).map(|_| target)
                })
                .collect()
        }
    }
}

/// 判斷技能施放距離是否符合 range 設定
/// - skill: 技能資料
/// - from: 施放者座標
/// - to: 目標座標
/// 回傳：是否符合技能距離限制
pub fn is_skill_in_range(range: (usize, usize), from: Pos, to: Pos) -> bool {
    let dx = from.x as isize - to.x as isize;
    let dy = from.y as isize - to.y as isize;
    let dist = dx.abs() + dy.abs();
    let (min_range, max_range) = (range.0 as isize, range.1 as isize);
    min_range <= dist && dist <= max_range
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
    let (min_range, max_range) = (range.0 as isize, range.1 as isize);
    for dy in -max_range..=max_range {
        for dx in -max_range..=max_range {
            let dist = dx.abs() + dy.abs();
            if dist < min_range || dist > max_range {
                continue;
            }
            let x = active_unit_pos.x as isize + dx;
            let y = active_unit_pos.y as isize + dy;
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
            area.push(pos);
        }
    }
    area
}

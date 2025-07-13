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
    pub fn preview_skill_area(
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
        calc_shape_area(board, from, to, shape)
    }
}

/// 計算技能形狀範圍
/// - board: 棋盤狀態
/// - pos: 技能施放中心座標
/// - shape: 技能形狀
/// 回傳：座標列表
pub fn calc_shape_area(board: &Board, from: Pos, to: Pos, shape: &Shape) -> Vec<Pos> {
    match shape {
        Shape::Point => vec![to],
        Shape::Circle(r) => {
            let r = *r as isize - 1; // 半徑減 1，因為中心點也算一格
            let mut area = Vec::new();
            for dx in -r..=r {
                for dy in -r..=r {
                    if dx.abs() + dy.abs() > r {
                        continue;
                    }
                    let x = to.x as isize + dx;
                    let y = to.y as isize + dy;
                    if x < 0 || y < 0 {
                        continue;
                    }
                    let target = Pos {
                        x: x as usize,
                        y: y as usize,
                    };
                    if board.get_tile(target).is_none() {
                        continue;
                    }
                    area.push(target);
                }
            }
            area
        }
        Shape::Rectangle(w, h) => {
            let mut area = Vec::new();
            for dx in 0..*w {
                for dy in 0..*h {
                    let x = to.x + dx;
                    let y = to.y + dy;
                    let target = Pos { x, y };
                    if board.get_tile(target).is_none() {
                        continue;
                    }
                    area.push(target);
                }
            }
            area
        }
        Shape::Line(len) => {
            // 若 from == to，回傳空 vec
            if from == to {
                return vec![];
            }
            // 依 from→to 方向，產生長度為 len 的直線座標
            let mut area = Vec::new();
            let dx = to.x as isize - from.x as isize;
            let dy = to.y as isize - from.y as isize;
            // 單位向量
            let dist = ((dx * dx + dy * dy) as f64).sqrt();
            let sx = dx as f64 / dist;
            let sy = dy as f64 / dist;
            let mut x = from.x as f64;
            let mut y = from.y as f64;
            for _ in 0..*len {
                x += sx;
                y += sy;
                let xi = x.round() as isize;
                let yi = y.round() as isize;
                if xi < 0 || yi < 0 {
                    break;
                }
                let target = Pos {
                    x: xi as usize,
                    y: yi as usize,
                };
                if board.get_tile(target).is_none() {
                    break;
                }
                area.push(target);
            }
            area
        }
        Shape::Cone(len, degree) => {
            // 若 from == to，回傳空 vec
            if from == to {
                return vec![];
            }
            // 以 45 度為單位，計算主方向（8向）
            let mut area = Vec::new();
            let dx = to.x as isize - from.x as isize;
            let dy = to.y as isize - from.y as isize;
            // 取得主方向（上下左右斜角）
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
            // TODO 確認是否正確
            // 錐形寬度：每 45 度為 1 格寬
            let cone_width = (*degree as isize / 45).max(1);
            for step in 1..=*len {
                let cx = from.x as isize + dir_x * step as isize;
                let cy = from.y as isize + dir_y * step as isize;
                for w in -cone_width..=cone_width {
                    // 展開方向：垂直於主方向
                    let (tx, ty) = if dir_x == 0 {
                        (cx + w, cy)
                    } else if dir_y == 0 {
                        (cx, cy + w)
                    } else {
                        // 斜角時，展開兩側
                        (cx + w, cy + w)
                    };
                    if tx < 0
                        || ty < 0
                        || tx as usize >= board.width()
                        || ty as usize >= board.height()
                    {
                        continue;
                    }
                    let target = Pos {
                        x: tx as usize,
                        y: ty as usize,
                    };
                    if board.get_tile(target).is_none() {
                        continue;
                    }
                    area.push(target);
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
pub fn skill_casting_area_around(board: &Board, from: Pos, range: (usize, usize)) -> Vec<Pos> {
    let mut area = Vec::new();
    let (min_range, max_range) = (range.0 as isize, range.1 as isize);
    for dy in -max_range..=max_range {
        for dx in -max_range..=max_range {
            let dist = dx.abs() + dy.abs();
            if dist < min_range || dist > max_range {
                continue;
            }
            let x = from.x as isize + dx;
            let y = from.y as isize + dy;
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

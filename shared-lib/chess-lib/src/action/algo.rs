use crate::*;
use std::collections::{BTreeSet, HashMap};

/// 路徑搜尋專用棋盤介面，供 dijkstra 演算法使用
pub trait PathfindingBoard {
    /// 判斷座標是否合法
    fn is_valid(&self, pos: Pos) -> bool;
    /// 判斷座標是否可通行
    fn is_passable(&self, active_unit_pos: Pos, pos: Pos, total: MovementCost) -> bool;
    /// 取得座標移動成本
    fn get_cost(&self, pos: Pos) -> MovementCost;
    /// 取得鄰近座標
    fn get_neighbors(&self, pos: Pos) -> Vec<Pos>;
}

// https://github.com/TheAlgorithms/Rust/blob/master/src/graph/dijkstra.rs
/// Dijkstra 最短路徑演算法，計算從起點到所有可達座標的最短距離與前驅座標
/// 回傳 HashMap<Pos, (MovementCost, Pos)>，key 為座標，value 為 (累積成本, 前驅座標)
pub fn dijkstra(graph: &impl PathfindingBoard, start: Pos) -> HashMap<Pos, (MovementCost, Pos)> {
    let mut ans = HashMap::new();
    let mut prio = BTreeSet::new();

    ans.insert(start, (0, start));

    // 初始化起點鄰居
    for new in graph.get_neighbors(start) {
        if !graph.is_valid(new) {
            continue;
        }
        let weight = graph.get_cost(new);
        if !graph.is_passable(start, new, weight) {
            continue;
        }
        ans.insert(new, (weight, start));
        prio.insert((weight, new));
    }

    // 主迴圈：每次取出最小成本座標，更新鄰居
    while let Some((path_weight, vertex)) = prio.pop_first() {
        for next in graph.get_neighbors(vertex) {
            if !graph.is_valid(next) {
                continue;
            }
            let new_weight = path_weight + graph.get_cost(next);
            if !graph.is_passable(start, next, new_weight) {
                continue;
            }
            match ans.get(&next) {
                Some((dist_next, _)) if new_weight >= *dist_next => {}
                _ => {
                    if let Some((prev_weight, _)) = ans.insert(next, (new_weight, vertex)) {
                        prio.remove(&(prev_weight, next));
                    };
                    prio.insert((new_weight, next));
                }
            }
        }
    }

    ans
}

pub fn bresenham_line(from: Pos, to: Pos, len: usize, is_valid: impl Fn(Pos) -> bool) -> Vec<Pos> {
    let mut points = Vec::new();

    let dx = (to.x as isize - from.x as isize).abs();
    let dy = (to.y as isize - from.y as isize).abs();
    let sx = if from.x < to.x { 1 } else { -1 };
    let sy = if from.y < to.y { 1 } else { -1 };

    let mut err = dx - dy;
    let mut x = from.x as isize;
    let mut y = from.y as isize;

    for _ in 0..len {
        if x < 0 || y < 0 {
            break;
        }
        let pos = Pos {
            x: x as usize,
            y: y as usize,
        };
        if is_valid(pos) {
            points.push(pos);
        } else {
            break; // 若超出板子範圍可終止
        }
        if x as usize == to.x && y as usize == to.y {
            break; // 若到達目標可終止
        }

        let e2 = err * 2;
        if e2 > -dy {
            err -= dy;
            x += sx;
        }
        if e2 < dx {
            err += dx;
            y += sy;
        }
    }
    points
}

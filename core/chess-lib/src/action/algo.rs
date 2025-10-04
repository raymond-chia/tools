//! 本檔案僅收錄「有名且有固定公式」的演算法。
//! 例如：A* 路徑尋找、Dijkstra、命中率 FEH 公式等。
//! 若為專案自訂、尚未標準化或僅用於單一場景的邏輯，請勿放於此處。
//! 請維護演算法的正確性、可重現性與註解完整性。
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct MockBoard {
        width: usize,
        height: usize,
        impassable: Vec<Pos>,
        slower: Vec<Pos>,
    }

    impl PathfindingBoard for MockBoard {
        fn is_valid(&self, pos: Pos) -> bool {
            pos.x < self.width && pos.y < self.height
        }
        fn is_passable(&self, _active_unit_pos: Pos, pos: Pos, _total: MovementCost) -> bool {
            !self.impassable.contains(&pos)
        }
        fn get_cost(&self, _pos: Pos) -> MovementCost {
            if self.slower.contains(&_pos) {
                20 // 模擬較慢的地形
            } else {
                10 // 普通地形
            }
        }
        fn get_neighbors(&self, pos: Pos) -> Vec<Pos> {
            let mut neighbors = Vec::new();
            let dirs = [(-1, 0), (1, 0), (0, -1), (0, 1)];
            for (dx, dy) in dirs {
                let nx = pos.x as isize + dx;
                let ny = pos.y as isize + dy;
                if nx >= 0 && ny >= 0 {
                    neighbors.push(Pos {
                        x: nx as usize,
                        y: ny as usize,
                    });
                }
            }
            neighbors
        }
    }

    #[test]
    fn test_dijkstra_simple() {
        let board = MockBoard {
            width: 3,
            height: 3,
            impassable: vec![],
            slower: vec![],
        };
        let test_data = [
            (
                Pos { x: 0, y: 0 },
                vec![
                    (Pos { x: 0, y: 0 }, 0),
                    (Pos { x: 1, y: 1 }, 20),
                    (Pos { x: 1, y: 2 }, 30),
                    (Pos { x: 2, y: 2 }, 40),
                ],
            ),
            (
                Pos { x: 1, y: 1 },
                vec![
                    (Pos { x: 0, y: 0 }, 20),
                    (Pos { x: 1, y: 1 }, 0),
                    (Pos { x: 1, y: 2 }, 10),
                    (Pos { x: 2, y: 2 }, 20),
                ],
            ),
            (
                Pos { x: 2, y: 0 },
                vec![
                    (Pos { x: 0, y: 0 }, 20),
                    (Pos { x: 1, y: 1 }, 20),
                    (Pos { x: 1, y: 2 }, 30),
                    (Pos { x: 2, y: 2 }, 20),
                ],
            ),
            (
                Pos { x: 2, y: 2 },
                vec![
                    (Pos { x: 0, y: 0 }, 40),
                    (Pos { x: 1, y: 1 }, 20),
                    (Pos { x: 1, y: 2 }, 10),
                    (Pos { x: 2, y: 2 }, 0),
                ],
            ),
        ];
        for (start, expected) in test_data {
            let result = dijkstra(&board, start);
            for (to, cost) in expected {
                assert_eq!(result.get(&to).unwrap().0, cost);
            }
        }
    }

    #[test]
    fn test_dijkstra_with_slower() {
        let board = MockBoard {
            width: 3,
            height: 3,
            impassable: vec![],
            slower: vec![Pos { x: 0, y: 1 }],
        };

        let test_data = [
            (
                Pos { x: 0, y: 0 },
                vec![
                    (Pos { x: 0, y: 0 }, 0),
                    (Pos { x: 1, y: 0 }, 10),
                    (Pos { x: 0, y: 1 }, 20),
                ],
            ),
            (
                Pos { x: 1, y: 0 },
                vec![
                    (Pos { x: 0, y: 0 }, 10),
                    (Pos { x: 1, y: 0 }, 0),
                    (Pos { x: 0, y: 1 }, 30),
                ],
            ),
            (
                Pos { x: 0, y: 1 },
                vec![
                    (Pos { x: 0, y: 0 }, 10),
                    (Pos { x: 1, y: 0 }, 20),
                    (Pos { x: 0, y: 1 }, 0),
                ],
            ),
        ];

        for (start, expected) in test_data {
            let result = dijkstra(&board, start);
            for (to, cost) in expected {
                assert_eq!(result.get(&to).unwrap().0, cost);
            }
        }
    }

    #[test]
    fn test_dijkstra_with_impassable() {
        let board = MockBoard {
            width: 3,
            height: 3,
            impassable: vec![Pos { x: 1, y: 0 }, Pos { x: 1, y: 1 }],
            slower: vec![],
        };

        let test_data = [
            (
                Pos { x: 0, y: 0 },
                vec![
                    (Pos { x: 0, y: 0 }, Some((0, Pos { x: 0, y: 0 }))),
                    (Pos { x: 1, y: 0 }, None),
                    (Pos { x: 1, y: 1 }, None),
                    (Pos { x: 0, y: 2 }, Some((20, Pos { x: 0, y: 1 }))),
                    (Pos { x: 2, y: 0 }, Some((60, Pos { x: 2, y: 1 }))),
                    (Pos { x: 2, y: 2 }, Some((40, Pos { x: 1, y: 2 }))),
                ],
            ),
            (
                Pos { x: 2, y: 0 },
                vec![
                    (Pos { x: 0, y: 0 }, Some((60, Pos { x: 0, y: 1 }))),
                    (Pos { x: 1, y: 0 }, None),
                    (Pos { x: 1, y: 1 }, None),
                    (Pos { x: 0, y: 2 }, Some((40, Pos { x: 1, y: 2 }))),
                    (Pos { x: 2, y: 0 }, Some((0, Pos { x: 2, y: 0 }))),
                    (Pos { x: 2, y: 2 }, Some((20, Pos { x: 2, y: 1 }))),
                ],
            ),
            (
                Pos { x: 2, y: 2 },
                vec![
                    (Pos { x: 0, y: 0 }, Some((40, Pos { x: 0, y: 1 }))),
                    (Pos { x: 1, y: 0 }, None),
                    (Pos { x: 1, y: 1 }, None),
                    (Pos { x: 0, y: 2 }, Some((20, Pos { x: 1, y: 2 }))),
                    (Pos { x: 2, y: 0 }, Some((20, Pos { x: 2, y: 1 }))),
                    (Pos { x: 2, y: 2 }, Some((0, Pos { x: 2, y: 2 }))),
                ],
            ),
        ];

        for (start, expected) in test_data {
            let result = dijkstra(&board, start);
            for (to, expected) in expected {
                match expected {
                    Some((cost, prestep)) => {
                        assert_eq!(
                            result.get(&to).unwrap().0,
                            cost,
                            "from {:?} to {:?} cost mismatch",
                            start,
                            to
                        );
                        assert_eq!(
                            result.get(&to).unwrap().1,
                            prestep,
                            "from {:?} to {:?} prestep mismatch",
                            start,
                            to
                        );
                    }
                    None => assert!(
                        result.get(&to).is_none(),
                        "from {:?} to {:?} should be unreachable",
                        start,
                        to
                    ),
                }
            }
        }
    }

    #[test]
    fn test_bresenham_line() {
        let test_data = [
            (
                10,
                Pos { x: 0, y: 0 },
                Pos { x: 3, y: 3 },
                vec![
                    Pos { x: 0, y: 0 },
                    Pos { x: 1, y: 1 },
                    Pos { x: 2, y: 2 },
                    Pos { x: 3, y: 3 },
                ],
            ),
            (
                2,
                Pos { x: 0, y: 0 },
                Pos { x: 3, y: 3 },
                vec![Pos { x: 0, y: 0 }, Pos { x: 1, y: 1 }],
            ),
            (
                5,
                Pos { x: 0, y: 0 },
                Pos { x: 3, y: 1 },
                vec![
                    Pos { x: 0, y: 0 },
                    Pos { x: 1, y: 0 },
                    Pos { x: 2, y: 1 },
                    Pos { x: 3, y: 1 },
                ],
            ),
        ];
        for (len, from, to, expected) in test_data {
            let line = bresenham_line(from, to, len, |_| true);
            assert_eq!(line, expected);
        }
    }

    #[test]
    fn test_bresenham_line_with_invalid() {
        let from = Pos { x: 0, y: 0 };
        let to = Pos { x: 3, y: 0 };
        let line = bresenham_line(from, to, 10, |pos| pos.x < 2);
        let expected = vec![Pos { x: 0, y: 0 }, Pos { x: 1, y: 0 }];
        assert_eq!(line, expected);
    }
}

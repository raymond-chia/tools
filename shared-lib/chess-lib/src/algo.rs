use crate::*;
use std::collections::{BTreeSet, HashMap};

pub trait Board {
    fn is_valid(&self, pos: Pos) -> bool;
    fn is_passable(&self, active_unit_pos: Pos, pos: Pos) -> bool;
    fn get_cost(&self, pos: Pos) -> MovementCost;
    fn get_neighbors(&self, pos: Pos) -> Vec<Pos>;
}

// https://github.com/TheAlgorithms/Rust/blob/master/src/graph/dijkstra.rs
pub fn dijkstra(graph: &impl Board, start: Pos) -> HashMap<Pos, (MovementCost, Pos)> {
    let mut ans = HashMap::new();
    let mut prio = BTreeSet::new();

    ans.insert(start, (0, start));

    for new in graph.get_neighbors(start) {
        if !graph.is_valid(new) {
            continue;
        }
        if !graph.is_passable(start, new) {
            continue;
        }
        let weight = graph.get_cost(new);
        ans.insert(new, (weight, start));
        prio.insert((weight, new));
    }

    while let Some((path_weight, vertex)) = prio.pop_first() {
        for next in graph.get_neighbors(vertex) {
            if !graph.is_valid(next) {
                continue;
            }
            if !graph.is_passable(start, next) {
                continue;
            }
            let new_weight = path_weight + graph.get_cost(next);
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

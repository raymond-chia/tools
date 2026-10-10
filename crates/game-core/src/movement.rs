//! 空間、地形通行、路徑搜尋與移動。
use crate::error::{self, GameError};
use crate::game::Game;
use crate::model::{Board, Footprint, GridPos, MovePreview, Phase, Pos, Turn, Unit};
use crate::terrain::{
    TerrainEntry, footprint_movement_cost, footprint_on_impassable, terrain_ends_movement,
};
use bevy_ecs::prelude::{Entity, World};
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, HashSet},
};

struct MovePlan {
    entity: Entity,
    turn: Turn,
    first_budget: u32,
    second_budget: u32,
    path: Vec<GridPos>,
}

impl Game {
    pub fn preview_move(&self, actor: i64, end: GridPos) -> Result<MovePreview, GameError> {
        let MovePlan {
            entity,
            turn: _,
            first_budget,
            second_budget: _,
            path,
        } = self.move_plan(actor, end)?;
        let footprint = *self
            .world
            .get::<Footprint>(entity)
            .expect("預覽移動的單位應具有 Footprint 元件");
        let interrupted = path
            .last()
            .is_some_and(|position| terrain_ends_movement(&self.world, *position, footprint));
        let mut first = Vec::new();
        let mut second = Vec::new();
        let mut spent = 0;
        if first_budget == 0 {
            second.push(path[0]);
        } else {
            first.push(path[0]);
        }
        for position in path.into_iter().skip(1) {
            spent += footprint_movement_cost(&self.world, position, footprint);
            if spent <= first_budget {
                first.push(position);
            } else {
                if second.is_empty() {
                    second.push(
                        *first
                            .last()
                            .expect("第二段移動開始前，第一段路徑應包含起點"),
                    );
                }
                second.push(position);
            }
        }
        Ok(MovePreview {
            first,
            second,
            interrupted,
            total_cost: spent,
        })
    }
    pub(crate) fn move_to(&mut self, a: i64, end: GridPos) -> Result<(), GameError> {
        let plan = self.move_plan(a, end)?;
        self.execute_move_plan(a, plan)
    }

    /// AI 使用已評估的路徑；移動結算與玩家共用。
    pub(crate) fn move_along(&mut self, actor: i64, path: Vec<GridPos>) -> Result<(), GameError> {
        self.ensure(actor)?;
        let entity = self.entity(actor).ok_or(error::missing_move_unit())?;
        let turn @ Turn {
            actor: _,
            phase: _,
            movement_remaining: _,
            movement_segments_used: _,
        } = self.world.resource::<Turn>().clone();
        if !can_move(&turn) {
            return Err(error::cannot_move());
        }
        let allowance = self
            .world
            .get::<Unit>(entity)
            .expect("移動單位應具有 Unit")
            .movement;
        let (first_budget, second_budget) = movement_budgets(&turn, allowance);
        self.execute_move_plan(
            actor,
            MovePlan {
                entity,
                turn,
                first_budget,
                second_budget,
                path,
            },
        )
    }

    fn execute_move_plan(&mut self, a: i64, plan: MovePlan) -> Result<(), GameError> {
        let MovePlan {
            entity: e,
            turn,
            first_budget,
            second_budget,
            path,
        } = plan;
        let Turn {
            actor: _,
            phase: _,
            movement_remaining: _,
            movement_segments_used,
        } = turn;
        let spent = self.execute_move_path(e, a, &path);
        if self.world.get_entity(e).is_err() {
            return Ok(());
        }
        self.activate_enemies_near(&[e]);
        let Turn {
            actor: _,
            phase,
            movement_remaining,
            movement_segments_used: segments_used,
        } = &mut *self.world.resource_mut::<Turn>();
        if movement_segments_used == 0 && spent <= first_budget {
            *movement_remaining = first_budget - spent;
        } else if movement_segments_used == 0 {
            *segments_used = 1;
            *movement_remaining = second_budget - (spent - first_budget);
        } else {
            *movement_remaining = second_budget - spent;
        }
        if *movement_remaining == 0 {
            *segments_used += 1;
            *phase = Phase::AfterMove;
        } else {
            *phase = Phase::Moving;
        }
        Ok(())
    }
    /// 玩家與敵方共用逐格移動及地形傷害結算；回傳實際消耗的移動成本。
    pub(crate) fn execute_move_path(&mut self, e: Entity, a: i64, path: &[GridPos]) -> u32 {
        let footprint = *self
            .world
            .get::<Footprint>(e)
            .expect("執行移動的單位應具有 Footprint 元件");
        let mut spent = 0;
        for p in path.iter().copied().skip(1) {
            spent += footprint_movement_cost(&self.world, p, footprint);
            self.world
                .get_mut::<Pos>(e)
                .expect("已建立的戰鬥單位應具有 Pos 元件")
                .0 = p;
            let ends_movement = terrain_ends_movement(&self.world, p, footprint);
            let downed = self.apply_terrain_entry(e, a, TerrainEntry::Movement);
            if ends_movement || downed {
                break;
            }
        }
        spent
    }
    fn move_plan(&self, actor: i64, end: GridPos) -> Result<MovePlan, GameError> {
        self.ensure(actor)?;
        let entity = self.entity(actor).ok_or(error::missing_move_unit())?;
        let allowance = self
            .world
            .get::<Unit>(entity)
            .expect("已建立的戰鬥單位應具有 Unit 元件")
            .movement;
        let turn @ Turn {
            actor: _,
            phase: _,
            movement_remaining: _,
            movement_segments_used: _,
        } = self.world.resource::<Turn>().clone();
        if !can_move(&turn) {
            return Err(error::cannot_move());
        }
        let start = self
            .world
            .get::<Pos>(entity)
            .expect("已建立的戰鬥單位應具有 Pos 元件")
            .0;
        let footprint = *self
            .world
            .get::<Footprint>(entity)
            .expect("已建立的戰鬥單位應具有 Footprint 元件");
        let (first_budget, second_budget) = movement_budgets(&turn, allowance);
        let first_path = if first_budget > 0 {
            find_path(&self.world, entity, start, end, footprint, first_budget)
        } else {
            None
        };
        // 玩家保留第一段路徑；只有第一段無路可走時才使用兩段額度。
        let mut path = first_path
            .or_else(|| {
                find_path(
                    &self.world,
                    entity,
                    start,
                    end,
                    footprint,
                    first_budget + second_budget,
                )
            })
            .ok_or(error::unreachable_destination())?;
        truncate_at_hazard(&self.world, footprint, &mut path);
        Ok(MovePlan {
            entity,
            turn,
            first_budget,
            second_budget,
            path,
        })
    }
}

pub(crate) fn footprint_cells(position: GridPos, footprint: Footprint) -> Vec<GridPos> {
    (position.y..position.y + footprint.height)
        .flat_map(|y| (position.x..position.x + footprint.width).map(move |x| GridPos { x, y }))
        .collect()
}
pub(crate) fn unit_at_cell(world: &World, position: GridPos) -> Option<Entity> {
    world
        .iter_entities()
        .find(|entity| {
            entity
                .get::<Pos>()
                .zip(entity.get::<Footprint>())
                .is_some_and(|(pos, footprint)| {
                    overlap(
                        position,
                        Footprint {
                            width: 1,
                            height: 1,
                        },
                        pos.0,
                        *footprint,
                    )
                })
        })
        .map(|entity| entity.id())
}

pub(crate) fn fits(b: &Board, p: GridPos, f: Footprint) -> bool {
    let Board {
        width,
        height,
        terrains: _,
        terrain_types: _,
    } = b;
    p.x >= 0
        && p.y >= 0
        && i64::from(p.x) + i64::from(f.width) <= i64::from(*width)
        && i64::from(p.y) + i64::from(f.height) <= i64::from(*height)
}
pub(crate) fn distance(a: GridPos, b: GridPos) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}
pub(crate) fn overlap(a: GridPos, af: Footprint, b: GridPos, bf: Footprint) -> bool {
    a.x < b.x + bf.width && a.x + af.width > b.x && a.y < b.y + bf.height && a.y + af.height > b.y
}
pub(crate) fn occupied(w: &World, ignore: Entity, p: GridPos, f: Footprint) -> bool {
    w.iter_entities().any(|e| {
        e.id() != ignore
            && e.get::<Pos>()
                .zip(e.get::<Footprint>())
                .is_some_and(|(q, g)| overlap(p, f, q.0, *g))
    })
}
pub(crate) fn entity_distance(w: &World, a: Entity, b: Entity) -> i32 {
    let ap = w.get::<Pos>(a).expect("已建立的戰鬥單位應具有 Pos 元件").0;
    let af = *w
        .get::<Footprint>(a)
        .expect("已建立的戰鬥單位應具有 Footprint 元件");
    let bp = w.get::<Pos>(b).expect("已建立的戰鬥單位應具有 Pos 元件").0;
    let bf = *w
        .get::<Footprint>(b)
        .expect("已建立的戰鬥單位應具有 Footprint 元件");
    footprint_distance(ap, af, bp, bf)
}
/// 單位佔用範圍到指定格子的距離；技能射程與目標選格共用。
pub(crate) fn footprint_cell_distance(
    position: GridPos,
    footprint: Footprint,
    cell: GridPos,
) -> i32 {
    footprint_distance(
        position,
        footprint,
        cell,
        Footprint {
            width: 1,
            height: 1,
        },
    )
}

pub(crate) fn footprint_distance(a: GridPos, af: Footprint, b: GridPos, bf: Footprint) -> i32 {
    let dx = (b.x - (a.x + af.width - 1))
        .max(a.x - (b.x + bf.width - 1))
        .max(0);
    let dy = (b.y - (a.y + af.height - 1))
        .max(a.y - (b.y + bf.height - 1))
        .max(0);
    dx + dy
}
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct PathState {
    danger: u32,
    position: GridPos,
}

#[derive(Eq)]
struct Node {
    movement: u32,
    state: PathState,
}
impl Ord for Node {
    fn cmp(&self, o: &Self) -> Ordering {
        o.state
            .danger
            .cmp(&self.state.danger)
            .then_with(|| o.movement.cmp(&self.movement))
            .then_with(|| self.state.position.x.cmp(&o.state.position.x))
            .then_with(|| self.state.position.y.cmp(&o.state.position.y))
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl PartialEq for Node {
    fn eq(&self, o: &Self) -> bool {
        self.movement == o.movement && self.state == o.state
    }
}

pub(crate) struct Paths {
    best: HashMap<GridPos, PathState>,
    previous: HashMap<PathState, PathState>,
}
fn paths(w: &World, e: Entity, start: GridPos, f: Footprint, budget: u32) -> Paths {
    let board = w.resource::<Board>();
    let start_state = PathState {
        danger: 0,
        position: start,
    };
    let mut dist = HashMap::from([(start_state, 0)]);
    let mut labels = HashMap::from([(start, vec![(0, 0)])]);
    let mut best = HashMap::new();
    let mut previous = HashMap::new();
    let mut heap = BinaryHeap::from([Node {
        movement: 0,
        state: start_state,
    }]);
    while let Some(Node { movement, state }) = heap.pop() {
        if movement
            > *dist
                .get(&state)
                .expect("加入搜尋佇列的狀態應已有移動成本紀錄")
        {
            continue;
        }
        best.entry(state.position).or_insert(state);
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let n = GridPos {
                x: state.position.x + dx,
                y: state.position.y + dy,
            };
            if !fits(board, n, f) || occupied(w, e, n, f) || footprint_on_impassable(w, n, f) {
                continue;
            }
            let next_state = PathState {
                danger: state.danger + u32::from(terrain_ends_movement(w, n, f)),
                position: n,
            };
            let next_movement = movement + footprint_movement_cost(w, n, f);
            let dominated = labels.get(&n).is_some_and(|known| {
                known.iter().any(|(danger, movement)| {
                    *danger <= next_state.danger && *movement <= next_movement
                })
            });
            if next_movement <= budget && !dominated {
                dist.insert(next_state, next_movement);
                labels
                    .entry(n)
                    .or_default()
                    .push((next_state.danger, next_movement));
                previous.insert(next_state, state);
                heap.push(Node {
                    movement: next_movement,
                    state: next_state,
                })
            }
        }
    }
    Paths { best, previous }
}
pub(crate) fn find_path(
    w: &World,
    e: Entity,
    s: GridPos,
    end: GridPos,
    f: Footprint,
    b: u32,
) -> Option<Vec<GridPos>> {
    let search = paths(w, e, s, f, b);
    reconstruct_path(&search, s, end)
}

fn reconstruct_path(search: &Paths, start: GridPos, end: GridPos) -> Option<Vec<GridPos>> {
    let Paths { best, previous } = search;
    let mut state = *best.get(&end)?;
    let mut out = vec![state.position];
    while state.position != start {
        state = *previous
            .get(&state)
            .expect("非起點的最佳尋路狀態必須有前一個狀態");
        out.push(state.position)
    }
    out.reverse();
    Some(out)
}
fn truncate_at_hazard(world: &World, footprint: Footprint, path: &mut Vec<GridPos>) {
    if let Some(index) = path
        .iter()
        .skip(1)
        .position(|position| terrain_ends_movement(world, *position, footprint))
    {
        path.truncate(index + 2);
    }
}

pub(crate) struct MovementOption {
    pub(crate) position: GridPos,
    pub(crate) path: Vec<GridPos>,
    pub(crate) cost: u32,
}

/// 同一批候選位置共用尋路結果；沿用玩家預覽的第一段優先與危險地形截斷。
pub(crate) fn movement_options(
    world: &World,
    entity: Entity,
    turn: &Turn,
    include_second: bool,
) -> Vec<MovementOption> {
    let start = world.get::<Pos>(entity).expect("移動單位應具有位置").0;
    let mut options = vec![MovementOption {
        position: start,
        path: vec![start],
        cost: 0,
    }];
    if !can_move(turn) {
        return options;
    }
    let footprint = *world
        .get::<Footprint>(entity)
        .expect("移動單位應具有佔用尺寸");
    let allowance = world
        .get::<Unit>(entity)
        .expect("移動單位應具有 Unit")
        .movement;
    let (first_budget, second_budget) = movement_budgets(turn, allowance);
    let first = if first_budget > 0 {
        Some(paths(world, entity, start, footprint, first_budget))
    } else {
        None
    };
    let second = if include_second {
        Some(paths(
            world,
            entity,
            start,
            footprint,
            first_budget + second_budget,
        ))
    } else {
        None
    };
    for (is_second, search) in first
        .iter()
        .map(|search| (false, search))
        .chain(second.iter().map(|search| (true, search)))
    {
        let Paths { best, previous: _ } = search;
        let mut positions: Vec<_> = best
            .keys()
            .copied()
            .filter(|position| {
                *position != start
                    // 第一段只排除實際可到達的停點；被尖刺截斷的路徑不能遮蔽第二段安全路徑。
                    && !(is_second && options.iter().any(|option| option.position == *position))
            })
            .collect();
        positions.sort_by_key(|position| (position.y, position.x));
        for position in positions {
            let mut path =
                reconstruct_path(search, start, position).expect("搜尋結果中的位置應可重建路徑");
            truncate_at_hazard(world, footprint, &mut path);
            if path.last() != Some(&position) {
                continue;
            }
            let cost = path
                .iter()
                .skip(1)
                .map(|position| footprint_movement_cost(world, *position, footprint))
                .sum();
            options.push(MovementOption {
                position,
                path,
                cost,
            });
        }
    }
    options
}

/// 同一次接敵規劃共用全圖可達路徑，搜尋結果不依目標或技能改變。
pub(crate) fn full_board_paths(w: &World, e: Entity, s: GridPos, f: Footprint) -> Paths {
    let Board {
        width,
        height,
        terrains: _,
        terrain_types: _,
    } = w.resource::<Board>();
    let Footprint {
        width: footprint_width,
        height: footprint_height,
    } = f;
    let search_budget = (0..=*height - footprint_height)
        .flat_map(|y| (0..=*width - footprint_width).map(move |x| GridPos { x, y }))
        .fold(0_u32, |total, cell| {
            total.saturating_add(footprint_movement_cost(w, cell, f))
        });
    paths(w, e, s, f, search_budget)
}

pub(crate) fn toward_skill_range(
    w: &World,
    search: &Paths,
    s: GridPos,
    target: GridPos,
    target_footprint: Footprint,
    f: Footprint,
    b: u32,
    min_range: i32,
    max_range: i32,
    preferred_range: i32,
) -> Option<Vec<GridPos>> {
    let Paths { best, previous: _ } = search;
    // 先確認整張地圖上可到達合法射程，再依本回合預算截取路徑；完全無路可達時不靠牆追敵。
    let end = *best
        .keys()
        .filter(|p| {
            footprint_cells(target, target_footprint)
                .into_iter()
                .any(|cell| {
                    crate::sight::check_sight_in_range(w, **p, f, cell, min_range, max_range)
                        .is_ok()
                })
        })
        .min_by_key(|p| {
            let range = footprint_distance(**p, f, target, target_footprint);
            ((range - preferred_range).abs(), distance(**p, s), p.y, p.x)
        })?;
    let mut route = reconstruct_path(search, s, end).expect("選出的終點應可重建路徑");
    let mut spent = 0;
    let mut steps = 1;
    for cell in route.iter().skip(1) {
        let cost = footprint_movement_cost(w, *cell, f);
        if cost > b.saturating_sub(spent) {
            break;
        }
        spent += cost;
        steps += 1;
        if terrain_ends_movement(w, *cell, f) {
            break;
        }
    }
    route.truncate(steps);
    Some(route)
}
fn reach(w: &World, e: Entity, b: u32) -> Vec<GridPos> {
    let p = w.get::<Pos>(e).expect("已建立的戰鬥單位應具有 Pos 元件").0;
    let f = *w
        .get::<Footprint>(e)
        .expect("已建立的戰鬥單位應具有 Footprint 元件");
    let Paths { best, previous: _ } = paths(w, e, p, f, b);
    let mut v: Vec<_> = best.into_keys().collect();
    v.sort_by_key(|p| (p.y, p.x));
    v
}
/// 本回合是否還能移動；每回合最多兩段移動。
pub(crate) fn can_move(turn: &Turn) -> bool {
    let Turn {
        actor: _,
        phase,
        movement_remaining: _,
        movement_segments_used,
    } = turn;
    matches!(phase, Phase::Ready | Phase::Moving | Phase::AfterMove) && *movement_segments_used < 2
}

/// 第一段尚未用完時保留第二段完整額度；進入第二段後只使用剩餘額度。
pub(crate) fn movement_budgets(turn: &Turn, allowance: u32) -> (u32, u32) {
    let Turn {
        actor: _,
        phase,
        movement_remaining,
        movement_segments_used,
    } = turn;
    if *movement_segments_used == 0 {
        (*movement_remaining, allowance)
    } else if *phase == Phase::AfterMove {
        (0, allowance)
    } else {
        (0, *movement_remaining)
    }
}

/// 呼叫端須先以 can_move 確認仍可移動。
pub(crate) fn movement_ranges(w: &World, e: Entity, turn: &Turn) -> (Vec<GridPos>, Vec<GridPos>) {
    let allowance = w
        .get::<Unit>(e)
        .expect("已建立的戰鬥單位應具有 Unit 元件")
        .movement;
    let (first_budget, second_budget) = movement_budgets(turn, allowance);
    let reachable = if first_budget > 0 {
        reach(w, e, first_budget)
    } else {
        Vec::new()
    };
    let first_cells: HashSet<_> = reachable.iter().copied().collect();
    let second_reachable = reach(w, e, first_budget + second_budget)
        .into_iter()
        .filter(|cell| !first_cells.contains(cell))
        .collect();
    (reachable, second_reachable)
}

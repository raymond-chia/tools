//! 空間、地形通行、路徑搜尋與移動。
use crate::error::{self, GameError};
use crate::game::Game;
use crate::model::{
    Footprint, GridPos, Log, MovePreview, MovementTransition, Phase, Pos, TerrainGrid, Turn, Unit,
};
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
        let grid = self.world.resource::<TerrainGrid>();
        let interrupted = path
            .last()
            .is_some_and(|position| terrain_ends_movement(grid, *position, footprint));
        let mut first = Vec::new();
        let mut second = Vec::new();
        let mut spent = 0;
        if first_budget == 0 {
            second.push(path[0]);
        } else {
            first.push(path[0]);
        }
        for position in path.into_iter().skip(1) {
            spent += footprint_movement_cost(grid, position, footprint);
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
        if !self.finish_position_change(e, a) {
            return Ok(());
        }
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
    /// 普通移動與位移技能共用落點收尾；回傳單位是否仍在場上。
    pub(crate) fn finish_position_change(&mut self, entity: Entity, actor: i64) -> bool {
        if self.world.get_entity(entity).is_err() || self.escape_with_cargo(entity, actor) {
            return false;
        }
        self.activate_enemies_near(&[entity]);
        true
    }
    /// 玩家與敵方共用逐格移動及地形傷害結算；回傳實際消耗的移動成本。
    pub(crate) fn execute_move_path(&mut self, e: Entity, a: i64, path: &[GridPos]) -> u32 {
        let footprint = *self
            .world
            .get::<Footprint>(e)
            .expect("執行移動的單位應具有 Footprint 元件");
        let log_insert_index = self.world.resource::<Log>().0.len();
        let mut actual_path = vec![*path.first().expect("已規劃的移動路徑應包含起點")];
        let mut spent = 0;
        for p in path.iter().copied().skip(1) {
            let grid = self.world.resource::<TerrainGrid>();
            spent += footprint_movement_cost(grid, p, footprint);
            let ends_movement = terrain_ends_movement(grid, p, footprint);
            self.world
                .get_mut::<Pos>(e)
                .expect("已建立的戰鬥單位應具有 Pos 元件")
                .0 = p;
            actual_path.push(p);
            let downed = self.apply_terrain_entry(e, a, TerrainEntry::Movement);
            if ends_movement || downed {
                break;
            }
        }
        if actual_path.len() > 1 {
            self.movements.push(MovementTransition {
                unit_id: a,
                path: actual_path,
                log_insert_index,
            });
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
        truncate_at_hazard(self.world.resource::<TerrainGrid>(), footprint, &mut path);
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

pub(crate) fn fits(width: i32, height: i32, p: GridPos, f: Footprint) -> bool {
    p.x >= 0
        && p.y >= 0
        && i64::from(p.x) + i64::from(f.width) <= i64::from(width)
        && i64::from(p.y) + i64::from(f.height) <= i64::from(height)
}
pub(crate) fn distance(a: GridPos, b: GridPos) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}
pub(crate) fn overlap(a: GridPos, af: Footprint, b: GridPos, bf: Footprint) -> bool {
    a.x < b.x + bf.width && a.x + af.width > b.x && a.y < b.y + bf.height && a.y + af.height > b.y
}
/// 每格上的單位；單位不會重疊，每格最多一個。需要逐格查詢多次時先整理一次。
pub(crate) fn unit_cells(w: &World) -> HashMap<GridPos, Entity> {
    w.iter_entities()
        .filter_map(|e| Some((e.id(), e.get::<Pos>()?, e.get::<Footprint>()?)))
        .flat_map(|(entity, position, footprint)| {
            footprint_cells(position.0, *footprint)
                .into_iter()
                .map(move |cell| (cell, entity))
        })
        .collect()
}

/// 除了 ignore 以外所有單位佔用的格子；同一次搜尋只建立一次。
pub(crate) fn occupied_cells(w: &World, ignore: Entity) -> HashSet<GridPos> {
    unit_cells(w)
        .into_iter()
        .filter(|(_, entity)| *entity != ignore)
        .map(|(cell, _)| cell)
        .collect()
}

pub(crate) fn footprint_blocked(blocked: &HashSet<GridPos>, p: GridPos, f: Footprint) -> bool {
    footprint_cells(p, f)
        .iter()
        .any(|cell| blocked.contains(cell))
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

#[derive(Clone, Copy)]
pub(crate) enum PathPriority {
    Safety,
    MovementCost,
}

#[derive(Eq)]
struct Node {
    movement: u32,
    state: PathState,
    priority: (u32, u32),
}
impl Ord for Node {
    fn cmp(&self, o: &Self) -> Ordering {
        o.priority
            .cmp(&self.priority)
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
        self.priority == o.priority && self.state == o.state
    }
}

pub(crate) struct Paths {
    best: HashMap<GridPos, PathState>,
    previous: HashMap<PathState, PathState>,
}
fn paths(
    grid: &TerrainGrid,
    blocked: &HashSet<GridPos>,
    start: GridPos,
    f: Footprint,
    budget: u32,
    priority: PathPriority,
) -> Paths {
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
        priority: (0, 0),
    }]);
    while let Some(Node {
        movement,
        state,
        priority: _,
    }) = heap.pop()
    {
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
            if !fits(grid.width, grid.height, n, f)
                || footprint_blocked(blocked, n, f)
                || footprint_on_impassable(grid, n, f)
            {
                continue;
            }
            let next_state = PathState {
                danger: state.danger + u32::from(terrain_ends_movement(grid, n, f)),
                position: n,
            };
            let next_movement = movement + footprint_movement_cost(grid, n, f);
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
                    priority: match priority {
                        PathPriority::Safety => (next_state.danger, next_movement),
                        PathPriority::MovementCost => (next_movement, next_state.danger),
                    },
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
    let search = paths(
        w.resource::<TerrainGrid>(),
        &occupied_cells(w, e),
        s,
        f,
        b,
        PathPriority::Safety,
    );
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
fn truncate_at_hazard(grid: &TerrainGrid, footprint: Footprint, path: &mut Vec<GridPos>) {
    if let Some(index) = path
        .iter()
        .skip(1)
        .position(|position| terrain_ends_movement(grid, *position, footprint))
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
    let grid = world.resource::<TerrainGrid>();
    let blocked = occupied_cells(world, entity);
    let first = if first_budget > 0 {
        Some(paths(
            grid,
            &blocked,
            start,
            footprint,
            first_budget,
            PathPriority::Safety,
        ))
    } else {
        None
    };
    let second = if include_second {
        Some(paths(
            grid,
            &blocked,
            start,
            footprint,
            first_budget + second_budget,
            PathPriority::Safety,
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
            truncate_at_hazard(grid, footprint, &mut path);
            if path.last() != Some(&position) {
                continue;
            }
            let cost = path
                .iter()
                .skip(1)
                .map(|position| footprint_movement_cost(grid, *position, footprint))
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
pub(crate) fn full_board_paths(
    grid: &TerrainGrid,
    blocked: &HashSet<GridPos>,
    s: GridPos,
    f: Footprint,
    priority: PathPriority,
) -> Paths {
    let Footprint {
        width: footprint_width,
        height: footprint_height,
    } = f;
    let search_budget = (0..=grid.height - footprint_height)
        .flat_map(|y| (0..=grid.width - footprint_width).map(move |x| GridPos { x, y }))
        .fold(0_u32, |total, cell| {
            total.saturating_add(footprint_movement_cost(grid, cell, f))
        });
    paths(grid, blocked, s, f, search_budget, priority)
}

/// 從共用全圖搜尋結果選出實際移動成本最低的合法終點。
pub(crate) fn shortest_route(
    grid: &TerrainGrid,
    search: &Paths,
    start: GridPos,
    footprint: Footprint,
    accepts: impl Fn(GridPos) -> bool,
) -> Option<MovementOption> {
    let Paths { best, previous: _ } = search;
    best.keys()
        .copied()
        .filter(|position| accepts(*position))
        .map(|position| {
            let path =
                reconstruct_path(search, start, position).expect("全圖搜尋中的終點應可重建路徑");
            let cost = path
                .iter()
                .skip(1)
                .map(|cell| footprint_movement_cost(grid, *cell, footprint))
                .sum();
            MovementOption {
                position,
                path,
                cost,
            }
        })
        .min_by_key(
            |MovementOption {
                 position,
                 path: _,
                 cost,
             }| (*cost, position.y, position.x),
        )
}

pub(crate) fn toward_skill_range(
    grid: &TerrainGrid,
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
                    crate::sight::check_sight_in_range(grid, **p, f, cell, min_range, max_range)
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
        let cost = footprint_movement_cost(grid, *cell, f);
        if cost > b.saturating_sub(spent) {
            break;
        }
        spent += cost;
        steps += 1;
        if terrain_ends_movement(grid, *cell, f) {
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
    let Paths { best, previous: _ } = paths(
        w.resource::<TerrainGrid>(),
        &occupied_cells(w, e),
        p,
        f,
        b,
        PathPriority::Safety,
    );
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

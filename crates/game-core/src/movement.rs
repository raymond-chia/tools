//! 空間、地形通行、路徑搜尋與移動。
use crate::game::Game;
use crate::gameplay_config;
use crate::model::{
    Board, CombatLogEvent, Downed, Encounter, Footprint, ForcedEntry, GridPos, Hp, Id, Log,
    MovePreview, Phase, Pos, Team, TemporaryTerrains, TerrainTypeDef, Turn, Unit,
};
use bevy_ecs::prelude::{Entity, World};
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, HashSet},
};

pub(crate) fn terrain_type<'a>(board: &'a Board, kind: &str) -> &'a TerrainTypeDef {
    board
        .terrain_types
        .get(kind)
        .expect("載入時已驗證所有地形種類")
}

pub(crate) fn terrain_damage(board: &Board, kind: &str) -> i32 {
    terrain_type(board, kind).damage
}

struct MovePlan {
    entity: Entity,
    turn: Turn,
    first_budget: u32,
    second_budget: u32,
    path: Vec<GridPos>,
}

impl Game {
    pub fn preview_move(&self, actor: &str, end: GridPos) -> Result<MovePreview, String> {
        let MovePlan {
            entity: _,
            turn: _,
            first_budget,
            second_budget: _,
            path,
        } = self.move_plan(actor, end)?;
        let interrupted = path
            .last()
            .is_some_and(|position| terrain_ends_movement(&self.world, *position));
        let mut first = Vec::new();
        let mut second = Vec::new();
        let mut spent = 0;
        if first_budget == 0 {
            second.push(path[0]);
        } else {
            first.push(path[0]);
        }
        for position in path.into_iter().skip(1) {
            spent += movement_cost(&self.world, position);
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
    pub(crate) fn move_to(&mut self, a: &str, end: GridPos) -> Result<(), String> {
        let MovePlan {
            entity: e,
            turn,
            first_budget,
            second_budget,
            path,
        } = self.move_plan(a, end)?;
        let mut spent = 0;
        for p in path.into_iter().skip(1) {
            spent += movement_cost(&self.world, p);
            self.world
                .get_mut::<Pos>(e)
                .expect("已建立的戰鬥單位應具有 Pos 元件")
                .0 = p;
            self.reveal(e);
            if let Some(k) = terrain_at(&self.world, p) {
                if !terrain_ends_movement(&self.world, p) {
                    continue;
                }
                let unit = self
                    .world
                    .get::<Unit>(e)
                    .expect("已建立的戰鬥單位應具有 Unit 元件");
                let target = unit.name.clone();
                let target_team = unit.team.clone();
                let terrain_definition = terrain_type(self.world.resource::<Board>(), &k);
                let terrain_name_key = terrain_definition.name_key.clone();
                let log_key = terrain_definition
                    .forced_entry_log_key
                    .clone()
                    .unwrap_or_else(|| "COMBAT_LOG_TERRAIN_DAMAGE".into());
                let damage = terrain_definition.damage;
                if damage > 0 {
                    let mut hp = self
                        .world
                        .get_mut::<Hp>(e)
                        .expect("已建立的戰鬥單位應具有 Hp 元件");
                    hp.current = (hp.current - damage).max(0);
                    let remaining_hp = hp.current;
                    let max_hp = hp.maximum;
                    let downed = remaining_hp == 0;
                    if downed {
                        self.world.entity_mut(e).insert(Downed);
                        self.world
                            .resource_mut::<Encounter>()
                            .participants
                            .remove(a);
                    }
                    self.world
                        .resource_mut::<Log>()
                        .0
                        .push(CombatLogEvent::TerrainDamage {
                            target,
                            target_team,
                            terrain: k,
                            terrain_name_key,
                            log_key,
                            damage,
                            remaining_hp,
                            max_hp,
                            downed,
                        });
                } else {
                    self.world
                        .resource_mut::<Log>()
                        .0
                        .push(CombatLogEvent::StatusApplied {
                            target,
                            target_team,
                            status: k,
                            status_name_key: terrain_name_key,
                        });
                }
                break;
            }
        }
        let mut t = self.world.resource_mut::<Turn>();
        if turn.moves == 0 && spent <= first_budget {
            t.remaining = first_budget - spent;
        } else if turn.moves == 0 {
            t.moves = 1;
            t.remaining = second_budget - (spent - first_budget);
        } else {
            t.remaining = second_budget - spent;
        }
        if t.remaining == 0 {
            t.moves += 1;
            t.phase = Phase::AfterMove;
        } else {
            t.phase = Phase::Moving;
        }
        Ok(())
    }
    fn move_plan(&self, actor: &str, end: GridPos) -> Result<MovePlan, String> {
        self.ensure(actor)?;
        let entity = self.entity(actor).ok_or("找不到移動單位")?;
        let allowance = self
            .world
            .get::<Unit>(entity)
            .expect("已建立的戰鬥單位應具有 Unit 元件")
            .movement;
        let turn = self.world.resource::<Turn>().clone();
        if !matches!(turn.phase, Phase::Ready | Phase::Moving | Phase::AfterMove) || turn.moves >= 2
        {
            return Err("目前不能移動".into());
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
        let first_budget = if turn.moves == 0 { turn.remaining } else { 0 };
        let second_budget = allowance;
        let first_path = if first_budget > 0 {
            find_path(&self.world, entity, start, end, footprint, first_budget)
        } else {
            None
        };
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
            .ok_or("目的地不可達")?;
        if let Some(trigger_index) = path
            .iter()
            .skip(1)
            .position(|position| terrain_ends_movement(&self.world, *position))
        {
            path.truncate(trigger_index + 2);
        }
        Ok(MovePlan {
            entity,
            turn,
            first_budget,
            second_budget,
            path,
        })
    }
    fn reveal(&mut self, mover: Entity) {
        let p = self
            .world
            .get::<Pos>(mover)
            .expect("已建立的戰鬥單位應具有 Pos 元件")
            .0;
        let active = self.world.resource::<Encounter>().participants.clone();
        let add: Vec<_> = self
            .world
            .query::<(&Id, &Pos, &Unit)>()
            .iter(&self.world)
            .filter(|(i, q, f)| {
                f.team != Team::Player
                    && !active.contains(&i.0)
                    && distance(p, q.0) <= gameplay_config::ENEMY_REVEAL_RANGE
            })
            .map(|(i, _, _)| i.0.clone())
            .collect();
        if !add.is_empty() {
            self.world
                .resource_mut::<Encounter>()
                .participants
                .extend(add);
        }
    }
}

pub(crate) fn terrain_at(w: &World, position: GridPos) -> Option<String> {
    w.resource::<TemporaryTerrains>()
        .0
        .get(&position)
        .map(|terrain| terrain.kind.clone())
        .or_else(|| w.resource::<Board>().triggers.get(&position).cloned())
}

pub(crate) fn movement_cost(w: &World, position: GridPos) -> u32 {
    let board = w.resource::<Board>();
    let base = board.costs[(position.y * board.width + position.x) as usize];
    base + terrain_at(w, position)
        .map(|kind| terrain_type(board, &kind).movement_cost_bonus)
        .unwrap_or(0)
}

pub(crate) fn terrain_ends_movement(w: &World, position: GridPos) -> bool {
    terrain_at(w, position)
        .is_some_and(|kind| terrain_type(w.resource::<Board>(), &kind).ends_movement)
}

pub(crate) fn footprint_on_impassable(
    board: &Board,
    position: GridPos,
    footprint: Footprint,
) -> bool {
    (position.y..position.y + footprint.height).any(|y| {
        (position.x..position.x + footprint.width).any(|x| {
            board
                .triggers
                .get(&GridPos { x, y })
                .is_some_and(|kind| !terrain_type(board, kind).passable)
        })
    })
}

pub(crate) fn footprint_blocks_forced_entry(
    board: &Board,
    position: GridPos,
    footprint: Footprint,
) -> bool {
    (position.y..position.y + footprint.height).any(|y| {
        (position.x..position.x + footprint.width).any(|x| {
            board
                .triggers
                .get(&GridPos { x, y })
                .is_some_and(|kind| terrain_type(board, kind).forced_entry == ForcedEntry::Blocked)
        })
    })
}

pub(crate) fn footprint_cells(position: GridPos, footprint: Footprint) -> Vec<GridPos> {
    (position.y..position.y + footprint.height)
        .flat_map(|y| (position.x..position.x + footprint.width).map(move |x| GridPos { x, y }))
        .collect()
}
pub(crate) fn fits(b: &Board, p: GridPos, f: Footprint) -> bool {
    p.x >= 0 && p.y >= 0 && p.x + f.width <= b.width && p.y + f.height <= b.height
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
            && e.get::<Downed>().is_none()
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

struct Paths {
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
            if !fits(board, n, f) || occupied(w, e, n, f) || footprint_on_impassable(board, n, f) {
                continue;
            }
            let next_state = PathState {
                danger: state.danger + u32::from(terrain_ends_movement(w, n)),
                position: n,
            };
            let next_movement = movement + movement_cost(w, n);
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
    let Paths { best, previous } = paths(w, e, s, f, b);
    let mut state = *best.get(&end)?;
    let mut out = vec![state.position];
    while state.position != s {
        state = *previous
            .get(&state)
            .expect("非起點的最佳尋路狀態必須有前一個狀態");
        out.push(state.position)
    }
    out.reverse();
    Some(out)
}
pub(crate) fn toward(
    w: &World,
    e: Entity,
    s: GridPos,
    target: GridPos,
    f: Footprint,
    b: u32,
) -> Option<Vec<GridPos>> {
    let board = w.resource::<Board>();
    let search_budget = (0..board.height)
        .flat_map(|y| (0..board.width).map(move |x| GridPos { x, y }))
        .fold(0_u32, |total, cell| {
            total.saturating_add(movement_cost(w, cell))
        });
    let Paths { best, previous } = paths(w, e, s, f, search_budget);
    let end = *best
        .keys()
        .min_by_key(|p| (distance(**p, target), p.y, p.x))?;
    let mut state = *best.get(&end).expect("選出的終點應有尋路狀態");
    let mut route = vec![state.position];
    while state.position != s {
        state = *previous
            .get(&state)
            .expect("非起點的最佳尋路狀態必須有前一個狀態");
        route.push(state.position);
    }
    route.reverse();
    let mut spent = 0;
    let mut steps = 1;
    for cell in route.iter().skip(1) {
        let cost = movement_cost(w, *cell);
        if cost > b.saturating_sub(spent) {
            break;
        }
        spent += cost;
        steps += 1;
        if terrain_ends_movement(w, *cell) {
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
    let mut v: Vec<_> = paths(w, e, p, f, b).best.into_keys().collect();
    v.sort_by_key(|p| (p.y, p.x));
    v
}
pub(crate) fn movement_ranges(w: &World, e: Entity, turn: &Turn) -> (Vec<GridPos>, Vec<GridPos>) {
    if turn.moves >= 2 {
        return (Vec::new(), Vec::new());
    }
    let allowance = w
        .get::<Unit>(e)
        .expect("已建立的戰鬥單位應具有 Unit 元件")
        .movement;
    let first_budget = if turn.moves == 0 { turn.remaining } else { 0 };
    let reachable = if first_budget > 0 {
        reach(w, e, first_budget)
    } else {
        Vec::new()
    };
    let first_cells: HashSet<_> = reachable.iter().copied().collect();
    let second_reachable = reach(w, e, first_budget + allowance)
        .into_iter()
        .filter(|cell| !first_cells.contains(cell))
        .collect();
    (reachable, second_reachable)
}

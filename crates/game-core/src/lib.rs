//! Godot 無關的權威戰棋核心。
use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, HashSet},
};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct GridPos {
    pub x: i32,
    pub y: i32,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Team {
    Player,
    Enemy,
}
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Ongoing,
    Victory,
    Defeat,
}
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RollDegree {
    CriticalFailure,
    Failure,
    Success,
    CriticalSuccess,
}
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AttackResult {
    Dodge,
    Block,
    Hit,
}
#[derive(Component)]
struct Id(String);
#[derive(Component, Clone, Copy)]
struct Pos(GridPos);
#[derive(Component, Clone, Copy)]
struct Footprint {
    width: i32,
    height: i32,
}
#[derive(Component)]
struct Hp {
    current: i32,
    maximum: i32,
}
#[derive(Component, Clone)]
struct Fighter {
    name: String,
    team: Team,
    group: String,
    movement: u32,
    initiative: i32,
    dodge: i32,
    block: i32,
    melee: i32,
    ranged: i32,
    damage: i32,
    range: i32,
}
#[derive(Component)]
struct Downed;
#[derive(Resource, Clone)]
struct Board {
    width: i32,
    height: i32,
    costs: Vec<u32>,
    triggers: HashMap<GridPos, String>,
}
#[derive(Resource, Default, Clone)]
struct Encounter {
    participants: HashSet<String>,
    order: Vec<String>,
    cursor: usize,
    round: u32,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Phase {
    Ready,
    Moving,
    AfterMove,
    Ended,
}
#[derive(Resource, Clone)]
struct Turn {
    actor: Option<String>,
    phase: Phase,
    remaining: u32,
    moves: u8,
}
#[derive(Resource)]
struct Random(u64);
#[derive(Resource, Default)]
struct Log(Vec<CombatLogEvent>);
#[derive(Resource)]
struct ResultState(Outcome);
#[derive(Resource)]
struct Skills(HashMap<String, SkillDef>);

#[derive(Deserialize)]
struct Definition {
    map: MapDef,
    skills: Vec<SkillDef>,
    units: Vec<UnitDef>,
}
#[derive(Clone, Deserialize)]
struct SkillDef {
    id: String,
    name: String,
    ranged: bool,
    attack_bonus: i32,
    damage_bonus: i32,
    range: Option<i32>,
}
#[derive(Deserialize)]
struct MapDef {
    width: i32,
    height: i32,
    costs: Vec<u32>,
    #[serde(default)]
    triggers: Vec<TriggerDef>,
}
#[derive(Deserialize)]
struct TriggerDef {
    x: i32,
    y: i32,
    kind: String,
}
#[derive(Deserialize)]
struct UnitDef {
    id: String,
    name: String,
    team: Team,
    group: String,
    x: i32,
    y: i32,
    #[serde(default = "one")]
    width: i32,
    #[serde(default = "one")]
    height: i32,
    hp: i32,
    movement: u32,
    initiative: i32,
    dodge: i32,
    block: i32,
    melee: i32,
    ranged: i32,
    damage: i32,
    range: i32,
}
fn one() -> i32 {
    1
}
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    Start,
    Move {
        actor: String,
        x: i32,
        y: i32,
    },
    Skill {
        actor: String,
        target: String,
        skill: String,
    },
    EndTurn {
        actor: String,
    },
}
#[derive(Serialize)]
pub struct Snapshot {
    pub width: i32,
    pub height: i32,
    pub costs: Vec<u32>,
    pub terrain_effects: Vec<TerrainEffectView>,
    pub terrain_cells: Vec<TerrainCellView>,
    pub units: Vec<UnitView>,
    pub reachable: Vec<GridPos>,
    pub second_reachable: Vec<GridPos>,
    pub skill_ranges: Vec<SkillRangeView>,
    pub turn: TurnView,
    pub round: u32,
    pub outcome: Outcome,
    pub log: Vec<CombatLogEvent>,
}
#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CombatLogEvent {
    NewRound {
        round: u32,
    },
    Skill {
        actor: String,
        actor_team: Team,
        skill: String,
        target: String,
        target_team: Team,
        roll: i32,
        attack_modifier: i32,
        attack_total: i32,
        dodge_target: i32,
        block_target: i32,
        result: AttackResult,
        critical: bool,
        damage: i32,
        remaining_hp: i32,
        max_hp: i32,
        downed: bool,
    },
    StatusApplied {
        target: String,
        target_team: Team,
        status: String,
    },
}
#[derive(Serialize)]
pub struct SkillRangeView {
    pub id: String,
    pub cells: Vec<GridPos>,
}
#[derive(Serialize)]
pub struct MovePreview {
    pub first: Vec<GridPos>,
    pub second: Vec<GridPos>,
    pub interrupted: bool,
}
#[derive(Serialize)]
pub struct UnitView {
    pub id: String,
    pub name: String,
    pub team: Team,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub movement: u32,
    pub initiative: i32,
    pub dodge: i32,
    pub block: i32,
    pub melee: i32,
    pub ranged: i32,
    pub damage: i32,
    pub range: i32,
    pub downed: bool,
    pub active: bool,
}
#[derive(Serialize)]
pub struct TerrainEffectView {
    pub x: i32,
    pub y: i32,
    pub effect: String,
}
#[derive(Serialize)]
pub struct TerrainCellView {
    pub x: i32,
    pub y: i32,
    pub kind: String,
    pub cost: u32,
    pub effect: String,
}
#[derive(Serialize)]
pub struct TurnView {
    pub actor: Option<String>,
    pub phase: String,
    pub move_remaining: u32,
    pub can_move: bool,
    pub can_skill: bool,
}

pub struct Game {
    world: World,
}
struct MovePlan {
    entity: Entity,
    turn: Turn,
    first_budget: u32,
    second_budget: u32,
    path: Vec<GridPos>,
}
impl Game {
    pub fn from_toml(s: &str) -> Result<Self, String> {
        let d: Definition = toml::from_str(s).map_err(|e| e.to_string())?;
        if d.map.width <= 0
            || d.map.height <= 0
            || d.map.costs.len() != (d.map.width * d.map.height) as usize
        {
            return Err("map.costs 數量與尺寸不符".into());
        }
        if d.map.costs.contains(&0) {
            return Err("movement cost 必須大於 0".into());
        }
        let mut w = World::new();
        let triggers = d
            .map
            .triggers
            .into_iter()
            .map(|t| (GridPos { x: t.x, y: t.y }, t.kind))
            .collect();
        w.insert_resource(Board {
            width: d.map.width,
            height: d.map.height,
            costs: d.map.costs,
            triggers,
        });
        w.insert_resource(Encounter::default());
        w.insert_resource(Turn {
            actor: None,
            phase: Phase::Ended,
            remaining: 0,
            moves: 0,
        });
        w.insert_resource(Random(0xc0ffee));
        w.insert_resource(Log::default());
        w.insert_resource(ResultState(Outcome::Ongoing));
        let mut skills = HashMap::new();
        for skill in d.skills {
            if skills.insert(skill.id.clone(), skill).is_some() {
                return Err("duplicate skill id".into());
            }
        }
        w.insert_resource(Skills(skills));
        let mut ids = HashSet::new();
        for u in d.units {
            if !ids.insert(u.id.clone()) {
                return Err(format!("重複 id {}", u.id));
            }
            let f = Footprint {
                width: u.width,
                height: u.height,
            };
            let p = GridPos { x: u.x, y: u.y };
            if !fits(w.resource::<Board>(), p, f) {
                return Err(format!("{} 超出地圖", u.id));
            }
            w.spawn((
                Id(u.id),
                Pos(p),
                f,
                Hp {
                    current: u.hp,
                    maximum: u.hp,
                },
                Fighter {
                    name: u.name,
                    team: u.team,
                    group: u.group,
                    movement: u.movement,
                    initiative: u.initiative,
                    dodge: u.dodge,
                    block: u.block,
                    melee: u.melee,
                    ranged: u.ranged,
                    damage: u.damage,
                    range: u.range,
                },
            ));
        }
        Ok(Self { world: w })
    }
    pub fn command(&mut self, c: Command) -> Result<Snapshot, String> {
        if self.world.resource::<ResultState>().0 != Outcome::Ongoing {
            return Ok(self.snapshot());
        }
        match c {
            Command::Start => self.start(),
            Command::Move { actor, x, y } => self.move_to(&actor, GridPos { x, y }),
            Command::Skill {
                actor,
                target,
                skill,
            } => {
                let definition = self
                    .world
                    .resource::<Skills>()
                    .0
                    .get(&skill)
                    .cloned()
                    .ok_or_else(|| format!("unknown skill: {skill}"))?;
                self.use_skill(&actor, &target, definition)
            }
            Command::EndTurn { actor } => {
                self.ensure(&actor)?;
                self.finish();
                Ok(())
            }
        }?;
        self.enemy_turns()?;
        self.outcome();
        Ok(self.snapshot())
    }
    pub fn preview_move(&self, actor: &str, end: GridPos) -> Result<MovePreview, String> {
        let MovePlan {
            entity: _,
            turn: _,
            first_budget,
            second_budget: _,
            path,
        } = self.move_plan(actor, end)?;
        let board = self.world.resource::<Board>();
        let interrupted = path
            .last()
            .is_some_and(|position| board.triggers.contains_key(position));
        let mut first = Vec::new();
        let mut second = Vec::new();
        let mut spent = 0;
        if first_budget == 0 {
            second.push(path[0]);
        } else {
            first.push(path[0]);
        }
        for position in path.into_iter().skip(1) {
            spent += cost(board, position);
            if spent <= first_budget {
                first.push(position);
            } else {
                if second.is_empty() {
                    second.push(*first.last().unwrap());
                }
                second.push(position);
            }
        }
        Ok(MovePreview {
            first,
            second,
            interrupted,
        })
    }
    fn start(&mut self) -> Result<(), String> {
        if self.world.resource::<Encounter>().round > 0 {
            return Ok(());
        }
        let ids: Vec<_> = self
            .world
            .query::<(&Id, &Fighter)>()
            .iter(&self.world)
            .filter(|(_, f)| f.team == Team::Player || f.group == "wolves")
            .map(|(i, _)| i.0.clone())
            .collect();
        self.world
            .resource_mut::<Encounter>()
            .participants
            .extend(ids);
        self.roll_round();
        Ok(())
    }
    fn roll_round(&mut self) {
        let active = self.world.resource::<Encounter>().participants.clone();
        let entries: Vec<_> = self
            .world
            .query::<(&Id, &Fighter, Has<Downed>)>()
            .iter(&self.world)
            .filter(|(i, _, d)| active.contains(&i.0) && !*d)
            .map(|(i, f, _)| (i.0.clone(), f.initiative))
            .collect();
        let mut rolled: Vec<_> = entries
            .into_iter()
            .map(|(id, m)| (die(&mut self.world, 20) as i32 + m, id))
            .collect();
        rolled.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        {
            let mut e = self.world.resource_mut::<Encounter>();
            e.round += 1;
            e.order = rolled.into_iter().map(|(_, i)| i).collect();
            e.cursor = 0;
        }
        let round = self.world.resource::<Encounter>().round;
        self.world
            .resource_mut::<Log>()
            .0
            .push(CombatLogEvent::NewRound { round });
        self.begin()
    }
    fn begin(&mut self) {
        let actor = self
            .world
            .resource::<Encounter>()
            .order
            .get(self.world.resource::<Encounter>().cursor)
            .cloned();
        let remaining = actor
            .as_ref()
            .and_then(|a| self.entity(a))
            .map(|e| self.world.get::<Fighter>(e).unwrap().movement)
            .unwrap_or(0);
        *self.world.resource_mut::<Turn>() = Turn {
            actor,
            phase: Phase::Ready,
            remaining,
            moves: 0,
        }
    }
    fn finish(&mut self) {
        self.world.resource_mut::<Turn>().phase = Phase::Ended;
        let end = {
            let mut e = self.world.resource_mut::<Encounter>();
            e.cursor += 1;
            e.cursor >= e.order.len()
        };
        if end { self.roll_round() } else { self.begin() }
    }
    fn ensure(&self, a: &str) -> Result<(), String> {
        if self.world.resource::<Turn>().actor.as_deref() != Some(a) {
            Err("不是該單位的回合".into())
        } else {
            Ok(())
        }
    }
    fn move_to(&mut self, a: &str, end: GridPos) -> Result<(), String> {
        let MovePlan {
            entity: e,
            turn,
            first_budget,
            second_budget,
            path,
        } = self.move_plan(a, end)?;
        let mut spent = 0;
        for p in path.into_iter().skip(1) {
            spent += cost(self.world.resource::<Board>(), p);
            self.world.get_mut::<Pos>(e).unwrap().0 = p;
            self.reveal(e);
            if let Some(k) = self.world.resource::<Board>().triggers.get(&p).cloned() {
                let fighter = self.world.get::<Fighter>(e).unwrap();
                let target = fighter.name.clone();
                let target_team = fighter.team;
                self.world
                    .resource_mut::<Log>()
                    .0
                    .push(CombatLogEvent::StatusApplied {
                        target,
                        target_team,
                        status: k,
                    });
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
        let entity = self.entity(actor).unwrap();
        let allowance = self.world.get::<Fighter>(entity).unwrap().movement;
        let turn = self.world.resource::<Turn>().clone();
        if !matches!(turn.phase, Phase::Ready | Phase::Moving | Phase::AfterMove) || turn.moves >= 2
        {
            return Err("目前不能移動".into());
        }
        let start = self.world.get::<Pos>(entity).unwrap().0;
        let footprint = *self.world.get::<Footprint>(entity).unwrap();
        let first_budget = if turn.moves == 0 { turn.remaining } else { 0 };
        let second_budget = allowance;
        let mut path = find_path(
            &self.world,
            entity,
            start,
            end,
            footprint,
            first_budget + second_budget,
        )
        .ok_or("目的地不可達")?;
        if let Some(trigger_index) = path.iter().skip(1).position(|position| {
            self.world
                .resource::<Board>()
                .triggers
                .contains_key(position)
        }) {
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
        let p = self.world.get::<Pos>(mover).unwrap().0;
        let active = self.world.resource::<Encounter>().participants.clone();
        let add: Vec<_> = self
            .world
            .query::<(&Id, &Pos, &Fighter)>()
            .iter(&self.world)
            .filter(|(i, q, f)| {
                f.team == Team::Enemy && !active.contains(&i.0) && distance(p, q.0) <= 4
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
    fn use_skill(&mut self, a: &str, target: &str, skill: SkillDef) -> Result<(), String> {
        self.ensure(a)?;
        let turn = self.world.resource::<Turn>();
        if !can_use_skill(turn) {
            return Err("目前不能使用 Skill".into());
        }
        let ae = self.entity(a).ok_or("找不到攻擊者")?;
        let te = self.entity(target).ok_or("找不到目標")?;
        if self.world.get::<Downed>(te).is_some() {
            return Err("目標已倒下".into());
        }
        let af = self.world.get::<Fighter>(ae).unwrap().clone();
        let tf = self.world.get::<Fighter>(te).unwrap().clone();
        if af.team == tf.team {
            return Err("不能攻擊友軍".into());
        }
        let range = skill
            .range
            .unwrap_or(if skill.ranged { af.range } else { 1 });
        if entity_distance(&self.world, ae, te) > range {
            return Err("目標超出射程".into());
        }
        let modifier = if skill.ranged { af.ranged } else { af.melee } + skill.attack_bonus;
        let natural = die(&mut self.world, 20) as i32;
        let degree = degree(natural, modifier, 10 + tf.dodge);
        let result = if matches!(degree, RollDegree::Failure | RollDegree::CriticalFailure) {
            AttackResult::Dodge
        } else if natural + modifier < 10 + tf.dodge + tf.block {
            AttackResult::Block
        } else {
            AttackResult::Hit
        };
        let base = af.damage
            + skill.damage_bonus
            + if degree == RollDegree::CriticalSuccess {
                af.damage
            } else {
                0
            };
        let damage = match result {
            AttackResult::Dodge => 0,
            AttackResult::Block => (base - 2).max(0),
            AttackResult::Hit => base,
        };
        let mut downed = false;
        if damage > 0 {
            let mut hp = self.world.get_mut::<Hp>(te).unwrap();
            hp.current = (hp.current - damage).max(0);
            if hp.current == 0 {
                downed = true;
                self.world.entity_mut(te).insert(Downed);
                self.world
                    .resource_mut::<Encounter>()
                    .participants
                    .remove(target);
            }
        }
        let hp = self.world.get::<Hp>(te).unwrap();
        let remaining_hp = hp.current;
        let max_hp = hp.maximum;
        self.world
            .resource_mut::<Log>()
            .0
            .push(CombatLogEvent::Skill {
                actor: af.name,
                actor_team: af.team,
                skill: skill.name,
                target: tf.name,
                target_team: tf.team,
                roll: natural,
                attack_modifier: modifier,
                attack_total: natural + modifier,
                dodge_target: 10 + tf.dodge,
                block_target: 10 + tf.dodge + tf.block,
                result,
                critical: degree == RollDegree::CriticalSuccess,
                damage,
                remaining_hp,
                max_hp,
                downed,
            });
        self.finish();
        Ok(())
    }
    fn enemy_turns(&mut self) -> Result<(), String> {
        loop {
            let a = match self.world.resource::<Turn>().actor.clone() {
                Some(v) => v,
                None => return Ok(()),
            };
            let e = self.entity(&a).ok_or("先攻單位不存在")?;
            if self.world.get::<Downed>(e).is_some() {
                self.finish();
                continue;
            }
            if self.world.get::<Fighter>(e).unwrap().team == Team::Player {
                return Ok(());
            }
            let t = match self.closest(e) {
                Some(v) => v,
                None => {
                    self.finish();
                    continue;
                }
            };
            if entity_distance(&self.world, e, t) > 1 {
                let goal = self.world.get::<Pos>(t).unwrap().0;
                let start = self.world.get::<Pos>(e).unwrap().0;
                let fp = *self.world.get::<Footprint>(e).unwrap();
                let b = self.world.get::<Fighter>(e).unwrap().movement;
                if let Some(last) = toward(&self.world, e, start, goal, fp, b)
                    .as_ref()
                    .and_then(|path| path.last())
                {
                    self.world.get_mut::<Pos>(e).unwrap().0 = *last
                }
            }
            if entity_distance(&self.world, e, t) <= 1 {
                let id = self.world.get::<Id>(t).unwrap().0.clone();
                let skill = self
                    .world
                    .resource::<Skills>()
                    .0
                    .get("melee_attack")
                    .cloned()
                    .ok_or("找不到 melee_attack 技能")?;
                self.use_skill(&a, &id, skill)?
            } else {
                self.finish()
            }
            self.outcome();
            if self.world.resource::<ResultState>().0 != Outcome::Ongoing {
                return Ok(());
            }
        }
    }
    fn entity(&self, id: &str) -> Option<Entity> {
        self.world
            .iter_entities()
            .find(|e| e.get::<Id>().is_some_and(|i| i.0 == id))
            .map(|e| e.id())
    }
    fn closest(&self, e: Entity) -> Option<Entity> {
        let p = self.world.get::<Pos>(e)?.0;
        self.world
            .iter_entities()
            .filter(|q| {
                q.get::<Fighter>().is_some_and(|f| f.team == Team::Player)
                    && q.get::<Downed>().is_none()
            })
            .min_by_key(|q| distance(p, q.get::<Pos>().unwrap().0))
            .map(|q| q.id())
    }
    fn outcome(&mut self) {
        let mut p = false;
        let mut e = false;
        for q in self.world.iter_entities() {
            if let Some(f) = q.get::<Fighter>().filter(|_| q.get::<Downed>().is_none()) {
                match f.team {
                    Team::Player => p = true,
                    Team::Enemy => e = true,
                }
            }
        }
        self.world.resource_mut::<ResultState>().0 = if !p {
            Outcome::Defeat
        } else if !e {
            Outcome::Victory
        } else {
            Outcome::Ongoing
        }
    }
    pub fn snapshot(&mut self) -> Snapshot {
        let b = self.world.resource::<Board>().clone();
        let enc = self.world.resource::<Encounter>().clone();
        let turn = self.world.resource::<Turn>().clone();
        let mut units: Vec<_> = self
            .world
            .iter_entities()
            .filter_map(|e| {
                Some((
                    e.get::<Id>()?,
                    e.get::<Pos>()?,
                    e.get::<Footprint>()?,
                    e.get::<Hp>()?,
                    e.get::<Fighter>()?,
                    e.get::<Downed>().is_some(),
                ))
            })
            .map(|(i, p, fp, h, f, d)| UnitView {
                id: i.0.clone(),
                name: f.name.clone(),
                team: f.team,
                x: p.0.x,
                y: p.0.y,
                width: fp.width,
                height: fp.height,
                hp: h.current,
                max_hp: h.maximum,
                movement: f.movement,
                initiative: f.initiative,
                dodge: f.dodge,
                block: f.block,
                melee: f.melee,
                ranged: f.ranged,
                damage: f.damage,
                range: f.range,
                downed: d,
                active: enc.participants.contains(&i.0),
            })
            .collect();
        units.sort_by(|a, b| a.id.cmp(&b.id));
        let movement_ranges = turn
            .actor
            .as_ref()
            .and_then(|a| self.entity(a))
            .map(|entity| movement_ranges(&self.world, entity, &turn));
        let (reachable, second_reachable) = movement_ranges.unwrap_or_default();
        let skill_ranges = turn
            .actor
            .as_ref()
            .and_then(|actor| self.entity(actor))
            .map(|entity| skill_ranges(&self.world, entity))
            .unwrap_or_default();
        let can_skill = can_use_skill(&turn);
        let terrain_cells = (0..b.height)
            .flat_map(|y| {
                let board = &b;
                (0..b.width).map(move |x| {
                    let position = GridPos { x, y };
                    let cost = board.costs[(y * board.width + x) as usize];
                    let effect = board.triggers.get(&position).cloned().unwrap_or_default();
                    let kind = if effect == "grease" {
                        "grease"
                    } else if cost > 1 {
                        "rough"
                    } else {
                        "plain"
                    };
                    TerrainCellView {
                        x,
                        y,
                        kind: kind.to_string(),
                        cost,
                        effect,
                    }
                })
            })
            .collect();
        Snapshot {
            width: b.width,
            height: b.height,
            costs: b.costs,
            terrain_effects: {
                let mut effects: Vec<_> = b
                    .triggers
                    .into_iter()
                    .map(|(position, effect)| TerrainEffectView {
                        x: position.x,
                        y: position.y,
                        effect,
                    })
                    .collect();
                effects.sort_by_key(|effect| (effect.y, effect.x));
                effects
            },
            terrain_cells,
            units,
            reachable,
            second_reachable,
            skill_ranges,
            turn: TurnView {
                actor: turn.actor,
                phase: format!("{:?}", turn.phase).to_lowercase(),
                move_remaining: turn.remaining,
                can_move: matches!(turn.phase, Phase::Ready | Phase::Moving | Phase::AfterMove)
                    && turn.moves < 2,
                can_skill,
            },
            round: enc.round,
            outcome: self.world.resource::<ResultState>().0,
            log: self.world.resource::<Log>().0.iter().cloned().collect(),
        }
    }
}
fn can_use_skill(turn: &Turn) -> bool {
    matches!(turn.phase, Phase::Ready | Phase::Moving) && turn.moves == 0
        || matches!(turn.phase, Phase::AfterMove) && turn.moves == 1
}
pub fn degree(n: i32, m: i32, t: i32) -> RollDegree {
    if n == 1 {
        RollDegree::CriticalFailure
    } else if n == 20 {
        RollDegree::CriticalSuccess
    } else if n + m >= t {
        RollDegree::Success
    } else {
        RollDegree::Failure
    }
}
fn die(w: &mut World, s: u32) -> u32 {
    let mut r = w.resource_mut::<Random>();
    r.0 = r.0.wrapping_mul(6364136223846793005).wrapping_add(1);
    ((r.0 >> 32) as u32 % s) + 1
}
fn cost(b: &Board, p: GridPos) -> u32 {
    b.costs[(p.y * b.width + p.x) as usize]
}
fn fits(b: &Board, p: GridPos, f: Footprint) -> bool {
    p.x >= 0 && p.y >= 0 && p.x + f.width <= b.width && p.y + f.height <= b.height
}
fn distance(a: GridPos, b: GridPos) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}
fn overlap(a: GridPos, af: Footprint, b: GridPos, bf: Footprint) -> bool {
    a.x < b.x + bf.width && a.x + af.width > b.x && a.y < b.y + bf.height && a.y + af.height > b.y
}
fn occupied(w: &World, ignore: Entity, p: GridPos, f: Footprint) -> bool {
    w.iter_entities().any(|e| {
        e.id() != ignore
            && e.get::<Downed>().is_none()
            && e.get::<Pos>()
                .zip(e.get::<Footprint>())
                .is_some_and(|(q, g)| overlap(p, f, q.0, *g))
    })
}
fn entity_distance(w: &World, a: Entity, b: Entity) -> i32 {
    let ap = w.get::<Pos>(a).unwrap().0;
    let af = *w.get::<Footprint>(a).unwrap();
    let bp = w.get::<Pos>(b).unwrap().0;
    let bf = *w.get::<Footprint>(b).unwrap();
    footprint_distance(ap, af, bp, bf)
}
fn footprint_distance(a: GridPos, af: Footprint, b: GridPos, bf: Footprint) -> i32 {
    let dx = (b.x - (a.x + af.width - 1))
        .max(a.x - (b.x + bf.width - 1))
        .max(0);
    let dy = (b.y - (a.y + af.height - 1))
        .max(a.y - (b.y + bf.height - 1))
        .max(0);
    dx + dy
}
#[derive(Eq)]
struct Node {
    c: u32,
    p: GridPos,
}
impl Ord for Node {
    fn cmp(&self, o: &Self) -> Ordering {
        o.c.cmp(&self.c)
            .then_with(|| self.p.x.cmp(&o.p.x))
            .then_with(|| self.p.y.cmp(&o.p.y))
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl PartialEq for Node {
    fn eq(&self, o: &Self) -> bool {
        self.c == o.c && self.p == o.p
    }
}
fn paths(
    w: &World,
    e: Entity,
    start: GridPos,
    f: Footprint,
    budget: u32,
) -> (HashMap<GridPos, u32>, HashMap<GridPos, GridPos>) {
    let board = w.resource::<Board>();
    let mut dist = HashMap::from([(start, 0)]);
    let mut prev = HashMap::new();
    let mut heap = BinaryHeap::from([Node { c: 0, p: start }]);
    while let Some(Node { c, p }) = heap.pop() {
        if c > *dist.get(&p).unwrap() {
            continue;
        }
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let n = GridPos {
                x: p.x + dx,
                y: p.y + dy,
            };
            if !fits(board, n, f) || occupied(w, e, n, f) {
                continue;
            }
            let nc = c + cost(board, n);
            if nc <= budget && nc < *dist.get(&n).unwrap_or(&u32::MAX) {
                dist.insert(n, nc);
                prev.insert(n, p);
                heap.push(Node { c: nc, p: n })
            }
        }
    }
    (dist, prev)
}
fn find_path(
    w: &World,
    e: Entity,
    s: GridPos,
    end: GridPos,
    f: Footprint,
    b: u32,
) -> Option<Vec<GridPos>> {
    let (d, p) = paths(w, e, s, f, b);
    if !d.contains_key(&end) {
        return None;
    }
    let mut out = vec![end];
    while *out.last().unwrap() != s {
        out.push(*p.get(out.last().unwrap()).unwrap())
    }
    out.reverse();
    Some(out)
}
fn toward(
    w: &World,
    e: Entity,
    s: GridPos,
    target: GridPos,
    f: Footprint,
    b: u32,
) -> Option<Vec<GridPos>> {
    let (d, _) = paths(w, e, s, f, b);
    let end = *d.keys().min_by_key(|p| distance(**p, target))?;
    find_path(w, e, s, end, f, b)
}
fn reach(w: &World, e: Entity, b: u32) -> Vec<GridPos> {
    let p = w.get::<Pos>(e).unwrap().0;
    let f = *w.get::<Footprint>(e).unwrap();
    let mut v: Vec<_> = paths(w, e, p, f, b).0.into_keys().collect();
    v.sort_by_key(|p| (p.y, p.x));
    v
}
fn movement_ranges(w: &World, e: Entity, turn: &Turn) -> (Vec<GridPos>, Vec<GridPos>) {
    if turn.moves >= 2 {
        return (Vec::new(), Vec::new());
    }
    let allowance = w.get::<Fighter>(e).unwrap().movement;
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
fn skill_ranges(w: &World, e: Entity) -> Vec<SkillRangeView> {
    let board = w.resource::<Board>();
    let position = w.get::<Pos>(e).unwrap().0;
    let footprint = *w.get::<Footprint>(e).unwrap();
    let fighter = w.get::<Fighter>(e).unwrap();
    let mut ranges: Vec<_> = w
        .resource::<Skills>()
        .0
        .values()
        .map(|skill| {
            let range = skill
                .range
                .unwrap_or(if skill.ranged { fighter.range } else { 1 });
            let mut cells = Vec::new();
            for y in 0..board.height {
                for x in 0..board.width {
                    let cell = GridPos { x, y };
                    let cell_distance = footprint_distance(
                        position,
                        footprint,
                        cell,
                        Footprint {
                            width: 1,
                            height: 1,
                        },
                    );
                    if cell_distance > 0 && cell_distance <= range {
                        cells.push(cell);
                    }
                }
            }
            SkillRangeView {
                id: skill.id.clone(),
                cells,
            }
        })
        .collect();
    ranges.sort_by(|a, b| a.id.cmp(&b.id));
    ranges
}

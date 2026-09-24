//! 載入、命令分派、回合流程與快照。
use crate::model::{
    Board, CombatLogEvent, Command, Definition, Downed, Encounter, Footprint, GridPos, Hp, Id,
    InitiativeRollLog, Log, MovementTransition, Outcome, Phase, Pos, Random, ResultState,
    SkillEffect, Skills, Snapshot, Team, TemporaryTerrains, TerrainCellView, TerrainEffectView,
    Turn, TurnView, Unit, UnitView,
};
use crate::movement::{
    distance, entity_distance, fits, footprint_cells, footprint_on_impassable, movement_cost,
    movement_ranges, terrain_damage, terrain_type, toward,
};
use crate::skill::{
    can_use_skill, closest_occupied_cell, detail, effective_block, effective_dodge, skill_ranges,
};
use crate::{authoring, gameplay_config};
use bevy_ecs::prelude::{Entity, Has, World};
use std::collections::{HashMap, HashSet};

pub struct Game {
    pub(crate) world: World,
    pub(crate) movements: Vec<MovementTransition>,
}

impl Game {
    pub fn from_documents(definitions: &str, map: &str) -> Result<Self, String> {
        let definitions: authoring::Definitions =
            toml::from_str(definitions).map_err(|e| e.to_string())?;
        let map: authoring::Map = toml::from_str(map).map_err(|e| e.to_string())?;
        Self::from_authoring(definitions, map)
    }
    pub fn from_authoring(
        definitions: authoring::Definitions,
        map: authoring::Map,
    ) -> Result<Self, String> {
        Self::from_definition(authoring::into_definition(definitions, map)?)
    }
    pub(crate) fn from_definition(d: Definition) -> Result<Self, String> {
        if d.map.width <= 0
            || d.map.height <= 0
            || d.map
                .width
                .checked_mul(d.map.height)
                .is_none_or(|cells| d.map.costs.len() != cells as usize)
        {
            return Err("map.costs 數量與尺寸不符".into());
        }
        if d.map.costs.contains(&0) {
            return Err("movement cost 必須大於 0".into());
        }
        if d.map.triggers.iter().any(|trigger| {
            trigger.x < 0 || trigger.y < 0 || trigger.x >= d.map.width || trigger.y >= d.map.height
        }) {
            return Err("map trigger 超出地圖範圍".into());
        }
        for required in ["plain", "rough"] {
            if !d.terrain_types.contains_key(required) {
                return Err(format!("缺少必要地形種類 {required}"));
            }
        }
        for trigger in &d.map.triggers {
            if !d.terrain_types.contains_key(&trigger.kind) {
                return Err(format!("找不到地形種類 {}", trigger.kind));
            }
        }
        let mut terrain_positions = HashSet::new();
        for trigger in &d.map.triggers {
            if !terrain_positions.insert((trigger.x, trigger.y)) {
                return Err(format!("地形格子重複 ({}, {})", trigger.x, trigger.y));
            }
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
            terrain_types: d.terrain_types,
        });
        w.insert_resource(Encounter::default());
        w.insert_resource(TemporaryTerrains::default());
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
        let mut ai_default = None;
        for skill in d.skills {
            if skill.min_range < 0 || skill.max_range < skill.min_range {
                return Err(format!(
                    "{} 的距離必須符合 0 ≤ min_range ≤ max_range",
                    skill.id
                ));
            }
            if skill.effect == SkillEffect::Heal
                && !matches!(skill.heal_amount, Some(amount) if amount > 0)
            {
                return Err(format!("{} 的 heal_amount 必須大於 0", skill.id));
            }
            if skill.duration == Some(0) {
                return Err(format!("{} 的 duration 必須大於 0", skill.id));
            }
            if skill.effect == SkillEffect::Mire
                && !skill.terrain.as_ref().is_some_and(|terrain| {
                    w.resource::<Board>().terrain_types.contains_key(terrain)
                })
            {
                return Err(format!("{} 必須指定已定義的 terrain", skill.id));
            }
            if skill.ai_default {
                if ai_default.replace(skill.id.clone()).is_some() {
                    return Err("只能有一個 ai_default 技能".into());
                }
            }
            if skills.insert(skill.id.clone(), skill).is_some() {
                return Err("duplicate skill id".into());
            }
        }
        let ai_default = ai_default.ok_or("缺少 ai_default 技能")?;
        w.insert_resource(Skills {
            definitions: skills,
            ai_default,
        });
        let all_skill_ids: Vec<_> = w.resource::<Skills>().definitions.keys().cloned().collect();
        let mut ids = HashSet::new();
        let mut occupied = HashSet::new();
        for u in d.units {
            if !ids.insert(u.id.clone()) {
                return Err(format!("重複 id {}", u.id));
            }
            let f = Footprint {
                width: u.width,
                height: u.height,
            };
            if f.width <= 0 || f.height <= 0 || u.hp <= 0 {
                return Err(format!("{} 的佔用尺寸與 HP 必須大於 0", u.id));
            }
            let p = GridPos { x: u.x, y: u.y };
            if !fits(w.resource::<Board>(), p, f) {
                return Err(format!("{} 超出地圖", u.id));
            }
            if footprint_on_impassable(w.resource::<Board>(), p, f) {
                return Err(format!("{} 不可放置在峭壁或懸崖", u.id));
            }
            if footprint_cells(p, f)
                .iter()
                .any(|cell| !occupied.insert(*cell))
            {
                return Err(format!("{} 與其他單位重疊", u.id));
            }
            if let Some(unknown) = u
                .skills
                .iter()
                .find(|skill| !w.resource::<Skills>().definitions.contains_key(*skill))
            {
                return Err(format!("{} 使用未知技能 {}", u.id, unknown));
            }
            w.spawn((
                Id(u.id),
                Pos(p),
                f,
                Hp {
                    current: u.hp,
                    maximum: u.hp,
                },
                Unit {
                    name: u.name,
                    visual: u.visual,
                    team: u.team,
                    movement: u.movement,
                    initiative: u.initiative,
                    dodge: u.dodge,
                    block: u.block,
                    attack: u.attack,
                    damage: u.damage,
                    skills: if u.skills.is_empty() {
                        all_skill_ids.clone()
                    } else {
                        u.skills
                    },
                },
            ));
        }
        Ok(Self {
            world: w,
            movements: Vec::new(),
        })
    }
    // 每次只執行一個命令或一段自動回合，讓畫面先完成呈現再推進。
    pub fn command(&mut self, c: Command) -> Result<Snapshot, String> {
        self.apply_command(c)?;
        Ok(self.snapshot())
    }
    fn apply_command(&mut self, c: Command) -> Result<(), String> {
        self.movements.clear();
        if self.world.resource::<ResultState>().0 != Outcome::Ongoing {
            return Ok(());
        }
        match c {
            Command::Start => self.start(),
            Command::AutoStep => self.auto_step(),
            Command::Move { actor, x, y } => self.move_to(&actor, GridPos { x, y }),
            Command::Skill {
                actor,
                target,
                x,
                y,
                skill,
            } => {
                let definition = self
                    .world
                    .resource::<Skills>()
                    .definitions
                    .get(&skill)
                    .cloned()
                    .ok_or_else(|| format!("unknown skill: {skill}"))?;
                self.use_skill(&actor, &target, GridPos { x, y }, definition)
            }
            Command::CellSkill { actor, x, y, skill } => {
                let definition = self
                    .world
                    .resource::<Skills>()
                    .definitions
                    .get(&skill)
                    .cloned()
                    .ok_or_else(|| format!("unknown skill: {skill}"))?;
                self.use_cell_skill(&actor, GridPos { x, y }, definition)
            }
            Command::EndTurn { actor } => {
                self.ensure(&actor)?;
                self.finish();
                Ok(())
            }
            Command::Delay { actor, after } => self.delay(&actor, &after),
        }?;
        self.outcome();
        Ok(())
    }
    pub fn set_random_seed(&mut self, seed: u64) {
        self.world.resource_mut::<Random>().0 = seed;
    }
    pub(crate) fn start(&mut self) -> Result<(), String> {
        if self.world.resource::<Encounter>().round > 0 {
            return Ok(());
        }
        let ids: Vec<_> = self
            .world
            .query::<&Id>()
            .iter(&self.world)
            .map(|i| i.0.clone())
            .collect();
        self.world
            .resource_mut::<Encounter>()
            .participants
            .extend(ids);
        self.roll_round();
        Ok(())
    }
    fn roll_round(&mut self) {
        let next_round = self.world.resource::<Encounter>().round + 1;
        self.world
            .resource_mut::<TemporaryTerrains>()
            .0
            .retain(|_, terrain| terrain.expires_after_round >= next_round);
        let active = self.world.resource::<Encounter>().participants.clone();
        let entries: Vec<_> = self
            .world
            .query::<(&Id, &Unit, Has<Downed>)>()
            .iter(&self.world)
            .filter(|(i, _, d)| active.contains(&i.0) && !*d)
            .map(|(i, f, _)| (i.0.clone(), f.name.clone(), f.team.clone(), f.initiative))
            .collect();
        let mut rolled: Vec<_> = entries
            .into_iter()
            .map(|(id, name, team, modifier)| {
                let roll = die(&mut self.world, gameplay_config::INITIATIVE_DIE_SIDES) as i32;
                (roll + modifier, id, name, team, roll, modifier)
            })
            .collect();
        rolled.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        let initiative_rolls = rolled
            .iter()
            .map(
                |(total, _id, unit, team, roll, modifier)| InitiativeRollLog {
                    unit: unit.clone(),
                    team: team.clone(),
                    roll: *roll,
                    die_sides: gameplay_config::INITIATIVE_DIE_SIDES,
                    modifier: *modifier,
                    total: *total,
                },
            )
            .collect();
        {
            let mut e = self.world.resource_mut::<Encounter>();
            e.round += 1;
            e.order = rolled.into_iter().map(|(_, id, _, _, _, _)| id).collect();
            e.cursor = 0;
        }
        let round = self.world.resource::<Encounter>().round;
        self.world
            .resource_mut::<Log>()
            .0
            .push(CombatLogEvent::NewRound {
                round,
                initiative_rolls,
            });
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
            .map(|e| {
                self.world
                    .get::<Unit>(e)
                    .expect("已建立的戰鬥單位應具有 Unit 元件")
                    .movement
            })
            .unwrap_or(0);
        *self.world.resource_mut::<Turn>() = Turn {
            actor,
            phase: Phase::Ready,
            remaining,
            moves: 0,
        }
    }
    pub(crate) fn finish(&mut self) {
        self.world.resource_mut::<Turn>().phase = Phase::Ended;
    }
    fn advance_turn(&mut self) {
        let end = {
            let mut e = self.world.resource_mut::<Encounter>();
            e.cursor += 1;
            e.cursor >= e.order.len()
        };
        if end { self.roll_round() } else { self.begin() }
    }
    fn auto_step_available(&self) -> bool {
        if self.world.resource::<ResultState>().0 != Outcome::Ongoing
            || self.world.resource::<Encounter>().round == 0
        {
            return false;
        }
        let turn = self.world.resource::<Turn>();
        if turn.phase == Phase::Ended {
            return true;
        }
        turn.actor
            .as_ref()
            .and_then(|actor| self.entity(actor))
            .is_some_and(|entity| {
                self.world.get::<Downed>(entity).is_some()
                    || self
                        .world
                        .get::<Unit>(entity)
                        .expect("已建立的戰鬥單位應具有 Unit 元件")
                        .team
                        != Team::Player
            })
    }
    fn auto_step(&mut self) -> Result<(), String> {
        if !self.auto_step_available() {
            return Err("目前沒有可推進的自動回合".into());
        }
        if self.world.resource::<Turn>().phase == Phase::Ended {
            self.advance_turn();
            return Ok(());
        }
        self.enemy_turn_once()
    }
    fn delay(&mut self, actor: &str, after: &str) -> Result<(), String> {
        self.ensure(actor)?;
        let turn = self.world.resource::<Turn>();
        if turn.phase != Phase::Ready || turn.moves != 0 {
            return Err("開始行動後不能延後".into());
        }
        let encounter = self.world.resource::<Encounter>();
        let target_index = encounter
            .order
            .iter()
            .position(|unit_id| unit_id == after)
            .ok_or("找不到延後目標")?;
        if target_index <= encounter.cursor {
            return Err("只能延後到尚未行動的單位之後".into());
        }
        let mut encounter = self.world.resource_mut::<Encounter>();
        let current_index = encounter.cursor;
        let delayed_actor = encounter.order.remove(current_index);
        encounter.order.insert(target_index, delayed_actor);
        drop(encounter);
        self.begin();
        Ok(())
    }
    pub(crate) fn ensure(&self, a: &str) -> Result<(), String> {
        if self.world.resource::<Turn>().actor.as_deref() != Some(a) {
            Err("不是該單位的回合".into())
        } else {
            Ok(())
        }
    }
    fn enemy_turn_once(&mut self) -> Result<(), String> {
        let a = self
            .world
            .resource::<Turn>()
            .actor
            .clone()
            .ok_or("先攻單位不存在")?;
        let e = self.entity(&a).ok_or("先攻單位不存在")?;
        if self.world.get::<Downed>(e).is_some() {
            self.finish();
            return Ok(());
        }
        let t = match self.closest(e) {
            Some(v) => v,
            None => {
                self.finish();
                return Ok(());
            }
        };
        if entity_distance(&self.world, e, t) > gameplay_config::AI_ENGAGEMENT_RANGE {
            let goal = self
                .world
                .get::<Pos>(t)
                .expect("已建立的戰鬥單位應具有 Pos 元件")
                .0;
            let start = self
                .world
                .get::<Pos>(e)
                .expect("已建立的戰鬥單位應具有 Pos 元件")
                .0;
            let fp = *self
                .world
                .get::<Footprint>(e)
                .expect("已建立的戰鬥單位應具有 Footprint 元件");
            let b = self
                .world
                .get::<Unit>(e)
                .expect("已建立的戰鬥單位應具有 Unit 元件")
                .movement;
            if let Some(path) = toward(&self.world, e, start, goal, fp, b) {
                if let [_, .., last] = path.as_slice() {
                    self.movements.push(MovementTransition {
                        unit_id: a.clone(),
                        path: path.clone(),
                        before_log_index: self.world.resource::<Log>().0.len(),
                    });
                    self.world
                        .get_mut::<Pos>(e)
                        .expect("已建立的戰鬥單位應具有 Pos 元件")
                        .0 = *last
                }
            }
        }
        if entity_distance(&self.world, e, t) <= gameplay_config::AI_ENGAGEMENT_RANGE {
            let id = self
                .world
                .get::<Id>(t)
                .expect("已建立的戰鬥單位應具有 Id 元件")
                .0
                .clone();
            let skills = self.world.resource::<Skills>();
            let skill = skills
                .definitions
                .get(&skills.ai_default)
                .cloned()
                .expect("載入時已驗證 ai_default 技能");
            let target_cell = closest_occupied_cell(&self.world, e, t);
            self.use_skill(&a, &id, target_cell, skill)?
        } else {
            self.finish()
        }
        Ok(())
    }
    pub(crate) fn entity(&self, id: &str) -> Option<Entity> {
        self.world
            .iter_entities()
            .find(|e| e.get::<Id>().is_some_and(|i| i.0 == id))
            .map(|e| e.id())
    }
    fn closest(&self, e: Entity) -> Option<Entity> {
        let p = self.world.get::<Pos>(e)?.0;
        let attacker = &self.world.get::<Unit>(e)?.team;
        self.world
            .iter_entities()
            .filter(|q| {
                q.get::<Unit>().is_some_and(|f| &f.team != attacker) && q.get::<Downed>().is_none()
            })
            .min_by_key(|q| {
                distance(
                    p,
                    q.get::<Pos>().expect("已建立的戰鬥單位應具有 Pos 元件").0,
                )
            })
            .map(|q| q.id())
    }
    fn outcome(&mut self) {
        let mut p = false;
        let mut e = false;
        for q in self.world.iter_entities() {
            if let Some(f) = q.get::<Unit>().filter(|_| q.get::<Downed>().is_none()) {
                match &f.team {
                    Team::Player => p = true,
                    Team::Enemy(_) => e = true,
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
        let temporary_terrains = self.world.resource::<TemporaryTerrains>().clone();
        let enc = self.world.resource::<Encounter>().clone();
        let turn = self.world.resource::<Turn>().clone();
        let mut units: Vec<_> = self
            .world
            .iter_entities()
            .filter(|entity| entity.get::<Downed>().is_none())
            .filter_map(|e| {
                Some((
                    e.id(),
                    e.get::<Id>()?,
                    e.get::<Pos>()?,
                    e.get::<Footprint>()?,
                    e.get::<Hp>()?,
                    e.get::<Unit>()?,
                    e.get::<Downed>().is_some(),
                ))
            })
            .map(|(entity, i, p, fp, h, f, d)| UnitView {
                id: i.0.clone(),
                name: f.name.clone(),
                visual: f.visual.clone(),
                team: f.team.clone(),
                x: p.0.x,
                y: p.0.y,
                width: fp.width,
                height: fp.height,
                large: fp.width > 1 || fp.height > 1,
                occupied_cells: footprint_cells(p.0, *fp),
                health_ratio: h.current as f32 / h.maximum as f32,
                hp: h.current,
                max_hp: h.maximum,
                movement: f.movement,
                initiative: f.initiative,
                dodge: effective_dodge(&self.world, entity),
                block: effective_block(&self.world, entity),
                attack: f.attack,
                damage: f.damage,
                downed: d,
                active: enc.participants.contains(&i.0),
            })
            .collect();
        units.sort_by(|a, b| a.id.cmp(&b.id));
        let player_turn = turn
            .actor
            .as_ref()
            .and_then(|actor| self.entity(actor))
            .is_some_and(|entity| {
                self.world
                    .get::<Unit>(entity)
                    .expect("已建立的戰鬥單位應具有 Unit 元件")
                    .team
                    == Team::Player
            });
        let can_move = player_turn
            && matches!(turn.phase, Phase::Ready | Phase::Moving | Phase::AfterMove)
            && turn.moves < 2;
        let movement_ranges = if can_move {
            turn.actor
                .as_ref()
                .and_then(|actor| self.entity(actor))
                .map(|entity| movement_ranges(&self.world, entity, &turn))
        } else {
            None
        };
        let (reachable, second_reachable) = movement_ranges.unwrap_or_default();
        let skill_ranges = if player_turn && turn.phase != Phase::Ended {
            turn.actor
                .as_ref()
                .and_then(|actor| self.entity(actor))
                .map(|entity| skill_ranges(&self.world, entity))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let can_skill = player_turn && can_use_skill(&turn);
        let turn_order: Vec<_> = enc
            .order
            .iter()
            .skip(enc.cursor + 1)
            .filter(|unit_id| {
                self.entity(unit_id)
                    .is_some_and(|entity| self.world.get::<Downed>(entity).is_none())
            })
            .cloned()
            .collect();
        let can_delay = player_turn
            && turn.phase == Phase::Ready
            && turn.moves == 0
            && turn.actor.is_some()
            && !turn_order.is_empty();
        let terrain_cells = (0..b.height)
            .flat_map(|y| {
                let board = &b;
                let temporary = &temporary_terrains;
                let world = &self.world;
                let units = &units;
                (0..b.width).map(move |x| {
                    let position = GridPos { x, y };
                    let base_cost = board.costs[(y * board.width + x) as usize];
                    let temporary_terrain = temporary.0.get(&position);
                    let effect = temporary_terrain
                        .map(|terrain| terrain.kind.clone())
                        .or_else(|| board.triggers.get(&position).cloned())
                        .unwrap_or_default();
                    let cost = movement_cost(world, position);
                    let kind = if !effect.is_empty() {
                        effect.as_str()
                    } else if base_cost > 1 {
                        "rough"
                    } else {
                        "plain"
                    };
                    let definition = terrain_type(board, kind);
                    let damage = definition.damage;
                    let remaining_rounds = temporary_terrain
                        .map(|terrain| terrain.expires_after_round.saturating_sub(enc.round) + 1);
                    let effect_arguments = if remaining_rounds.is_some()
                        && (definition.dodge_penalty > 0
                            || definition.block_penalty > 0
                            || definition.movement_cost_bonus > 0)
                    {
                        vec![
                            definition.dodge_penalty.max(definition.block_penalty),
                            definition.movement_cost_bonus as i32,
                            remaining_rounds.unwrap_or(0) as i32,
                        ]
                    } else if damage > 0 {
                        vec![damage]
                    } else {
                        Vec::new()
                    };
                    let effect_description = detail(&definition.effect_key, &effect_arguments);
                    TerrainCellView {
                        x,
                        y,
                        kind: kind.to_string(),
                        name_key: definition.name_key.clone(),
                        visual: definition.visual.clone(),
                        passable: definition.passable,
                        base_kind: if base_cost > 1 { "rough" } else { "plain" }.to_string(),
                        unit_id: units
                            .iter()
                            .find(|unit| unit.occupied_cells.contains(&position))
                            .map(|unit| unit.id.clone()),
                        cost,
                        damage,
                        remaining_rounds,
                        effect_description,
                        effect,
                    }
                })
            })
            .collect();
        Snapshot {
            width: b.width,
            height: b.height,
            costs: b.costs.clone(),
            terrain_effects: {
                let mut effects: Vec<_> = b
                    .triggers
                    .clone()
                    .into_iter()
                    .filter(|(position, _)| !temporary_terrains.0.contains_key(position))
                    .map(|(position, effect)| TerrainEffectView {
                        x: position.x,
                        y: position.y,
                        damage: terrain_damage(&b, &effect),
                        visual: terrain_type(&b, &effect).visual.clone(),
                        effect,
                        remaining_rounds: None,
                    })
                    .collect();
                effects.extend(temporary_terrains.0.into_iter().map(|(position, terrain)| {
                    TerrainEffectView {
                        x: position.x,
                        y: position.y,
                        damage: terrain_damage(&b, &terrain.kind),
                        visual: terrain_type(&b, &terrain.kind).visual.clone(),
                        effect: terrain.kind,
                        remaining_rounds: Some(
                            terrain.expires_after_round.saturating_sub(enc.round) + 1,
                        ),
                    }
                }));
                effects.sort_by_key(|effect| (effect.y, effect.x));
                effects
            },
            terrain_cells,
            units,
            reachable,
            second_reachable,
            skill_ranges,
            turn_order,
            turn: TurnView {
                can_end_turn: player_turn && turn.phase != Phase::Ended,
                actor: turn.actor,
                phase: format!("{:?}", turn.phase).to_lowercase(),
                auto_step: self.auto_step_available(),
                move_remaining: turn.remaining,
                can_move,
                can_skill,
                can_delay,
            },
            round: enc.round,
            outcome: self.world.resource::<ResultState>().0,
            log: self.world.resource::<Log>().0.iter().cloned().collect(),
            movements: self.movements.clone(),
        }
    }
}

pub(crate) fn die(w: &mut World, s: u32) -> u32 {
    let mut r = w.resource_mut::<Random>();
    r.0 = r.0.wrapping_mul(6364136223846793005).wrapping_add(1);
    ((r.0 >> 32) as u32 % s) + 1
}

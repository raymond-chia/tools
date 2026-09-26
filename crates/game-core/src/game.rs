//! 載入、命令分派、回合流程與快照。
use crate::error::GameError;
use crate::model::{
    BattleMode, Board, CombatLogEvent, Command, Definition, DeliveredLogCount, Encounter,
    Exploration, Footprint, GridPos, Hp, Id, InitiativeRollLog, Log, MovementTransition, Outcome,
    Phase, Pos, Random, ResultState, SkillEffect, Skills, Snapshot, Team, TemporaryTerrains,
    TerrainCellView, TerrainDescriptionValues, TerrainDescriptionView, TerrainEffectView, Turn,
    TurnView, Unit, UnitView,
};
use crate::movement::{
    distance, entity_distance, fits, footprint_cells, footprint_distance, footprint_on_impassable,
    movement_cost, movement_ranges, terrain_damage, terrain_type, toward_skill_range, unit_at_cell,
};
use crate::skill::{
    can_use_skill, closest_occupied_cell, effective_block, effective_dodge, skill_ranges,
};
use crate::{authoring, error, gameplay_config};
use bevy_ecs::prelude::{Entity, World};
use std::collections::{HashMap, HashSet};

pub struct Game {
    pub(crate) world: World,
    pub(crate) movements: Vec<MovementTransition>,
}

impl Game {
    pub fn from_documents(definitions: &str, map: &str) -> Result<Self, GameError> {
        let definitions: authoring::Definitions = toml::from_str(definitions)
            .map_err(|e| error::definitions_toml_parse(e.to_string()))?;
        let map: authoring::Map =
            toml::from_str(map).map_err(|e| error::map_toml_parse(e.to_string()))?;
        Self::from_authoring(definitions, map)
    }
    pub fn from_authoring(
        definitions: authoring::Definitions,
        map: authoring::Map,
    ) -> Result<Self, GameError> {
        Self::from_definition(authoring::into_definition(definitions, map)?)
    }
    pub(crate) fn from_definition(d: Definition) -> Result<Self, GameError> {
        if d.map.width <= 0 || d.map.height <= 0 || d.map.width.checked_mul(d.map.height).is_none()
        {
            return Err(error::invalid_map_dimensions());
        }
        if d.map.terrains.iter().any(|terrain| {
            terrain.x < 0 || terrain.y < 0 || terrain.x >= d.map.width || terrain.y >= d.map.height
        }) {
            return Err(error::terrain_out_of_bounds());
        }
        for terrain in &d.map.terrains {
            if !d.terrain_types.contains_key(&terrain.kind) {
                return Err(error::unknown_terrain_type(&terrain.kind));
            }
        }
        let mut terrain_positions = HashSet::new();
        for terrain in &d.map.terrains {
            if !terrain_positions.insert((terrain.x, terrain.y, terrain.kind.as_str())) {
                return Err(error::duplicate_terrain(terrain.x, terrain.y));
            }
        }
        let mut w = World::new();
        let mut terrains: HashMap<GridPos, Vec<String>> = HashMap::new();
        for terrain in d.map.terrains {
            terrains
                .entry(GridPos {
                    x: terrain.x,
                    y: terrain.y,
                })
                .or_default()
                .push(terrain.kind);
        }
        for kinds in terrains.values_mut() {
            kinds.sort();
        }
        w.insert_resource(Board {
            width: d.map.width,
            height: d.map.height,
            terrains,
            terrain_types: d.terrain_types,
        });
        w.insert_resource(Encounter::default());
        w.insert_resource(Exploration {
            mode: BattleMode::Exploring,
            turns: HashMap::new(),
        });
        w.insert_resource(TemporaryTerrains::default());
        w.insert_resource(Turn {
            actor: None,
            phase: Phase::Ended,
            movement_remaining: 0,
            movement_segments_used: 0,
        });
        w.insert_resource(Random(0xc0ffee));
        w.insert_resource(Log::default());
        w.insert_resource(DeliveredLogCount::default());
        w.insert_resource(ResultState(Outcome::Ongoing));
        let mut skills = HashMap::new();
        for skill in d.skills {
            if skill.min_range < 0 || skill.max_range < skill.min_range {
                return Err(error::invalid_skill_range(&skill.id));
            }
            if let SkillEffect::Mire { terrain, duration } = &skill.effect {
                if *duration == 0 {
                    return Err(error::invalid_duration(&skill.id));
                }
                if !w.resource::<Board>().terrain_types.contains_key(terrain) {
                    return Err(error::invalid_skill_terrain(&skill.id));
                }
            }
            if skills.insert(skill.id.clone(), skill).is_some() {
                return Err(error::duplicate_skill_id());
            }
        }
        w.insert_resource(Skills {
            definitions: skills,
        });
        let mut ids = HashSet::new();
        let mut occupied = HashSet::new();
        for u in d.units {
            if !ids.insert(u.id) {
                return Err(error::duplicate_unit_id(u.id));
            }
            let f = Footprint {
                width: u.width,
                height: u.height,
            };
            if f.width <= 0 || f.height <= 0 || u.hp <= 0 {
                return Err(error::invalid_unit_size_or_hp(u.id));
            }
            let p = GridPos { x: u.x, y: u.y };
            if !fits(w.resource::<Board>(), p, f) {
                return Err(error::unit_out_of_bounds(u.id));
            }
            if footprint_on_impassable(w.resource::<Board>(), p, f) {
                return Err(error::unit_on_impassable(u.id));
            }
            if footprint_cells(p, f)
                .iter()
                .any(|cell| !occupied.insert(*cell))
            {
                return Err(error::overlapping_unit(u.id));
            }
            if let Some(unknown) = u
                .skills
                .iter()
                .find(|skill| !w.resource::<Skills>().definitions.contains_key(*skill))
            {
                return Err(error::unknown_unit_skill(u.id, unknown));
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
                    unit_type: u.unit_type,
                    visual: u.visual,
                    team: u.team,
                    movement: u.movement,
                    initiative: u.initiative,
                    dodge: u.dodge,
                    block: u.block,
                    attack: u.attack,
                    power: u.power,
                    skills: u.skills,
                },
            ));
        }
        Ok(Self {
            world: w,
            movements: Vec::new(),
        })
    }
    // 每次只執行一個命令或一段自動回合，讓畫面先完成呈現再推進。
    pub fn command(&mut self, c: Command) -> Result<Snapshot, GameError> {
        self.apply_command(c)?;
        Ok(self.snapshot())
    }
    fn apply_command(&mut self, c: Command) -> Result<(), GameError> {
        self.movements.clear();
        if self.world.resource::<ResultState>().0 != Outcome::Ongoing {
            return Ok(());
        }
        match c {
            Command::Start => self.start(),
            Command::Continue => self.continue_battle(),
            Command::SelectUnit { actor } => self.select_exploration_unit(actor),
            Command::Move { actor, x, y } => {
                self.move_to(actor, GridPos { x, y })?;
                if self.world.resource::<Exploration>().mode == BattleMode::Exploring
                    && self.has_active_enemy()
                {
                    self.world.resource_mut::<Exploration>().mode = BattleMode::Combat;
                    self.roll_round();
                }
                Ok(())
            }
            Command::Skill { actor, x, y, skill } => {
                let definition = self
                    .world
                    .resource::<Skills>()
                    .definitions
                    .get(&skill)
                    .cloned()
                    .ok_or_else(|| error::unknown_skill(&skill))?;
                let attack = matches!(
                    definition.effect,
                    SkillEffect::Attack { .. } | SkillEffect::Push { .. }
                );
                let target_id = unit_at_cell(&self.world, GridPos { x, y })
                    .and_then(|entity| self.world.get::<Id>(entity).map(|id| id.0));
                self.use_skill_at_cell(actor, GridPos { x, y }, definition)?;
                if attack && self.world.resource::<Exploration>().mode == BattleMode::Exploring {
                    self.world.resource_mut::<Exploration>().mode = BattleMode::AttackPending;
                    if let Some(id) = target_id {
                        if self.entity(id).is_some() {
                            self.world
                                .resource_mut::<Encounter>()
                                .participants
                                .insert(id);
                        }
                    }
                    self.activate_enemies_near_players();
                }
                Ok(())
            }
            Command::EndTurn { actor } => {
                self.ensure(actor)?;
                self.finish();
                Ok(())
            }
            Command::Delay { actor, after } => self.delay(actor, after),
        }?;
        self.outcome();
        if self.world.resource::<Exploration>().mode != BattleMode::Combat
            && self.world.resource::<Turn>().phase == Phase::Ended
            && self.world.resource::<ResultState>().0 == Outcome::Ongoing
        {
            self.advance_exploration();
        }
        Ok(())
    }
    pub fn set_random_seed(&mut self, seed: u64) {
        self.world.resource_mut::<Random>().0 = seed;
    }
    pub(crate) fn start(&mut self) -> Result<(), GameError> {
        if self.world.resource::<Turn>().actor.is_some() {
            return Ok(());
        }
        let players: Vec<_> = self
            .world
            .query::<(&Id, &Unit)>()
            .iter(&self.world)
            .filter(|(_, unit)| unit.team == Team::Player)
            .map(|(id, _)| id.0)
            .collect();
        let mut players = players;
        players.sort();
        self.world
            .resource_mut::<Encounter>()
            .participants
            .extend(players.iter().copied());
        if self.activate_enemies_near_players() {
            self.world.resource_mut::<Exploration>().mode = BattleMode::Combat;
            self.roll_round();
        } else {
            self.reset_exploration_turns(&players);
        }
        Ok(())
    }
    fn has_active_enemy(&self) -> bool {
        self.world
            .resource::<Encounter>()
            .participants
            .iter()
            .any(|id| {
                self.entity(*id).is_some_and(|entity| {
                    self.world
                        .get::<Unit>(entity)
                        .expect("單位應具有 Unit")
                        .team
                        != Team::Player
                })
            })
    }
    fn activate_enemies_near_players(&mut self) -> bool {
        let players: Vec<_> = self
            .world
            .iter_entities()
            .filter(|entity| {
                entity
                    .get::<Unit>()
                    .is_some_and(|unit| unit.team == Team::Player)
            })
            .map(|entity| entity.id())
            .collect();
        let enemies: Vec<_> = self
            .world
            .iter_entities()
            .filter(|entity| {
                entity
                    .get::<Unit>()
                    .is_some_and(|unit| unit.team != Team::Player)
            })
            .filter(|entity| {
                players.iter().any(|player| {
                    entity_distance(&self.world, *player, entity.id())
                        <= gameplay_config::ENCOUNTER_RANGE
                })
            })
            .filter_map(|entity| entity.get::<Id>().map(|id| id.0))
            .collect();
        let found = !enemies.is_empty();
        self.world
            .resource_mut::<Encounter>()
            .participants
            .extend(enemies);
        found
    }
    fn reset_exploration_turns(&mut self, players: &[i64]) {
        let mut turns = HashMap::new();
        for id in players {
            if let Some(entity) = self.entity(*id) {
                let movement = self
                    .world
                    .get::<Unit>(entity)
                    .expect("玩家單位應具有 Unit")
                    .movement;
                turns.insert(
                    *id,
                    Turn {
                        actor: Some(*id),
                        phase: Phase::Ready,
                        movement_remaining: movement,
                        movement_segments_used: 0,
                    },
                );
            }
        }
        let next = players.iter().find_map(|id| turns.get(id).cloned());
        self.world.resource_mut::<Exploration>().turns = turns;
        if let Some(turn) = next {
            *self.world.resource_mut::<Turn>() = turn;
        }
    }
    fn select_exploration_unit(&mut self, actor: i64) -> Result<(), GameError> {
        if self.world.resource::<Exploration>().mode == BattleMode::Combat {
            return Err(error::wrong_turn());
        }
        if self.world.resource::<Turn>().actor == Some(actor) {
            return Ok(());
        }
        let next = self
            .world
            .resource::<Exploration>()
            .turns
            .get(&actor)
            .cloned()
            .ok_or(error::wrong_turn())?;
        let current = self.world.resource::<Turn>().clone();
        if let Some(id) = current.actor {
            self.world
                .resource_mut::<Exploration>()
                .turns
                .insert(id, current);
        }
        *self.world.resource_mut::<Turn>() = next;
        Ok(())
    }
    fn advance_exploration(&mut self) {
        let actor = self.world.resource::<Turn>().actor;
        if let Some(id) = actor {
            self.world.resource_mut::<Exploration>().turns.remove(&id);
        }
        let mut remaining: Vec<_> = self
            .world
            .resource::<Exploration>()
            .turns
            .keys()
            .copied()
            .collect();
        remaining.sort();
        if let Some(id) = remaining.first() {
            let next = self
                .world
                .resource::<Exploration>()
                .turns
                .get(id)
                .expect("剩餘單位應有回合")
                .clone();
            *self.world.resource_mut::<Turn>() = next;
        } else if self.world.resource::<Exploration>().mode == BattleMode::AttackPending {
            self.world.resource_mut::<Exploration>().mode = BattleMode::Combat;
            self.roll_round();
        } else {
            let mut players: Vec<_> = self
                .world
                .iter_entities()
                .filter(|entity| {
                    entity
                        .get::<Unit>()
                        .is_some_and(|unit| unit.team == Team::Player)
                })
                .filter_map(|entity| entity.get::<Id>().map(|id| id.0))
                .collect();
            players.sort();
            self.reset_exploration_turns(&players);
        }
    }
    fn roll_round(&mut self) {
        let next_round = self.world.resource::<Encounter>().round + 1;
        self.world
            .resource_mut::<TemporaryTerrains>()
            .0
            .retain(|_, terrains| {
                terrains.retain(|_, terrain| terrain.expires_after_round >= next_round);
                !terrains.is_empty()
            });
        let active = self.world.resource::<Encounter>().participants.clone();
        let entries: Vec<_> = self
            .world
            .query::<(&Id, &Unit)>()
            .iter(&self.world)
            .filter(|(i, _)| active.contains(&i.0))
            .map(|(i, f)| (i.0, f.unit_type.clone(), f.team.clone(), f.initiative))
            .collect();
        let mut rolled: Vec<_> = entries
            .into_iter()
            .map(|(id, unit_type, team, modifier)| {
                let roll = die(&mut self.world, gameplay_config::INITIATIVE_DIE_SIDES) as i32;
                (roll + modifier, id, unit_type, team, roll, modifier)
            })
            .collect();
        rolled.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        let initiative_rolls = rolled
            .iter()
            .map(
                |(total, id, unit_type, team, roll, modifier)| InitiativeRollLog {
                    unit: *id,
                    unit_type: unit_type.clone(),
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
            movement_remaining: remaining,
            movement_segments_used: 0,
        }
    }
    pub(crate) fn finish(&mut self) {
        self.world.resource_mut::<Turn>().phase = Phase::Ended;
    }
    pub(crate) fn remove_unit(&mut self, entity: Entity, id: i64) {
        self.world.resource_mut::<Exploration>().turns.remove(&id);
        let is_actor = self.world.resource::<Turn>().actor == Some(id);
        let mut encounter = self.world.resource_mut::<Encounter>();
        encounter.participants.remove(&id);
        if let Some(index) = encounter.order.iter().position(|unit_id| *unit_id == id) {
            encounter.order.remove(index);
            if index < encounter.cursor {
                encounter.cursor -= 1;
            }
        }
        if is_actor {
            let mut turn = self.world.resource_mut::<Turn>();
            turn.actor = None;
            turn.phase = Phase::Ended;
        }
        self.world.entity_mut(entity).despawn();
    }
    fn advance_turn(&mut self) {
        let actor_removed = self.world.resource::<Turn>().actor.is_none();
        let end = {
            let mut e = self.world.resource_mut::<Encounter>();
            if !actor_removed {
                e.cursor += 1;
            }
            e.cursor >= e.order.len()
        };
        if end { self.roll_round() } else { self.begin() }
    }
    fn can_continue(&self) -> bool {
        if self.world.resource::<ResultState>().0 != Outcome::Ongoing
            || self.world.resource::<Exploration>().mode != BattleMode::Combat
        {
            return false;
        }
        let turn = self.world.resource::<Turn>();
        if turn.phase == Phase::Ended {
            return true;
        }
        turn.actor
            .and_then(|actor| self.entity(actor))
            .is_some_and(|entity| {
                self.world
                    .get::<Unit>(entity)
                    .expect("已建立的戰鬥單位應具有 Unit 元件")
                    .team
                    != Team::Player
            })
    }
    fn continue_battle(&mut self) -> Result<(), GameError> {
        if !self.can_continue() {
            return Err(error::cannot_continue());
        }
        if self.world.resource::<Turn>().phase == Phase::Ended {
            self.advance_turn();
            return Ok(());
        }
        self.enemy_turn_once()
    }
    fn delay(&mut self, actor: i64, after: i64) -> Result<(), GameError> {
        self.ensure(actor)?;
        let turn = self.world.resource::<Turn>();
        if turn.phase != Phase::Ready || turn.movement_segments_used != 0 {
            return Err(error::delay_after_action());
        }
        let encounter = self.world.resource::<Encounter>();
        let target_index = encounter
            .order
            .iter()
            .position(|unit_id| *unit_id == after)
            .ok_or(error::missing_delay_target())?;
        if target_index <= encounter.cursor {
            return Err(error::invalid_delay_target());
        }
        let mut encounter = self.world.resource_mut::<Encounter>();
        let current_index = encounter.cursor;
        let delayed_actor = encounter.order.remove(current_index);
        encounter.order.insert(target_index, delayed_actor);
        drop(encounter);
        self.begin();
        Ok(())
    }
    pub(crate) fn ensure(&self, a: i64) -> Result<(), GameError> {
        if self.world.resource::<Turn>().actor != Some(a) {
            Err(error::wrong_turn())
        } else {
            Ok(())
        }
    }
    fn enemy_turn_once(&mut self) -> Result<(), GameError> {
        let a = self
            .world
            .resource::<Turn>()
            .actor
            .clone()
            .ok_or(error::missing_initiative_unit())?;
        let e = self.entity(a).ok_or(error::missing_initiative_unit())?;
        let first_skill = self
            .world
            .get::<Unit>(e)
            .expect("已建立的戰鬥單位應具有 Unit 元件")
            .skills
            .first()
            .ok_or_else(|| error::missing_ai_skill(a))?;
        let skill = self
            .world
            .resource::<Skills>()
            .definitions
            .get(first_skill)
            .expect("載入時已驗證單位技能 ID")
            .clone();
        let target = if matches!(skill.effect, SkillEffect::Heal { .. }) {
            self.closest_wounded_ally(e, skill.min_range)
        } else {
            self.closest(e)
        };
        let t = match target {
            Some(v) => v,
            None => {
                self.finish();
                return Ok(());
            }
        };
        let target_footprint = *self.world.get::<Footprint>(t).expect("目標應具有佔用尺寸");
        let current_distance = entity_distance(&self.world, e, t);
        if current_distance < skill.min_range || current_distance > skill.max_range {
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
            if let Some(path) = toward_skill_range(
                &self.world,
                e,
                start,
                goal,
                target_footprint,
                fp,
                b,
                skill.min_range,
                skill.max_range,
            ) {
                if let [_, .., last] = path.as_slice() {
                    self.movements.push(MovementTransition {
                        unit_id: a,
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
        let target_cell = closest_occupied_cell(&self.world, e, t);
        let position = self.world.get::<Pos>(e).expect("單位應具有位置").0;
        let footprint = *self.world.get::<Footprint>(e).expect("單位應具有佔用尺寸");
        let target_distance = footprint_distance(
            position,
            footprint,
            target_cell,
            Footprint {
                width: 1,
                height: 1,
            },
        );
        if target_distance >= skill.min_range && target_distance <= skill.max_range {
            if matches!(skill.effect, SkillEffect::Mire { .. }) {
                return self.use_cell_skill(a, target_cell, skill);
            }
            let id = self
                .world
                .get::<Id>(t)
                .expect("已建立的戰鬥單位應具有 Id 元件")
                .0
                .clone();
            self.use_skill(a, id, target_cell, skill)?
        } else {
            self.finish()
        }
        Ok(())
    }
    pub(crate) fn entity(&self, id: i64) -> Option<Entity> {
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
            .filter(|q| q.get::<Unit>().is_some_and(|f| &f.team != attacker))
            .min_by_key(|q| {
                distance(
                    p,
                    q.get::<Pos>().expect("已建立的戰鬥單位應具有 Pos 元件").0,
                )
            })
            .map(|q| q.id())
    }
    fn closest_wounded_ally(&self, e: Entity, min_range: i32) -> Option<Entity> {
        let position = self.world.get::<Pos>(e)?.0;
        let team = &self.world.get::<Unit>(e)?.team;
        self.world
            .iter_entities()
            .filter(|candidate| {
                (min_range == 0 || candidate.id() != e)
                    && candidate
                        .get::<Unit>()
                        .is_some_and(|unit| &unit.team == team)
                    && candidate
                        .get::<Hp>()
                        .is_some_and(|hp| hp.current < hp.maximum)
            })
            .min_by_key(|candidate| {
                distance(
                    position,
                    candidate
                        .get::<Pos>()
                        .expect("已建立的戰鬥單位應具有 Pos 元件")
                        .0,
                )
            })
            .map(|candidate| candidate.id())
    }
    fn outcome(&mut self) {
        let mut p = false;
        let mut e = false;
        for q in self.world.iter_entities() {
            if let Some(f) = q.get::<Unit>() {
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
        let delivered_count = self.world.resource::<DeliveredLogCount>().0;
        let log = self.world.resource::<Log>().0[delivered_count..].to_vec();
        self.world.resource_mut::<DeliveredLogCount>().0 += log.len();
        let b = self.world.resource::<Board>().clone();
        let temporary_terrains = self.world.resource::<TemporaryTerrains>().clone();
        let enc = self.world.resource::<Encounter>().clone();
        let turn = self.world.resource::<Turn>().clone();
        let mut units: Vec<_> = self
            .world
            .iter_entities()
            .filter_map(|e| {
                Some((
                    e.id(),
                    e.get::<Id>()?,
                    e.get::<Pos>()?,
                    e.get::<Footprint>()?,
                    e.get::<Hp>()?,
                    e.get::<Unit>()?,
                ))
            })
            .map(|(entity, i, p, fp, h, f)| UnitView {
                id: i.0,
                unit_type: f.unit_type.clone(),
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
                power: f.power,
                active: enc.participants.contains(&i.0),
            })
            .collect();
        units.sort_by(|a, b| a.id.cmp(&b.id));
        let player_turn = turn
            .actor
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
            && turn.movement_segments_used < 2;
        let movement_ranges = if can_move {
            turn.actor
                .and_then(|actor| self.entity(actor))
                .map(|entity| movement_ranges(&self.world, entity, &turn))
        } else {
            None
        };
        let (reachable, second_reachable) = movement_ranges.unwrap_or_default();
        let skill_ranges = if player_turn && turn.phase != Phase::Ended {
            turn.actor
                .and_then(|actor| self.entity(actor))
                .map(|entity| skill_ranges(&self.world, entity))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let can_skill = player_turn && can_use_skill(&turn);
        let exploring = self.world.resource::<Exploration>().mode != BattleMode::Combat;
        let turn_order: Vec<_> = if exploring {
            let mut available: Vec<_> = self
                .world
                .resource::<Exploration>()
                .turns
                .keys()
                .copied()
                .filter(|id| Some(*id) != turn.actor)
                .collect();
            available.sort();
            available
        } else {
            enc.order
                .iter()
                .skip(enc.cursor + usize::from(turn.actor.is_some()))
                .cloned()
                .collect()
        };
        let can_delay = !exploring
            && player_turn
            && turn.phase == Phase::Ready
            && turn.movement_segments_used == 0
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
                    let terrains = crate::movement::terrains_at(world, position);
                    let cost = movement_cost(world, position);
                    let damage = terrains
                        .iter()
                        .map(|kind| terrain_type(board, kind).damage)
                        .sum();
                    let effect_descriptions = terrains
                        .iter()
                        .filter(|kind| kind.as_str() != "rough")
                        .map(|kind| {
                            let definition = terrain_type(board, kind);
                            let remaining = temporary
                                .0
                                .get(&position)
                                .and_then(|items| items.get(kind))
                                .map(|terrain| {
                                    terrain.expires_after_round.saturating_sub(enc.round) + 1
                                });
                            let values = TerrainDescriptionValues {
                                defense_penalty: remaining.map(|_| {
                                    definition.dodge_penalty.max(definition.block_penalty)
                                }),
                                extra_movement_cost: remaining
                                    .map(|_| definition.extra_movement_cost as i32),
                                remaining_rounds: remaining.map(|rounds| rounds as i32),
                                damage: (remaining.is_none() && definition.damage > 0)
                                    .then_some(definition.damage),
                            };
                            TerrainDescriptionView {
                                terrain: kind.clone(),
                                values,
                            }
                        })
                        .collect();
                    TerrainCellView {
                        x,
                        y,
                        passable: terrains.iter().all(|kind| {
                            terrain_type(board, kind).entry_rule
                                == crate::model::TerrainEntryRule::Walkable
                        }),
                        base_kind: if terrains.iter().any(|kind| kind == "rough") {
                            "rough"
                        } else {
                            "plain"
                        }
                        .to_string(),
                        terrains,
                        unit_id: units
                            .iter()
                            .find(|unit| unit.occupied_cells.contains(&position))
                            .map(|unit| unit.id),
                        cost,
                        damage,
                        effect_descriptions,
                    }
                })
            })
            .collect();
        Snapshot {
            width: b.width,
            height: b.height,
            terrain_effects: {
                let mut effects = Vec::new();
                for (position, kinds) in &b.terrains {
                    for kind in kinds {
                        effects.push(TerrainEffectView {
                            x: position.x,
                            y: position.y,
                            damage: terrain_damage(&b, kind),
                            visual: terrain_type(&b, kind).visual.clone(),
                            effect: kind.clone(),
                            remaining_rounds: None,
                        });
                    }
                }
                for (position, kinds) in &temporary_terrains.0 {
                    for (kind, terrain) in kinds {
                        if b.terrains
                            .get(position)
                            .is_some_and(|items| items.contains(kind))
                        {
                            continue;
                        }
                        effects.push(TerrainEffectView {
                            x: position.x,
                            y: position.y,
                            damage: terrain_damage(&b, kind),
                            visual: terrain_type(&b, kind).visual.clone(),
                            effect: kind.clone(),
                            remaining_rounds: Some(
                                terrain.expires_after_round.saturating_sub(enc.round) + 1,
                            ),
                        });
                    }
                }
                effects.sort_by_key(|effect| (effect.y, effect.x, effect.effect.clone()));
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
                can_continue: self.can_continue(),
                move_remaining: turn.movement_remaining,
                can_move,
                can_skill,
                can_delay,
            },
            round: enc.round,
            battle_mode: match self.world.resource::<Exploration>().mode {
                BattleMode::Exploring => "exploring",
                BattleMode::AttackPending => "attack_pending",
                BattleMode::Combat => "combat",
            }
            .to_owned(),
            outcome: self.world.resource::<ResultState>().0,
            // 完整紀錄每次複製並傳給 Godot，會隨回合數增加造成嚴重效能問題。
            log,
            movements: self.movements.clone(),
        }
    }
}

pub(crate) fn die(w: &mut World, s: u32) -> u32 {
    let mut r = w.resource_mut::<Random>();
    r.0 = r.0.wrapping_mul(6364136223846793005).wrapping_add(1);
    ((r.0 >> 32) as u32 % s) + 1
}

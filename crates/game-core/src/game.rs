//! 載入、命令分派、回合流程與快照。
use crate::error::GameError;
use crate::model::{
    BattleMode, Board, CombatLogEvent, Command, DeliveredLogCount, Encounter, Exploration,
    Footprint, GridPos, Hp, Id, InitiativeRollLog, Log, MovementTransition, Outcome, Phase, Pos,
    Random, ResultState, SkillDef, SkillEffect, SkillTargetKind, Skills, Snapshot, Team,
    TemporaryTerrains, TerrainCellView, TerrainDescriptionValues, TerrainDescriptionView,
    TerrainEffectView, TerrainLayer, Turn, TurnView, Unit, UnitView,
};
use crate::movement::{
    can_move, distance, entity_distance, fits, footprint_cell_distance, footprint_cells,
    movement_ranges, toward_skill_range, unit_at_cell,
};
use crate::skill::{
    can_use_skill, check_skill_range, closest_occupied_cell, effective_block, effective_dodge,
    skill_ranges, target_kind,
};
use crate::terrain::{
    footprint_on_impassable, ground_at, movement_cost, terrain_damage, terrain_type, terrains_at,
};
use crate::{authoring, error, gameplay_config};
use bevy_ecs::prelude::{Entity, World};
use std::collections::{HashMap, HashSet};

pub struct Game {
    pub(crate) world: World,
    pub(crate) movements: Vec<MovementTransition>,
}

pub(crate) struct DamageResult {
    pub(crate) remaining_hp: i32,
    pub(crate) max_hp: i32,
    pub(crate) downed: bool,
}

impl Game {
    pub fn from_documents(definitions: &str, map: &str) -> Result<Self, GameError> {
        let definitions: authoring::Definitions = toml::from_str(definitions)
            .map_err(|e| error::definitions_toml_parse(e.to_string()))?;
        let map: authoring::Map =
            toml::from_str(map).map_err(|e| error::map_toml_parse(e.to_string()))?;
        Self::from_authoring(definitions, map)
    }
    /// 驗證作者資料並建立戰鬥；所有載入驗證集中於此。
    pub fn from_authoring(
        definitions: authoring::Definitions,
        map: authoring::Map,
    ) -> Result<Self, GameError> {
        let authoring::Definitions {
            terrain_types,
            skills,
            unit_types,
        } = definitions;
        let authoring::Map {
            name,
            width,
            height,
            terrains: terrain_placements,
            units: placements,
        } = map;
        if name.trim().is_empty() {
            return Err(error::empty_map_name());
        }
        if placements
            .iter()
            .any(|unit| matches!(&unit.team, Team::Enemy(name) if name.trim().is_empty()))
        {
            return Err(error::empty_enemy_faction());
        }
        let mut terrains = HashMap::new();
        for terrain in terrain_types {
            if terrains.insert(terrain.id.clone(), terrain).is_some() {
                return Err(error::duplicate_terrain_type_id());
            }
        }
        let terrain_types = terrains;
        // 地形數值不以負值反轉成治療或加成。
        if let Some(kind) = terrain_types
            .iter()
            .filter(|(_, terrain)| {
                terrain.damage < 0 || terrain.dodge_penalty < 0 || terrain.block_penalty < 0
            })
            .map(|(kind, _)| kind)
            .min()
        {
            return Err(error::invalid_terrain_values(kind));
        }
        if terrain_types
            .get(gameplay_config::DEFAULT_GROUND_TERRAIN)
            .is_none_or(|terrain| terrain.layer != TerrainLayer::Ground)
        {
            return Err(error::missing_default_ground_terrain());
        }
        let skill_ids: HashSet<_> = skills.iter().map(|skill| skill.id.as_str()).collect();
        let mut types = HashMap::new();
        for kind in unit_types {
            if kind.id.trim().is_empty() || kind.hp <= 0 || kind.width <= 0 || kind.height <= 0 {
                return Err(error::invalid_unit_type(&kind.id));
            }
            if let Some(unknown) = kind
                .skills
                .iter()
                .find(|skill| !skill_ids.contains(skill.as_str()))
            {
                return Err(error::unknown_unit_skill(&kind.id, unknown));
            }
            let mut unit_skills = HashSet::new();
            if let Some(duplicate) = kind
                .skills
                .iter()
                .find(|skill| !unit_skills.insert(skill.as_str()))
            {
                return Err(error::duplicate_unit_skill(&kind.id, duplicate));
            }
            if types.insert(kind.id.clone(), kind).is_some() {
                return Err(error::duplicate_unit_type_id());
            }
        }
        let mut used_ids = HashSet::new();
        for placement in &placements {
            if placement.id <= 0 {
                return Err(error::invalid_unit_placement_id());
            }
            if !used_ids.insert(placement.id) {
                return Err(error::duplicate_unit_placement_id(placement.id));
            }
            let kind = types
                .get(&placement.unit_type)
                .ok_or_else(|| error::unknown_unit_type(&placement.unit_type))?;
            // 敵方 AI 依技能決定行動，沒有技能就無法推進回合。
            if matches!(placement.team, Team::Enemy(_)) && kind.skills.is_empty() {
                return Err(error::missing_ai_skill(placement.id));
            }
        }
        if width <= 0
            || height <= 0
            || width.checked_mul(height).is_none()
            || width.checked_add(height).is_none()
        {
            return Err(error::invalid_map_dimensions());
        }
        validate_numeric_ranges(&terrain_types, &skills, &types, width, height)?;
        if terrain_placements.iter().any(|terrain| {
            terrain.x < 0 || terrain.y < 0 || terrain.x >= width || terrain.y >= height
        }) {
            return Err(error::terrain_out_of_bounds());
        }
        for terrain in &terrain_placements {
            if !terrain_types.contains_key(&terrain.kind) {
                return Err(error::unknown_terrain_type(&terrain.kind));
            }
        }
        let mut terrain_positions = HashSet::new();
        let mut ground_positions = HashSet::new();
        for terrain in &terrain_placements {
            if !terrain_positions.insert((terrain.x, terrain.y, terrain.kind.as_str())) {
                return Err(error::duplicate_terrain(terrain.x, terrain.y));
            }
            if terrain_types[&terrain.kind].layer == TerrainLayer::Ground
                && !ground_positions.insert((terrain.x, terrain.y))
            {
                return Err(error::multiple_ground_terrains(terrain.x, terrain.y));
            }
        }
        let mut skill_definitions = HashMap::new();
        for skill in skills {
            if skill.min_range < 0 || skill.max_range < skill.min_range {
                return Err(error::invalid_skill_range(&skill.id));
            }
            if let SkillEffect::Mire { terrain, duration } = &skill.effect {
                if *duration == 0 {
                    return Err(error::invalid_duration(&skill.id));
                }
                let definition = terrain_types
                    .get(terrain)
                    .ok_or_else(|| error::invalid_skill_terrain(&skill.id))?;
                // 暫時地形疊在既有地形上，不能是 ground，否則同格會有兩個 ground。
                if definition.layer != TerrainLayer::Overlay {
                    return Err(error::skill_terrain_not_overlay(&skill.id));
                }
            }
            if skill_definitions.insert(skill.id.clone(), skill).is_some() {
                return Err(error::duplicate_skill_id());
            }
        }
        let mut terrains: HashMap<GridPos, Vec<String>> = HashMap::new();
        for terrain in terrain_placements {
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
        let mut w = World::new();
        w.insert_resource(Board {
            width,
            height,
            terrains,
            terrain_types,
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
        w.insert_resource(Skills {
            definitions: skill_definitions,
        });
        // 佔用格判斷需要已建立的棋盤，因此在建立 Board 後驗證。
        let mut occupied = HashSet::new();
        for placement in placements {
            let authoring::UnitPlacement {
                id,
                unit_type,
                team,
                x,
                y,
            } = placement;
            let authoring::UnitType {
                id: _,
                visual,
                width,
                height,
                hp,
                movement,
                initiative,
                dodge,
                block,
                attack,
                power,
                skills,
            } = types
                .get(&unit_type)
                .expect("單位配置的類型已在上方驗證")
                .clone();
            if x.checked_add(width).is_none() || y.checked_add(height).is_none() {
                return Err(error::unit_out_of_bounds(id));
            }
            let f = Footprint { width, height };
            let p = GridPos { x, y };
            if !fits(w.resource::<Board>(), p, f) {
                return Err(error::unit_out_of_bounds(id));
            }
            if footprint_on_impassable(&w, p, f) {
                return Err(error::unit_on_impassable(id));
            }
            if footprint_cells(p, f)
                .iter()
                .any(|cell| !occupied.insert(*cell))
            {
                return Err(error::overlapping_unit(id));
            }
            w.spawn((
                Id(id),
                Pos(p),
                f,
                Hp {
                    current: hp,
                    maximum: hp,
                },
                Unit {
                    unit_type,
                    visual,
                    team,
                    movement,
                    initiative,
                    dodge,
                    block,
                    attack,
                    power,
                    skills,
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
        let mut snapshot = self.snapshot(None);
        let delivered_count = self.world.resource::<DeliveredLogCount>().0;
        snapshot.log = self.world.resource::<Log>().0[delivered_count..].to_vec();
        self.world.resource_mut::<DeliveredLogCount>().0 += snapshot.log.len();
        snapshot.movements = self.movements.clone();
        Ok(snapshot)
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
                let attack = target_kind(&definition.effect) == SkillTargetKind::Enemy;
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
        let Encounter {
            participants,
            order: _,
            cursor: _,
            round: _,
        } = self.world.resource::<Encounter>();
        participants.iter().any(|id| {
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
        self.activate_enemies_near(&players)
    }
    /// 集中處理遭遇距離與敵方參與者；呼叫端決定要檢查哪些單位。
    pub(crate) fn activate_enemies_near(&mut self, units: &[Entity]) -> bool {
        let enemies: Vec<_> = self
            .world
            .iter_entities()
            .filter(|entity| {
                entity
                    .get::<Unit>()
                    .is_some_and(|unit| unit.team != Team::Player)
            })
            .filter(|entity| {
                units.iter().any(|unit| {
                    entity_distance(&self.world, *unit, entity.id())
                        <= gameplay_config::ENCOUNTER_RANGE
                })
            })
            .filter_map(|entity| entity.get::<Id>().map(|id| id.0))
            .collect();
        let found = !enemies.is_empty();
        let Encounter {
            participants,
            order: _,
            cursor: _,
            round: _,
        } = &mut *self.world.resource_mut::<Encounter>();
        participants.extend(enemies);
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
        let current @ Turn {
            actor,
            phase: _,
            movement_remaining: _,
            movement_segments_used: _,
        } = self.world.resource::<Turn>().clone();
        if let Some(id) = actor {
            self.world
                .resource_mut::<Exploration>()
                .turns
                .insert(id, current);
        }
        *self.world.resource_mut::<Turn>() = next;
        Ok(())
    }
    fn advance_exploration(&mut self) {
        let Turn {
            actor,
            phase: _,
            movement_remaining: _,
            movement_segments_used: _,
        } = self.world.resource::<Turn>();
        let actor = *actor;
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
        let Encounter {
            participants,
            order: _,
            cursor: _,
            round,
        } = self.world.resource::<Encounter>();
        let next_round = *round + 1;
        let active = participants.clone();
        self.world
            .resource_mut::<TemporaryTerrains>()
            .0
            .retain(|_, terrains| {
                terrains.retain(|_, terrain| terrain.expires_after_round >= next_round);
                !terrains.is_empty()
            });
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
            let Encounter {
                participants: _,
                order,
                cursor,
                round,
            } = &mut *self.world.resource_mut::<Encounter>();
            *round += 1;
            *order = rolled.into_iter().map(|(_, id, _, _, _, _)| id).collect();
            *cursor = 0;
        }
        let Encounter {
            participants: _,
            order: _,
            cursor: _,
            round,
        } = self.world.resource::<Encounter>();
        let round = *round;
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
        let Encounter {
            participants: _,
            order,
            cursor,
            round: _,
        } = self.world.resource::<Encounter>();
        let actor = order.get(*cursor).copied();
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
    /// 扣除生命值；歸零時立即移除單位，因此呼叫端應改用回傳值而非再讀取 Hp。
    pub(crate) fn apply_damage(&mut self, entity: Entity, id: i64, amount: i32) -> DamageResult {
        let mut hp = self
            .world
            .get_mut::<Hp>(entity)
            .expect("已建立的戰鬥單位應具有 Hp 元件");
        hp.current = (hp.current - amount).max(0);
        let result @ DamageResult {
            remaining_hp: _,
            max_hp: _,
            downed,
        } = DamageResult {
            remaining_hp: hp.current,
            max_hp: hp.maximum,
            downed: hp.current == 0,
        };
        if downed {
            self.remove_unit(entity, id);
        }
        result
    }
    pub(crate) fn remove_unit(&mut self, entity: Entity, id: i64) {
        self.world.resource_mut::<Exploration>().turns.remove(&id);
        let is_actor = self.world.resource::<Turn>().actor == Some(id);
        let Encounter {
            participants,
            order,
            cursor,
            round: _,
        } = &mut *self.world.resource_mut::<Encounter>();
        participants.remove(&id);
        if let Some(index) = order.iter().position(|unit_id| *unit_id == id) {
            order.remove(index);
            if index < *cursor {
                *cursor -= 1;
            }
        }
        if is_actor {
            let Turn {
                actor,
                phase,
                movement_remaining: _,
                movement_segments_used: _,
            } = &mut *self.world.resource_mut::<Turn>();
            *actor = None;
            *phase = Phase::Ended;
        }
        self.world.entity_mut(entity).despawn();
    }
    fn advance_turn(&mut self) {
        let actor_removed = self.world.resource::<Turn>().actor.is_none();
        let end = {
            let Encounter {
                participants: _,
                order,
                cursor,
                round: _,
            } = &mut *self.world.resource_mut::<Encounter>();
            if !actor_removed {
                *cursor += 1;
            }
            *cursor >= order.len()
        };
        if end { self.roll_round() } else { self.begin() }
    }
    fn can_continue(&self) -> bool {
        if self.world.resource::<ResultState>().0 != Outcome::Ongoing
            || self.world.resource::<Exploration>().mode != BattleMode::Combat
        {
            return false;
        }
        let Turn {
            actor,
            phase,
            movement_remaining: _,
            movement_segments_used: _,
        } = self.world.resource::<Turn>();
        if *phase == Phase::Ended {
            return true;
        }
        actor
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
        if !can_delay(turn) {
            return Err(error::delay_after_action());
        }
        let Encounter {
            participants: _,
            order,
            cursor,
            round: _,
        } = self.world.resource::<Encounter>();
        let target_index = order
            .iter()
            .position(|unit_id| *unit_id == after)
            .ok_or(error::missing_delay_target())?;
        if target_index <= *cursor {
            return Err(error::invalid_delay_target());
        }
        {
            let Encounter {
                participants: _,
                order,
                cursor,
                round: _,
            } = &mut *self.world.resource_mut::<Encounter>();
            let delayed_actor = order.remove(*cursor);
            order.insert(target_index, delayed_actor);
        }
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
        let Turn {
            actor,
            phase: _,
            movement_remaining: _,
            movement_segments_used: _,
        } = self.world.resource::<Turn>();
        let a = actor.ok_or(error::missing_initiative_unit())?;
        let e = self.entity(a).ok_or(error::missing_initiative_unit())?;
        let first_skill = self
            .world
            .get::<Unit>(e)
            .expect("已建立的戰鬥單位應具有 Unit 元件")
            .skills
            .first()
            .expect("載入時已驗證敵方單位至少有一個技能");
        let Skills { definitions } = self.world.resource::<Skills>();
        let skill @ SkillDef {
            id: _,
            ranged: _,
            min_range,
            max_range,
            effect: _,
        } = definitions
            .get(first_skill)
            .expect("載入時已驗證單位技能 ID")
            .clone();
        let target = if target_kind(&skill.effect) == SkillTargetKind::Ally {
            self.closest_wounded_ally(e, min_range)
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
        if check_skill_range(current_distance, min_range, max_range).is_err() {
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
                min_range,
                max_range,
            ) {
                if path.len() > 1 {
                    self.movements.push(MovementTransition {
                        unit_id: a,
                        path: path.clone(),
                        before_log_index: self.world.resource::<Log>().0.len(),
                    });
                    self.execute_move_path(e, a, &path);
                    if self.world.get_entity(e).is_err() {
                        return Ok(());
                    }
                }
            }
        }
        let target_cell = closest_occupied_cell(&self.world, e, t);
        let position = self.world.get::<Pos>(e).expect("單位應具有位置").0;
        let footprint = *self.world.get::<Footprint>(e).expect("單位應具有佔用尺寸");
        let target_distance = footprint_cell_distance(position, footprint, target_cell);
        if check_skill_range(target_distance, min_range, max_range).is_ok() {
            self.use_skill_at_cell(a, target_cell, skill)?
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
    /// 唯讀顯示查詢；事件只由 command 回傳，查看單位不儲存在核心。
    pub fn snapshot(&self, inspected_actor: Option<i64>) -> Snapshot {
        let b @ Board {
            width,
            height,
            terrains: fixed_terrains,
            terrain_types: _,
        } = self.world.resource::<Board>();
        let TemporaryTerrains(temporary_terrains) = self.world.resource::<TemporaryTerrains>();
        let Encounter {
            participants: _,
            order,
            cursor,
            round,
        } = self.world.resource::<Encounter>();
        let turn @ Turn {
            actor: turn_actor,
            phase,
            movement_remaining,
            movement_segments_used: _,
        } = self.world.resource::<Turn>();
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
                large: fp.width > 1 || fp.height > 1,
                occupied_cells: footprint_cells(p.0, *fp),
                hp: h.current,
                max_hp: h.maximum,
                movement: f.movement,
                initiative: f.initiative,
                dodge: effective_dodge(&self.world, entity),
                block: effective_block(&self.world, entity),
                attack: f.attack,
                power: f.power,
            })
            .collect();
        units.sort_by(|a, b| a.id.cmp(&b.id));
        let player_turn = turn_actor
            .and_then(|actor| self.entity(actor))
            .is_some_and(|entity| {
                self.world
                    .get::<Unit>(entity)
                    .expect("已建立的戰鬥單位應具有 Unit 元件")
                    .team
                    == Team::Player
            });
        let can_move = player_turn && can_move(turn);
        let movement_ranges = if can_move {
            turn_actor
                .and_then(|actor| self.entity(actor))
                .map(|entity| movement_ranges(&self.world, entity, turn))
        } else {
            None
        };
        let (reachable, second_reachable) = movement_ranges.unwrap_or_default();
        let (inspected_reachable, inspected_second_reachable) = inspected_actor
            .and_then(|actor| self.entity(actor))
            .map(|entity| {
                let actor = self
                    .world
                    .get::<Id>(entity)
                    .expect("戰鬥單位應具有 Id 元件")
                    .0;
                let inspected_turn = if *turn_actor == Some(actor) {
                    (*turn).clone()
                } else {
                    let movement = self
                        .world
                        .get::<Unit>(entity)
                        .expect("戰鬥單位應具有 Unit 元件")
                        .movement;
                    Turn {
                        actor: Some(actor),
                        phase: Phase::Ready,
                        movement_remaining: movement,
                        movement_segments_used: 0,
                    }
                };
                if crate::movement::can_move(&inspected_turn) {
                    crate::movement::movement_ranges(&self.world, entity, &inspected_turn)
                } else {
                    (Vec::new(), Vec::new())
                }
            })
            .unwrap_or_default();
        let skill_ranges = if player_turn && *phase != Phase::Ended {
            turn_actor
                .and_then(|actor| self.entity(actor))
                .map(|entity| skill_ranges(&self.world, entity))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let can_skill = player_turn && can_use_skill(turn);
        let Exploration { mode, turns } = self.world.resource::<Exploration>();
        let exploring = *mode != BattleMode::Combat;
        let turn_order: Vec<_> = if exploring {
            let mut available: Vec<_> = turns
                .keys()
                .copied()
                .filter(|id| Some(*id) != *turn_actor)
                .collect();
            available.sort();
            available
        } else {
            order
                .iter()
                .skip(*cursor + usize::from(turn_actor.is_some()))
                .cloned()
                .collect()
        };
        let can_delay = !exploring
            && player_turn
            && can_delay(turn)
            && turn_actor.is_some()
            && !turn_order.is_empty();
        let terrain_cells = (0..*height)
            .flat_map(|y| {
                let board = b;
                let temporary = temporary_terrains;
                let world = &self.world;
                let units = &units;
                (0..*width).map(move |x| {
                    let position = GridPos { x, y };
                    let terrains = terrains_at(world, position);
                    let cost = movement_cost(world, position);
                    let damage = terrains
                        .iter()
                        .map(|kind| terrain_type(board, kind).damage)
                        .sum();
                    let effect_descriptions = terrains
                        .iter()
                        .map(|kind| {
                            let crate::model::TerrainTypeDef {
                                id: _,
                                layer: _,
                                entry_rule,
                                damage,
                                extra_movement_cost,
                                dodge_penalty,
                                block_penalty,
                            } = terrain_type(board, kind);
                            let remaining = temporary
                                .get(&position)
                                .and_then(|items| items.get(kind))
                                .filter(|_| {
                                    !fixed_terrains
                                        .get(&position)
                                        .is_some_and(|items| items.contains(kind))
                                })
                                .map(|terrain| {
                                    terrain.expires_after_round.saturating_sub(*round) + 1
                                });
                            let values = TerrainDescriptionValues {
                                dodge_penalty: (*dodge_penalty > 0).then_some(*dodge_penalty),
                                block_penalty: (*block_penalty > 0).then_some(*block_penalty),
                                extra_movement_cost: (*extra_movement_cost > 0)
                                    .then_some(*extra_movement_cost),
                                remaining_rounds: remaining,
                                damage: (*damage > 0).then_some(*damage),
                            };
                            TerrainDescriptionView {
                                terrain: kind.clone(),
                                entry_rule: *entry_rule,
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
                        ground_visual: terrain_type(
                            board,
                            ground_at(board, &terrains).expect("terrains_at 必含 ground"),
                        )
                        .id
                        .clone(),
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
            width: *width,
            height: *height,
            terrain_effects: {
                let mut effects = Vec::new();
                for (position, kinds) in fixed_terrains {
                    // ground 由 terrain_cells 的 ground_visual 呈現，這裡只列 overlay。
                    for kind in kinds
                        .iter()
                        .filter(|kind| terrain_type(b, kind).layer == TerrainLayer::Overlay)
                    {
                        effects.push(TerrainEffectView {
                            x: position.x,
                            y: position.y,
                            damage: terrain_damage(b, kind),
                            visual: kind.clone(),
                            effect: kind.clone(),
                            remaining_rounds: None,
                        });
                    }
                }
                for (position, kinds) in temporary_terrains {
                    for (kind, terrain) in kinds {
                        if fixed_terrains
                            .get(position)
                            .is_some_and(|items| items.contains(kind))
                        {
                            continue;
                        }
                        effects.push(TerrainEffectView {
                            x: position.x,
                            y: position.y,
                            damage: terrain_damage(b, kind),
                            visual: kind.clone(),
                            effect: kind.clone(),
                            remaining_rounds: Some(
                                terrain.expires_after_round.saturating_sub(*round) + 1,
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
            inspected_reachable,
            inspected_second_reachable,
            skill_ranges,
            turn_order,
            turn: TurnView {
                can_end_turn: player_turn && *phase != Phase::Ended,
                actor: *turn_actor,
                phase: *phase,
                can_continue: self.can_continue(),
                move_remaining: *movement_remaining,
                can_move,
                can_skill,
                can_delay,
            },
            round: *round,
            battle_mode: *mode,
            outcome: self.world.resource::<ResultState>().0,
            // 查詢不重送事件；command 負責附上尚未送出的紀錄與移動。
            log: Vec::new(),
            movements: Vec::new(),
        }
    }
}

/// 尚未開始行動（未移動、未使用技能）才能延後。
fn can_delay(turn: &Turn) -> bool {
    let Turn {
        actor: _,
        phase,
        movement_remaining: _,
        movement_segments_used,
    } = turn;
    *phase == Phase::Ready && *movement_segments_used == 0
}

pub(crate) fn die(w: &mut World, s: u32) -> u32 {
    let mut r = w.resource_mut::<Random>();
    r.0 = r.0.wrapping_mul(6364136223846793005).wrapping_add(1);
    ((r.0 >> 32) as u32 % s) + 1
}

/// 載入時建立座標、地形疊加與戰鬥數值可安全運算的不變量。
fn validate_numeric_ranges(
    terrains: &HashMap<String, crate::model::TerrainTypeDef>,
    skills: &[SkillDef],
    units: &HashMap<String, authoring::UnitType>,
    width: i32,
    height: i32,
) -> Result<(), GameError> {
    // 同格最多一種 ground，overlay 與技能建立的地形都按種類去重。
    let maximum_cell_value = |value: fn(&crate::model::TerrainTypeDef) -> i128| {
        let ground = terrains
            .values()
            .filter(|terrain| terrain.layer == TerrainLayer::Ground)
            .map(value)
            .max()
            .unwrap_or(0);
        ground
            + terrains
                .values()
                .filter(|terrain| terrain.layer == TerrainLayer::Overlay)
                .map(value)
                .sum::<i128>()
    };
    let cost = 1 + maximum_cell_value(|terrain| i128::from(terrain.extra_movement_cost));
    let cells = i128::from(width) * i128::from(height);
    if cost * (cells + 1) > i128::from(u32::MAX)
        || maximum_cell_value(|terrain| i128::from(terrain.damage)) > i128::from(i32::MAX)
        || maximum_cell_value(|terrain| i128::from(terrain.dodge_penalty)) > i128::from(i32::MAX)
        || maximum_cell_value(|terrain| i128::from(terrain.block_penalty)) > i128::from(i32::MAX)
    {
        return Err(error::numeric_range("terrain_types"));
    }
    let maximum_hp = units.values().map(|unit| unit.hp).max().unwrap_or(0);
    let dodge_penalty = maximum_cell_value(|terrain| i128::from(terrain.dodge_penalty));
    let block_penalty = maximum_cell_value(|terrain| i128::from(terrain.block_penalty));
    for unit in units.values() {
        if i128::from(unit.dodge) - dodge_penalty < i128::from(i32::MIN)
            || i128::from(unit.block) - block_penalty < i128::from(i32::MIN)
            || i128::from(unit.movement) * 2 + cost > i128::from(u32::MAX)
            || width.checked_add(unit.width).is_none()
            || height.checked_add(unit.height).is_none()
            || unit
                .initiative
                .checked_add(gameplay_config::INITIATIVE_DIE_SIDES as i32)
                .is_none()
            || i64::from(gameplay_config::BASE_DEFENSE)
                + i64::from(unit.dodge.max(0))
                + i64::from(unit.block.max(0))
                > i64::from(i32::MAX)
        {
            return Err(error::numeric_range(&unit.id));
        }
        for skill in skills
            .iter()
            .filter(|skill| unit.skills.contains(&skill.id))
        {
            let (attack_bonus, power_bonus) = match skill.effect {
                SkillEffect::Attack {
                    attack_bonus,
                    power_bonus,
                }
                | SkillEffect::Push {
                    attack_bonus,
                    power_bonus,
                } => (attack_bonus, power_bonus),
                SkillEffect::Heal { power_bonus } => (0, power_bonus),
                SkillEffect::Mire { .. } => continue,
            };
            let attack = i64::from(unit.attack) + i64::from(attack_bonus);
            let power = i64::from(unit.power) + i64::from(power_bonus);
            if attack < i64::from(i32::MIN)
                || attack
                    + i64::from(gameplay_config::FLANKING_ATTACK_BONUS)
                    + i64::from(gameplay_config::ATTACK_DIE_SIDES)
                    > i64::from(i32::MAX)
                || power < i64::from(i32::MIN)
                || power.max(0) * 2 > i64::from(i32::MAX)
                || matches!(skill.effect, SkillEffect::Heal { .. })
                    && power.max(0) + i64::from(maximum_hp) > i64::from(i32::MAX)
            {
                return Err(error::numeric_range(&unit.id));
            }
        }
    }
    Ok(())
}

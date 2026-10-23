//! 載入、命令分派、回合流程與快照。
use crate::error::GameError;
use crate::model::{
    ActiveSkillTargetKind, ActiveSkills, BattleMode, Board, CombatLogEvent, Command, Encounter,
    Exploration, Footprint, GridPos, Hp, Id, InitiativeRollLog, Log, MovementTransition, Outcome,
    Phase, Pos, Random, ResultState, Snapshot, Team, TemporaryTerrains, TerrainCellView,
    TerrainDescriptionValues, TerrainDescriptionView, TerrainEffectView, TerrainEntryRule,
    TerrainGrid, TerrainLayer, Turn, TurnView, Unit, UnitView,
};
use crate::movement::{
    can_move, entity_distance, footprint_cells, movement_ranges, unit_at_cell, unit_cells,
};
use crate::skill::{can_use_skill, effective_block, effective_dodge, skill_ranges, target_kind};
use crate::terrain::{
    expire_temporary_terrains, grid_cell, ground_at, temporary_remaining_rounds, terrain_damage,
    terrain_type, terrains_at,
};
use crate::{authoring, error, gameplay_config};
use bevy_ecs::prelude::{Entity, World};
use std::collections::{HashMap, HashSet};

pub struct Game {
    pub(crate) world: World,
    pub(crate) movements: Vec<MovementTransition>,
    presentation_events: Vec<crate::model::PresentationEvent>,
}

pub(crate) struct DamageResult {
    pub(crate) remaining_hp: i32,
    pub(crate) max_hp: i32,
    pub(crate) downed: bool,
}

impl Game {
    pub fn from_documents(
        definitions: &str,
        map: &str,
        difficulty: authoring::Difficulty,
    ) -> Result<Self, GameError> {
        let definitions: authoring::Definitions = toml::from_str(definitions)
            .map_err(|e| error::definitions_toml_parse(e.to_string()))?;
        let map: authoring::Map =
            toml::from_str(map).map_err(|e| error::map_toml_parse(e.to_string()))?;
        Self::from_authoring(definitions, map, difficulty)
    }
    /// 委派共用作者輸入驗證與建局準備，執行期直接使用已驗證的世界。
    pub fn from_authoring(
        definitions: authoring::Definitions,
        map: authoring::Map,
        difficulty: authoring::Difficulty,
    ) -> Result<Self, GameError> {
        let authoring::validation::LoadedMap {
            world,
            presentation_events,
        } = authoring::validation::load_map(definitions, map, difficulty)?;
        Ok(Self {
            world,
            movements: Vec::new(),
            presentation_events,
        })
    }
    // 每次只執行一個命令或一段自動回合，讓畫面先完成呈現再推進。
    pub fn command(&mut self, c: Command) -> Result<Snapshot, GameError> {
        self.apply_command(c)?;
        Ok(self.take_snapshot(None))
    }
    /// 輸出狀態並取走待送事件；純查詢 snapshot 不消耗、也不重送事件。
    pub fn take_snapshot(&mut self, inspected_actor: Option<i64>) -> Snapshot {
        let mut snapshot = self.snapshot(inspected_actor);
        snapshot.presentation_events = std::mem::take(&mut self.presentation_events);
        snapshot.log = std::mem::take(&mut self.world.resource_mut::<Log>().0);
        snapshot.movements = std::mem::take(&mut self.movements);
        snapshot
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
                    .resource::<ActiveSkills>()
                    .definitions
                    .get(&skill)
                    .cloned()
                    .ok_or_else(|| error::unknown_skill(&skill))?;
                let attack = target_kind(&definition.effect) == ActiveSkillTargetKind::Enemy;
                let triggered_enemies = if attack
                    && self.world.resource::<Exploration>().mode == BattleMode::Exploring
                {
                    unit_at_cell(&self.world, GridPos { x, y })
                        .map(|target| self.connected_enemies(&[target]))
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };
                self.use_skill_at_cell(actor, GridPos { x, y }, definition)?;
                if attack && self.world.resource::<Exploration>().mode == BattleMode::Exploring {
                    self.world.resource_mut::<Exploration>().mode = BattleMode::AttackPending;
                    // 攻擊前先找出整批，讓第一擊倒地的敵人仍能觸發附近同伴。
                    let surviving_enemies: Vec<_> = triggered_enemies
                        .into_iter()
                        .filter(|id| self.entity(*id).is_some())
                        .collect();
                    self.world
                        .resource_mut::<Encounter>()
                        .participants
                        .extend(surviving_enemies);
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
    /// 先找出遭遇範圍內的敵人，再連鎖觸發附近一批。
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
            .map(|entity| entity.id())
            .collect();
        let enemies = self.connected_enemies(&enemies);
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
    /// 依敵人彼此距離擴展整個連通群，不以陣營識別碼決定參戰範圍。
    fn connected_enemies(&self, seeds: &[Entity]) -> Vec<i64> {
        let enemies: Vec<_> = self
            .world
            .iter_entities()
            .filter(|entity| {
                entity
                    .get::<Unit>()
                    .is_some_and(|unit| unit.team != Team::Player)
            })
            .map(|entity| entity.id())
            .collect();
        let mut connected: Vec<_> = seeds
            .iter()
            .copied()
            .filter(|entity| enemies.contains(entity))
            .collect();
        let mut visited: HashSet<_> = connected.iter().copied().collect();
        let mut cursor = 0;
        while cursor < connected.len() {
            let source = connected[cursor];
            for enemy in &enemies {
                if !visited.contains(enemy)
                    && entity_distance(&self.world, source, *enemy)
                        <= gameplay_config::ENCOUNTER_RANGE
                {
                    visited.insert(*enemy);
                    connected.push(*enemy);
                }
            }
            cursor += 1;
        }
        connected
            .into_iter()
            .map(|entity| self.world.get::<Id>(entity).expect("敵人應具有 Id").0)
            .collect()
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
        // 行動中的單位只存在 Turn，不在表中留副本。
        let next = players.iter().find_map(|id| turns.remove(id));
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
            .resource_mut::<Exploration>()
            .turns
            .remove(&actor)
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
                .resource_mut::<Exploration>()
                .turns
                .remove(id)
                .expect("剩餘單位應有回合");
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
        expire_temporary_terrains(&mut self.world, next_round);
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
        self.drop_cargo(entity, id);
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
        self.run_ai_turn()
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
    pub(crate) fn entity(&self, id: i64) -> Option<Entity> {
        self.world
            .iter_entities()
            .find(|e| e.get::<Id>().is_some_and(|i| i.0 == id))
            .map(|e| e.id())
    }
    fn outcome(&mut self) {
        use authoring::{ConditionMode, OutcomeCondition, OutcomeGroup};
        let crate::model::OutcomeRules { victory, defeat } =
            self.world.resource::<crate::model::OutcomeRules>();
        let matches = |OutcomeGroup { mode, conditions }: &OutcomeGroup| {
            let evaluate = |condition: &OutcomeCondition| match condition {
                OutcomeCondition::AllEnemiesDefeated => !self.world.iter_entities().any(|e| {
                    e.get::<Unit>()
                        .is_some_and(|u| matches!(u.team, Team::Enemy(_)))
                }),
                OutcomeCondition::TeamDefeated { team } => !self
                    .world
                    .iter_entities()
                    .any(|e| e.get::<Unit>().is_some_and(|unit| &unit.team == team)),
                OutcomeCondition::NoCargoCarried => self
                    .world
                    .get_resource::<crate::model::Cargo>()
                    .is_none_or(|cargo| {
                        !cargo
                            .items
                            .iter()
                            .any(|item| matches!(item, crate::model::CargoState::Carried(_)))
                    }),
                OutcomeCondition::CargoEscaped => self
                    .world
                    .get_resource::<crate::model::Cargo>()
                    .is_some_and(|cargo| cargo.items.contains(&crate::model::CargoState::Escaped)),
            };
            match mode {
                ConditionMode::All => conditions.iter().all(evaluate),
                ConditionMode::Any => conditions.iter().any(evaluate),
            }
        };
        let outcome = if matches(defeat) {
            Outcome::Defeat
        } else if matches(victory) {
            Outcome::Victory
        } else {
            Outcome::Ongoing
        };
        self.world.resource_mut::<ResultState>().0 = outcome;
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
                carrying_cargo: self
                    .world
                    .get_resource::<crate::model::Cargo>()
                    .is_some_and(|cargo| {
                        cargo
                            .items
                            .contains(&crate::model::CargoState::Carried(i.0))
                    }),
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
                hit: f.hit,
                physical_power: f.physical_power,
                magical_power: f.magical_power,
                block_reduction: f.block_reduction,
                equipment: f.equipment.clone(),
                active_skills: crate::skill::active_skill_views(&self.world, entity),
                passive_skills: crate::skill::passive_skill_views(&self.world, entity),
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
            let mut available: Vec<_> = turns.keys().copied().collect();
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
        let unit_cells: HashMap<GridPos, i64> = unit_cells(&self.world)
            .into_iter()
            .map(|(cell, entity)| {
                let id = self.world.get::<Id>(entity).expect("戰鬥單位應具有 Id").0;
                (cell, id)
            })
            .collect();
        let terrain_cells = (0..*height)
            .flat_map(|y| {
                let board = b;
                let world = &self.world;
                let grid = self.world.resource::<TerrainGrid>();
                let unit_cells = &unit_cells;
                (0..*width).map(move |x| {
                    let position = GridPos { x, y };
                    let terrains = terrains_at(world, position);
                    let cell = grid_cell(grid, position);
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
                                blocks_sight,
                                extra_movement_cost,
                                dodge_penalty,
                                block_penalty,
                            } = terrain_type(board, kind);
                            let values = TerrainDescriptionValues {
                                dodge_penalty: (*dodge_penalty > 0).then_some(*dodge_penalty),
                                block_penalty: (*block_penalty > 0).then_some(*block_penalty),
                                extra_movement_cost: (*extra_movement_cost > 0)
                                    .then_some(*extra_movement_cost),
                                remaining_rounds: temporary_remaining_rounds(world, position, kind),
                                damage: (*damage > 0).then_some(*damage),
                            };
                            TerrainDescriptionView {
                                terrain: kind.clone(),
                                entry_rule: *entry_rule,
                                blocks_sight: *blocks_sight,
                                values,
                            }
                        })
                        .collect();
                    TerrainCellView {
                        x,
                        y,
                        passable: cell.passable,
                        ground_visual: ground_at(board, &terrains)
                            .expect("terrains_at 必含 ground")
                            .clone(),
                        terrains,
                        unit_id: unit_cells.get(&position).copied(),
                        cost: cell.movement_cost,
                        damage,
                        effect_descriptions,
                    }
                })
            })
            .collect();
        Snapshot {
            cargo: self
                .world
                .get_resource::<crate::model::Cargo>()
                .map(|cargo| {
                    let crate::model::Cargo { items, exits } = cargo;
                    crate::model::CargoView {
                        items: items.clone(),
                        exits: exits.clone(),
                    }
                }),
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
                            blocks_movement: terrain_type(b, kind).entry_rule
                                != TerrainEntryRule::Walkable,
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
                    for kind in kinds.keys() {
                        // 與固定地形同種類時已由上方列出。
                        let remaining_rounds =
                            temporary_remaining_rounds(&self.world, *position, kind);
                        if remaining_rounds.is_none() {
                            continue;
                        }
                        effects.push(TerrainEffectView {
                            blocks_movement: terrain_type(b, kind).entry_rule
                                != TerrainEntryRule::Walkable,
                            x: position.x,
                            y: position.y,
                            damage: terrain_damage(b, kind),
                            visual: kind.clone(),
                            effect: kind.clone(),
                            remaining_rounds,
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
            // 純查詢不重送事件；載入與 command 透過 take_snapshot 取走待送內容。
            log: Vec::new(),
            movements: Vec::new(),
            presentation_events: Vec::new(),
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
